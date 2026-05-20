FROM rust:1-bookworm AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY migrations ./migrations
COPY src ./src
COPY tests ./tests

RUN cargo build --release --bin memorynexus

FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/memorynexus /usr/local/bin/memorynexus

EXPOSE 8080

CMD ["memorynexus"]
