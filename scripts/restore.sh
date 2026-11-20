#!/usr/bin/env bash
set -Eeuo pipefail
umask 077
ROOT_DIR="$(cd "$(dirname "$BASH_SOURCE")/.." && pwd)"
COMPOSE_FILE="$ROOT_DIR/deploy/compose/docker-compose.yml"
BACKUP_DIR="$ROOT_DIR/backups"
if [[ -n "${RIOTGIT_BACKUP_DIR:-}" ]]; then BACKUP_DIR="$RIOTGIT_BACKUP_DIR"; fi
START_SSH=0
RESTART_SERVICES=0
API_CONTAINER=""
DATA_VOLUME=""
SSH_HOSTKEYS_VOLUME=""

log() { printf '[riotgit-restore] %s\n' "$*"; }
fail() { log "ERROR: $*" >&2; exit 1; }

restart_services() {
  if [[ "$RESTART_SERVICES" -eq 1 ]]; then
    docker compose -f "$COMPOSE_FILE" start api web >/dev/null || log "WARNING: could not restart API/web automatically."
    if [[ "$START_SSH" -eq 1 ]]; then
      docker compose -f "$COMPOSE_FILE" --profile ssh start ssh >/dev/null || log "WARNING: could not restart SSH automatically."
    fi
  fi
}
trap restart_services EXIT

verify_checksum() {
  local archive="$1"
  local checksum_file="$archive.sha256"
  if ! command -v sha256sum >/dev/null 2>&1; then
    log "WARNING: sha256sum is unavailable; archive integrity cannot be verified."
    return 0
  fi
  if [[ ! -f "$checksum_file" ]]; then
    log "WARNING: checksum sidecar is missing for $archive; restoring an unverified legacy archive."
    return 0
  fi

  local expected actual
  read -r expected _ < "$checksum_file" || fail "Checksum sidecar is empty: $checksum_file"
  [[ "$expected" =~ ^[[:xdigit:]]{64}$ ]] || fail "Checksum sidecar is malformed: $checksum_file"
  actual="$(sha256sum "$archive" | awk '{print $1}')"
  [[ "$actual" == "$expected" ]] || fail "Archive checksum mismatch: $archive"
}

validate_archive() {
  local archive="$1"
  [[ -f "$archive" ]] || fail "Archive not found: $archive"
  tar -tzf "$archive" >/dev/null || fail "Archive is corrupt or unreadable: $archive"
  while IFS= read -r entry; do
    normalized="$(printf '%s' "$entry" | sed 's#^\./##')"
    case "$normalized" in /*|..|../*|*/../*|*/.. ) fail "Unsafe archive path: $entry" ;; esac
  done < <(tar -tzf "$archive")
}

[[ $# -eq 1 ]] || fail "Usage: scripts/restore.sh <data-archive.tar.gz>"
ARCHIVE="$(realpath "$1")"
verify_checksum "$ARCHIVE"
validate_archive "$ARCHIVE"
SSH_ARCHIVE="${ARCHIVE%.tar.gz}-ssh-hostkeys.tar.gz"
HAS_SSH_ARCHIVE=0
if [[ -f "$SSH_ARCHIVE" ]]; then
  verify_checksum "$SSH_ARCHIVE"
  validate_archive "$SSH_ARCHIVE"
  HAS_SSH_ARCHIVE=1
fi

command -v docker >/dev/null 2>&1 || fail "Docker CLI is required."
docker compose version >/dev/null 2>&1 || fail "Docker Compose plugin is required."
API_CONTAINER="$(docker compose -f "$COMPOSE_FILE" ps -q api)"
[[ -n "$API_CONTAINER" ]] || fail "API container is not running."
DATA_VOLUME="$(docker inspect --format '{{range .Mounts}}{{if eq .Destination "/var/lib/riotgit"}}{{if eq .Type "volume"}}{{.Name}}{{end}}{{end}}{{end}}' "$API_CONTAINER")"
[[ -n "$DATA_VOLUME" ]] || fail "Could not resolve application data volume."
PROJECT_NAME="$(docker inspect --format '{{ index .Config.Labels "com.docker.compose.project" }}' "$API_CONTAINER")"
[[ -n "$PROJECT_NAME" && "$PROJECT_NAME" != "<no value>" ]] || fail "Could not resolve Compose project name."
SSH_CONTAINER="$(docker compose -f "$COMPOSE_FILE" --profile ssh ps -q ssh || true)"
if [[ -n "$SSH_CONTAINER" ]]; then
  START_SSH=1
  SSH_HOSTKEYS_VOLUME="$(docker inspect --format '{{range .Mounts}}{{if eq .Destination "/etc/ssh/hostkeys"}}{{if eq .Type "volume"}}{{.Name}}{{end}}{{end}}{{end}}' "$SSH_CONTAINER")"
fi
if [[ -z "$SSH_HOSTKEYS_VOLUME" ]]; then
  SSH_HOSTKEYS_VOLUME="$(docker volume ls -q --filter "label=com.docker.compose.project=$PROJECT_NAME" --filter "label=com.docker.compose.volume=riotgit-ssh-hostkeys" | head -n 1)"
fi
if [[ "$HAS_SSH_ARCHIVE" -eq 1 && -z "$SSH_HOSTKEYS_VOLUME" ]]; then
  SSH_HOSTKEYS_VOLUME="$PROJECT_NAME"'_riotgit-ssh-hostkeys'
  docker volume create --label "com.docker.compose.project=$PROJECT_NAME" --label "com.docker.compose.volume=riotgit-ssh-hostkeys" "$SSH_HOSTKEYS_VOLUME" >/dev/null
fi

mkdir -p "$BACKUP_DIR"
chmod 700 "$BACKUP_DIR" 2>/dev/null || true
BACKUP_DIR="$(cd "$BACKUP_DIR" && pwd)"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
SAFETY_DATA="riotgit-pre-restore-$STAMP.tar.gz"
SAFETY_SSH="riotgit-pre-restore-$STAMP-ssh-hostkeys.tar.gz"
log "This replaces the complete data volume $DATA_VOLUME."
read -r -p "Type RESTORE to continue: " CONFIRM
[[ "$CONFIRM" == "RESTORE" ]] || fail "Restore cancelled."

RESTART_SERVICES=1
docker compose -f "$COMPOSE_FILE" --profile ssh stop api web
if [[ "$START_SSH" -eq 1 ]]; then docker compose -f "$COMPOSE_FILE" --profile ssh stop ssh; fi

docker run --rm --network none -v "$DATA_VOLUME:/data:ro" -v "$BACKUP_DIR:/backup" alpine:3.22 sh -ec 'cd /data && tar -czf "/backup/$1" .' sh "$SAFETY_DATA"
if [[ -n "$SSH_HOSTKEYS_VOLUME" ]]; then
  docker run --rm --network none -v "$SSH_HOSTKEYS_VOLUME:/data:ro" -v "$BACKUP_DIR:/backup" alpine:3.22 sh -ec 'cd /data && tar -czf "/backup/$1" .' sh "$SAFETY_SSH"
fi

docker run --rm --network none -v "$DATA_VOLUME:/data" -v "$ARCHIVE:/restore/input.tar.gz:ro" alpine:3.22 sh -ec 'find /data -mindepth 1 -maxdepth 1 -exec rm -rf {} \; && tar -xzf /restore/input.tar.gz -C /data && chown -R 10001:10001 /data'
if [[ "$HAS_SSH_ARCHIVE" -eq 1 ]]; then
  [[ -n "$SSH_HOSTKEYS_VOLUME" ]] || fail "Could not resolve the SSH host-key volume."
  log "Restoring SSH host keys from $SSH_ARCHIVE into volume $SSH_HOSTKEYS_VOLUME."
  docker run --rm --network none -v "$SSH_HOSTKEYS_VOLUME:/data" -v "$SSH_ARCHIVE:/restore/input.tar.gz:ro" alpine:3.22 sh -ec 'find /data -mindepth 1 -maxdepth 1 -exec rm -rf {} \; && tar -xzf /restore/input.tar.gz -C /data && chown -R 0:0 /data && chmod 0600 /data/ssh_host_ed25519_key && chmod 0644 /data/ssh_host_ed25519_key.pub'

  HOSTKEY_ENTRY="$(tar -tzf "$SSH_ARCHIVE" | grep -F 'ssh_host_ed25519_key.pub' | head -n 1 || true)"
  [[ -n "$HOSTKEY_ENTRY" ]] || fail "SSH host-key archive does not contain ssh_host_ed25519_key.pub."
  EXPECTED_HOSTKEY_SHA="$(tar -xOzf "$SSH_ARCHIVE" "$HOSTKEY_ENTRY" | sha256sum | awk '{print $1}')"
  ACTUAL_HOSTKEY_SHA="$(docker run --rm --network none -v "$SSH_HOSTKEYS_VOLUME:/data:ro" alpine:3.22 sh -ec 'sha256sum /data/ssh_host_ed25519_key.pub | cut -d " " -f1')"
  [[ "$EXPECTED_HOSTKEY_SHA" == "$ACTUAL_HOSTKEY_SHA" ]] || fail "Restored SSH host-key volume checksum does not match the archive (volume=$SSH_HOSTKEYS_VOLUME)."
  log "SSH host-key archive verified in volume $SSH_HOSTKEYS_VOLUME."
fi
log "Restore complete. Safety snapshots retained in $BACKUP_DIR."
