# Builds kamichain-node and kamichain-bridge into a single small runtime image.
# The bridge spawns the node as a child process at runtime, so one container
# (and one image) is all the public demo needs.

FROM rust:1-bookworm AS builder
WORKDIR /build
COPY . .
RUN cargo build --release -p kamichain-node -p kamichain-bridge

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --create-home --home-dir /home/kami kami

COPY --from=builder /build/target/release/kamichain-node /usr/local/bin/kamichain-node
COPY --from=builder /build/target/release/kamichain-bridge /usr/local/bin/kamichain-bridge

USER kami
WORKDIR /home/kami
VOLUME ["/data/kamichain"]
EXPOSE 8080

ENTRYPOINT ["kamichain-bridge"]
