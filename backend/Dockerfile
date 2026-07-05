FROM rust:1.92-slim AS builder
WORKDIR /app
COPY Cargo.toml .
COPY src/ src/
ENV DATABASE_URL=postgres://ajna:ajna@host.docker.internal:5432/ajna_verify
RUN apt-get update && apt-get install -y pkg-config libssl-dev clang libclang-dev && rm -rf /var/lib/apt/lists/*
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/ajna-verify-backend /usr/local/bin/
EXPOSE 8080
CMD ["ajna-verify-backend"]
