#!/usr/bin/env bash
set -Eeuo pipefail
umask 077
API_BASE="http://127.0.0.1:${RIOTGIT_API_PORT:-8787}"
SSH_PORT="${RIOTGIT_SSH_PORT:-2222}"
ADMIN_USERNAME="${RIOTGIT_ADMIN_USERNAME:?Set RIOTGIT_ADMIN_USERNAME in .env}"
ADMIN_PASSWORD="${RIOTGIT_ADMIN_PASSWORD:?Set RIOTGIT_ADMIN_PASSWORD in .env}"
STATE_FILE="${1:?Usage: scripts/smoke-git-ssh-restore.sh <repository-state-file>}"
TMP_DIR="$(mktemp -d)"
KEY_FILE="$TMP_DIR/ssh-restore"
KNOWN_HOSTS="$TMP_DIR/known_hosts"
CLONE_DIR="$TMP_DIR/restored-repository"
TOKEN=""
KEY_ID=""

cleanup() {
  if [[ -n "$TOKEN" && -n "$KEY_ID" ]]; then
    curl --fail --silent --show-error -X DELETE "$API_BASE/api/v1/user/ssh-keys/$KEY_ID" \\
      -H "Authorization: Bearer $TOKEN" >/dev/null 2>&1 || true
  fi
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

for command_name in curl jq ssh-keygen ssh-keyscan git; do
  command -v "$command_name" >/dev/null 2>&1 || {
    echo "Required command not found: $command_name" >&2
    exit 1
  }
done

[[ -f "$STATE_FILE" ]] || { echo "Repository state file is missing." >&2; exit 1; }
IFS=/ read -r OWNER REPO EXTRA < "$STATE_FILE"
[[ -n "$OWNER" && -n "$REPO" && -z "${EXTRA:-}" ]] || {
  echo "Repository state file is malformed." >&2
  exit 1
}
[[ "$OWNER" =~ ^[A-Za-z0-9][A-Za-z0-9._-]{0,99}$ && "$REPO" =~ ^[A-Za-z0-9][A-Za-z0-9._-]{0,99}$ ]] || {
  echo "Repository state file contains invalid owner or repository names." >&2
  exit 1
}
[[ "$OWNER" != "." && "$OWNER" != ".." && "$REPO" != "." && "$REPO" != ".." ]] || {
  echo "Repository state file contains invalid path segments." >&2
  exit 1
}

TOKEN="$(curl --fail --silent --show-error -X POST "$API_BASE/api/v1/auth/login" \\
  -H "Content-Type: application/json" \\
  -d "$(jq -n --arg username "$ADMIN_USERNAME" --arg password "$ADMIN_PASSWORD" '{username:$username,password:$password}')" \\
  | jq -er '.access_token')"
ssh-keygen -q -t ed25519 -N "" -f "$KEY_FILE"
PUBLIC_KEY="$(cat "$KEY_FILE.pub")"
KEY_ID="$(curl --fail --silent --show-error -X POST "$API_BASE/api/v1/user/ssh-keys" \\
  -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \\
  -d "$(jq -n --arg public_key "$PUBLIC_KEY" '{public_key:$public_key}')" | jq -er '.id')"
ssh-keyscan -T 5 -t ed25519 -p "$SSH_PORT" 127.0.0.1 2>/dev/null > "$KNOWN_HOSTS"
test -s "$KNOWN_HOSTS"

RESTORE_GIT_SSH_COMMAND="ssh -i $KEY_FILE -p $SSH_PORT -o IdentitiesOnly=yes -o StrictHostKeyChecking=yes -o UserKnownHostsFile=$KNOWN_HOSTS -o BatchMode=yes -o ControlMaster=no -o ControlPath=none"
RESTORED_URL="ssh://git@127.0.0.1:$SSH_PORT/$OWNER/$REPO.git"
GIT_SSH_COMMAND="$RESTORE_GIT_SSH_COMMAND" git clone --quiet "$RESTORED_URL" "$CLONE_DIR"
GIT_SSH_COMMAND="$RESTORE_GIT_SSH_COMMAND" git -C "$CLONE_DIR" fetch --quiet --prune origin
git --git-dir="$CLONE_DIR/.git" fsck --full
git -C "$CLONE_DIR" show origin/main:README.md | grep -Fxq "SSH transport round-trip passed"
echo "R!OTGIT restore smoke passed: existing repository clone, fetch, fsck, and marker verification."
