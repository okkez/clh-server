FROM rust:1.94.0 as build

RUN apt-get update && apt-get install -qq -y libpq-dev && apt-get clean

RUN USER=root cargo new --bin app
WORKDIR /app

COPY ./Cargo.toml ./Cargo.toml
RUN cargo build -j 4 --release
RUN curl -sSL https://github.com/diesel-rs/diesel/releases/download/v2.3.7/diesel_cli-installer.sh | sh
RUN rm -rf ./src

COPY ./src ./src
COPY ./migrations ./migrations
COPY ./diesel.toml ./diesel.toml
COPY ./setup-db.sh ./setup-db.sh

RUN cargo clean && cargo build -j 4 --release

FROM debian:bookworm-slim

WORKDIR /app
RUN apt-get update && apt-get install -qq -y libpq-dev && apt-get clean

COPY --from=build /app/target/release/clh-server .
COPY --from=build /app/diesel.toml .
