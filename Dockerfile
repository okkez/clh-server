FROM rust:1.94.0 as build

RUN apt-get update && apt-get install -qq -y libpq-dev && apt-get clean

RUN USER=root cargo new --bin app
WORKDIR /app

COPY ./Cargo.toml ./Cargo.toml
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo build -j 4 --release
RUN curl -sSL https://github.com/diesel-rs/diesel/releases/download/v2.3.7/diesel_cli-installer.sh | sh
RUN rm -rf ./src

COPY ./src ./src
COPY ./migrations ./migrations
COPY ./diesel.toml ./diesel.toml
COPY ./setup-db.sh ./setup-db.sh

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo build -j 4 --release && \
    cp /app/target/release/clh-server /tmp/clh-server

RUN mv /tmp/clh-server /app/target/release/clh-server

FROM debian:bookworm-slim

WORKDIR /app
RUN apt-get update && apt-get install -qq -y libpq-dev && apt-get clean

COPY --from=build /app/target/release/clh-server .
COPY --from=build /app/diesel.toml .
