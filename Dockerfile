# Build stage
FROM rust:1.93-bookworm AS builder

WORKDIR /app

# Copy dependency files first for better caching
COPY Cargo.toml Cargo.lock ./
COPY ./src ./src

# Build the release binary
RUN cargo build --release --locked

# Runtime stage
FROM debian:bookworm

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

# Create non-root user for security
RUN useradd -m -s /bin/bash openat

# Copy the binary from builder
COPY --from=builder /app/target/release/openat /usr/local/bin/openat

# Set ownership
RUN chown -R openat:openat /usr/local/bin/openat

# Switch to non-root user
USER openat

# Set entrypoint
ENTRYPOINT ["openat"]

# Default port (can be overridden via docker run -p)
EXPOSE 18790

# Environment variables with defaults
ENV RUST_LOG=openat=info
ENV RUST_BACKTRACE=0
