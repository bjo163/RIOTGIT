FROM rust:1.99-bookworm AS builder

WORKDIR /src
COPY Cargo.toml Cargo.toml
COPY apps/api/Cargo.toml apps/api/Cargo.toml
COPY apps/cli/Cargo.toml apps/cli/Cargo.toml
COPY apps/api/migrations apps/api/migrations
COPY apps/api/src apps/api/src
COPY apps/cli/src apps/cli/src

RUN cargo build --release -p riotgit-api \
    --bin riotgit-authorized-keys \
    --bin riotgit-ssh \
    --bin riotgit-ssh-hook

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates git openssh-server \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --home-dir /var/lib/riotgit --create-home --shell /bin/bash riotgit \
    && useradd --system --uid 10001 --non-unique --gid 10001 --home-dir /var/lib/riotgit --no-create-home --shell /bin/bash git \
    && mkdir -p /etc/ssh/hostkeys /usr/local/lib/riotgit/hooks /run/sshd \
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
