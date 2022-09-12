FROM rust:1 as builder
WORKDIR /tconbackend
COPY . .
COPY ./Rocket.toml ./Rocket.toml
RUN cargo install --path .


FROM debian:buster-slim as runner
COPY --from=builder /usr/local/cargo/bin/tconbackend /usr/local/bin/tconbackend
COPY ./Rocket.toml ./Rocket.toml
ENV ROCKET_ADDRESS=0.0.0.0
EXPOSE 8000
CMD ["tconbackend"]