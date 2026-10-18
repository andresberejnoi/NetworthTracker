# syntax=docker/dockerfile:1

# --- Build stage -------------------------------------------------------------
# Built from the repository root so the shared `docs/` guide that the domain
# crate includes as doctests is available. The `backend/` level is preserved so
# the crate-relative include paths resolve exactly as they do locally.
FROM rust:1.95-slim AS build

WORKDIR /usr/src

# Copy manifests first so dependency compilation is cached independently of
# source changes.
COPY backend/Cargo.toml backend/Cargo.lock* ./backend/
COPY backend/crates/domain/Cargo.toml ./backend/crates/domain/
COPY backend/crates/application/Cargo.toml ./backend/crates/application/
COPY backend/crates/adapters/Cargo.toml ./backend/crates/adapters/
COPY backend/crates/api/Cargo.toml ./backend/crates/api/

RUN cd backend \
    && mkdir -p crates/domain/src crates/application/src crates/adapters/src crates/api/src \
    && echo '' > crates/domain/src/lib.rs \
    && echo '' > crates/application/src/lib.rs \
    && echo '' > crates/adapters/src/lib.rs \
    && echo 'fn main() {}' > crates/api/src/main.rs \
    && echo '' > crates/api/src/lib.rs \
    && cargo build --release --workspace \
    && rm -rf crates/*/src

COPY backend/crates ./backend/crates
# `sqlx::migrate!` embeds the migrations at compile time, so they must be
# present before building the real sources.
COPY backend/migrations ./backend/migrations
COPY docs ./docs

# Touch sources so cargo rebuilds them after the stub build above.
RUN cd backend \
    && touch crates/*/src/*.rs crates/api/src/main.rs \
    && cargo build --release -p networth-api

# --- Runtime stage -----------------------------------------------------------
FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=build /usr/src/backend/target/release/networth-api /usr/local/bin/networth-api

ENV BACKEND_ADDR=0.0.0.0:8080
EXPOSE 8080

CMD ["networth-api"]
