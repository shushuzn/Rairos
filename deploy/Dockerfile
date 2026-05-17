FROM rust:1.75 as builder

WORKDIR /app

RUN apt-get update && apt-get install -y \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY crates/rairos-api-gateway ./crates/rairos-api-gateway

RUN mkdir -p .cargo && \
    echo '[target.x86_64-unknown-linux-gnu]' >> .cargo/config.toml && \
    echo 'linker = "gcc"' >> .cargo/config.toml && \
    echo 'rustflags = "-C target-feature=-crt-static"' >> .cargo/config.toml

RUN cargo build --release -p rairos-api-gateway

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    libpq5 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/rairos-api-gateway /usr/local/bin/

ENV DATABASE_URL=postgres://postgres:postgres@localhost:5432/rairos_api
ENV RUST_LOG=info
ENV ADDR=0.0.0.0:8081

EXPOSE 8081

CMD ["rairos-api-gateway"]
