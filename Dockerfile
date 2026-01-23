FROM rust:latest as builder

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
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install dependencies
RUN apt-get update && apt-get install -y libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*

# Copy binary
COPY --from=builder /app/target/release/ifem-radar-v2 .

# Expose port
EXPOSE 8080

CMD ["./ifem-radar-v2"]
