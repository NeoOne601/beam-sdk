# Ajna verify backend. Build context is the REPO ROOT (see docker-compose.yml /
# deploy configs) because the backend has a path dependency on
# crates/ajna-crypto and uses the committed .sqlx offline query cache.
FROM rust:1.92-slim AS builder
WORKDIR /app
RUN apt-get update \
 && apt-get install -y pkg-config libssl-dev clang libclang-dev \
 && rm -rf /var/lib/apt/lists/*

# Only the crates the backend actually needs, plus the workspace manifest and
# the offline sqlx cache so `sqlx::query!` macros compile without a live DB.
COPY Cargo.toml Cargo.lock ./
COPY .sqlx/ .sqlx/
COPY backend/ backend/
COPY core/ core/
COPY core/fuzz/ core/fuzz/
COPY crates/ crates/

# Offline sqlx: compile against the checked-in query metadata, no DB at build.
ENV SQLX_OFFLINE=true
RUN cargo build --release -p ajna-verify-backend

FROM debian:bookworm-slim
RUN apt-get update \
 && apt-get install -y ca-certificates libssl3 \
 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/ajna-verify-backend /usr/local/bin/
EXPOSE 8080
CMD ["ajna-verify-backend"]
