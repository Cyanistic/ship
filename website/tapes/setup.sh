#!/usr/bin/env bash
# Builds the demo world on a fresh Ship server: a workspace with nested tabs,
# the scene's three agents ready and idle, and nothing prompted yet. record.sh
# runs this once per tape, so every tape starts from the same world.
#
# Expects from record.sh: ROOT (an empty temp dir, already holding pi/ with
# the copied token on real takes), PORT, DRY (true or false), TAPES,
# SHIP_SERVER_URL, SHIP_CONFIG, the XDG_*_HOME dirs, and `ship` on PATH.
# With the model proxy, also PROXY_MODE (record or replay) and PROXY_URL.
# Leaves the server running; its PID is in $ROOT/server.pid, and the world's
# tab IDs are in $ROOT/world.env for tapes to source.
#
#   ▾ ship
#     ▾ api      claude | pi       the scene
#         tests  shell
#       docs     pi                finishes unseen
#     ▸ infra    shell             folded by the tape
#         deploy pi                idle filler, never prompted
#   ▾ blog       nvim | shell
set -euo pipefail

die() { printf 'setup: %s\n' "$*" >&2; exit 1; }

cp -R "$TAPES/harbor" "$ROOT/harbor"
mkdir -p "$ROOT/blog/posts"
cat >"$ROOT/blog/posts/reading-tide-tables.md" <<'EOF'
# Reading tide tables

A tide table lists the height of the water at each hour. High water is the
tallest reading of the day, low water the shortest, and the range is the
distance between them.

Before you leave the harbor, check which hours give you more water than your
keel needs.
EOF

# `claude` and `pi` on the server's PATH are wrappers that add the isolation
# flags (or run the stub on a dry run), so an agent in the demo that starts
# another agent gets the same setup. Claude without the user's settings,
# plugins or MCP servers, and in manual mode so running the tests needs
# permission. Pi without extensions, skills or context files.
mkdir -p "$ROOT/agents"
if $DRY; then
  printf '#!/bin/sh\nexec "%s" claude-code 6\n' "$TAPES/stub-agent.sh" >"$ROOT/agents/claude"
  printf '#!/bin/sh\nexec "%s" pi 6\n' "$TAPES/stub-agent.sh" >"$ROOT/agents/pi"
else
  # Through the model proxy, both agents send model requests there and make
  # no other connections; a replay also swaps claude's login for a
  # placeholder. Claude offers a Workflow tool with a token login but not
  # with Cyan's subscription login, and the proxy matches requests on tool
  # names, so it is off in record and replay alike.
  # On OpenRouter both agents use its free router, and the key stays in the
  # server's environment rather than in the wrappers.
  # The relay lead's `ship pane send --wait` lasts as long as pi's whole turn;
  # past claude's default 2-minute Bash timeout it lingers as a background
  # shell in claude's footer.
  claude_env="env BASH_DEFAULT_TIMEOUT_MS=600000 " claude_extra="" pi_env="" pi_model=openai-codex/gpt-6-luna
  if ${OPENROUTER:-false}; then
    claude_env+='ANTHROPIC_BASE_URL=https://openrouter.ai/api ANTHROPIC_AUTH_TOKEN="$OPENROUTER_API_KEY" ANTHROPIC_API_KEY= ANTHROPIC_DEFAULT_HAIKU_MODEL=openrouter/free CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1 '
    pi_env="env PI_OFFLINE=1 "
    pi_model=openrouter/openrouter/free
  elif [[ -n ${PROXY_MODE:-} ]]; then
    claude_env+="ANTHROPIC_BASE_URL=$PROXY_URL CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1 "
    [[ $PROXY_MODE == replay ]] && claude_env+="ANTHROPIC_AUTH_TOKEN=replay-placeholder "
    claude_extra="--disallowedTools Workflow "
    pi_env="env PI_OFFLINE=1 "
  fi
  # Both also get harbor.md, unseen in the video, which keeps them inside the
  # project: no network APIs, no new dependencies, no long deliberation.
  printf '#!/bin/sh\nexec %s"%s" --model haiku --setting-sources project --strict-mcp-config --permission-mode manual --append-system-prompt-file "%s" %s"$@"\n' \
    "$claude_env" "$(command -v claude)" "$TAPES/harbor.md" "$claude_extra" >"$ROOT/agents/claude"
  printf '#!/bin/sh\nexec %s"%s" --model %s --no-skills --no-extensions --no-context-files --no-prompt-templates --append-system-prompt "%s" "$@"\n' \
    "$pi_env" "$(command -v pi)" "$pi_model" "$TAPES/harbor.md" >"$ROOT/agents/pi"
fi
chmod +x "$ROOT/agents/claude" "$ROOT/agents/pi"
export PATH=$ROOT/agents:$PATH

# A clean environment, so panes show nothing from the recording machine's
# shell setup or from another Ship session.
key=()
${OPENROUTER:-false} && key=(OPENROUTER_API_KEY="$OPENROUTER_API_KEY")
(cd "$ROOT/harbor" && exec env -i ${key[@]+"${key[@]}"} HOME="$HOME" PATH="$PATH" LANG=en_US.UTF-8 \
  TERM=xterm-256color SHELL=/bin/bash BASH_SILENCE_DEPRECATION_WARNING=1 \
  XDG_STATE_HOME="$XDG_STATE_HOME" XDG_CONFIG_HOME="$XDG_CONFIG_HOME" \
  XDG_DATA_HOME="$XDG_DATA_HOME" XDG_CACHE_HOME="$XDG_CACHE_HOME" \
  PI_CODING_AGENT_DIR="$ROOT/pi" PI_SKIP_VERSION_CHECK=1 DISABLE_AUTOUPDATER=1 \
  ship --config "$SHIP_CONFIG" server --port "$PORT" --no-restore) >"$ROOT/server.log" 2>&1 &
echo $! >"$ROOT/server.pid"
for _ in $(seq 50); do ship server status >/dev/null 2>&1 && break; sleep 0.1; done
ship server status >/dev/null || die "the server didn't start; see $ROOT/server.log"

# The api pi's stub takes longer turns, so it is still busy at the reattach.
claude=(claude)
pi=(pi)
pi_api=(pi)
$DRY && pi_api=("$TAPES/stub-agent.sh" pi 30)
editor=(less)
command -v nvim >/dev/null && editor=(nvim)

tab() { ship tab create "$@" | jq -r .id; }
pane() { ship pane create "$@" | jq -r .id; }

ship_tab=$(tab --name ship)
api=$(tab --name api "$ship_tab")
tests=$(tab --name tests "$api")
docs=$(tab --name docs "$ship_tab")
infra=$(tab --name infra "$ship_tab")
deploy=$(tab --name deploy "$infra")
blog=$(tab --name blog)

h=$ROOT/harbor
claude_pane=$(pane --tab "$api" --cwd "$h" -- "${claude[@]}")
pi_api_pane=$(pane --pane "$claude_pane" --direction right --cwd "$h" -- "${pi_api[@]}")
pane --tab "$tests" --cwd "$h" >/dev/null
docs_pane=$(pane --tab "$docs" --cwd "$h" -- "${pi[@]}")
pane --tab "$infra" --cwd "$h" >/dev/null
deploy_pane=$(pane --tab "$deploy" --cwd "$h" -- "${pi[@]}")
post=$(pane --tab "$blog" --cwd "$ROOT/blog" -- "${editor[@]}" posts/reading-tide-tables.md)
pane --pane "$post" --direction right --cwd "$ROOT/blog" >/dev/null

# Wait until every agent reports idle. Claude asks once whether to trust a
# new folder, with "No, exit" preselected.
for target in "$claude_pane" "$pi_api_pane" "$docs_pane" "$deploy_pane"; do
  state=""
  for _ in $(seq 120); do
    state=$(ship pane get --pane "$target" | jq -r '.program[""].state // empty')
    [[ $state == idle ]] && break
    if ship pane read --pane "$target" | grep -q "Yes, I trust this folder"; then
      ship pane send --pane "$target" --key down --key enter >/dev/null
    fi
    sleep 0.5
  done
  [[ $state == idle ]] || die "pane $target never reported idle"
done

cat >"$ROOT/world.env" <<EOF
SHIP_TAB=$ship_tab
API_TAB=$api
DOCS_TAB=$docs
INFRA_TAB=$infra
BLOG_TAB=$blog
EOF
