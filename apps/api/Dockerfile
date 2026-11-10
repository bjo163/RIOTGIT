FROM rust:1.99-bookworm AS builder

WORKDIR /src
COPY Cargo.toml Cargo.toml
COPY apps/api/Cargo.toml apps/api/Cargo.toml
COPY apps/cli/Cargo.toml apps/cli/Cargo.toml
COPY apps/cli/src apps/cli/src
COPY apps/api/migrations apps/api/migrations
COPY apps/api/src apps/api/src

# Build the API and all restricted SSH helper binaries in one shared layer.
# The API and SSH runtime targets reuse this exact artifact stage.
RUN cargo build --release -p riotgit-api --bins

FROM debian:bookworm-slim AS runtime-base

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl git \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --home-dir /var/lib/riotgit --create-home riotgit \
    && mkdir -p /var/lib/riotgit/repositories \
    && chown -R riotgit:riotgit /var/lib/riotgit

FROM runtime-base AS runtime

COPY --from=builder /src/target/release/riotgit-api /usr/local/bin/riotgit-api

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

FROM debian:bookworm-slim AS ssh-runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates git openssh-server openssl \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --gid 10001 riotgit \
    && useradd --system --uid 10001 --gid 10001 --home-dir /var/lib/riotgit --create-home --shell /bin/bash riotgit \
    && useradd --system --uid 10001 --non-unique --gid 10001 --home-dir /var/lib/riotgit --no-create-home --shell /bin/bash git \
    && RANDOM_GIT_PASSWORD="$(head -c 48 /dev/urandom | base64 | tr -d '\\\\n')" \
    && usermod --password "$(openssl passwd -6 "$RANDOM_GIT_PASSWORD")" git \
    && unset RANDOM_GIT_PASSWORD \
    && mkdir -p /etc/ssh/hostkeys /usr/local/lib/riotgit/hooks /run/sshd /var/empty \
    && chown -R riotgit:riotgit /var/lib/riotgit

COPY --from=builder /src/target/release/riotgit-authorized-keys /usr/local/bin/riotgit-authorized-keys
COPY --from=builder /src/target/release/riotgit-ssh /usr/local/bin/riotgit-ssh
COPY --from=builder /src/target/release/riotgit-ssh-hook /usr/local/bin/riotgit-ssh-hook
COPY apps/ssh/sshd_config /etc/ssh/sshd_config
COPY apps/ssh/entrypoint.sh /usr/local/bin/riotgit-ssh-entrypoint
COPY apps/ssh/hooks/pre-receive /usr/local/lib/riotgit/hooks/pre-receive

RUN chmod 0755 /usr/local/bin/riotgit-authorized-keys /usr/local/bin/riotgit-ssh \
      /usr/local/bin/riotgit-ssh-hook /usr/local/bin/riotgit-ssh-entrypoint \
      /usr/local/lib/riotgit/hooks/pre-receive \
    && chmod 0755 /usr/local/lib/riotgit/hooks

ENV RIOTGIT_DATABASE_PATH=/var/lib/riotgit/riotgit.db \
    RIOTGIT_STORAGE_ROOT=/var/lib/riotgit/repositories \
    RIOTGIT_GIT_BIN=git

EXPOSE 22
VOLUME ["/var/lib/riotgit", "/etc/ssh/hostkeys"]
ENTRYPOINT ["/usr/local/bin/riotgit-ssh-entrypoint"]
