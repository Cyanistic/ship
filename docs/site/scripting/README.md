# Scripting Ship

<!--
Scope: What every `ship` command shares for scripts (JSON output, explicit IDs, the `--pane` default) and the list of recipes.
Not here: the recipes themselves.
-->

Anything you can do with keys, you can also do with `ship` commands. They print JSON and take explicit IDs, so a script never acts on whichever pane you happen to have selected. Inside a Ship pane, `--pane` defaults to that pane. In these examples, `$PANE` holds a pane ID, like one printed by `ship pane create` or listed by `ship tab list`.

- [Reading output](reading-output.md)
- [Typing into a pane](typing-into-a-pane.md)
- [Handing work to another agent](handing-work-to-another-agent.md)
- [Reading command errors](reading-command-errors.md)
- [Steering a window](steering-a-window.md)
