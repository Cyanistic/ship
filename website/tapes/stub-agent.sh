#!/usr/bin/env bash
# Dry-run stand-in for an agent: `record.sh --dry` runs it instead of claude
# and pi so tape timing can be tuned without spending model usage. It reports
# OSC 7501 states like the real agents do. Never ship a dry render: the landing
# page only shows real sessions (website/PRODUCT.md).
#
# Usage: stub-agent.sh <app> <seconds per turn>
app=$1
turn=$2

status() { printf '\e]7501;state=%s:app=%s%s\e\\' "$1" "$app" "${2:-}"; }

echo "stub $app (dry run, not a real agent) in $PWD"
status idle
while IFS= read -r line; do
  echo "> $line"
  status working
  sleep "$turn"
  if [[ $app == claude-code && $line == *"run the tests"* ]]; then
    echo "Bash: python3 -m unittest -v"
    echo "Do you want to proceed?  1. Yes  2. No"
    status blocked :kind=permission
    read -r _
    status working
    python3 -m unittest -v 2>&1 | tail -3
    sleep 2
  fi
  if [[ $app == claude-code && $line == *"ship CLI"* ]]; then
    # Drive a pi worker the way the real claude is asked to.
    worker=$(ship pane create --direction right -- pi | jq -r .id)
    echo "Bash: ship pane create --direction right -- pi"
    ship pane wait --pane "$worker" --status idle >/dev/null
    ship pane send --pane "$worker" "Write tide_cli.py." --key enter --wait >/dev/null
    echo "Bash: ship pane send --pane $worker ... --wait"
    ship pane read --pane "$worker" --lines 3
  fi
  echo "done"
  status done
done
