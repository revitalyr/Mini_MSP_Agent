FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY target/release/agent /usr/local/bin/agent

ENTRYPOINT ["agent"]
CMD ["--config", "/app/config.toml"]
