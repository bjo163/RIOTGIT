#!/usr/bin/env bash
set -Eeuo pipefail

ROOT_DIR="$(cd "$(dirname "\${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d)"
API_BASE="http://127.0.0.1:\${RIOTGIT_API_PORT:-8787}"
SSH_PORT="\${RIOTGIT_SSH_PORT:-2222}"
ADMIN_USERNAME="\${RIOTGIT_ADMIN_USERNAME:?Set RIOTGIT_ADMIN_USERNAME in .env}"
ADMIN_PASSWORD="\${RIOTGIT_ADMIN_PASSWORD:?Set RIOTGIT_ADMIN_PASSWORD in .env}"
KNOWN_HOSTS="$TMP_DIR/known_hosts"
KEY_FILE="$TMP_DIR/ssh-smoke"
REPO_NAME="ssh-smoke-\$(date -u +%H%M%S)"

cleanup() {
  status=$?
  if [[ "$status" -ne 0 ]]; then
    echo "R!OTGIT SSH smoke failed. Recent service logs:" >&2
    docker compose -f "$ROOT_DIR/deploy/compose/docker-compose.yml" --env-file "$ROOT_DIR/.env" --profile ssh logs --tail=120 api ssh >&2 || true
  fi
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

for command_name in curl jq ssh ssh-keygen ssh-keyscan git docker; do
  command -v "$command_name" >/dev/null 2>&1 || {
    echo "Required command not found: $command_name" >&2
    exit 1
  }
done

ssh-keygen -q -t ed25519 -N "" -f "$KEY_FILE"
PUBLIC_KEY="$(cat "$KEY_FILE.pub")"
TOKEN="$(curl --fail --silent --show-error -X POST "$API_BASE/api/v1/auth/login" \
  -H "Content-Type: application/json" \
  -d "$(jq -n --arg username "$ADMIN_USERNAME" --arg password "$ADMIN_PASSWORD" '{username:$username,password:$password}')" \
  | jq -er '.access_token')"

KEY_ID="$(curl --fail --silent --show-error -X POST "$API_BASE/api/v1/user/ssh-keys" \
  -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d "$(jq -n --arg public_key "$PUBLIC_KEY" '{public_key:$public_key}')" | jq -er '.id')"

REPO_RESPONSE="$(curl --fail --silent --show-error -X POST "$API_BASE/api/v1/repositories" \
  -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d "$(jq -n --arg owner "$ADMIN_USERNAME" --arg name "$REPO_NAME" '{owner:$owner,name:$name,visibility:"private"}')")"
jq -e '.state == "ready"' <<<"$REPO_RESPONSE" >/dev/null

READY=0
for _ in $(seq 1 40); do
  if ssh-keyscan -T 2 -p "$SSH_PORT" 127.0.0.1 2>/dev/null >>"$KNOWN_HOSTS"; then
    READY=1
    break
  fi
  sleep 0.5
done
[[ "$READY" -eq 1 ]] || { echo "SSH gateway did not become ready on port $SSH_PORT" >&2; exit 1; }

export GIT_SSH_COMMAND="ssh -i $KEY_FILE -p $SSH_PORT -o IdentitiesOnly=yes -o StrictHostKeyChecking=yes -o UserKnownHostsFile=$KNOWN_HOSTS -o BatchMode=yes"
CLONE_URL="ssh://git@127.0.0.1:$SSH_PORT/$ADMIN_USERNAME/$REPO_NAME.git"
CLONE_DIR="$TMP_DIR/clone"

git clone --quiet "$CLONE_URL" "$CLONE_DIR"
git -C "$CLONE_DIR" config user.name "R!OTGIT SSH CI"
git -C "$CLONE_DIR" config user.email "riotgit-ssh-ci@example.invalid"
printf 'SSH transport round-trip passed\n' >"$CLONE_DIR/README.md"
git -C "$CLONE_DIR" add README.md
git -C "$CLONE_DIR" commit -m "test: verify SSH clone and push" >/dev/null
git -C "$CLONE_DIR" push --quiet origin main

# Branch rules must apply through SSH receive-pack, not only Smart HTTP.
curl --fail --silent --show-error -X POST "$API_BASE/api/v1/repositories/$ADMIN_USERNAME/$REPO_NAME/branch-protection" \
  -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d '{"branch_name":"main","require_pull_request":true,"prevent_force_push":true,"prevent_deletion":true}' >/dev/null
printf 'This protected direct push must be denied\n' >>"$CLONE_DIR/README.md"
git -C "$CLONE_DIR" add README.md
git -C "$CLONE_DIR" commit -m "test: protected SSH direct push must fail" >/dev/null
if git -C "$CLONE_DIR" push origin main >/dev/null 2>&1; then
  echo "ERROR: SSH direct push bypassed branch protection." >&2
  exit 1
fi

# No interactive shell may be obtained through the Git account.
if ssh -p "$SSH_PORT" -i "$KEY_FILE" -o IdentitiesOnly=yes \
  -o StrictHostKeyChecking=yes -o UserKnownHostsFile="$KNOWN_HOSTS" \
  -o BatchMode=yes git@127.0.0.1 true >/dev/null 2>&1; then
  echo "ERROR: restricted Git account accepted an interactive command." >&2
  exit 1
fi

curl --fail --silent --show-error -X DELETE "$API_BASE/api/v1/user/ssh-keys/$KEY_ID" \
  -H "Authorization: Bearer $TOKEN" >/dev/null
if git -C "$CLONE_DIR" ls-remote origin >/dev/null 2>&1; then
  echo "ERROR: revoked SSH key still authenticated." >&2
  exit 1
fi

echo "R!OTGIT SSH smoke passed: key auth -> clone -> push -> branch-protection denial -> no shell -> revoke."
