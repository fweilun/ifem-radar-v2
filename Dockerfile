FROM docker.io/library/rust:1-bookworm as builder

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src
COPY migrations ./migrations

# Copy sqlx-data.json. 
# We use a wildcard or exact match. If it doesn't exist, this fails.
COPY .sqlx ./.sqlx

# Build with SQLX_OFFLINE=true
ENV SQLX_OFFLINE=true
RUN cargo build --release --bin ifem-radar-v2

# Runtime stage
FROM docker.io/library/ubuntu:24.04

WORKDIR /app

# Install dependencies
RUN apt-get update && \
    apt-get install -y ca-certificates libssl3 && \
    rm -rf /var/lib/apt/lists/*

# Copy binary
COPY --from=builder /app/target/release/ifem-radar-v2 .

# Expose port
EXPOSE 8080

CMD ["./ifem-radar-v2"]
