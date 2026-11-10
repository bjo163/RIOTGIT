#!/usr/bin/env bash
set -Eeuo pipefail
umask 077

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
COMPOSE_FILE="$ROOT_DIR/deploy/compose/docker-compose.yml"
BACKUP_DIR="${RIOTGIT_BACKUP_DIR:-$ROOT_DIR/backups}"
RESTART_SERVICES=0
VOLUME_NAME=""
API_CONTAINER=""

log() { printf '[riotgit-backup] %s\n' "$*"; }
fail() { log "ERROR: $*" >&2; exit 1; }

restart_services() {
  if [[ "$RESTART_SERVICES" -eq 1 ]]; then
    log "Restarting API and web services..."
    docker compose -f "$COMPOSE_FILE" start api web >/dev/null || log "WARNING: automatic service restart failed; run docker compose start api web manually."
  fi
}
trap restart_services EXIT

command -v docker >/dev/null 2>&1 || fail "Docker CLI is required."
docker compose version >/dev/null 2>&1 || fail "Docker Compose plugin is required."
[[ -f "$COMPOSE_FILE" ]] || fail "Compose file not found: $COMPOSE_FILE"

API_CONTAINER="$(docker compose -f "$COMPOSE_FILE" ps -q api)"
[[ -n "$API_CONTAINER" ]] || fail "R!OTGIT API container is not running. Start the Compose stack first."

VOLUME_NAME="$(docker inspect --format '{{range .Mounts}}{{if eq .Destination "/var/lib/riotgit"}}{{if eq .Type "volume"}}{{.Name}}{{end}}{{end}}{{end}}' "$API_CONTAINER")"
[[ -n "$VOLUME_NAME" ]] || fail "Could not resolve the named /var/lib/riotgit volume from the API container."
docker volume inspect "$VOLUME_NAME" >/dev/null 2>&1 || fail "Data volume does not exist: $VOLUME_NAME"

mkdir -p "$BACKUP_DIR"
chmod 700 "$BACKUP_DIR" 2>/dev/null || true
BACKUP_DIR="$(cd "$BACKUP_DIR" && pwd)"
TIMESTAMP="$(date -u +%Y%m%dT%H%M%SZ)"
ARCHIVE_NAME="riotgit-data-$TIMESTAMP.tar.gz"
ARCHIVE_PATH="$BACKUP_DIR/$ARCHIVE_NAME"

log "Data volume: $VOLUME_NAME"
log "Stopping API and web so SQLite metadata and bare Git repositories are snapshotted consistently..."
RESTART_SERVICES=1
docker compose -f "$COMPOSE_FILE" stop api web

log "Creating backup archive..."
docker run --rm --network none \
  -v "$VOLUME_NAME:/data:ro" \
  -v "$BACKUP_DIR:/backup" \
  alpine:3.22 sh -ec 'cd /data && tar -czf "/backup/$1" .' sh "$ARCHIVE_NAME"

if command -v sha256sum >/dev/null 2>&1; then
  (cd "$BACKUP_DIR" && sha256sum "$ARCHIVE_NAME" > "$ARCHIVE_NAME.sha256")
fi

log "Backup created: $ARCHIVE_PATH"
[[ ! -f "$ARCHIVE_PATH.sha256" ]] || log "Checksum: $ARCHIVE_PATH.sha256"
