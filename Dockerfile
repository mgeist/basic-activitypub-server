# BUILDER
FROM rust:1.65.0 AS builder

WORKDIR /app

COPY Cargo* ./
COPY src/ src/

RUN cargo build --bin server --release

# FINAL IMAGE
FROM debian:buster-slim

ENV AP_USER=first-test
ENV AP_DOMAIN=wispy-violet-1010.fly.dev

EXPOSE 8080/tcp

WORKDIR /app

COPY --from=builder /app/target/release/server /app/server

ENTRYPOINT ["./server"]