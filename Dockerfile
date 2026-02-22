# Build stage
FROM rust:1.93-bookworm AS builder

WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates/openat-common/Cargo.toml ./crates/openat-common/
COPY crates/openat-cli/Cargo.toml ./crates/openat-cli/
COPY crates/openat-agent/Cargo.toml ./crates/openat-agent/
COPY crates/openat-channels/Cargo.toml ./crates/openat-channels/
COPY crates/openat-config/Cargo.toml ./crates/openat-config/
COPY crates/openat-runtime/Cargo.toml ./crates/openat-runtime/
COPY crates/openat-types/Cargo.toml ./crates/openat-types/
COPY crates/openat-tools/Cargo.toml ./crates/openat-tools/
COPY crates/openat-providers/Cargo.toml ./crates/openat-providers/

# Copy source code
COPY crates/openat-common/src ./crates/openat-common/src
COPY crates/openat-cli/src ./crates/openat-cli/src
COPY crates/openat-agent/src ./crates/openat-agent/src
COPY crates/openat-channels/src ./crates/openat-channels/src
COPY crates/openat-config/src ./crates/openat-config/src
COPY crates/openat-runtime/src ./crates/openat-runtime/src
COPY crates/openat-types/src ./crates/openat-types/src
COPY crates/openat-tools/src ./crates/openat-tools/src
COPY crates/openat-providers/src ./crates/openat-providers/src

# Build the release binary
RUN cargo build --release -p openat-cli

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
COPY --from=builder /app/target/release/openat-cli /usr/local/bin/openat

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
