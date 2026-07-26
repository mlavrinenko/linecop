#import "@local/mindtape:0.2.0": *

#show: task.with(
  title: "Write a linecop agent skill",
  status: proposed(2026, 7, 26),
)

= Summary

Agents hit linecop as a wall rather than a tool: they discover a limit only when
the gate rejects a file they already wrote, then spend the change compressing
prose instead of on the work. The capability that would prevent this already
ships — `linecop --baseline <PCT>` reports every file at or above that share of
its limit — but nothing tells an agent it exists at the moment it would help.

A skill under `skills/linecop/` closes that gap, and makes the knowledge
portable to any repo that adopts linecop instead of living in one project's
AGENTS.md.

= Scope

- `--baseline <PCT>` as the pre-edit check: run it before adding to an existing
  file so the cuts get budgeted up front. Default is 100 (violations only);
  85 is a useful warning band.
- Where limits come from: `.linecop.yaml`, `limits` by language, `overrides`
  by glob (`limit`, `exclude`), `count_mode`, `exclude_dirs`. Read the config
  before writing, not after the gate trips.
- `--format paths` for piping (the `ejectest` pairing that ejects oversized
  inline Rust tests), `--format json` for structured consumers, `-q` for
  exit-code-only use in a gate.
- Splitting up front as the intended response to a near-limit file, and when
  raising a limit is the honest answer instead — with the reasoning recorded
  in a config comment so the number is not re-litigated.
- Ship evals with a tracked fixture, and gate the fixture. An eval fixture that
  no gate validates rots silently; this was observed first-hand in mindtape,
  where the skill's sandbox outlived two config migrations unnoticed because it
  sat in a gitignored scratch tree.

= Notes

Trigger phrasing should cover the indirect cases, not just the tool name: "this
file is too long", "the size gate is failing", "split this module", "what is
near the limit".
