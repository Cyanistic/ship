# Shell completions

<!--
Scope: Loading `ship completions` in each supported shell.
Not here: the commands themselves (`ship --help`).
-->

`ship completions <shell>` prints a script that lets your shell tab-complete Ship's commands, flags and fixed values. It supports bash, zsh, fish, elvish and PowerShell. Load it from your shell's startup file:

```sh
# ~/.bashrc
source <(ship completions bash)

# ~/.zshrc, after compinit
source <(ship completions zsh)

# fish
ship completions fish > ~/.config/fish/completions/ship.fish
```

The bash and zsh lines regenerate the script each time a shell starts, so it stays current after you upgrade Ship. The fish file doesn't, so rerun that command after an upgrade. Completion covers names Ship knows ahead of time; it doesn't look up tab or pane IDs.
