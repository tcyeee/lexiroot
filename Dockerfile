# Multi-stage build for the LexiRoot web demo server.
#
# The server is dependency-free std-only HTTP; the only heavy dependency is
# rusqlite (bundled), which compiles SQLite from C — so the build stage needs a
# C toolchain (present in the full `rust` image). The release database is baked
# into the image so the container is fully self-contained.

FROM rust:1-bookworm AS build
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
# --locked fails the build on a stale Cargo.lock rather than quietly resolving
# to newer versions, so the image matches what was tested locally.
RUN cargo build --release --locked -p lexiroot-web

FROM debian:bookworm-slim AS runtime
WORKDIR /app
COPY --from=build /app/target/release/lexiroot-web /usr/local/bin/lexiroot-web
COPY data/dist/lexiroot.sqlite /app/data/dist/lexiroot.sqlite
EXPOSE 8080
# Bind all interfaces inside the container; TLS and exposure are handled by the
# fronting nginx reverse proxy, never by this process directly.
CMD ["lexiroot-web", "--host", "0.0.0.0", "--port", "8080", "--db", "/app/data/dist/lexiroot.sqlite"]
