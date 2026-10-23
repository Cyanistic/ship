# The landing-page demo: three real agents in Ship, about 20 seconds.
# Record it with `record.sh waterline.tape`, which builds the world in
# setup.sh first. The tape drives everything like a user at the keyboard:
# while hidden it moves between panes and types the prompts, then it waits on
# the state Ship draws in the sidebar. Hidden waits cut agent think time, so
# the length stays the same however slow a take is.
#
# Navigation: alt-a jumps to the next agent (with nobody waiting, the first
# one), alt-h/alt-l move between panes, and alt-g enters tabs mode, where j/k
# move between tabs, h folds and esc leaves.

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
Type `cd "$DEMO_PROJECT" && PS1="\W $ " && clear` Enter
Type "ship" Enter
Wait+Screen@20s /alt-right to open one/

# Fold infra: from claude in api, tabs mode goes tests, docs, infra.
Alt+a
Wait+Screen /▾ api/
Alt+g
Type "jjj"
Type "h"
Escape
Wait+Screen /▸ infra/

# Beat 1 prompts. Typing is slow next to an agent's turn, so type all three
# first and press Enter on each at the end, close together. Alt-a lands on
# claude, the first agent, since nobody is waiting yet.
Alt+a
Sleep 300ms
Type@10ms "Read tide.py and test_tide.py, then explain in three sentences what this project does and name one edge case the tests miss."
Alt+l
Sleep 300ms
Type@10ms "Add type hints and one-line docstrings to every function in tide.py, make each function handle an empty list, add a next_high_water(readings, after_hour) function, write tests for all of it in test_tide.py, and run them with python3 -m unittest -v until they pass."
Alt+g
Type "jj"
Escape
Sleep 300ms
Type@10ms "Write a short README.md for this project: one paragraph and a usage example."
Enter
# Back to api, which remembers the pi pane, so the docs pi finishes unseen.
Alt+g
Type "kk"
Escape
Sleep 300ms
Enter
Alt+h
Sleep 300ms
Enter
Alt+l

# Beat 1: three agents working.
Wait+Screen@90s /● claude-code working[\s\S]*● pi working[\s\S]*● pi working/
Show
Sleep 3.5s

# Beat 2: the docs pi finished off screen, so it shows unseen.
Hide
Wait+Screen@120s /◆ pi/
Show
Sleep 3.5s

# Beat 3: ask claude to run the tests. It wants permission, so it's blocked
# and sorts to the top. The api pi gets a follow-up so it is still busy at the
# reattach; focus stays on it so alt-a has a jump to make.
Hide
Alt+h
Sleep 300ms
Type@10ms "Now run the tests with python3 -m unittest -v and tell me in one line whether they pass. Don't run anything else."
Enter
Alt+l
Sleep 300ms
Type@10ms "Next, add tide_cli.py that prints the day's high water, low water and safe hours for a draft given on the command line, with tests in test_tide_cli.py, and document it in README.md. Run everything with python3 -m unittest -v until it passes."
Enter
Wait+Screen@120s /▲ claude-code blocked/
Show
Sleep 3s

# Beat 4: alt-a jumps to claude; approving puts it back to work.
Alt+a
Sleep 1.5s
Enter
Wait+Screen@30s /● claude-code working/
Sleep 2s

# Beat 5: detach, then reattach; everything kept running. A new window opens
# on no tab; alt-a jumps to an agent.
Alt+q
Wait+Screen@10s /detached/
Sleep 1s
Type "ship"
Enter
Wait+Screen@10s /alt-right to open one/
Sleep 1s
Alt+a
Wait+Screen@10s /ship-demo\/harbor/
Sleep 3.5s
