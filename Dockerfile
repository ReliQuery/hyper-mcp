#######################
# Step 1: Build stage #
#######################
FROM --platform=$BUILDPLATFORM rust:1.89 AS builder
WORKDIR /app
RUN cargo install cargo-auditable

COPY Cargo.toml Cargo.lock ./
RUN cargo fetch
COPY src ./src
RUN cargo auditable build --release --locked

#######################
# Step 3: Certs stage #
#######################
FROM debian:bullseye-slim AS certs
RUN apt-get update && apt-get install -y ca-certificates

#######################
# Step 3: Final stage #
#######################
FROM scratch

LABEL org.opencontainers.image.authors="me@tuananh.org" \
    org.opencontainers.image.url="https://github.com/tuananh/hyper-mcp" \
    org.opencontainers.image.source="https://github.com/tuananh/hyper-mcp" \
    org.opencontainers.image.vendor="github.com/tuananh/hyper-mcp"

# Copy the statically compiled binary from the builder stage
COPY --from=builder /app/target/release/hyper-mcp /hyper-mcp

# Copy CA certificates from a trusted source
COPY --from=certs /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/

# Set the binary as the entrypoint
ENTRYPOINT ["/hyper-mcp"]
