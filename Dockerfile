FROM rust:1.78.0 as build

RUN apt-get update && apt-get install -qq -y libpq-dev && apt-get clean

RUN USER=root cargo new --bin app
WORKDIR /app

COPY ./Cargo.toml ./Cargo.toml
RUN cargo build -j 4 --release
RUN rm -rf ./src

COPY ./src ./src
COPY ./migrations ./migrations
COPY ./diesel.toml ./diesel.toml

RUN cargo clean && cargo build -j 4 --release

FROM debian:bookworm-slim

WORKDIR /app
RUN apt-get update && apt-get install -qq -y libpq-dev && apt-get clean

COPY --from=build /app/target/release/clh-server .
COPY --from=build /app/diesel.toml .
