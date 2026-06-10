# Build stage
FROM rust:1.96-slim AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

# Final stage
FROM debian:bookworm-slim
WORKDIR /root/
COPY --from=builder /app/target/release/rust /usr/local/bin/rust
EXPOSE 8080
CMD ["rust"]
