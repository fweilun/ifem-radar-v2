FROM docker.io/library/rust:1-bookworm as builder

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src
COPY scripts ./scripts
COPY migrations ./migrations

# NOTE:
# This project currently uses runtime sqlx::query (no compile-time checked macros),
# so we don't require a .sqlx directory for Docker builds.
RUN cargo build --release --bin ifem-radar-v2

# Runtime stage
FROM docker.io/library/ubuntu:24.04

WORKDIR /app

# Install dependencies
RUN apt-get update && \
    apt-get install -y ca-certificates libssl3 curl && \
    rm -rf /var/lib/apt/lists/*

# Copy binary
COPY --from=builder /app/target/release/ifem-radar-v2 .

# Expose port
EXPOSE 8080

CMD ["./ifem-radar-v2"]
