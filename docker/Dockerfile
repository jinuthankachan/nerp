# syntax=docker/dockerfile:1
#
# nERP production image — single, self-contained image.
# Build from the repo root:   docker build -f docker/Dockerfile -t nerp .
#
# Multi-stage: Node compiles the frontend, Rust compiles the binary, and the
# final runtime image carries ONLY the binary + assets + config (no Node, no
# node_modules, no cargo).

# ── Stage 1: build frontend assets with Vite ─────────────────────────────────
FROM node:24-bookworm-slim AS assets
WORKDIR /app
COPY package.json package-lock.json ./
RUN npm ci
COPY vite.config.mjs ./
COPY frontend ./frontend
RUN npm run build
# → /app/assets/static/dist/{app.css,app.js,tabler-icons.*}

# ── Stage 2: build the Rust binary ───────────────────────────────────────────
FROM rust:1-bookworm AS build
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY migration ./migration
COPY src ./src
COPY config ./config
COPY assets ./assets
# overlay the Vite-built assets from stage 1
COPY --from=assets /app/assets/static/dist ./assets/static/dist
RUN cargo build --release --bin nerp-cli

# ── Stage 3: slim runtime ────────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime
WORKDIR /app
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates \
 && rm -rf /var/lib/apt/lists/*
COPY --from=build /app/target/release/nerp-cli /usr/local/bin/nerp
COPY --from=build /app/assets ./assets
COPY --from=build /app/config ./config

# loco loads config/<LOCO_ENV>.yaml. config/production.yaml is gitignored (it
# holds secrets) — supply it at deploy (mounted volume or a derived image), and
# inject values like DATABASE_URL via env (the config reads env vars).
ENV LOCO_ENV=production
EXPOSE 5150
CMD ["nerp", "start"]
