#!/usr/bin/env bash
set -Eeuo pipefail
umask 077

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
COMPOSE_FILE="$ROOT_DIR/deploy/compose/docker-compose.yml"
BACKUP_DIR="${RIOTGIT_BACKUP_DIR:-$ROOT_DIR/backups}"
RESTART_SERVICES=0
VOLUME_NAME=""
API_CONTAINER=""

log() { printf '[riotgit-restore] %s\n' "$*"; }
fail() { log "ERROR: $*" >&2; exit 1; }

restart_services() {
  if [[ "$RESTART_SERVICES" -eq 1 ]]; then
    log "Restarting API and web services..."
    docker compose -f "$COMPOSE_FILE" start api web >/dev/null || log "WARNING: automatic service restart failed; run docker compose start api web manually."
  fi
}
trap restart_services EXIT

[[ $# -eq 1 ]] || fail "Usage: scripts/restore.sh <backup-archive.tar.gz>"
ARCHIVE="$(realpath "$1")"
[[ -f "$ARCHIVE" ]] || fail "Backup archive not found: $ARCHIVE"
command -v docker >/dev/null 2>&1 || fail "Docker CLI is required."
docker compose version >/dev/null 2>&1 || fail "Docker Compose plugin is required."
[[ -f "$COMPOSE_FILE" ]] || fail "Compose file not found: $COMPOSE_FILE"
command -v tar >/dev/null 2>&1 || fail "Host tar is required to inspect the archive."

# Reject absolute and parent-traversal paths before the archive is mounted into the data volume.
while IFS= read -r entry; do
  normalized="$entry"
  while [[ "$normalized" == ./* ]]; do normalized="${normalized#./}"; done
  case "$normalized" in
    ""|"." ) ;;
    /*|..|../*|*/../*|*/.. ) fail "Unsafe path in archive: $entry" ;;
  esac
done < <(tar -tzf "$ARCHIVE")

API_CONTAINER="$(docker compose -f "$COMPOSE_FILE" ps -q api)"
[[ -n "$API_CONTAINER" ]] || fail "R!OTGIT API container is not running. Start the Compose stack first."
VOLUME_NAME="$(docker inspect --format '{{range .Mounts}}{{if eq .Destination "/var/lib/riotgit"}}{{if eq .Type "volume"}}{{.Name}}{{end}}{{end}}{{end}}' "$API_CONTAINER")"
[[ -n "$VOLUME_NAME" ]] || fail "Could not resolve the named /var/lib/riotgit volume from the API container."
docker volume inspect "$VOLUME_NAME" >/dev/null 2>&1 || fail "Data volume does not exist: $VOLUME_NAME"

mkdir -p "$BACKUP_DIR"
chmod 700 "$BACKUP_DIR" 2>/dev/null || true
BACKUP_DIR="$(cd "$BACKUP_DIR" && pwd)"
TIMESTAMP="$(date -u +%Y%m%dT%H%M%SZ)"
SAFETY_NAME="riotgit-pre-restore-$TIMESTAMP.tar.gz"

log "This will REPLACE all data in volume $VOLUME_NAME."
log "The supplied archive: $ARCHIVE"
log "A safety snapshot will be written to $BACKUP_DIR/$SAFETY_NAME before replacement."
read -r -p "Type RESTORE to continue: " CONFIRM
[[ "$CONFIRM" == "RESTORE" ]] || fail "Restore cancelled."

log "Stopping API and web services..."
RESTART_SERVICES=1
docker compose -f "$COMPOSE_FILE" stop api web

log "Creating pre-restore safety snapshot..."
docker run --rm --network none \
  -v "$VOLUME_NAME:/data:ro" \
  -v "$BACKUP_DIR:/backup" \
  alpine:3.22 sh -ec 'cd /data && tar -czf "/backup/$1" .' sh "$SAFETY_NAME"
if command -v sha256sum >/dev/null 2>&1; then
  (cd "$BACKUP_DIR" && sha256sum "$SAFETY_NAME" > "$SAFETY_NAME.sha256")
fi

log "Replacing volume contents from archive..."
docker run --rm --network none \
  -v "$VOLUME_NAME:/data" \
  -v "$ARCHIVE:/restore/input.tar.gz:ro" \
  alpine:3.22 sh -ec '
    find /data -mindepth 1 -maxdepth 1 -exec rm -rf {} \;
    tar -xzf /restore/input.tar.gz -C /data
    chown -R 10001:10001 /data
  '

log "Restore extraction completed."
log "Safety snapshot retained at $BACKUP_DIR/$SAFETY_NAME"
log "Start-up and health verification will follow when services restart."
