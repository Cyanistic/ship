#!/usr/bin/env bash
# Records one tape against a fresh demo world, into website/tapes/out/:
# <tape>.webm (muted) and <tape>.png (poster).
#
#   website/tapes/record.sh waterline.tape          real take, real models
#   website/tapes/record.sh waterline.tape --dry    stub agents, for tuning
#                                                   timing for free; writes
#                                                   out/dry/, never ship it
#   POSTER_AT=9 ...                                  poster time in seconds
#   OUT=/tmp/x ...                                   write outputs there
#                                                   instead of out/
#
# Model proxy, opt-in (model-proxy.py on 127.0.0.1:43332):
#
#   record.sh waterline.tape --record DIR   real take whose model replies are
#                                           saved to DIR (must hold none yet)
#   record.sh waterline.tape --replay DIR   take served from DIR alone: no
#                                           logins, no outbound connections
#
# Both point claude (ANTHROPIC_BASE_URL) and pi (models.json baseUrl, SSE
# transport) at the proxy and write the proxy's log to <tape>.proxy.log next
# to the webm. A replay gives claude a placeholder ANTHROPIC_AUTH_TOKEN and pi
# a fake token, so no credential is read at all.
#
#   record.sh waterline.tape --openrouter   real take with both agents on
#                                           OpenRouter's free router; needs
#                                           OPENROUTER_API_KEY in the
#                                           environment, e.g. from
#                                           `infisical run --env=dev --`
#
# A real take spends Claude (haiku) and Codex (gpt-6-luna) usage.
#
# Everything runs from /tmp/ship-demo: temp XDG dirs, a Ship server on port
# 43331 built by setup.sh, and an isolated Pi agent dir. That dir gets the
# openai-codex access token copied from ~/.pi/agent/auth.json without its
# refresh token, so a take can never rotate Cyan's login. The whole directory,
# token included, is deleted when the script exits.
set -euo pipefail

TAPES=$(cd "$(dirname "$0")" && pwd)
REPO=$(cd "$TAPES/../.." && pwd)
ROOT=/tmp/ship-demo
PORT=43331
DRY=false
PROXY_MODE=""
PROXY_DIR=""
PROXY_PORT=43332
OPENROUTER=false

say() { printf 'record: %s\n' "$*" >&2; }
die() { say "$*"; exit 1; }

usage="usage: record.sh <tape in website/tapes> [--dry | --record DIR | --replay DIR | --openrouter]"
tape=${1:-}
[[ -n $tape && -f $TAPES/$tape ]] || die "$usage"
case ${2:-} in
  --dry) DRY=true ;;
  --openrouter)
    OPENROUTER=true
    [[ -n ${OPENROUTER_API_KEY:-} ]] || die "--openrouter needs OPENROUTER_API_KEY in the environment"
    ;;
  --record | --replay)
    PROXY_MODE=${2#--}
    [[ -n ${3:-} ]] || die "$usage"
    mkdir -p "$3"
    PROXY_DIR=$(cd "$3" && pwd)
    ;;
esac
name=$(basename "$tape" .tape)

for tool in vhs ttyd ffmpeg jq python3 lsof; do
  command -v "$tool" >/dev/null || die "$tool is not on PATH"
done
$DRY || for tool in claude pi; do
  command -v "$tool" >/dev/null || die "$tool is not on PATH"
done
if lsof -nP -iTCP:"$PORT" -sTCP:LISTEN >/dev/null; then
  die "port $PORT is already in use; stop whatever holds it first"
fi
if [[ -n $PROXY_MODE ]] && lsof -nP -iTCP:"$PROXY_PORT" -sTCP:LISTEN >/dev/null; then
  die "port $PROXY_PORT is already in use; stop whatever holds it first"
fi

(cd "$REPO" && cargo build --release --quiet -p ship)

# On a machine where claude has never run (CI), it opens on a theme picker
# instead of the prompt. An existing ~/.claude.json is left alone.
if ! $DRY && [[ ! -e $HOME/.claude.json ]]; then
  echo '{"hasCompletedOnboarding": true}' >"$HOME/.claude.json"
fi

export TAPES ROOT PORT DRY PROXY_MODE OPENROUTER
[[ -n $PROXY_MODE ]] && export PROXY_URL=http://127.0.0.1:$PROXY_PORT
export SHIP_SERVER_URL=http://127.0.0.1:$PORT
export SHIP_CONFIG=$TAPES/ship.toml
export PATH=$REPO/target/release:$PATH

cleanup() {
  if [[ -f $ROOT/server.pid ]]; then
    local server
    server=$(cat "$ROOT/server.pid")
    ship server stop >/dev/null 2>&1 || kill "$server" 2>/dev/null || true
    while kill -0 "$server" 2>/dev/null; do sleep 0.1; done
  fi
  if [[ -f $ROOT/proxy.pid ]]; then
    local proxy
    proxy=$(cat "$ROOT/proxy.pid")
    kill "$proxy" 2>/dev/null || true
    while kill -0 "$proxy" 2>/dev/null; do sleep 0.1; done
  fi
  rm -rf "$ROOT"
}
trap cleanup EXIT

rm -rf "$ROOT"
mkdir -p "$ROOT"/xdg/{state,config,data,cache} "$ROOT/pi" "$ROOT/bin"
chmod 700 "$ROOT" "$ROOT/pi"
export XDG_STATE_HOME=$ROOT/xdg/state XDG_CONFIG_HOME=$ROOT/xdg/config \
  XDG_DATA_HOME=$ROOT/xdg/data XDG_CACHE_HOME=$ROOT/xdg/cache

if [[ $PROXY_MODE == replay ]]; then
  # A fake login: pi only needs a JWT-shaped access token naming an account
  # and an expiry it won't try to refresh. The proxy never forwards it.
  python3 - "$ROOT/pi" <<'EOF'
import base64, json, os, sys
part = lambda obj: base64.urlsafe_b64encode(json.dumps(obj).encode()).decode().rstrip("=")
claims = {"https://api.openai.com/auth": {"chatgpt_account_id": "replay-placeholder"}}
token = {"type": "oauth", "access": part({"alg": "none"}) + "." + part(claims) + ".replay",
         "refresh": "", "expires": 4102444800000, "accountId": "replay-placeholder"}
path = os.path.join(sys.argv[1], "auth.json")
with os.fdopen(os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600), "w") as f:
    json.dump({"openai-codex": token}, f)
EOF
elif $OPENROUTER; then
  # pi's built-in openrouter provider reads OPENROUTER_API_KEY itself. Its
  # catalogue caps the free router at 4K output tokens, too few to write a
  # file in one call.
  echo '{"providers": {"openrouter": {"modelOverrides": {"openrouter/free": {"maxTokens": 32768}}}}}' >"$ROOT/pi/models.json"
elif ! $DRY; then
  # Refuse a token near expiry: with no refresh token, pi couldn't renew it
  # mid-take. Running pi normally once renews the one in ~/.pi.
  python3 - "$ROOT/pi" <<'EOF' || die "copying pi's openai-codex token failed"
import json, os, sys, time
entry = json.load(open(os.path.expanduser("~/.pi/agent/auth.json")))["openai-codex"]
left = entry["expires"] / 1000 - time.time()
if left < 3600:
    sys.exit("record: the openai-codex token expires within an hour; run pi once to renew it")
token = {key: entry[key] for key in ("type", "access", "expires", "accountId")}
token["refresh"] = ""
path = os.path.join(sys.argv[1], "auth.json")
with os.fdopen(os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600), "w") as f:
    json.dump({"openai-codex": token}, f)
EOF
fi
echo '{"quietStartup": true}' >"$ROOT/pi/settings.json"

out=$TAPES/out
$DRY && out=$TAPES/out/dry
out=${OUT:-$out}
mkdir -p "$out"

if [[ -n $PROXY_MODE ]]; then
  # The proxy only speaks SSE, so pi must not try its WebSocket transport.
  echo '{"quietStartup": true, "transport": "sse"}' >"$ROOT/pi/settings.json"
  printf '{"providers": {"openai-codex": {"baseUrl": "%s/backend-api"}}}\n' "$PROXY_URL" >"$ROOT/pi/models.json"
  python3 "$TAPES/model-proxy.py" "$PROXY_MODE" --dir "$PROXY_DIR" --port "$PROXY_PORT" \
    >"$out/$name.proxy.log" 2>&1 &
  echo $! >"$ROOT/proxy.pid"
  for _ in $(seq 50); do lsof -nP -iTCP:"$PROXY_PORT" -sTCP:LISTEN >/dev/null && break; sleep 0.1; done
  lsof -nP -iTCP:"$PROXY_PORT" -sTCP:LISTEN >/dev/null || die "the model proxy didn't start; see $out/$name.proxy.log"
fi

"$TAPES/setup.sh"

# VHS runs xterm.js under ttyd, and on macOS xterm.js turns Alt+a into a
# plain "a" unless Option counts as Meta. VHS can't set that, so a wrapper
# first on PATH passes it to ttyd. Only checked on macOS.
printf '#!/bin/sh\nexec "%s" -t macOptionIsMeta=true "$@"\n' "$(command -v ttyd)" >"$ROOT/bin/ttyd"
chmod +x "$ROOT/bin/ttyd"

# The tape starts its shell here, so the prompt reads `harbor $`. Tapes that
# add to the world source DEMO_WORLD for the tab IDs and find the agent
# wrappers on PATH.
export DEMO_PROJECT=$ROOT/harbor DEMO_WORLD=$ROOT/world.env
export PATH=$ROOT/agents:$PATH
cd "$TAPES"
PATH=$ROOT/bin:$PATH vhs -q -o "$out/$name.webm" "$tape"

ffmpeg -loglevel error -y -ss "${POSTER_AT:-9}" -i "$out/$name.webm" -frames:v 1 "$out/$name.png"
say "wrote $out/$name.webm and $out/$name.png"
