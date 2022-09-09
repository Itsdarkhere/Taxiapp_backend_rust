#[macro_use] extern crate rocket;
use pwhash::{sha512_crypt, HashSetup};
use rocket::serde::{Deserialize, json::Json, Serialize};
use rocket_db_pools::{sqlx, Database, Connection, sqlx::Row};


// https://api.rocket.rs/master/rocket_db_pools/#configuration
#[derive(Database)]
#[database("my_postgres")]
struct MyPg(sqlx::PgPool);

// Destructure login/signup requests into this struct
#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
struct Login {
    username: String,
    password: String, 
}

// This is used for adding addresses to the db
// Added login so that If I wanted to I could check that both username and password exist 
// B4 adding an address, but since this is not going to production I only care about username
#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
struct AddAddress {
    login: Login,
    address: String,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
struct Addresses {
    username: String,
    addresses: Vec<String>
}

// Plain success: false/true is good enough for most apis
#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
struct GenericResponse {
    success: bool,
}

// Rocket server is async, functions ( like this one ) might need some special treatment
// But this is not meant for production so I dont mind
/// Checks the login struct for empty values
fn empty(login: &Login) -> bool {
    // Check username is not empty
    if login.username == "" {
        return true
    } 

    // Check password is not empty
    if login.password == "" {
        return true
    }

    false
}

/// Verify's the users login info
#[get("/", format = "json", data = "<login>")]
async fn login(mut db: Connection<MyPg>, login: Json<Login>) -> Json<GenericResponse> {
    if empty(&login) {
        return Json(GenericResponse { success: false })
    }

    // Set hash salt to 10
    // https://en.wikipedia.org/wiki/Salt_(cryptography)
    let hash_setup = HashSetup {
        salt: Some("10"), 
        rounds: None,
    };

    // Hash using sha512 + salt
    let hashed_password = match sha512_crypt::hash_with(hash_setup, login.password.clone()) {
        Ok(pass) => pass,
        Err(_) => return Json(GenericResponse { success: false })
    };
    
    let row = sqlx::query(
        format!("SELECT username, password FROM login WHERE username = '{}' AND password = '{}'", login.username, hashed_password).as_str())
        .fetch_one(&mut *db).await;

    // If this errors, the password&username combo does not exist
    // Or database is down, but yeah
    match row {
        Ok(_) => return Json(GenericResponse { success: true }),
        Err(_) => return Json(GenericResponse { success: false }),
    };
}

/// Creates a new user profile
#[get("/", format = "json", data = "<login>")]
async fn signup(mut db: Connection<MyPg>, login: Json<Login>) -> Json<GenericResponse> {
    if empty(&login) {
        return Json(GenericResponse { success: false })
    }

    // Set hash salt to 10
    // https://en.wikipedia.org/wiki/Salt_(cryptography)
    let hash_setup = HashSetup {
        salt: Some("10"), 
        rounds: None,
    };

    // Hash using sha512 + salt
    let hashed_password = match sha512_crypt::hash_with(hash_setup, login.password.clone()) {
        Ok(pass) => pass,
        Err(_) => return Json(GenericResponse { success: false })
    };

    // username is a primaryKey, so if there is a duplicate we will error / modify nothing
    let row = sqlx::query(
        format!("INSERT INTO login (username, password) VALUES ('{}', '{}')", 
        login.username, hashed_password).as_str())
        .execute(&mut *db).await;

    match row {
        Ok(_) => return Json(GenericResponse { success: true }),
        Err(_) => return Json(GenericResponse { success: false }),
    };
}

// Nothing here safeguards from someone just fetching another users regular addresses
/// Gets top 5 most used addresses
#[get("/", format = "json", data = "<login>")]
async fn get_regular_addresses(mut db: Connection<MyPg>, login: Json<Login>) -> Json<Addresses> {

    let username = login.username.clone();
    // Check username is not empty
    if username == "" {
        return Json(Addresses { username, addresses: vec![] })
    } 

    // username is a primaryKey, so if there is a duplicate we will error / modify nothing
    let rows = sqlx::query(
        format!("SELECT address FROM addresses WHERE username = '{}' ORDER BY address_count LIMIT 5", username.clone()).as_str()
        ).fetch_all(&mut *db).await;

    match rows {
        Ok(addresses) => {
            let address_vec = addresses.iter().map(|v| v.get(0)).collect();
            return Json(Addresses { username, addresses: address_vec })
        },
        Err(_) => return Json(Addresses { username, addresses: vec![] }),
    };
}

/// Add address to the address table / increment the count of the address
#[post("/", format = "json", data = "<address_login>")]
async fn add_address(mut db: Connection<MyPg>, address_login: Json<AddAddress>) -> Json<GenericResponse> {
    
    // Checks we have something in both login related fields
    if empty(&address_login.login) {
        return Json(GenericResponse { success: false });
    }

    if address_login.address == "" {
        return Json(GenericResponse { success: false });
    }

    // address is a primaryKey, so if  there is a conflict, we just add to address_count
    let executed = sqlx::query(
        format!("INSERT INTO addresses
        VALUES ('{}', '{}', 1)
        ON CONFLICT (address)
        DO
        UPDATE
        SET address_count = addresses.address_count + 1;", address_login.login.username, address_login.address).as_str()
        ).execute(&mut *db).await;

    // Error is if no rows were affected, should not happen 
    // Unless db is completely fucked for some reason
    match executed {
        Ok(_) => return  Json(GenericResponse { success: true }),
        Err(_) =>  return Json(GenericResponse { success: false })
    };



}


/// Builds & runs the server
#[launch]
fn rocket() -> _ {
    rocket::build().attach(MyPg::init())
        .mount("/login", routes!(login))
        .mount("/signup", routes!(signup))
        .mount("/get_regular_routes", routes![get_regular_addresses])
        .mount("/add_address", routes![add_address])
        // If request does not include password/username fields:
        // Request errors with code: 422 Unprocessable Entity
}

