# Agent state detection

Evidence gathered on 2026-10-08 from Herdr's source and docs at commit `d6b40d4e` (2026-10-01), during a design conversation about milestone 3. Nothing was built or run in Ship. This note records a deferred direction so it can be picked up when milestone 3 starts. It is not an approved design.

## Summary

Hooks alone can't tell Ship what most agents are doing. Of the 22 agents Herdr detects, only six report state through hooks reliably enough that Herdr skips reading the screen. For the rest, including Claude Code, Herdr installs hooks only to learn the agent's session ID (for resume) and reads state from the screen. The window title and OSC 9;4 progress codes help, but never decide state on their own.

So Ship will need a screen rule engine for the agents Cyan uses beyond Pi. The current leaning, not decided:

- Read Herdr's per-agent TOML manifests unchanged (Herdr is Apache-2.0; bundle with attribution). Herdr's contributors keep them current as each agent's interface changes, and that upkeep is the real cost of screen detection.
- Write a small engine: roughly 300 to 400 lines, against Herdr's 1,536 in `manifest.rs` plus 1,012 in `manifest_update.rs`.
- Prefer explicit signals where they exist: hooks first, then title and OSC 9;4, then screen rules.

## Signals per agent

From `docs/next/website/src/content/docs/integrations.mdx` and `src/detect/mod.rs:323-343`:

| What hooks give Herdr | Agents |
|---|---|
| Full state; screen detection is skipped while they report | Pi, OMP, Kimi, OpenCode, Kilo, MastraCode |
| Turn start and stop with `--no-daemon`; visible blockers still come from the screen | Codex |
| Session identity only; state comes from screen manifests | Claude Code, Copilot, Cursor, Droid, Devin, Grok, Hermes, Qwen, Letta, Antigravity |

Herdr's stated reasons are missing transitions, not missing hooks. Devin: "hooks do not emit a reliable state transition after every permission cancellation or user interrupt". Droid: "hooks do not cover every lifecycle transition". A state driven only by hooks can stick at the wrong value after Esc or an answered permission prompt.

Eight manifests read the window title or OSC 9;4 (Amp, Claude, Codex, Grok, Hermes, Kiro, Letta, Qwen), always alongside screen rules. Claude's title spinner shows it is working, but can't separate "waiting for permission" from "done". Ship already stores the pane title. Whether libghostty exposes OSC 9;4 to Ship is unverified.

## Herdr's manifest format

One TOML file per agent in `src/detect/manifests/`, 22 files and 1,746 lines. Each `[[rules]]` entry has a state, a priority, a screen region, and matchers. An excerpt from `pi.toml`:

```toml
[[rules]]
id = "working_border"
state = "working"
priority = 100
region = "bottom_non_empty_lines(12)"
visible_working = true
any = [
  { line_regex = ['^── [⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏] Working ─+$'] },
  { line_regex = ['^[⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏] Working$'] },
]
```

The format has visibly built up patches: hand-picked priorities (1100, 980, 975, 970, 965), six matcher kinds nested inside each other, region names that are a small language inside strings, side flags (`visible_working`, `visible_blocker`, `skip_state_update`), and comments recording agent versions and issue numbers (`claude.toml`). Most of that comes from the job itself: each rule follows how an agent's interface looked in some release. A tidier format owned by Ship would gather the same patches, so renting the upkeep outweighs the format's looks.

## A smaller engine

Approximate split of `src/detect/manifest.rs` (1,536 lines), from function boundaries:

| Part | Lines | Ship needs it |
|---|---:|---|
| Compiling and matching rules, regions | ~400 | Yes |
| Explain output, evidence, JSON (`herdr agent explain`) | ~300 | Later, if a rule misfires and needs debugging |
| Loading and caching: bundled, local override, downloaded, hot reload | ~300 | Bundled only, plus perhaps a local override |
| Validation and complexity limits | ~250 | No |

`manifest_update.rs` (1,012 lines) downloads newer manifests. The validation limits exist because Herdr runs regexes it downloaded. If manifests are bundled at build time, from the same source as the code, failing to deserialize or compile a regex at startup is enough.

The matchers fit one recursive serde struct, because Herdr's tables combine several conditions with an implicit AND (`{ contains = [...], any = [...] }`):

```rust
#[derive(Deserialize)]
struct Gate {
    #[serde(default)] contains: Vec<String>,
    #[serde(default)] regex: Vec<Regex>,       // serde_regex
    #[serde(default)] line_regex: Vec<Regex>,
    #[serde(default)] any: Vec<Gate>,
    #[serde(default)] all: Vec<Gate>,
    #[serde(default)] not: Vec<Gate>,
}

impl Gate {
    fn matches(&self, text: &str) -> bool {
        self.contains.iter().all(|s| text.contains(s.as_str()))
            && self.regex.iter().all(|r| r.is_match(text))
            && self.line_regex.iter().all(|r| text.lines().any(|l| r.is_match(l)))
            && (self.any.is_empty() || self.any.iter().any(|g| g.matches(text)))
            && self.all.iter().all(|g| g.matches(text))
            && !self.not.iter().any(|g| g.matches(text))
    }
}
```

A rule is the flattened gate plus `state`, `priority` and `region`. Rules are sorted by priority once at load, and the first match wins. Herdr lowercases text for `contains`; this sketch skips that, and doesn't check Herdr's exact semantics for every matcher kind.

### Struct or enum

As a type, an enum is cleaner. Each node is exactly one thing, and an empty matcher that matches everything can't be written, a case Herdr guards at runtime (`gate_has_positive_matcher`):

```rust
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Matcher {
    Contains(Vec<String>),
    Regex(Vec<Regex>),
    LineRegex(Vec<Regex>),
    Any(Vec<Matcher>),
    All(Vec<Matcher>),
    Not(Box<Matcher>),
}
```

serde's default enum encoding expects one key per table, so it can't read Herdr's implicit-AND tables. The choice follows the format decision: renting Herdr's manifests means the struct, since that is their format's true shape. Converting to the enum with `#[serde(try_from = "Gate")]` adds a type and evaluates the same. Owning a format means the enum, writing `all = [...]` explicitly.

### Regions

Region use across all 141 rules:

| Region | Rules |
|---|---:|
| `bottom_non_empty_lines(N)` | 65 |
| `whole_recent` | 41 |
| `osc_title` | 19 |
| `osc_progress` | 6 |
| `top_non_empty_lines(N)`, `after_last_prompt_marker`, `after_last_horizontal_rule` | 2 each |
| `whole_recent_without_current_prompt_marker`, `prompt_box_body`, `last_non_empty_above_prompt_box`, `before_current_prompt_marker` | 1 each |

Four regions cover 131 of 141 rules. The rest are line-slicing functions of 10 to 20 lines each in Herdr (`manifest.rs:1276-1534`), about 150 lines for full compatibility. The alternative is to skip rules with an unknown region and add regions as agents in use need them.

## Open

- Rent Herdr's manifests or own a format.
- How hooks reach Ship: likely a documented `ship` call from each agent's own hook config, instead of Herdr's per-agent installers (`src/integration/`, 16,848 lines).
- How Ship identifies which agent runs in a pane. Herdr's `src/detect/mod.rs` spends most of its 2,018 lines on argv and process-name handling across wrappers and platforms. Matching the foreground process name only, behind `cfg(unix)`, is the smallest start.
- What `visible_*` and `skip_state_update` mean for Ship's states, and whether Ship needs them.
- Whether libghostty exposes OSC 9;4.
