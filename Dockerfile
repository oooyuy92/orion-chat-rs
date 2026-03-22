# Stage 1: Frontend Build
FROM node:20-alpine AS frontend-builder

WORKDIR /app

# Copy package files for dependency installation
COPY package.json package-lock.json ./

# Install dependencies
RUN npm ci

# Copy frontend source code
COPY src ./src
COPY static ./static
COPY svelte.config.js tsconfig.json vite.config.ts ./
COPY tailwind.config.ts ./

# Build frontend
RUN npm run build

# Stage 2: Backend Build
FROM rust:1.75-alpine AS backend-builder

WORKDIR /app

# Install build dependencies
RUN apk add --no-cache musl-dev openssl-dev pkgconfig

# Copy Cargo files for dependency caching
COPY src-tauri/Cargo.toml src-tauri/Cargo.lock ./

# Create dummy source to cache dependencies
RUN mkdir -p src/bin && \
    echo "fn main() {}" > src/bin/web_server.rs && \
    mkdir -p src && \
    echo "pub fn lib_main() {}" > src/lib.rs && \
    cargo build --release --bin web_server && \
    rm -rf src

# Copy actual source code
COPY src-tauri/src ./src
COPY src-tauri/build.rs ./

# Build the web_server binary
RUN cargo build --release --bin web_server

# Stage 3: Runtime Image
FROM alpine:latest

WORKDIR /app

# Install runtime dependencies
RUN apk add --no-cache libgcc openssl ca-certificates

# Copy frontend build artifacts from Stage 1
COPY --from=frontend-builder /app/build ./static

# Copy web_server binary from Stage 2
COPY --from=backend-builder /app/target/release/web_server /usr/local/bin/web_server

# Set environment variables
ENV DATABASE_PATH=/data/orion.db \
    DATA_DIR=/data \
    STATIC_DIR=/app/static \
    PORT=28080

# Create data directory
RUN mkdir -p /data

# Expose port
EXPOSE 28080

# Set the entrypoint
CMD ["/usr/local/bin/web_server"]
