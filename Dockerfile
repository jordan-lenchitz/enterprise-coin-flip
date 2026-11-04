# Build stage
FROM rust:1.96-slim AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

# Final stage (hardened non-root runtime)
FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends libssl3 ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --system --gid 10001 appuser \
    && useradd --system --uid 10001 --gid appuser --no-create-home --shell /usr/sbin/nologin appuser
WORKDIR /app
COPY --from=builder --chown=appuser:appuser /app/target/release/enterprise-coin-flip /usr/local/bin/enterprise-coin-flip
USER appuser:appuser
EXPOSE 8080
CMD ["enterprise-coin-flip"]
