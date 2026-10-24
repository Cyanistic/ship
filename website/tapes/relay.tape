# The "agents driving agents" demo: a real claude uses the ship CLI to open a
# pi worker beside itself, hands it a task, waits for it and reads the answer.
# Record it with `record.sh relay.tape`, which builds the shared world in
# setup.sh first. While hidden the tape adds a crew tab with claude in it and
# points the new window at it; after that it only types the prompt and waits
# on the state Ship draws in the sidebar. Hidden waits cut think time.

Set Shell bash
Set FontFamily "JetBrains Mono"
Set FontSize 30
Set Width 1920
Set Height 1080
Set Padding 18
Set Framerate 30
Set TypingSpeed 90ms
Source ship16.tape

Hide
Type `cd "$DEMO_PROJECT" && PS1="\W $ " && set +m && source "$DEMO_WORLD" && clear` Enter

# A crew tab under ship with claude alone in it, given only Bash, allowed to
# run ship and jq with it without asking, and told how to hand work to pi
# (relay-lead.md), the way a user's own agent instructions would.
Type `CREW=$(ship tab create --name crew "$SHIP_TAB" | jq -r .id)` Enter
Type `LEAD=$(ship pane create --tab "$CREW" -- claude --tools Bash --permission-mode dontAsk --allowedTools "Bash(ship:*)" "Bash(jq:*)" --append-system-prompt "$(cat "$TAPES/relay-lead.md")" | jq -r .id)` Enter
Type `ship pane wait --pane "$LEAD" --status idle >/dev/null && echo "lead""-ready"` Enter
Wait+Screen@60s /lead-ready/

# A new window opens on no tab, so select claude for it once it attaches.
Type `(until A=$(ship view list | jq -r 'keys[0] // empty') && [ -n "$A" ]; do sleep 0.1; done; ship view select --attachment "$A" "$LEAD") >/dev/null 2>&1 &` Enter
Type "clear && ship" Enter
Wait+Screen@20s /don't ask on|stub claude-code/

# Beat 1: ask claude to delegate.
Show
Sleep 1s
Type@20ms "Use the ship CLI to open pi in a pane to your right and have it write tide_cli.py, which prints the day's high and low water. Wait for pi to finish, then tell me in one line what it built."
Sleep 500ms
Enter

# Beat 2: claude opened the worker and typed the task into it.
Hide
Wait+Screen@180s /● pi working[^\n]*\n[^\n]*ship › crew/
Show
Sleep 4s

# Beat 3: pi finished, and claude read its answer back.
Hide
Wait+Screen@300s /○ claude-code idle[^\n]*\n[^\n]*ship › crew/
Show
Sleep 4.5s
