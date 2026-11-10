#!/usr/bin/env bash
set -Eeuo pipefail
umask 077
ROOT_DIR="$(cd "$(dirname "$BASH_SOURCE")/.." && pwd)"
COMPOSE_FILE="$ROOT_DIR/deploy/compose/docker-compose.yml"
BACKUP_DIR="$ROOT_DIR/backups"
if [[ -n "${RIOTGIT_BACKUP_DIR:-}" ]]; then BACKUP_DIR="$RIOTGIT_BACKUP_DIR"; fi
START_SSH=0
API_CONTAINER=""
DATA_VOLUME=""
SSH_HOSTKEYS_VOLUME=""

log() { printf '[riotgit-backup] %s\n' "$*"; }
fail() { log "ERROR: $*" >&2; exit 1; }

restart_services() {
  if [[ "$RESTART_SERVICES" -eq 1 ]]; then
    docker compose -f "$COMPOSE_FILE" start api web >/dev/null || log "WARNING: could not restart API/web automatically."
    if [[ "$START_SSH" -eq 1 ]]; then
      docker compose -f "$COMPOSE_FILE" --profile ssh start ssh >/dev/null || log "WARNING: could not restart SSH automatically."
    fi
  fi
}
RESTART_SERVICES=0
trap restart_services EXIT

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

mkdir -p "$BACKUP_DIR"
chmod 700 "$BACKUP_DIR" 2>/dev/null || true
BACKUP_DIR="$(cd "$BACKUP_DIR" && pwd)"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
DATA_ARCHIVE="riotgit-data-$STAMP.tar.gz"
SSH_ARCHIVE="riotgit-data-$STAMP-ssh-hostkeys.tar.gz"

log "Stopping API/web and optional SSH writers for a consistent snapshot."
RESTART_SERVICES=1
docker compose -f "$COMPOSE_FILE" --profile ssh stop api web
if [[ "$START_SSH" -eq 1 ]]; then docker compose -f "$COMPOSE_FILE" --profile ssh stop ssh; fi

docker run --rm --network none -v "$DATA_VOLUME:/data:ro" -v "$BACKUP_DIR:/backup" alpine:3.22 sh -ec 'cd /data && tar -czf "/backup/$1" .' sh "$DATA_ARCHIVE"
if [[ -n "$SSH_HOSTKEYS_VOLUME" ]]; then
  docker run --rm --network none -v "$SSH_HOSTKEYS_VOLUME:/data:ro" -v "$BACKUP_DIR:/backup" alpine:3.22 sh -ec 'cd /data && tar -czf "/backup/$1" .' sh "$SSH_ARCHIVE"
fi
if command -v sha256sum >/dev/null 2>&1; then
  (cd "$BACKUP_DIR" && sha256sum "$DATA_ARCHIVE" > "$DATA_ARCHIVE.sha256")
  if [[ -f "$BACKUP_DIR/$SSH_ARCHIVE" ]]; then
    (cd "$BACKUP_DIR" && sha256sum "$SSH_ARCHIVE" > "$SSH_ARCHIVE.sha256")
  fi
fi
log "Data backup: $BACKUP_DIR/$DATA_ARCHIVE"
if [[ -f "$BACKUP_DIR/$SSH_ARCHIVE" ]]; then log "SSH host-key backup: $BACKUP_DIR/$SSH_ARCHIVE"; fi
