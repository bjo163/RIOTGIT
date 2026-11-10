FROM rust:1.99-bookworm AS builder

WORKDIR /src
COPY Cargo.toml Cargo.toml
COPY apps/api/Cargo.toml apps/api/Cargo.toml
COPY apps/cli/Cargo.toml apps/cli/Cargo.toml
COPY apps/api/migrations apps/api/migrations
COPY apps/api/src apps/api/src
RUN cargo build --release -p riotgit-api

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl git \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --home-dir /var/lib/riotgit --create-home riotgit

COPY --from=builder /src/target/release/riotgit-api /usr/local/bin/riotgit-api

RUN mkdir -p /var/lib/riotgit/repositories \
    && chown -R riotgit:riotgit /var/lib/riotgit

ENV RIOTGIT_BIND=0.0.0.0:8787 \
    RIOTGIT_DATABASE_PATH=/var/lib/riotgit/riotgit.db \
    RIOTGIT_STORAGE_ROOT=/var/lib/riotgit/repositories \
    RIOTGIT_GIT_BIN=git \
    RUST_LOG=riotgit_api=info,sqlx=warn

USER 10001:10001
WORKDIR /var/lib/riotgit
EXPOSE 8787
VOLUME ["/var/lib/riotgit"]
HEALTHCHECK --interval=20s --timeout=3s --start-period=10s --retries=3 \
  CMD curl --fail --silent http://127.0.0.1:8787/health/ready || exit 1

ENTRYPOINT ["/usr/local/bin/riotgit-api"]
