#!/usr/bin/env sh
# Manage the Forge web UI and API pair that the operator reaches
# over a browser: the standalone web UI on `forge web serve` and
# the JSON API the web UI calls on `forge api serve`.
#
# Usage: scripts/web.sh {start|stop|restart|status|logs|reset-password}
#
# Ports: API on 127.0.0.1:8766, web UI on 127.0.0.1:4173. Both
# bind loopback by default; the operator may override --api-port
# or --web-port for a different layout. The script never publishes
# a non-loopback bind; the operator runs a reverse proxy in front
# if a public listener is required.
#
# `start` ensures a Forge administrator exists (the credential store
# keeps only an Argon2id hash, so an existing password can never be
# recovered) and prints the login URL, the account email and, when it
# just created the account, the generated password. When an account
# already exists it prints the email and points at `reset-password`
# instead of inventing a password. `reset-password` generates, sets
# and prints a fresh password once; it revokes existing sessions. The
# password is printed to stdout only: never written to .forge/run/state,
# never placed in argv, never logged.
#
# The web UI is vendored with a hard-coded API base URL
# (frontend/config.js). When --api-port is set, the script
# rewrites a copy of config.js under .forge/run/web-root/ so the
# web UI follows the operator's choice; the original
# frontend/config.js is never touched.
#
# State: pidfiles under .forge/run/, log under .forge/log/,
# generated web root under .forge/run/web-root/. All are
# gitignored. FORGE_BIN, FORGE_WEB_RUN_DIR, FORGE_WEB_LOG_DIR and
# FORGE_WEB_ROOT_DIR override the binary and runtime paths (used by
# the contract test to isolate a throwaway run directory).

set -eu

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="${FORGE_BIN:-$ROOT/target/debug/forge}"
API_HOST="127.0.0.1"
API_PORT="8766"
PROJECTS_ROOT=""
WEB_HOST="127.0.0.1"
WEB_PORT="4173"
ADMIN_EMAIL="operator@example.com"
RUN_DIR="${FORGE_WEB_RUN_DIR:-$ROOT/.forge/run}"
LOG_DIR="${FORGE_WEB_LOG_DIR:-$ROOT/.forge/log}"
WEB_ROOT_DIR="${FORGE_WEB_ROOT_DIR:-$RUN_DIR/web-root}"
API_PIDFILE="$RUN_DIR/api.pid"
WEB_PIDFILE="$RUN_DIR/web.pid"
API_LOG="$LOG_DIR/api.log"
WEB_LOG="$LOG_DIR/web.log"
STATE_FILE="$RUN_DIR/state"
DO_BUILD=""

usage() {
  cat >&2 <<EOF
usage: scripts/web.sh {start|stop|restart|status|logs|reset-password} [--api-port <p>] [--web-port <p>] [--projects-root <path>] [--admin-email <address>] [--build]

  start           Build (if --build) and start both services in the background.
                  Ensures an administrator exists and prints the login URL,
                  account email and (on first creation) the password.
  stop            Stop both services.
  restart         stop + start.
  status          Print whether each service is listening, its pid, and its log path.
  logs            Tail both logs (Ctrl-C to exit).
  reset-password  Generate a fresh administrator password, set it
                  non-interactively and print it once. Revokes existing sessions.

Options:
  --api-port        <p>        Override the API listener port (default 8766).
  --web-port        <p>        Override the web UI listener port (default 4173).
  --projects-root   <path>     Set FORGE_ADMIN_PROJECTS_ROOT for the API
                               process so the web UI's bulk Workspace
                               onboarding panel can discover siblings.
  --admin-email     <address>  Administrator email (default operator@example.com).
  --build                      Rebuild the binary with \`cargo build\` before starting.
EOF
}

ensure_layout() {
  mkdir -p "$RUN_DIR" "$LOG_DIR"
}

# Restore the ports the previous `start` chose. The script is
# invoked per-command, so a `status` or `stop` after a custom
# `--api-port` would otherwise see the default. The state file
# is rewritten by `start` and removed by `stop`.
load_state() {
  if [ ! -f "$STATE_FILE" ]; then
    return 0
  fi
  # shellcheck disable=SC1090
  . "$STATE_FILE"
}

save_state() {
  cat > "$STATE_FILE" <<EOF
API_HOST="$API_HOST"
API_PORT="$API_PORT"
WEB_HOST="$WEB_HOST"
WEB_PORT="$WEB_PORT"
PROJECTS_ROOT="$PROJECTS_ROOT"
EOF
}

prepare_web_root() {
  # The vendored frontend/config.js is hard-coded to one API base URL, so a
  # non-default --api-port needs a rewritten config. Rather than editing the
  # tracked file, stage a web root that symlinks every vendored asset and
  # carries a real, generated config.js.
  #
  # Symlinks rather than copies on purpose: a copy is a snapshot taken at
  # `start`, so every edit to frontend/ would need a restart before the
  # browser could see it. A symlink tracks the source, so only config.js
  # (the one file the script genuinely owns) is generated.
  rm -rf "$WEB_ROOT_DIR"
  mkdir -p "$WEB_ROOT_DIR"
  if [ ! -d "$ROOT/frontend" ]; then
    echo "web: frontend/ not found at $ROOT/frontend; refusing to start" >&2
    return 1
  fi
  for entry in "$ROOT/frontend"/*; do
    name="$(basename "$entry")"
    if [ "$name" = "config.js" ]; then
      continue
    fi
    ln -s "$entry" "$WEB_ROOT_DIR/$name"
  done
  api_base="http://$API_HOST:$API_PORT"
  printf '%s\n' \
    '// Generated by scripts/web.sh. Every other file in this directory is a' \
    '// symlink to the vendored frontend/, so edits there are live; only this' \
    '// file is generated, because it carries the operator-selected API port.' \
    "window.FORGE_API_BASE = \"$api_base\";" \
    > "$WEB_ROOT_DIR/config.js"
}

is_listening() {
  port="$1"
  ss -tln 2>/dev/null | awk '{print $4}' | grep -E "(^|:)$port$" >/dev/null 2>&1
}

pid_alive() {
  pidfile="$1"
  [ -f "$pidfile" ] || return 1
  pid="$(cat "$pidfile" 2>/dev/null || true)"
  [ -n "$pid" ] || return 1
  kill -0 "$pid" 2>/dev/null
}

stop_one() {
  label="$1"; pidfile="$2"
  if pid_alive "$pidfile"; then
    pid="$(cat "$pidfile")"
    kill "$pid" 2>/dev/null || true
    i=0
    while [ "$i" -lt 25 ] && kill -0 "$pid" 2>/dev/null; do
      i=$((i + 1))
      sleep 0.2
    done
    if kill -0 "$pid" 2>/dev/null; then
      kill -9 "$pid" 2>/dev/null || true
    fi
    rm -f "$pidfile"
    echo "web: $label stopped (pid $pid)"
  else
    rm -f "$pidfile"
    echo "web: $label not running"
  fi
}

start_one() {
  label="$1"; host="$2"; port="$3"; log="$4"; pidfile="$5"
  shift 5
  if pid_alive "$pidfile"; then
    echo "web: $label already running (pid $(cat "$pidfile")) on http://$host:$port"
    return 0
  fi
  if is_listening "$port"; then
    echo "web: $label port $port is already bound by another process; refusing to start" >&2
    return 1
  fi
  # Build a child-only env so the API process can see the
  # projects root and any other state-bound env the script
  # manages, without leaking the script's own shell state.
  child_env="FORGE_ADMIN_PROJECTS_ROOT=$PROJECTS_ROOT"
  setsid env $child_env nohup "$BIN" "$@" --bind "$host" --port "$port" > "$log" 2>&1 < /dev/null &
  pid=$!
  echo "$pid" > "$pidfile"
  i=0
  while [ "$i" -lt 25 ] && ! is_listening "$port"; do
    i=$((i + 1))
    sleep 0.2
  done
  if is_listening "$port"; then
    if [ -n "$PROJECTS_ROOT" ]; then
      echo "web: $label started (pid $pid) on http://$host:$port; FORGE_ADMIN_PROJECTS_ROOT=$PROJECTS_ROOT; log: $log"
    else
      echo "web: $label started (pid $pid) on http://$host:$port; log: $log"
    fi
  else
    echo "web: $label failed to bind $port; see $log" >&2
    rm -f "$pidfile"
    return 1
  fi
}

admin_status_field() {
  # $1 = `forge identity status` output, $2 = field name
  printf '%s\n' "$1" | sed -n "s/^$2:[[:space:]]*//p" | head -n1
}

print_login_banner() {
  email="$1"
  password="$2"
  printf '\n'
  printf 'web: login URL:  http://%s:%s/\n' "$WEB_HOST" "$WEB_PORT"
  printf 'web: account:    %s\n' "$email"
  if [ -n "$password" ]; then
    printf 'web: password:   %s\n' "$password"
    printf 'web: shown once and not recoverable; store it now.\n'
  else
    printf 'web: password:   (unchanged; run `scripts/web.sh reset-password` for a new one)\n'
  fi
  printf '\n'
}

# Ensure a Forge administrator exists before the services bind. The store
# keeps only an Argon2id hash, so an existing password can never be printed:
# on first creation we set one and print it once; when an account already
# exists we print the email and point at `reset-password`, never inventing
# a password.
ensure_admin() {
  status="$("$BIN" identity status 2>/dev/null || true)"
  configured="$(admin_status_field "$status" configured)"
  current_email="$(admin_status_field "$status" email)"
  if [ "$configured" = "true" ]; then
    print_login_banner "${current_email:-$ADMIN_EMAIL}" ""
    return 0
  fi
  password="$("$BIN" identity generate-password --length 20)"
  if ! printf '%s\n' "$password" | "$BIN" identity setup --email "$ADMIN_EMAIL" --password-stdin >/dev/null; then
    echo "web: failed to initialize the administrator account" >&2
    exit 1
  fi
  print_login_banner "$ADMIN_EMAIL" "$password"
}

cmd_start() {
  ensure_layout
  if [ -n "$DO_BUILD" ]; then
    echo "web: building $BIN"
    (cd "$ROOT" && cargo build)
  fi
  if [ ! -x "$BIN" ]; then
    echo "web: binary $BIN is missing; pass --build to build it" >&2
    exit 1
  fi
  ensure_admin
  save_state
  start_one "api" "$API_HOST" "$API_PORT" "$API_LOG" "$API_PIDFILE" api serve
  prepare_web_root || exit 1
  start_one "web" "$WEB_HOST" "$WEB_PORT" "$WEB_LOG" "$WEB_PIDFILE" web serve --root "$WEB_ROOT_DIR"
}

cmd_reset_password() {
  ensure_layout
  if [ ! -x "$BIN" ]; then
    echo "web: binary $BIN is missing; pass --build to build it" >&2
    exit 1
  fi
  status="$("$BIN" identity status 2>/dev/null || true)"
  configured="$(admin_status_field "$status" configured)"
  current_email="$(admin_status_field "$status" email)"
  password="$("$BIN" identity generate-password --length 20)"
  if [ "$configured" = "true" ]; then
    email="${current_email:-$ADMIN_EMAIL}"
    if ! printf '%s\n' "$password" | "$BIN" identity change-password --password-stdin >/dev/null; then
      echo "web: failed to reset the administrator password" >&2
      exit 1
    fi
    echo "web: existing browser sessions were revoked."
  else
    email="$ADMIN_EMAIL"
    if ! printf '%s\n' "$password" | "$BIN" identity setup --email "$email" --password-stdin >/dev/null; then
      echo "web: failed to initialize the administrator account" >&2
      exit 1
    fi
  fi
  print_login_banner "$email" "$password"
}

cmd_stop() {
  ensure_layout
  stop_one "web" "$WEB_PIDFILE"
  stop_one "api" "$API_PIDFILE"
  rm -rf "$WEB_ROOT_DIR"
  rm -f "$STATE_FILE"
}

cmd_status() {
  ensure_layout
  if [ -n "$PROJECTS_ROOT" ]; then
    printf 'cfg  FORGE_ADMIN_PROJECTS_ROOT=%s\n' "$PROJECTS_ROOT"
  else
    printf 'cfg  FORGE_ADMIN_PROJECTS_ROOT=(unset; bulk workspace onboarding in the web UI is unavailable)\n'
  fi
  for entry in "api:$API_HOST:$API_PORT:$API_PIDFILE:$API_LOG" "web:$WEB_HOST:$WEB_PORT:$WEB_PIDFILE:$WEB_LOG"; do
    label=$(echo "$entry" | cut -d: -f1)
    host=$(echo "$entry" | cut -d: -f2)
    port=$(echo "$entry" | cut -d: -f3)
    pidfile=$(echo "$entry" | cut -d: -f4)
    log=$(echo "$entry" | cut -d: -f5)
    if pid_alive "$pidfile"; then
      pid=$(cat "$pidfile")
      if is_listening "$port"; then
        printf '%-4s up   pid %-7s http://%s:%s  log %s\n' "$label" "$pid" "$host" "$port" "$log"
      else
        printf '%-4s pid  %-7s (not listening on %s); log %s\n' "$label" "$pid" "$port" "$log"
      fi
    elif is_listening "$port"; then
      printf '%-4s up   (no pidfile) http://%s:%s\n' "$label" "$host" "$port"
    else
      printf '%-4s down  (log %s)\n' "$label" "$log"
    fi
  done
}

cmd_logs() {
  ensure_layout
  tail -F "$API_LOG" "$WEB_LOG" 2>/dev/null
}

cmd=${1:-}
[ -n "$cmd" ] || { usage; exit 2; }
shift || true
ensure_layout
load_state
while [ "$#" -gt 0 ]; do
  case "$1" in
    --api-port) [ "$#" -ge 2 ] || { echo "web: --api-port requires a value" >&2; exit 2; }; API_PORT="$2"; shift 2 ;;
    --web-port) [ "$#" -ge 2 ] || { echo "web: --web-port requires a value" >&2; exit 2; }; WEB_PORT="$2"; shift 2 ;;
    --projects-root) [ "$#" -ge 2 ] || { echo "web: --projects-root requires a value" >&2; exit 2; }; PROJECTS_ROOT="$2"; shift 2 ;;
    --admin-email) [ "$#" -ge 2 ] || { echo "web: --admin-email requires a value" >&2; exit 2; }; ADMIN_EMAIL="$2"; shift 2 ;;
    --build)    DO_BUILD=1; shift ;;
    -h|--help)  usage; exit 0 ;;
    *) echo "web: unknown argument '$1'" >&2; usage; exit 2 ;;
  esac
done
case "$cmd" in
  start)   cmd_start ;;
  stop)    cmd_stop ;;
  restart) cmd_stop; cmd_start ;;
  status)  cmd_status ;;
  logs)    cmd_logs ;;
  reset-password) cmd_reset_password ;;
  -h|--help) usage; exit 0 ;;
  *) echo "web: unknown command '$cmd'" >&2; usage; exit 2 ;;
esac
