#######################
# Step 1: Build stage #
#######################
FROM --platform=$BUILDPLATFORM clux/muslrust:1.89.0-stable AS builder
WORKDIR /app

# Install cargo-auditable for reproducible builds
RUN cargo install cargo-auditable

# Cache dependencies
COPY Cargo.toml Cargo.lock ./
RUN cargo fetch

# Copy source code
COPY src ./src

# Map Docker arch -> Rust target triple
ARG TARGETARCH
ENV RUSTFLAGS="-C target-feature=+crt-static"
RUN set -eux; \
    case "$TARGETARCH" in \
    amd64)  TARGET_TRIPLE=x86_64-unknown-linux-musl ;; \
    arm64)  TARGET_TRIPLE=aarch64-unknown-linux-musl ;; \
    *) echo "unsupported arch: $TARGETARCH" && exit 1 ;; \
    esac; \
    rustup target add "$TARGET_TRIPLE"; \
    cargo auditable build --release --locked --target "$TARGET_TRIPLE"; \
    # Put the final binary in a stable path regardless of target triple
    mkdir -p /out && cp "target/$TARGET_TRIPLE/release/hyper-mcp" /out/hyper-mcp

#######################
# Step 3: Certs stage #
#######################
FROM alpine:latest AS certs
RUN apk add --no-cache ca-certificates

#######################
# Step 3: Final stage #
#######################
FROM  scratch

LABEL org.opencontainers.image.authors="me@tuananh.org" \
    org.opencontainers.image.url="https://github.com/tuananh/hyper-mcp" \
    org.opencontainers.image.source="https://github.com/tuananh/hyper-mcp" \
    org.opencontainers.image.vendor="github.com/tuananh/hyper-mcp" \
    org.opencontainers.image.description="Hyper-MCP server"

# Run as an unprivileged numeric user (no /etc/passwd on scratch)
USER 65532:65532

# Copy CA certificates from a trusted source
COPY --from=certs /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/

# Optional: make the CA path explicit for OpenSSL-based stacks
ENV SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt

# Copy the statically compiled binary from the builder stage
COPY --from=builder --chown=65532:65532 --chmod=0755 /out/hyper-mcp /hyper-mcp

# Set the binary as the entrypoint
ENTRYPOINT ["/hyper-mcp"]
