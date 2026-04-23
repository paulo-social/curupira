FROM rust:1.91 AS builder
WORKDIR /app

COPY Cargo.toml Cargo.toml
COPY edge-sentinel/Cargo.toml edge-sentinel/Cargo.toml
COPY backend-api backend-api
RUN mkdir -p edge-sentinel/src && echo "fn main() {}" > edge-sentinel/src/main.rs
RUN cargo build -p backend-api --release

FROM debian:bookworm-slim
WORKDIR /app
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/backend-api /usr/local/bin/backend-api
COPY backend-api/model.onnx /app/model.onnx
RUN mkdir -p /app/data

ENV DATABASE_URL=sqlite://data/alerts.db
ENV MODEL_PATH=/app/model.onnx
EXPOSE 8080

CMD ["backend-api"]
