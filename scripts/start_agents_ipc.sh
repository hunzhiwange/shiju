#!/usr/bin/env bash
set -euo pipefail

VIBE_DIR="${VIBEWINDOW_CONFIG_DIR:-${HOME}/.vibewindowdev}"
BASE_CFG="${VIBE_DIR}/vibewindow.json"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

AGENT_A_DIR="${VIBE_DIR}/agent-a"
AGENT_B_DIR="${VIBE_DIR}/agent-b"

CFG_A="${AGENT_A_DIR}/vibewindow.json"
CFG_B="${AGENT_B_DIR}/vibewindow.json"

WORK_A="${AGENT_A_DIR}/workspace"
WORK_B="${AGENT_B_DIR}/workspace"

RUN_DIR="${VIBE_DIR}/run"
LOG_DIR="${VIBE_DIR}/logs"

PID_A="${RUN_DIR}/agent-a.pid"
PID_B="${RUN_DIR}/agent-b.pid"
LOG_A="${LOG_DIR}/agent-a.log"
LOG_B="${LOG_DIR}/agent-b.log"

ACTION="${1:-start}"

RUN_CMD=()

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    printf 'Missing command: %s\n' "$1"
    exit 1
  }
}

is_running() {
  local pid_file="$1"
  [[ -f "$pid_file" ]] || return 1
  local pid
  pid="$(<"$pid_file")"
  [[ -n "$pid" ]] || return 1
  kill -0 "$pid" 2>/dev/null
}

ensure_ipc_config() {
  need_cmd python3
  local cfg_path="$1"
  python3 - "$cfg_path" <<'PY'
import json
import pathlib
import sys

cfg_path = pathlib.Path(sys.argv[1])
if not cfg_path.exists():
    print(f"Config not found: {cfg_path}")
    sys.exit(1)

data = json.loads(cfg_path.read_text(encoding="utf-8"))
agent = data.setdefault("agent", {})
ipc = agent.setdefault("agents_ipc", {})
ipc["enabled"] = True
ipc.setdefault("db_path", "~/.vibewindowdev/agents.db")
ipc.setdefault("staleness_secs", 300)

cfg_path.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
print("agents_ipc enabled in config")
PY
}

ensure_agent_config() {
  local agent_name="$1"
  local cfg_path="$2"
  local agent_dir
  agent_dir="$(dirname "$cfg_path")"

  mkdir -p "$agent_dir"
  if [[ ! -f "$cfg_path" ]]; then
    if [[ ! -f "$BASE_CFG" ]]; then
      printf 'Base config not found: %s\n' "$BASE_CFG"
      exit 1
    fi
    cp "$BASE_CFG" "$cfg_path"
    printf 'Created %s config: %s\n' "$agent_name" "$cfg_path"
  fi

  ensure_ipc_config "$cfg_path"
}

resolve_run_cmd() {
  if command -v shiju >/dev/null 2>&1; then
    RUN_CMD=("shiju")
    return
  fi

  need_cmd cargo
  RUN_CMD=("cargo" "run" "--manifest-path" "${REPO_DIR}/Cargo.toml" "--bin" "shiju" "--")
}

start_one() {
  local name="$1"
  local workdir="$2"
  local config_dir="$3"
  local pid_file="$4"
  local log_file="$5"

  mkdir -p "$workdir"

  if is_running "$pid_file"; then
    printf '%s already running (pid=%s)\n' "$name" "$(<"$pid_file")"
    return 0
  fi

  (
    cd "$workdir"
    nohup "${RUN_CMD[@]}" --config-dir "$config_dir" agent >>"$log_file" 2>&1 &
    echo $! >"$pid_file"
  )

  printf 'Started %s (pid=%s, config_dir=%s)\n' "$name" "$(<"$pid_file")" "$config_dir"
}

stop_one() {
  local name="$1"
  local pid_file="$2"

  if [[ ! -f "$pid_file" ]]; then
    printf '%s not running (no pid file)\n' "$name"
    return 0
  fi

  local pid
  pid="$(<"$pid_file")"
  if [[ -z "$pid" ]]; then
    rm -f "$pid_file"
    printf '%s pid file empty, cleaned\n' "$name"
    return 0
  fi

  if kill -0 "$pid" 2>/dev/null; then
    kill "$pid" || true
    sleep 1
    if kill -0 "$pid" 2>/dev/null; then
      kill -9 "$pid" || true
    fi
    printf 'Stopped %s (pid=%s)\n' "$name" "$pid"
  else
    printf '%s already stopped (stale pid=%s)\n' "$name" "$pid"
  fi

  rm -f "$pid_file"
}

status_one() {
  local name="$1"
  local pid_file="$2"
  if is_running "$pid_file"; then
    printf '%s: running (pid=%s)\n' "$name" "$(<"$pid_file")"
  else
    printf '%s: stopped\n' "$name"
  fi
}

start_all() {
  resolve_run_cmd
  mkdir -p "$RUN_DIR" "$LOG_DIR"

  ensure_agent_config "agent-a" "$CFG_A"
  ensure_agent_config "agent-b" "$CFG_B"

  mkdir -p "$WORK_A" "$WORK_B"

  start_one "agent-a" "$WORK_A" "$AGENT_A_DIR" "$PID_A" "$LOG_A"
  start_one "agent-b" "$WORK_B" "$AGENT_B_DIR" "$PID_B" "$LOG_B"

  printf '\nBoth agents started\n'
  printf 'Logs:\n  %s\n  %s\n' "$LOG_A" "$LOG_B"
  printf 'Config dirs:\n  %s\n  %s\n' "$AGENT_A_DIR" "$AGENT_B_DIR"
}

stop_all() {
  stop_one "agent-a" "$PID_A"
  stop_one "agent-b" "$PID_B"
}

status_all() {
  status_one "agent-a" "$PID_A"
  status_one "agent-b" "$PID_B"
}

logs_all() {
  printf 'agent-a log: %s\n' "$LOG_A"
  if [[ -f "$LOG_A" ]]; then
    tail -n 50 "$LOG_A"
  else
    printf '(no log)\n'
  fi
  printf '\nagent-b log: %s\n' "$LOG_B"
  if [[ -f "$LOG_B" ]]; then
    tail -n 50 "$LOG_B"
  else
    printf '(no log)\n'
  fi
}

case "$ACTION" in
start)
  start_all
  ;;
stop)
  stop_all
  ;;
restart)
  stop_all
  start_all
  ;;
status)
  status_all
  ;;
logs)
  logs_all
  ;;
*)
  printf 'Usage: %s {start|stop|restart|status|logs}\n' "$0"
  exit 1
  ;;
esac
