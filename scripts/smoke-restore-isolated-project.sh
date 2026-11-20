#!/usr/bin/env bash
set -Eeuo pipefail
umask 077

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
COMPOSE_FILE="$ROOT_DIR/deploy/compose/docker-compose.yml"
BACKUP_DIR="${RIOTGIT_BACKUP_DIR:?Set RIOTGIT_BACKUP_DIR}"
STATE_FILE="${RIOTGIT_SSH_SMOKE_STATE_FILE:?Set RIOTGIT_SSH_SMOKE_STATE_FILE}"
SOURCE_HOSTKEY_FILE="${RIOTGIT_SSH_HOSTKEY_BEFORE:?Set RIOTGIT_SSH_HOSTKEY_BEFORE}"
TARGET_PROJECT="riotgit-recovery-target"
TARGET_API_PORT="${RIOTGIT_RECOVERY_API_PORT:-18787}"
TARGET_WEB_PORT="${RIOTGIT_RECOVERY_WEB_PORT:-13000}"
TARGET_SSH_PORT="${RIOTGIT_RECOVERY_SSH_PORT:-12222}"
TMP_DIR="$(mktemp -d)"
TARGET_ENV="$TMP_DIR/target.env"
TARGET_BACKUPS="$TMP_DIR/target-backups"
TARGET_TOKEN="riotgit-ci-recovery-$(openssl rand -hex 32)"
DATA_ARCHIVE=""

log() { printf '[riotgit-recovery-project] %s\n' "$*"; }
fail() { log "ERROR: $*" >&2; exit 1; }

target_compose() {
  env \
    COMPOSE_PROJECT_NAME="$TARGET_PROJECT" \
    RIOTGIT_API_PORT="$TARGET_API_PORT" \
    RIOTGIT_WEB_PORT="$TARGET_WEB_PORT" \
    RIOTGIT_SSH_PORT="$TARGET_SSH_PORT" \
    docker compose --project-name "$TARGET_PROJECT" -f "$COMPOSE_FILE" --env-file "$TARGET_ENV" --profile ssh "$@"
}

cleanup() {
  local status=$?
  trap - EXIT
  target_compose down -v --remove-orphans >/dev/null 2>&1 || true
  rm -rf "$TMP_DIR"
  exit "$status"
}
trap cleanup EXIT

for command_name in docker curl ssh-keyscan sort awk cmp openssl; do
  command -v "$command_name" >/dev/null 2>&1 || fail "Required command not found: $command_name"
done
[[ -f "$ROOT_DIR/.env" ]] || fail "The source CI .env file is missing."
[[ -f "$STATE_FILE" ]] || fail "The SSH smoke repository state file is missing."
[[ -s "$SOURCE_HOSTKEY_FILE" ]] || fail "The source SSH host-key snapshot is missing."

set -a
. "$ROOT_DIR/.env"
set +a

DATA_ARCHIVE="$(find "$BACKUP_DIR" -maxdepth 1 -type f -name 'riotgit-data-*.tar.gz' ! -name '*-ssh-hostkeys.tar.gz' -print | sort | tail -n 1)"
[[ -n "$DATA_ARCHIVE" && -f "$DATA_ARCHIVE.sha256" ]] || fail "A checksummed source data backup is required."
[[ -f "${DATA_ARCHIVE%.tar.gz}-ssh-hostkeys.tar.gz" ]] || fail "The SSH host-key backup is missing."

mkdir -p "$TARGET_BACKUPS"
cat > "$TARGET_ENV" <<EOF
RIOTGIT_API_TOKEN=$TARGET_TOKEN
RIOTGIT_ADMIN_USERNAME=$RIOTGIT_ADMIN_USERNAME
RIOTGIT_ADMIN_PASSWORD=$RIOTGIT_ADMIN_PASSWORD
RIOTGIT_API_PORT=$TARGET_API_PORT
RIOTGIT_WEB_PORT=$TARGET_WEB_PORT
RIOTGIT_SSH_PORT=$TARGET_SSH_PORT
EOF

log "Starting an isolated Compose project with independent data and SSH volumes."
target_compose up -d api web ssh

TARGET_API_URL="http://127.0.0.1:$TARGET_API_PORT"
TARGET_WEB_URL="http://127.0.0.1:$TARGET_WEB_PORT"
TARGET_API_CONTAINER="$(target_compose ps -q api)"
[[ -n "$TARGET_API_CONTAINER" ]] || fail "Target API container did not start."
TARGET_DATA_VOLUME="$(docker inspect --format '{{range .Mounts}}{{if eq .Destination "/var/lib/riotgit"}}{{if eq .Type "volume"}}{{.Name}}{{end}}{{end}}{{end}}' "$TARGET_API_CONTAINER")"
[[ -n "$TARGET_DATA_VOLUME" ]] || fail "Could not resolve the target data volume."

SOURCE_API_CONTAINER="$(docker compose -f "$COMPOSE_FILE" ps -q api)"
[[ -n "$SOURCE_API_CONTAINER" ]] || fail "Source API container is no longer running."
SOURCE_DATA_VOLUME="$(docker inspect --format '{{range .Mounts}}{{if eq .Destination "/var/lib/riotgit"}}{{if eq .Type "volume"}}{{.Name}}{{end}}{{end}}{{end}}' "$SOURCE_API_CONTAINER")"
[[ -n "$SOURCE_DATA_VOLUME" && "$SOURCE_DATA_VOLUME" != "$TARGET_DATA_VOLUME" ]] || fail "Source and target Compose projects do not have isolated data volumes."

for attempt in $(seq 1 60); do
  if curl --fail --silent "$TARGET_API_URL/health/ready" >/dev/null \
    && curl --fail --silent "$TARGET_WEB_URL/login" >/dev/null; then
    break
  fi
  sleep 2
done
curl --fail --silent "$TARGET_API_URL/health/ready" >/dev/null || fail "Target API did not become ready."
curl --fail --silent "$TARGET_WEB_URL/login" >/dev/null || fail "Target Web did not become ready."

for attempt in $(seq 1 30); do
  ssh-keyscan -T 2 -t ed25519 -p "$TARGET_SSH_PORT" 127.0.0.1 2>/dev/null \
    | awk '{print $2, $3}' | sort > "$TMP_DIR/target-hostkey-before" || true
  if [[ -s "$TMP_DIR/target-hostkey-before" ]]; then break; fi
  sleep 1
done
test -s "$TMP_DIR/target-hostkey-before" || fail "Target SSH gateway did not publish a host key."
awk '{print $2, $3}' "$SOURCE_HOSTKEY_FILE" | sort > "$TMP_DIR/source-hostkey"
SSH_ARCHIVE="${DATA_ARCHIVE%.tar.gz}-ssh-hostkeys.tar.gz"
SSH_HOSTKEY_ENTRY="$(tar -tzf "$SSH_ARCHIVE" | grep -F 'ssh_host_ed25519_key.pub' | head -n 1 || true)"
[[ -n "$SSH_HOSTKEY_ENTRY" ]] || fail "Source SSH host-key archive is missing ssh_host_ed25519_key.pub."
ARCHIVE_PUBLIC_KEY="$(tar -xOzf "$SSH_ARCHIVE" "$SSH_HOSTKEY_ENTRY" | awk 'NF >= 2 { print $1, $2; exit }')"
SOURCE_PUBLIC_KEY="$(awk 'NF >= 2 { print $1, $2; exit }' "$TMP_DIR/source-hostkey")"
[[ -n "$ARCHIVE_PUBLIC_KEY" && "$ARCHIVE_PUBLIC_KEY" == "$SOURCE_PUBLIC_KEY" ]] || {
  log "Source host key and archived host key do not match."
  log "Source: $SOURCE_PUBLIC_KEY"
  log "Archive: $ARCHIVE_PUBLIC_KEY"
  fail "Backup archive does not contain the SSH host key advertised by the source instance."
}
if cmp -s "$TMP_DIR/source-hostkey" "$TMP_DIR/target-hostkey-before"; then
  fail "Fresh target Compose project unexpectedly reused the source SSH host key."
fi

log "Restoring the source archive into the separate target project's volumes."
env \
  COMPOSE_PROJECT_NAME="$TARGET_PROJECT" \
  RIOTGIT_API_TOKEN="$TARGET_TOKEN" \
  RIOTGIT_ADMIN_USERNAME="$RIOTGIT_ADMIN_USERNAME" \
  RIOTGIT_ADMIN_PASSWORD="$RIOTGIT_ADMIN_PASSWORD" \
  RIOTGIT_API_PORT="$TARGET_API_PORT" \
  RIOTGIT_WEB_PORT="$TARGET_WEB_PORT" \
  RIOTGIT_SSH_PORT="$TARGET_SSH_PORT" \
  RIOTGIT_BACKUP_DIR="$TARGET_BACKUPS" \
  bash -c 'printf "RESTORE\n" | bash "$1" "$2"' _ "$ROOT_DIR/scripts/restore.sh" "$DATA_ARCHIVE"

for attempt in $(seq 1 60); do
  if curl --fail --silent "$TARGET_API_URL/health/ready" >/dev/null \
    && curl --fail --silent "$TARGET_WEB_URL/login" >/dev/null; then
    break
  fi
  sleep 2
done
curl --fail --silent "$TARGET_API_URL/health/ready" >/dev/null || fail "Restored target API did not become ready."
curl --fail --silent "$TARGET_WEB_URL/login" >/dev/null || fail "Restored target Web did not become ready."

for attempt in $(seq 1 30); do
  ssh-keyscan -T 2 -t ed25519 -p "$TARGET_SSH_PORT" 127.0.0.1 2>/dev/null \
    | awk '{print $2, $3}' | sort > "$TMP_DIR/target-hostkey-after" || true
  if [[ -s "$TMP_DIR/target-hostkey-after" ]]; then break; fi
  sleep 1
done
TARGET_SSH_CONTAINER="$(target_compose ps -q ssh)"
[[ -n "$TARGET_SSH_CONTAINER" ]] || fail "Restored target SSH container is not running."
TARGET_SSH_HOSTKEY_VOLUME="$(docker inspect --format '{{range .Mounts}}{{if eq .Destination "/etc/ssh/hostkeys"}}{{if eq .Type "volume"}}{{.Name}}{{end}}{{end}}{{end}}' "$TARGET_SSH_CONTAINER")"
[[ -n "$TARGET_SSH_HOSTKEY_VOLUME" ]] || fail "Could not resolve the restored target SSH host-key volume."
TARGET_VOLUME_PUBLIC_KEY="$(docker run --rm --network none -v "$TARGET_SSH_HOSTKEY_VOLUME:/data:ro" alpine:3.22 cat /data/ssh_host_ed25519_key.pub | awk 'NF >= 2 { print $1, $2; exit }')"
TARGET_DAEMON_PUBLIC_KEY="$(awk 'NF >= 2 { print $1, $2; exit }' "$TMP_DIR/target-hostkey-after")"
log "Source public host key: $SOURCE_PUBLIC_KEY"
log "Backup public host key: $ARCHIVE_PUBLIC_KEY"
log "Target mounted volume ($TARGET_SSH_HOSTKEY_VOLUME): $TARGET_VOLUME_PUBLIC_KEY"
log "Target SSH daemon: $TARGET_DAEMON_PUBLIC_KEY"
[[ "$TARGET_VOLUME_PUBLIC_KEY" == "$SOURCE_PUBLIC_KEY" ]] || fail "Target SSH host-key volume does not contain the source host key."
[[ "$TARGET_DAEMON_PUBLIC_KEY" == "$TARGET_VOLUME_PUBLIC_KEY" ]] || fail "Target SSH daemon is not serving the public key stored in its mounted host-key volume."

log "Verifying a pre-backup repository through the restored target SSH gateway."
env \
  RIOTGIT_API_PORT="$TARGET_API_PORT" \
  RIOTGIT_SSH_PORT="$TARGET_SSH_PORT" \
  RIOTGIT_ADMIN_USERNAME="$RIOTGIT_ADMIN_USERNAME" \
  RIOTGIT_ADMIN_PASSWORD="$RIOTGIT_ADMIN_PASSWORD" \
  bash "$ROOT_DIR/scripts/smoke-git-ssh-restore.sh" "$STATE_FILE"

curl --fail --silent "http://127.0.0.1:${RIOTGIT_API_PORT:-8787}/health/ready" >/dev/null || fail "Source API was disrupted by the target restore."
curl --fail --silent "http://127.0.0.1:${RIOTGIT_WEB_PORT:-3000}/login" >/dev/null || fail "Source Web was disrupted by the target restore."
log "Isolated Compose restore passed: separate data volume, restored repository, SSH host-key continuity, and source stack integrity."
