#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
MATRIX_DIR="$ROOT_DIR/dev/matrix"
RUNTIME_DIR="$MATRIX_DIR/runtime"
STATE_FILE="$RUNTIME_DIR/state.env"
RUNTIME_CONFIG_DIR="$RUNTIME_DIR/config"
RUNTIME_CONFIG_FILE="$RUNTIME_CONFIG_DIR/vibewindow.json"
COMPOSE_FILE="$MATRIX_DIR/docker-compose.yml"

HOMESERVER="${MATRIX_HOMESERVER:-http://localhost:8008}"
SERVER_NAME="${MATRIX_SERVER_NAME:-localhost}"
BOT_USER="${MATRIX_BOT_USER:-vibewindow-bot}"
BOT_PASSWORD="${MATRIX_BOT_PASSWORD:-vibewindow-local-bot-password}"
DEV_USER="${MATRIX_DEV_USER:-developer}"
DEV_PASSWORD="${MATRIX_DEV_PASSWORD:-vibewindow-local-dev-password}"
TEST_USER="${MATRIX_TEST_USER:-tester}"
TEST_PASSWORD="${MATRIX_TEST_PASSWORD:-vibewindow-local-tester-password}"
REVIEW_USER="${MATRIX_REVIEW_USER:-reviewer}"
REVIEW_PASSWORD="${MATRIX_REVIEW_PASSWORD:-vibewindow-local-review-password}"
ROOM_ALIAS_LOCALPART="${MATRIX_ROOM_ALIAS:-vibewindow-scheduler}"
REGISTRATION_SECRET="${MATRIX_REGISTRATION_SECRET:-vibewindow-local-registration-secret}"
GATEWAY_URL="${VIBEWINDOW_GATEWAY_URL:-http://127.0.0.1:14682}"

mkdir -p "$RUNTIME_DIR" "$RUNTIME_CONFIG_DIR"

VIBEWINDOW_CMD=()

compose() {
  docker compose -f "$COMPOSE_FILE" "$@"
}

json_get() {
  python3 -c 'import json,sys; print(json.load(sys.stdin)[sys.argv[1]])' "$1"
}

urlencode() {
  python3 -c 'import sys, urllib.parse; print(urllib.parse.quote(sys.argv[1], safe=""))' "$1"
}

now_ms() {
  python3 - <<'PY'
import time
print(int(time.time() * 1000))
PY
}

load_state() {
  if [[ -f "$STATE_FILE" ]]; then
    # shellcheck disable=SC1090
    source "$STATE_FILE"
  fi
}

require_state_value() {
  local name="$1"
  local value="${!name:-}"
  if [[ -z "$value" ]]; then
    echo "Missing $name; run dev/matrix/matrix-local.sh up first" >&2
    return 1
  fi
}

gateway_api_base() {
  local base="${GATEWAY_URL%/}"
  if [[ "$base" == */v1 ]]; then
    printf '%s' "$base"
  else
    printf '%s/v1' "$base"
  fi
}

project_ref() {
  printf '%s' "${PROJECT_ID:-${PROJECT_PATH:-$ROOT_DIR}}"
}

project_dir() {
  printf '%s' "${PROJECT_PATH:-$ROOT_DIR}"
}

resolve_vibewindow_cmd() {
  if [[ ${#VIBEWINDOW_CMD[@]} -gt 0 ]]; then
    return 0
  fi

  if [[ -n "${VIBEWINDOW_BIN:-}" ]]; then
    VIBEWINDOW_CMD=("$VIBEWINDOW_BIN")
    return 0
  fi

  if [[ -x "$ROOT_DIR/target/debug/vibewindow" ]]; then
    VIBEWINDOW_CMD=("$ROOT_DIR/target/debug/vibewindow")
    return 0
  fi

  if command -v vibewindow >/dev/null 2>&1; then
    VIBEWINDOW_CMD=("vibewindow")
    return 0
  fi

  if ! command -v cargo >/dev/null 2>&1; then
    echo "Missing vibewindow binary and cargo fallback" >&2
    return 1
  fi

  VIBEWINDOW_CMD=(
    "cargo" "run"
    "--manifest-path" "$ROOT_DIR/Cargo.toml"
    "-p" "vw-cli"
    "--features" "channel-matrix"
    "--bin" "vibewindow"
    "--"
  )
}

running_matrix_worker_pids() {
  local project="$1"
  local worker_id="$2"
  ps -axo pid=,command= |
    python3 -c '
import shlex
import sys

project = sys.argv[1]
worker_id = sys.argv[2]

def option_value(args: list[str], name: str) -> str | None:
    for index, value in enumerate(args):
        if value == name and index + 1 < len(args):
            return args[index + 1]
        prefix = f"{name}="
        if value.startswith(prefix):
            return value[len(prefix):]
    return None

for line in sys.stdin:
    line = line.strip()
    if not line:
        continue
    pid, _, command = line.partition(" ")
    if "agent-scheduler" not in command or "matrix-worker" not in command:
        continue
    try:
        args = shlex.split(command)
    except ValueError:
        args = command.split()
    if option_value(args, "--project-dir") == project and option_value(args, "--worker-id") == worker_id:
        print(pid)
' "$project" "$worker_id"
}

stop_existing_matrix_workers() {
  local project="$1"
  local worker_id="$2"
  local pid existing=()
  while IFS= read -r pid; do
    [[ -n "$pid" ]] && existing+=("$pid")
  done < <(running_matrix_worker_pids "$project" "$worker_id")

  if [[ ${#existing[@]} -eq 0 ]]; then
    return 0
  fi

  echo "Stopping existing Matrix worker $worker_id for $project: ${existing[*]}" >&2
  kill "${existing[@]}" 2>/dev/null || true

  local attempt live=()
  for attempt in 1 2 3 4 5; do
    live=()
    for pid in "${existing[@]}"; do
      kill -0 "$pid" 2>/dev/null && live+=("$pid")
    done
    if [[ ${#live[@]} -eq 0 ]]; then
      return 0
    fi
    sleep 1
  done

  echo "Force stopping existing Matrix worker $worker_id: ${live[*]}" >&2
  kill -9 "${live[@]}" 2>/dev/null || true
}

export_gateway_env_from_url() {
  local parsed
  parsed="$(python3 - "$GATEWAY_URL" <<'PY'
import sys
from urllib.parse import urlparse

url = sys.argv[1]
parsed = urlparse(url if "://" in url else f"http://{url}")
host = parsed.hostname or "127.0.0.1"
port = parsed.port or (443 if parsed.scheme == "https" else 80)
print(f"{host}\n{port}")
PY
)"
  export VIBEWINDOW_GATEWAY_HOST
  export VIBEWINDOW_GATEWAY_PORT
  VIBEWINDOW_GATEWAY_HOST="$(printf '%s' "$parsed" | sed -n '1p')"
  VIBEWINDOW_GATEWAY_PORT="$(printf '%s' "$parsed" | sed -n '2p')"
}

worker_id_for_profile() {
  case "$1" in
    dev) printf '%s' "matrix-dev" ;;
    test) printf '%s' "matrix-test" ;;
    review) printf '%s' "matrix-review" ;;
    *)
      echo "Unknown worker profile: $1 (expected dev, test, review)" >&2
      return 1
      ;;
  esac
}

worker_role_for_profile() {
  case "$1" in
    dev) printf '%s' "executor" ;;
    test) printf '%s' "planner" ;;
    review) printf '%s' "reviewer" ;;
    *)
      echo "Unknown worker profile: $1 (expected dev, test, review)" >&2
      return 1
      ;;
  esac
}

worker_user_id_for_profile() {
  case "$1" in
    dev) printf '%s' "${MATRIX_DEV_USER_ID:-}" ;;
    test) printf '%s' "${MATRIX_TEST_USER_ID:-}" ;;
    review) printf '%s' "${MATRIX_REVIEW_USER_ID:-}" ;;
    *)
      echo "Unknown worker profile: $1 (expected dev, test, review)" >&2
      return 1
      ;;
  esac
}

wait_for_synapse() {
  for _ in {1..90}; do
    if curl -fsS "$HOMESERVER/_matrix/client/versions" >/dev/null; then
      return 0
    fi
    sleep 2
  done
  echo "Synapse did not become ready at $HOMESERVER" >&2
  return 1
}

register_user() {
  local user="$1"
  local password="$2"
  compose exec -T synapse register_new_matrix_user \
    -c /data/homeserver.yaml \
    -u "$user" \
    -p "$password" \
    --no-admin \
    "http://127.0.0.1:8008" >/dev/null || true
}

login_user() {
  local user="$1"
  local password="$2"
  local body
  body="$(python3 - <<PY
import json
print(json.dumps({
    "type": "m.login.password",
    "identifier": {"type": "m.id.user", "user": "$user"},
    "password": "$password",
    "device_id": "VIBEWINDOW_LOCAL",
}))
PY
)"
  curl -fsS -X POST "$HOMESERVER/_matrix/client/v3/login" \
    -H 'Content-Type: application/json' \
    --data "$body"
}

matrix_post() {
  local token="$1"
  local path="$2"
  local body="$3"
  curl -fsS -X POST "$HOMESERVER$path" \
    -H "Authorization: Bearer $token" \
    -H 'Content-Type: application/json' \
    --data "$body"
}

matrix_put() {
  local token="$1"
  local path="$2"
  local body="$3"
  curl -fsS -X PUT "$HOMESERVER$path" \
    -H "Authorization: Bearer $token" \
    -H 'Content-Type: application/json' \
    --data "$body"
}

ensure_room() {
  local token="$1"
  local response room_id
  response="$(matrix_post "$token" "/_matrix/client/v3/createRoom" "$(python3 - <<PY
import json
print(json.dumps({
    "room_alias_name": "$ROOM_ALIAS_LOCALPART",
    "name": "VibeWindow Scheduler Local",
    "preset": "private_chat",
    "visibility": "private",
}))
PY
)")" || true
  if [[ "$response" == *'"room_id"'* ]]; then
    room_id="$(printf '%s' "$response" | json_get room_id)"
  else
    local alias encoded_alias lookup
    alias="#$ROOM_ALIAS_LOCALPART:$SERVER_NAME"
    encoded_alias="$(urlencode "$alias")"
    lookup="$(curl -fsS "$HOMESERVER/_matrix/client/v3/directory/room/$encoded_alias")"
    room_id="$(printf '%s' "$lookup" | json_get room_id)"
  fi

  printf '%s' "$room_id"
}

invite_user() {
  local token="$1"
  local room_id="$2"
  local user_id="$3"
  local invite_body
  invite_body="$(python3 - <<PY
import json
print(json.dumps({"user_id": "$user_id"}))
PY
)"
  matrix_post "$token" "/_matrix/client/v3/rooms/$(urlencode "$room_id")/invite" "$invite_body" >/dev/null || true
}

join_room() {
  local token="$1"
  local room_id="$2"
  matrix_post "$token" "/_matrix/client/v3/join/$(urlencode "$room_id")" '{}' >/dev/null || true
}

write_state() {
  local bot_token="$1"
  local dev_token="$2"
  local tester_token="$3"
  local review_token="$4"
  local room_id="$5"
  cat > "$STATE_FILE" <<EOF
MATRIX_HOMESERVER="$HOMESERVER"
MATRIX_SERVER_NAME="$SERVER_NAME"
MATRIX_BOT_USER_ID="@$BOT_USER:$SERVER_NAME"
MATRIX_DEV_USER_ID="@$DEV_USER:$SERVER_NAME"
MATRIX_TEST_USER_ID="@$TEST_USER:$SERVER_NAME"
MATRIX_REVIEW_USER_ID="@$REVIEW_USER:$SERVER_NAME"
MATRIX_BOT_ACCESS_TOKEN="$bot_token"
MATRIX_DEV_ACCESS_TOKEN="$dev_token"
MATRIX_TEST_ACCESS_TOKEN="$tester_token"
MATRIX_REVIEW_ACCESS_TOKEN="$review_token"
MATRIX_ROOM_ID="$room_id"
MATRIX_ROOM_ALIAS="#$ROOM_ALIAS_LOCALPART:$SERVER_NAME"
EOF

  python3 - "$RUNTIME_CONFIG_FILE" "$GATEWAY_URL" "$HOMESERVER" "$room_id" \
    "@$BOT_USER:$SERVER_NAME" "$bot_token" \
    "@$DEV_USER:$SERVER_NAME" "$dev_token" \
    "@$TEST_USER:$SERVER_NAME" "$tester_token" \
    "@$REVIEW_USER:$SERVER_NAME" "$review_token" <<'PY'
import json
import pathlib
import sys
from urllib.parse import urlparse

(
    config_path,
    gateway_url,
    homeserver,
    room_id,
    bot_user_id,
    bot_token,
    dev_user_id,
    dev_token,
    test_user_id,
    test_token,
    review_user_id,
    review_token,
) = sys.argv[1:]

parsed_gateway = urlparse(gateway_url if "://" in gateway_url else f"http://{gateway_url}")
gateway_host = parsed_gateway.hostname or "127.0.0.1"
gateway_port = parsed_gateway.port or (443 if parsed_gateway.scheme == "https" else 80)
allowed_users = [bot_user_id, dev_user_id, test_user_id, review_user_id]

def matrix_config(user_id: str, token: str, device_id: str) -> dict[str, object]:
    return {
        "homeserver": homeserver,
        "access_token": token,
        "user_id": user_id,
        "device_id": device_id,
        "room_id": room_id,
        "allowed_users": allowed_users,
        "mention_only": False,
    }

def staff_member(
    member_id: str,
    name: str,
    role: str,
    level: str,
    matrix_instance_id = None,
) -> dict[str, object]:
    channel_bindings = []
    if matrix_instance_id:
        channel_bindings.append({
            "channel_type": "matrix",
            "instance_id": matrix_instance_id,
        })
    return {
        "id": member_id,
        "name": name,
        "role": role,
        "level": level,
        "status": "online",
        "enabled": True,
        "capacity": 1,
        "delegate_agent": "main",
        "channel_bindings": channel_bindings,
    }

config = {
    "default_temperature": 0.7,
    "gateway": {
        "host": gateway_host,
        "port": gateway_port,
    },
    "channels_config": {
        "cli": True,
        "matrix": matrix_config(bot_user_id, bot_token, "VIBEWINDOW_LOCAL_BOT"),
        "matrix_instances": [
            {"id": "matrix-dev", **matrix_config(dev_user_id, dev_token, "VIBEWINDOW_LOCAL_DEV")},
            {"id": "matrix-test", **matrix_config(test_user_id, test_token, "VIBEWINDOW_LOCAL_TEST")},
            {"id": "matrix-review", **matrix_config(review_user_id, review_token, "VIBEWINDOW_LOCAL_REVIEW")},
        ],
    },
    "staff": {
        "members": [
            staff_member("staff-orchestrator", "Orchestrator", "orchestrator", "principal"),
            staff_member("staff-planner", "Planner", "planner", "manager", "matrix-test"),
            staff_member("staff-executor", "Executor", "executor", "executor", "matrix-dev"),
            staff_member("staff-reviewer", "Reviewer", "reviewer", "manager", "matrix-review"),
            staff_member("staff-merger", "Merger", "merger", "manager"),
        ],
    },
}

path = pathlib.Path(config_path)
path.parent.mkdir(parents=True, exist_ok=True)
path.write_text(json.dumps(config, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
PY
  chmod 600 "$RUNTIME_CONFIG_FILE"
}

setup() {
  wait_for_synapse
  register_user "$BOT_USER" "$BOT_PASSWORD"
  register_user "$DEV_USER" "$DEV_PASSWORD"
  register_user "$TEST_USER" "$TEST_PASSWORD"
  register_user "$REVIEW_USER" "$REVIEW_PASSWORD"

  local bot_login dev_login tester_login review_login bot_token dev_token tester_token review_token room_id
  bot_login="$(login_user "$BOT_USER" "$BOT_PASSWORD")"
  dev_login="$(login_user "$DEV_USER" "$DEV_PASSWORD")"
  tester_login="$(login_user "$TEST_USER" "$TEST_PASSWORD")"
  review_login="$(login_user "$REVIEW_USER" "$REVIEW_PASSWORD")"
  bot_token="$(printf '%s' "$bot_login" | json_get access_token)"
  dev_token="$(printf '%s' "$dev_login" | json_get access_token)"
  tester_token="$(printf '%s' "$tester_login" | json_get access_token)"
  review_token="$(printf '%s' "$review_login" | json_get access_token)"
  room_id="$(ensure_room "$bot_token")"
  invite_user "$bot_token" "$room_id" "@$DEV_USER:$SERVER_NAME"
  invite_user "$bot_token" "$room_id" "@$TEST_USER:$SERVER_NAME"
  invite_user "$bot_token" "$room_id" "@$REVIEW_USER:$SERVER_NAME"
  join_room "$dev_token" "$room_id"
  join_room "$tester_token" "$room_id"
  join_room "$review_token" "$room_id"
  write_state "$bot_token" "$dev_token" "$tester_token" "$review_token" "$room_id"

  echo "Synapse: $HOMESERVER"
  echo "Element: http://localhost:8081"
  echo "Room: $room_id (#$ROOM_ALIAS_LOCALPART:$SERVER_NAME)"
  echo "Workers: matrix-dev, matrix-test, matrix-review"
  echo "Config: $RUNTIME_CONFIG_FILE"
}

send_text() {
  load_state
  local message="${1:-scheduler smoke test}"
  local txn_id="vw-local-$(date +%s%N)"
  local body
  body="$(python3 - <<PY
import json
print(json.dumps({"msgtype": "m.text", "body": "$message"}))
PY
)"
  matrix_put "$MATRIX_TEST_ACCESS_TOKEN" \
    "/_matrix/client/v3/rooms/$(urlencode "$MATRIX_ROOM_ID")/send/m.room.message/$txn_id" \
    "$body" >/dev/null
  echo "sent text to $MATRIX_ROOM_ID"
}

send_task_event() {
  load_state
  local task_id="${1:-T-local-1}"
  local status="${2:-running}"
  local txn_id="vw-task-$(date +%s%N)"
  local now_ms
  now_ms="$(python3 - <<'PY'
import time
print(int(time.time() * 1000))
PY
)"
  local body
  body="$(python3 - <<PY
import json
print(json.dumps({
    "version": 1,
    "event_id": "local:%s:%s:%s" % ("$task_id", "$status", "$now_ms"),
    "task_id": "$task_id",
    "actor": {
        "matrix_user_id": "$MATRIX_TEST_USER_ID",
        "display_name": "Local Tester",
    },
    "occurred_at_ms": int("$now_ms"),
    "kind": {
        "type": "task_status_changed",
        "to_status": "$status",
    },
}))
PY
)"
  matrix_put "$MATRIX_TEST_ACCESS_TOKEN" \
    "/_matrix/client/v3/rooms/$(urlencode "$MATRIX_ROOM_ID")/send/com.vibewindow.task_pool.event.v1/$txn_id" \
    "$body" >/dev/null
  echo "sent task event for $task_id -> $status"
}

sync_events() {
  load_state
  curl -fsS "$HOMESERVER/_matrix/client/v3/rooms/$(urlencode "$MATRIX_ROOM_ID")/messages?dir=b&limit=10" \
    -H "Authorization: Bearer $MATRIX_TEST_ACCESS_TOKEN" |
    python3 -c '
import json, sys
data = json.load(sys.stdin)
for event in data.get("chunk", []):
    print(json.dumps({
        "type": event.get("type"),
        "sender": event.get("sender"),
        "content": event.get("content"),
    }, ensure_ascii=False))
'
}

upsert_worker_snapshot() {
  local worker_id="$1"
  local matrix_user_id="$2"
  local worker_role="$3"
  local status="$4"
  local desired_state="$5"
  local container_state="$6"
  local gateway_base project encoded_project body timestamp worker_model
  gateway_base="$(gateway_api_base)"
  project="$(project_ref)"
  encoded_project="$(urlencode "$project")"
  timestamp="$(now_ms)"
  worker_model="${WORKER_MODEL:-auto}"
  body="$(python3 - "$worker_id" "$matrix_user_id" "$worker_role" "$status" \
    "$desired_state" "$container_state" "$MATRIX_ROOM_ID" "$timestamp" "$worker_model" <<'PY'
import json
import sys

(
    worker_id,
    matrix_user_id,
    worker_role,
    status,
    desired_state,
    container_state,
    room_id,
    timestamp,
    worker_model,
) = sys.argv[1:]
print(json.dumps({
    "worker_id": worker_id,
    "runtime": "matrix",
    "model": worker_model,
    "status": status,
    "desired_state": desired_state,
    "worker_role": worker_role,
    "matrix_user_id": matrix_user_id,
    "room_id": room_id,
    "container_state": container_state,
    "last_seen_at_ms": int(timestamp),
    "updated_at_ms": int(timestamp),
    "capabilities": [
        {"kind": "runtime", "name": "matrix"},
        {"kind": "model", "name": worker_model},
    ],
}))
PY
)"
  curl -fsS -X POST "$gateway_base/agent-scheduler/projects/$encoded_project/workers" \
    -H 'Content-Type: application/json' \
    --data "$body" >/dev/null
}

register_worker_profile() {
  load_state
  require_state_value MATRIX_ROOM_ID
  local profile="${1:-test}"
  local worker_id worker_role matrix_user_id
  worker_id="$(worker_id_for_profile "$profile")"
  worker_role="$(worker_role_for_profile "$profile")"
  matrix_user_id="$(worker_user_id_for_profile "$profile")"
  if [[ -z "$matrix_user_id" ]]; then
    echo "Missing Matrix user id for profile: $profile" >&2
    return 1
  fi

  upsert_worker_snapshot "$worker_id" "$matrix_user_id" "$worker_role" "idle" "Running" "running"
  echo "Registered worker $worker_id room $MATRIX_ROOM_ID"
}

disable_legacy_worker() {
  load_state
  require_state_value MATRIX_ROOM_ID
  upsert_worker_snapshot \
    "matrix-local" \
    "${MATRIX_TEST_USER_ID:-@tester:$SERVER_NAME}" \
    "worker" \
    "unavailable" \
    "Stopped" \
    "unavailable"
}

register_all_workers() {
  for profile in dev test review; do
    register_worker_profile "$profile"
  done
  disable_legacy_worker
}

run_matrix_worker() {
  load_state
  require_state_value MATRIX_ROOM_ID
  if [[ ! -f "$RUNTIME_CONFIG_FILE" ]]; then
    echo "Missing $RUNTIME_CONFIG_FILE; run dev/matrix/matrix-local.sh up first" >&2
    return 1
  fi
  local profile="${1:-test}"
  local worker_id
  worker_id="$(worker_id_for_profile "$profile")"
  stop_existing_matrix_workers "$(project_dir)" "$worker_id"
  resolve_vibewindow_cmd
  export_gateway_env_from_url
  echo "Starting worker $worker_id room $MATRIX_ROOM_ID using ${VIBEWINDOW_CMD[*]}"
  "${VIBEWINDOW_CMD[@]}" \
    --config-dir "$RUNTIME_CONFIG_DIR" \
    agent-scheduler matrix-worker \
    --project-dir "$(project_dir)" \
    --worker-id "$worker_id"
}

run_all_workers() {
  local pids=()
  local profile worker_id
  for profile in dev test review; do
    worker_id="$(worker_id_for_profile "$profile")"
    stop_existing_matrix_workers "$(project_dir)" "$worker_id"
  done
  for profile in dev test review; do
    run_matrix_worker "$profile" &
    pids+=("$!")
  done
  trap 'kill "${pids[@]}" 2>/dev/null || true' INT TERM EXIT
  wait "${pids[@]}"
}

unsupported_rust_worker_command() {
  local command_name="$1"
  echo "$command_name is not supported by the Rust Matrix worker runtime." >&2
  echo "Use worker-run <profile> or worker-run-all for continuous execution." >&2
  return 1
}

case "${1:-}" in
  up)
    compose up -d
    setup
    ;;
  setup)
    setup
    ;;
  down)
    compose down
    ;;
  reset)
    compose down -v
    rm -rf "$RUNTIME_DIR"
    ;;
  send-text)
    shift
    send_text "$*"
    ;;
  send-task-event)
    shift
    send_task_event "${1:-T-local-1}" "${2:-running}"
    ;;
  sync)
    sync_events
    ;;
  register-worker)
    shift
    register_worker_profile "${1:-test}"
    ;;
  register-workers)
    register_all_workers
    ;;
  worker-run-once)
    shift
    unsupported_rust_worker_command "worker-run-once"
    ;;
  worker-run)
    shift
    run_matrix_worker "${1:-test}"
    ;;
  worker-run-all)
    run_all_workers
    ;;
  worker-skip-history)
    shift
    unsupported_rust_worker_command "worker-skip-history"
    ;;
  worker-skip-history-all)
    unsupported_rust_worker_command "worker-skip-history-all"
    ;;
  *)
    cat <<EOF
Usage: $0 <command>

Commands:
  up                         Start Synapse, Element, register users, create room.
  setup                      Recreate runtime tokens and config for running Synapse.
  down                       Stop services.
  reset                      Stop services and delete local Matrix data.
  send-text [message]        Send a normal Matrix text message as tester.
  send-task-event [id] [to]  Send com.vibewindow.task_pool.event.v1.
  sync                       Print recent room events.
  register-worker [profile]  Register one local worker profile: dev, test, review.
  register-workers           Register dev/test/review workers in the same room.
  worker-run [profile]       Run one Rust Matrix worker loop.
  worker-run-all             Run dev/test/review Rust worker loops together.
EOF
    ;;
esac
