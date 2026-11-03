# Build stage
FROM rust:1.96-slim AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

# Final stage
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y libssl3 ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /root/
COPY --from=builder /app/target/release/enterprise-coin-flip /usr/local/bin/enterprise-coin-flip
EXPOSE 8080
CMD ["enterprise-coin-flip"]
