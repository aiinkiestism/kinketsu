# syntax=docker/dockerfile:1.7

# ---- Rust build ----
FROM rust:1.93-bookworm AS rust
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
# app/src-tauri is a workspace member but isn't built for the server image.
# Stub it out so cargo doesn't fail to resolve the workspace.
RUN mkdir -p app/src-tauri/src && \
    echo '[package]\nname = "kinketsu-app"\nversion = "0.0.0"\nedition = "2024"' > app/src-tauri/Cargo.toml && \
    echo 'fn main() {}' > app/src-tauri/src/main.rs
RUN cargo build --release -p kinketsu-server

# ---- runtime ----
FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=rust /src/target/release/kinketsu-server /usr/local/bin/kinketsu-server
ENV KINKETSU_DB=/data/kinketsu.db \
    KINKETSU_BIND=0.0.0.0:3000 \
    RUST_LOG=info
VOLUME ["/data"]
EXPOSE 3000
ENTRYPOINT ["/usr/local/bin/kinketsu-server"]
