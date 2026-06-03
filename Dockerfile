# Stage 1: Frontend build
FROM node:20-alpine AS frontend
WORKDIR /app/frontend
COPY frontend/package*.json ./
RUN npm ci --quiet
COPY frontend/ ./
RUN npm run build

# Stage 2: Rust build
FROM rust:1.85-alpine AS builder
RUN apk add --no-cache musl-dev
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
# Copy frontend build output
COPY --from=frontend /app/crates/web-api/src/dist crates/web-api/src/dist
RUN cargo build --release --bin dns-filter --bin ars-convert

# Stage 3: Minimal runtime image
FROM alpine:3.19
RUN apk add --no-cache ca-certificates
WORKDIR /app

COPY --from=builder /app/target/release/dns-filter   /usr/local/bin/dns-filter
COPY --from=builder /app/target/release/ars-convert  /usr/local/bin/ars-convert
COPY crates/dns-server/config.example.json           /etc/dns-filter/config.json

# DNS port + Web UI port
EXPOSE 53/udp 53/tcp 3000/tcp

VOLUME ["/etc/dns-filter", "/etc/dns-filter/rules"]

ENTRYPOINT ["dns-filter", "--config", "/etc/dns-filter/config.json"]
