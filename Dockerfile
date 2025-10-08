# Build stage
FROM rust:1.75 as builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/

# Build the application
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install CA certificates for HTTPS
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary from builder stage
COPY --from=builder /app/target/release/kubernetes-resource-app /app/myapp-controller

# Make it executable
RUN chmod +x /app/myapp-controller

# Create non-root user
RUN useradd -r -u 1000 controller
USER controller

ENTRYPOINT ["/app/myapp-controller"]