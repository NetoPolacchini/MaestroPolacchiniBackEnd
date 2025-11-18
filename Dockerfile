# --- Estágio de Build ---
FROM rust:latest as builder
WORKDIR /usr/src/app

COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release

# Apague o binário dummy e o src dummy
RUN rm -f /usr/src/app/target/release/backend
RUN rm -rf src

# Agora copie o src real e recompile
COPY src ./src
COPY migrations ./migrations
COPY .sqlx ./.sqlx
# Esta etapa agora IRÁ construir o binário real
RUN cargo build --release

# --- Estágio Final ---
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y openssl ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /usr/src/app
COPY --from=builder /usr/src/app/target/release/backend .
EXPOSE 3000
CMD ["./backend"]