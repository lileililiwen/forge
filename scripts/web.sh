#!/usr/bin/env sh
# Manage the Forge web UI and API pair that the operator reaches
# over a browser: the standalone web UI on `forge web serve` and
# the JSON API the web UI calls on `forge api serve`.
#
# Usage: scripts/web.sh {start|stop|restart|status|logs}
#
# Ports: API on 127.0.0.1:8765, web UI on 127.0.0.1:4173. Both
# bind loopback by default; the operator may override --api-port
# or --web-port for a different layout. The script never publishes
# a non-loopback bind; the operator runs a reverse proxy in front
# if a public listener is required.
#
# State: pidfiles under .forge/run/ (one per service); logs under
# .forge/log/. Both are gitignored.

set -eu

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="$ROOT/target/debug/forge"
API_HOST="127.0.0.1"
API_PORT="8765"
WEB_HOST="127.0.0.1"
WEB_PORT="4173"
RUN_DIR="$ROOT/.forge/run"
LOG_DIR="$ROOT/.forge/log"
API_PIDFILE="$RUN_DIR/api.pid"
WEB_PIDFILE="$RUN_DIR/web.pid"
API_LOG="$LOG_DIR/api.log"
WEB_LOG="$LOG_DIR/web.log"
DO_BUILD=""

usage() {
  cat >&2 <<EOF
usage: scripts/web.sh {start|stop|restart|status|logs} [--api-port <p>] [--web-port <p>] [--build]

  start    Build (if --build) and start both services in the background.
  stop     Stop both services.
  restart  stop + start.
  status   Print whether each service is listening, its pid, and its log path.
  logs     Tail both logs (Ctrl-C to exit).

Options:
  --api-port <p>  Override the API listener port (default 8765).
  --web-port <p>  Override the web UI listener port (default 4173).
  --build         Rebuild the binary with \`cargo build\` before starting.
EOF
}

ensure_layout() {
  mkdir -p "$RUN_DIR" "$LOG_DIR"
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
  setsid nohup "$BIN" "$@" --bind "$host" --port "$port" > "$log" 2>&1 < /dev/null &
  pid=$!
  echo "$pid" > "$pidfile"
  i=0
  while [ "$i" -lt 25 ] && ! is_listening "$port"; do
    i=$((i + 1))
    sleep 0.2
  done
  if is_listening "$port"; then
    echo "web: $label started (pid $pid) on http://$host:$port; log: $log"
  else
    echo "web: $label failed to bind $port; see $log" >&2
    rm -f "$pidfile"
    return 1
  fi
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
  start_one "api" "$API_HOST" "$API_PORT" "$API_LOG" "$API_PIDFILE" api serve
  start_one "web" "$WEB_HOST" "$WEB_PORT" "$WEB_LOG" "$WEB_PIDFILE" web serve
}

cmd_stop() {
  ensure_layout
  stop_one "web" "$WEB_PIDFILE"
  stop_one "api" "$API_PIDFILE"
}

cmd_status() {
  ensure_layout
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
while [ "$#" -gt 0 ]; do
  case "$1" in
    --api-port) [ "$#" -ge 2 ] || { echo "web: --api-port requires a value" >&2; exit 2; }; API_PORT="$2"; shift 2 ;;
    --web-port) [ "$#" -ge 2 ] || { echo "web: --web-port requires a value" >&2; exit 2; }; WEB_PORT="$2"; shift 2 ;;
    --build)     DO_BUILD=1; shift ;;
    -h|--help)   usage; exit 0 ;;
    *) echo "web: unknown argument '$1'" >&2; usage; exit 2 ;;
  esac
done
case "$cmd" in
  start)   cmd_start ;;
  stop)    cmd_stop ;;
  restart) cmd_stop; cmd_start ;;
  status)  cmd_status ;;
  logs)    cmd_logs ;;
  -h|--help) usage; exit 0 ;;
  *) echo "web: unknown command '$cmd'" >&2; usage; exit 2 ;;
esac
