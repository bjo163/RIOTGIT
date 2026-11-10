#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
API_BIN="$ROOT_DIR/target/debug/riotgit-api"
TMP_DIR="$(mktemp -d)"
PORT="18787"
ADMIN_TOKEN="riotgit-ci-emergency-token"
ADMIN_USER="admin"
ADMIN_PASSWORD="R!OTGIT-CI-Admin-1234"
BUILDER_PASSWORD="Builder-CI-Password-123!"
READER_PASSWORD="Reader-CI-Password-123!"
API_PID=""

cleanup() {
  status=$?
  if [[ "$status" -ne 0 && -f "$TMP_DIR/api.log" ]]; then
    echo "---- R!OTGIT API log ----" >&2
    cat "$TMP_DIR/api.log" >&2
  fi
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
export RIOTGIT_API_TOKEN="$ADMIN_TOKEN"
export RIOTGIT_ADMIN_USERNAME="$ADMIN_USER"
export RIOTGIT_ADMIN_PASSWORD="$ADMIN_PASSWORD"
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

BASE_URL="http://127.0.0.1:$PORT"
ADMIN_TOKEN="$(curl --fail --silent --show-error -X POST "$BASE_URL/api/v1/auth/login" -H "Content-Type: application/json" -d "{\"username\":\"$ADMIN_USER\",\"password\":\"$ADMIN_PASSWORD\"}" | jq -er '.access_token')"

curl --fail --silent --show-error -X POST "$BASE_URL/api/v1/admin/users" -H "Authorization: Bearer $ADMIN_TOKEN" -H "Content-Type: application/json" -d "{\"username\":\"builder\",\"display_name\":\"CI Builder\",\"password\":\"$BUILDER_PASSWORD\",\"role\":\"member\"}" >/dev/null
curl --fail --silent --show-error -X POST "$BASE_URL/api/v1/admin/users" -H "Authorization: Bearer $ADMIN_TOKEN" -H "Content-Type: application/json" -d "{\"username\":\"reader\",\"display_name\":\"CI Reader\",\"password\":\"$READER_PASSWORD\",\"role\":\"member\"}" >/dev/null

BUILDER_TOKEN="$(curl --fail --silent --show-error -X POST "$BASE_URL/api/v1/auth/login" -H "Content-Type: application/json" -d "{\"username\":\"builder\",\"password\":\"$BUILDER_PASSWORD\"}" | jq -er '.access_token')"
READER_TOKEN="$(curl --fail --silent --show-error -X POST "$BASE_URL/api/v1/auth/login" -H "Content-Type: application/json" -d "{\"username\":\"reader\",\"password\":\"$READER_PASSWORD\"}" | jq -er '.access_token')"

# SSH key management validates key type/blob, stores a fingerprint, and supports revocation.
ssh-keygen -q -t ed25519 -N "" -f "$TMP_DIR/riotgit-ci-ssh"
SSH_PUBLIC_KEY="$(cat "$TMP_DIR/riotgit-ci-ssh.pub")"
SSH_KEY_ID="$(curl --fail --silent --show-error -X POST "$BASE_URL/api/v1/user/ssh-keys" \
  -H "Authorization: Bearer $BUILDER_TOKEN" -H "Content-Type: application/json" \
  -d "$(jq -n --arg public_key "$SSH_PUBLIC_KEY" '{public_key:$public_key}')" | jq -er '.id')"
curl --fail --silent --show-error "$BASE_URL/api/v1/user/ssh-keys" \
  -H "Authorization: Bearer $BUILDER_TOKEN" | jq -e --arg id "$SSH_KEY_ID" 'any(.[]; .id == $id)' >/dev/null
curl --fail --silent --show-error -X DELETE "$BASE_URL/api/v1/user/ssh-keys/$SSH_KEY_ID" \
  -H "Authorization: Bearer $BUILDER_TOKEN" >/dev/null

# Per-username throttling blocks repeated failed sign-in attempts.
curl --fail --silent --show-error -X POST "$BASE_URL/api/v1/admin/users" \
  -H "Authorization: Bearer $ADMIN_TOKEN" -H "Content-Type: application/json" \
  -d '{"username":"rate-limited","display_name":"Rate Limited","password":"Rate-Limit-Test-Password-123!","role":"member"}' >/dev/null
for _ in $(seq 1 10); do
  FAILED_LOGIN_STATUS="$(curl --silent --output /dev/null --write-out '%{http_code}' \
    -X POST "$BASE_URL/api/v1/auth/login" -H "Content-Type: application/json" \
    -d '{"username":"rate-limited","password":"wrong-password"}')"
  if [[ "$FAILED_LOGIN_STATUS" != "401" ]]; then
    echo "ERROR: failed login returned $FAILED_LOGIN_STATUS instead of 401 before the threshold." >&2
    exit 1
  fi
done
THROTTLED_LOGIN_STATUS="$(curl --silent --output /dev/null --write-out '%{http_code}' \
  -X POST "$BASE_URL/api/v1/auth/login" -H "Content-Type: application/json" \
  -d '{"username":"rate-limited","password":"Rate-Limit-Test-Password-123!"}')"
if [[ "$THROTTLED_LOGIN_STATUS" != "429" ]]; then
  echo "ERROR: login throttling returned $THROTTLED_LOGIN_STATUS instead of 429." >&2
  exit 1
fi


curl --fail --silent --show-error -X POST "$BASE_URL/api/v1/repositories" -H "Authorization: Bearer $BUILDER_TOKEN" -H "Content-Type: application/json" -d '{"owner":"builder","name":"transport-smoke","visibility":"private"}' >/dev/null
curl --fail --silent --show-error -X PUT "$BASE_URL/api/v1/repositories/builder/transport-smoke/access/reader" -H "Authorization: Bearer $BUILDER_TOKEN" -H "Content-Type: application/json" -d '{"permission":"read"}' >/dev/null

ISSUE_NUMBER="$(curl --fail --silent --show-error -X POST "$BASE_URL/api/v1/repositories/builder/transport-smoke/issues" -H "Authorization: Bearer $BUILDER_TOKEN" -H "Content-Type: application/json" -d '{"title":"Issue workflow smoke","body":"Verify issue creation, visibility, comments, and close state."}' | jq -er '.number')"
curl --fail --silent --show-error "$BASE_URL/api/v1/repositories/builder/transport-smoke/issues" -H "Authorization: Bearer $READER_TOKEN" | jq -e --argjson number "$ISSUE_NUMBER" 'any(.[]; .number == $number)' >/dev/null
LABEL_ID="$(curl --fail --silent --show-error -X POST "$BASE_URL/api/v1/repositories/builder/transport-smoke/labels" -H "Authorization: Bearer $BUILDER_TOKEN" -H "Content-Type: application/json" -d '{"name":"bug","color":"FF5A1F","description":"Bug report"}' | jq -er '.id')"
curl --fail --silent --show-error -X PUT "$BASE_URL/api/v1/repositories/builder/transport-smoke/issues/$ISSUE_NUMBER/labels/$LABEL_ID" -H "Authorization: Bearer $BUILDER_TOKEN" >/dev/null
curl --fail --silent --show-error "$BASE_URL/api/v1/repositories/builder/transport-smoke/issues/$ISSUE_NUMBER/labels" -H "Authorization: Bearer $BUILDER_TOKEN" | jq -e --arg id "$LABEL_ID" 'any(.[]; .id == $id)' >/dev/null
MILESTONE_NUMBER="$(curl --fail --silent --show-error -X POST "$BASE_URL/api/v1/repositories/builder/transport-smoke/milestones" -H "Authorization: Bearer $BUILDER_TOKEN" -H "Content-Type: application/json" -d '{"title":"Smoke milestone","description":"Milestone smoke coverage"}' | jq -er '.number')"
curl --fail --silent --show-error -X PUT "$BASE_URL/api/v1/repositories/builder/transport-smoke/issues/$ISSUE_NUMBER/milestone" -H "Authorization: Bearer $BUILDER_TOKEN" -H "Content-Type: application/json" -d "{\"number\":$MILESTONE_NUMBER}" >/dev/null
curl --fail --silent --show-error -X DELETE "$BASE_URL/api/v1/repositories/builder/transport-smoke/issues/$ISSUE_NUMBER/milestone" -H "Authorization: Bearer $BUILDER_TOKEN" >/dev/null

READER_ISSUE_STATUS="$(curl --silent --output /dev/null --write-out '%{http_code}' -X POST "$BASE_URL/api/v1/repositories/builder/transport-smoke/issues" -H "Authorization: Bearer $READER_TOKEN" -H "Content-Type: application/json" -d '{"title":"Reader issue","body":"This must be denied."}')"
if [[ "$READER_ISSUE_STATUS" != "403" ]]; then
  echo "ERROR: read-only issue creation returned $READER_ISSUE_STATUS instead of 403." >&2
  exit 1
fi

curl --fail --silent --show-error -X POST "$BASE_URL/api/v1/repositories/builder/transport-smoke/issues/$ISSUE_NUMBER/comments" -H "Authorization: Bearer $READER_TOKEN" -H "Content-Type: application/json" -d '{"body":"Reader can comment on a repository they can read."}' >/dev/null
curl --fail --silent --show-error -X PUT "$BASE_URL/api/v1/repositories/builder/transport-smoke/issues/$ISSUE_NUMBER" -H "Authorization: Bearer $BUILDER_TOKEN" -H "Content-Type: application/json" -d '{"state":"closed"}' | jq -e '.state == "closed"' >/dev/null

COMMENT_STATUS="$(curl --silent --output /dev/null --write-out '%{http_code}' -X POST "$BASE_URL/api/v1/repositories/builder/transport-smoke/issues/$ISSUE_NUMBER/comments" -H "Authorization: Bearer $READER_TOKEN" -H "Content-Type: application/json" -d '{"body":"Closed issue comment must be rejected."}')"
if [[ "$COMMENT_STATUS" != "409" ]]; then
  echo "ERROR: comments on a closed issue returned $COMMENT_STATUS instead of 409." >&2
  exit 1
fi

SOURCE_DIR="$TMP_DIR/source"
CLONE_DIR="$TMP_DIR/reader-clone"
REVOKED_CLONE_DIR="$TMP_DIR/revoked-clone"
mkdir -p "$SOURCE_DIR"
git -C "$SOURCE_DIR" init --initial-branch=main >/dev/null
git -C "$SOURCE_DIR" config user.name "R!OTGIT CI"
git -C "$SOURCE_DIR" config user.email "riotgit-ci@example.invalid"
printf 'R!OTGIT Git smart HTTP permission smoke test\n' >"$SOURCE_DIR/README.md"
git -C "$SOURCE_DIR" add README.md
git -C "$SOURCE_DIR" commit -m "test: initial permission smoke commit" >/dev/null
git -C "$SOURCE_DIR" remote add origin "$BASE_URL/git/builder/transport-smoke.git"

export GIT_CONFIG_COUNT=1
export GIT_CONFIG_KEY_0="http.extraHeader"
export GIT_CONFIG_VALUE_0="Authorization: Bearer $BUILDER_TOKEN"
git -C "$SOURCE_DIR" push --quiet --set-upstream origin main

git -C "$SOURCE_DIR" checkout -b feature/pull-request-smoke >/dev/null
printf 'R!OTGIT pull request merge smoke marker\n' >"$SOURCE_DIR/PULL_REQUEST.md"
git -C "$SOURCE_DIR" add PULL_REQUEST.md
git -C "$SOURCE_DIR" commit -m "test: create pull request smoke change" >/dev/null
git -C "$SOURCE_DIR" push --quiet --set-upstream origin feature/pull-request-smoke
git -C "$SOURCE_DIR" checkout main >/dev/null


curl --fail --silent --show-error -X POST "$BASE_URL/api/v1/repositories/builder/transport-smoke/branch-protection" -H "Authorization: Bearer $BUILDER_TOKEN" -H "Content-Type: application/json" -d '{"branch_name":"main","require_pull_request":true,"prevent_force_push":true,"prevent_deletion":true}' >/dev/null
printf "protected branch direct push must fail\n" >>"$SOURCE_DIR/README.md"
git -C "$SOURCE_DIR" add README.md
git -C "$SOURCE_DIR" commit -m "test: protected direct push must fail" >/dev/null
if git -C "$SOURCE_DIR" push origin main >/dev/null 2>&1; then
  echo "ERROR: direct push to protected main branch was allowed." >&2
  exit 1
fi
curl --fail --silent --show-error -X DELETE "$BASE_URL/api/v1/repositories/builder/transport-smoke/branch-protection?branch_name=main" -H "Authorization: Bearer $BUILDER_TOKEN" >/dev/null
git -C "$SOURCE_DIR" push --quiet origin main
PR_NUMBER="$(curl --fail --silent --show-error -X POST "$BASE_URL/api/v1/repositories/builder/transport-smoke/pulls" -H "Authorization: Bearer $BUILDER_TOKEN" -H "Content-Type: application/json" -d '{"title":"Pull request merge smoke","body":"Validate protected merge requires an approval.","head_ref":"feature/pull-request-smoke","base_ref":"main"}' | jq -er '.number')"
curl --fail --silent --show-error -X POST "$BASE_URL/api/v1/repositories/builder/transport-smoke/branch-protection" -H "Authorization: Bearer $BUILDER_TOKEN" -H "Content-Type: application/json" -d '{"branch_name":"main","require_pull_request":true,"prevent_force_push":true,"prevent_deletion":true}' >/dev/null

MERGE_BLOCK_STATUS="$(curl --silent --output /dev/null --write-out '%{http_code}' -X POST "$BASE_URL/api/v1/repositories/builder/transport-smoke/pulls/$PR_NUMBER/merge" -H "Authorization: Bearer $BUILDER_TOKEN")"
if [[ "$MERGE_BLOCK_STATUS" != "409" ]]; then
  echo "ERROR: protected PR merge without approval returned $MERGE_BLOCK_STATUS instead of 409." >&2
  exit 1
fi

curl --fail --silent --show-error -X POST "$BASE_URL/api/v1/repositories/builder/transport-smoke/pulls/$PR_NUMBER/reviews" -H "Authorization: Bearer $READER_TOKEN" -H "Content-Type: application/json" -d '{"state":"approved","body":"Smoke reviewer approval."}' >/dev/null

MERGED_STATE="$(curl --fail --silent --show-error -X POST "$BASE_URL/api/v1/repositories/builder/transport-smoke/pulls/$PR_NUMBER/merge" -H "Authorization: Bearer $BUILDER_TOKEN" | jq -er '.state')"
if [[ "$MERGED_STATE" != "merged" ]]; then
  echo "ERROR: pull request did not reach merged state." >&2
  exit 1
fi

curl --fail --silent --show-error -X DELETE "$BASE_URL/api/v1/repositories/builder/transport-smoke/branch-protection?branch_name=main" -H "Authorization: Bearer $BUILDER_TOKEN" >/dev/null
git -C "$SOURCE_DIR" fetch --quiet origin main
git -C "$SOURCE_DIR" reset --hard origin/main >/dev/null
git -C "$SOURCE_DIR" cat-file -e origin/main:PULL_REQUEST.md

printf 'R!OTGIT diff smoke marker\n' >"$SOURCE_DIR/DIFF.txt"
git -C "$SOURCE_DIR" add DIFF.txt
git -C "$SOURCE_DIR" commit -m "test: add diff smoke marker" >/dev/null
git -C "$SOURCE_DIR" push --quiet origin main
DIFF_OUTPUT="$(curl --fail --silent --show-error "$BASE_URL/api/v1/repositories/builder/transport-smoke/diff?base=HEAD%7E1&head=HEAD" -H "Authorization: Bearer $BUILDER_TOKEN" | jq -er '.diff')"
if [[ "$DIFF_OUTPUT" != *"R!OTGIT diff smoke marker"* ]]; then
  echo "ERROR: repository diff endpoint did not return the pushed change." >&2
  exit 1
fi

export GIT_CONFIG_VALUE_0="Authorization: Bearer $READER_TOKEN"
curl --fail --silent --show-error "$BASE_URL/api/v1/repositories/builder/transport-smoke" -H "Authorization: Bearer $READER_TOKEN" >/dev/null
git clone --quiet "$BASE_URL/git/builder/transport-smoke.git" "$CLONE_DIR"
cmp "$SOURCE_DIR/README.md" "$CLONE_DIR/README.md"

printf 'reader write must be rejected\n' >>"$CLONE_DIR/README.md"
git -C "$CLONE_DIR" config user.name "R!OTGIT Reader"
git -C "$CLONE_DIR" config user.email "reader@example.invalid"
git -C "$CLONE_DIR" add README.md
git -C "$CLONE_DIR" commit -m "test: reader write must fail" >/dev/null
if git -C "$CLONE_DIR" push origin main >/dev/null 2>&1; then
  echo "ERROR: read-only user was allowed to push." >&2
  exit 1
fi

export GIT_CONFIG_VALUE_0="Authorization: Bearer $BUILDER_TOKEN"
curl --fail --silent --show-error -X DELETE "$BASE_URL/api/v1/repositories/builder/transport-smoke/access/reader" -H "Authorization: Bearer $BUILDER_TOKEN" >/dev/null

export GIT_CONFIG_VALUE_0="Authorization: Bearer $READER_TOKEN"
if git clone --quiet "$BASE_URL/git/builder/transport-smoke.git" "$REVOKED_CLONE_DIR" >/dev/null 2>&1; then
  echo "ERROR: revoked read permission still allowed cloning." >&2
  exit 1
fi

curl --fail --silent --show-error -X POST "$BASE_URL/api/v1/auth/logout" -H "Authorization: Bearer $READER_TOKEN" >/dev/null
if curl --fail --silent "$BASE_URL/api/v1/auth/me" -H "Authorization: Bearer $READER_TOKEN" >/dev/null; then
  echo "ERROR: logged-out session was still accepted." >&2
  exit 1
fi

echo "R!OTGIT smoke passed: auth/throttle/SSH keys -> issues/labels/milestones -> branch protection -> PR approval/merge -> diff -> Git clone/push -> revoke -> logout."
