# syntax=docker/dockerfile:1
# Build context is the repo root (the Cargo workspace lives there):
#   docker build -f apps/api/Dockerfile -t voclara-api .

FROM rust:1-bookworm AS builder
WORKDIR /src
COPY Cargo.toml Cargo.lock* clippy.toml ./
COPY apps/api apps/api
# Prompts are embedded at compile time (include_str!).
COPY packages/prompts packages/prompts
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release -p voclara-api \
 && cp target/release/voclara-api /voclara-api

# Distroless: no shell, no package manager, runs as non-root.
FROM gcr.io/distroless/cc-debian12:nonroot
COPY --from=builder /voclara-api /app/voclara-api
ENV BIND_ADDR=0.0.0.0:8080 \
    LOG_FORMAT=json
EXPOSE 8080
USER nonroot
ENTRYPOINT ["/app/voclara-api"]
