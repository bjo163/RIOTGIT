#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
API_BIN="$ROOT_DIR/target/debug/riotgit-api"
TMP_DIR="$(mktemp -d)"
PORT="18787"
TOKEN="riotgit-ci-smoke-token"
API_PID=""

cleanup() {
  if [[ -n "$API_PID" ]]; then
    kill "$API_PID" 2>/dev/null || true
    wait "$API_PID" 2>/dev/null || true
  fi
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

if [[ ! -x "$API_BIN" ]]; then
  echo "R!OTGIT API binary not found or not executable: $API_BIN" >&2
  echo "Build it first with: cargo build -p riotgit-api" >&2
  exit 1
fi

export RIOTGIT_BIND="127.0.0.1:$PORT"
export RIOTGIT_API_TOKEN="$TOKEN"
export RIOTGIT_DATABASE_PATH="$TMP_DIR/riotgit.db"
export RIOTGIT_STORAGE_ROOT="$TMP_DIR/repositories"
export RIOTGIT_GIT_BIN="git"
export RUST_LOG="riotgit_api=info,sqlx=warn"

"$API_BIN" >"$TMP_DIR/api.log" 2>&1 &
API_PID="$!"

READY=0
for _ in $(seq 1 40); do
  if curl --fail --silent "http://127.0.0.1:$PORT/health/ready" >/dev/null; then
    READY=1
    break
  fi
  sleep 0.25
done

if [[ "$READY" -ne 1 ]]; then
  echo "R!OTGIT API did not become ready." >&2
  cat "$TMP_DIR/api.log" >&2
  exit 1
fi

curl --fail --silent --show-error \
  -X POST "http://127.0.0.1:$PORT/api/v1/repositories" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"owner":"ci","name":"transport-smoke","visibility":"private"}' \
  >/dev/null

SOURCE_DIR="$TMP_DIR/source"
CLONE_DIR="$TMP_DIR/clone"
mkdir -p "$SOURCE_DIR"
git -C "$SOURCE_DIR" init --initial-branch=main >/dev/null
git -C "$SOURCE_DIR" config user.name "R!OTGIT CI"
git -C "$SOURCE_DIR" config user.email "riotgit-ci@example.invalid"
printf 'R!OTGIT Git smart HTTP smoke test\n' >"$SOURCE_DIR/README.md"
git -C "$SOURCE_DIR" add README.md
git -C "$SOURCE_DIR" commit -m "test: initial transport smoke commit" >/dev/null
git -C "$SOURCE_DIR" remote add origin "http://127.0.0.1:$PORT/git/ci/transport-smoke.git"

export GIT_CONFIG_COUNT=1
export GIT_CONFIG_KEY_0="http.extraHeader"
export GIT_CONFIG_VALUE_0="Authorization: Bearer $TOKEN"

git -C "$SOURCE_DIR" push --quiet --set-upstream origin main
git clone --quiet "http://127.0.0.1:$PORT/git/ci/transport-smoke.git" "$CLONE_DIR"
cmp "$SOURCE_DIR/README.md" "$CLONE_DIR/README.md"

echo "R!OTGIT Git smart HTTP smoke test passed: create -> push -> clone."
