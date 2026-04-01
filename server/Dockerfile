FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY target/release/server /usr/local/bin/server

EXPOSE 8080

CMD ["server", "--port", "8080"]
