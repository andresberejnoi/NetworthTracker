# --- Build stage -------------------------------------------------------------
FROM rust:1.95-slim AS build

WORKDIR /usr/src/app

# Copy manifests first so dependency compilation is cached independently of
# source changes.
COPY Cargo.toml Cargo.lock* ./
COPY crates/domain/Cargo.toml crates/domain/
COPY crates/application/Cargo.toml crates/application/
COPY crates/adapters/Cargo.toml crates/adapters/
COPY crates/api/Cargo.toml crates/api/
RUN mkdir -p crates/domain/src crates/application/src crates/adapters/src crates/api/src \
    && echo '' > crates/domain/src/lib.rs \
    && echo '' > crates/application/src/lib.rs \
    && echo '' > crates/adapters/src/lib.rs \
    && echo 'fn main() {}' > crates/api/src/main.rs \
    && echo '' > crates/api/src/lib.rs \
    && cargo build --release --workspace \
    && rm -rf crates/*/src

COPY crates ./crates
# Touch sources so cargo rebuilds them after the stub build above.
RUN touch crates/*/src/*.rs crates/api/src/main.rs \
    && cargo build --release -p networth-api

# --- Runtime stage -----------------------------------------------------------
FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=build /usr/src/app/target/release/networth-api /usr/local/bin/networth-api

ENV BACKEND_ADDR=0.0.0.0:8080
EXPOSE 8080

CMD ["networth-api"]
