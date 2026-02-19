FROM rust:1.75-slim-bookworm as builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY examples ./examples
COPY migrations ./migrations

RUN cargo build --example server --features dashboard --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/examples/server /usr/local/bin/goldfish-server
COPY --from=builder /app/migrations ./migrations

VOLUME /app/goldfish_server_data

EXPOSE 3000

CMD ["goldfish-server"]
