# Multi-stage build for optimized production image
FROM rust:1.83-bookworm as builder

# Enable nightly feature for latest crates
RUN rustup install nightly && rustup default nightly

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Create app user for build
RUN useradd --create-home --shell /bin/bash app
USER app
WORKDIR /home/app

# Copy dependency files first for better layer caching
COPY --chown=app:app Cargo.toml ./

# Create a dummy src/main.rs to build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies (this layer will be cached)
RUN cargo build --release
RUN rm src/main.rs

# Copy the actual source code and migrations
COPY --chown=app:app src ./src
COPY --chown=app:app migrations ./migrations

# Build the application
RUN cargo build --release

# Production stage - minimal runtime image
FROM debian:bookworm-slim

# Install runtime dependencies and create app user
RUN apt-get update && apt-get install -y \
    ca-certificates \
    sqlite3 \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --shell /bin/bash --uid 1001 app

# Switch to app user
USER app
WORKDIR /home/app

# Create directories for application data
RUN mkdir -p /home/app/data /home/app/static /home/app/templates /home/app/migrations

# Copy the binary from builder stage
COPY --from=builder --chown=app:app /home/app/target/release/tobelog /usr/local/bin/tobelog

# Copy runtime assets
COPY --chown=app:app static/ ./static/
COPY --chown=app:app templates/ ./templates/
COPY --chown=app:app migrations/ ./migrations/

# Set environment variables
ENV RUST_LOG=info
ENV DATABASE_URL=sqlite:///home/app/data/blog.db
ENV SERVER_HOST=0.0.0.0
ENV SERVER_PORT=3000

# Expose port
EXPOSE 3000

# Install curl for health checks
USER root
RUN apt-get update && apt-get install -y curl && rm -rf /var/lib/apt/lists/*
USER app

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3000/health || exit 1

# Run the application
CMD ["/usr/local/bin/tobelog"]