#import "@local/mindtape:0.2.0": *

#show: task.with(
  title: "split violation and operational exit codes",
  status: done(2026, 10, 3),
  tags: ("cli",),
)

`linecop` returns `ExitCode::FAILURE` (1) both when files exceed their limits and
when the run could not happen at all — a malformed `.linecop.yaml`, an unreadable
path, a bad glob in `overrides`. A caller cannot tell "the codebase needs
attention" from "the gate never ran".

This matters most in an aggregate gate recipe, where a broken config reads as
ordinary violations: someone silences the arm, or waives it, and the check
quietly stops running.

`outdatty` already models this correctly — `0` in sync, `1` drift, `2`
operational error — so aligning also makes the two tools consistent for anyone
wiring both into the same `just check`.

== Scope

- Keep `0` = no violations, `1` = violations found.
- Introduce `2` = operational error: config parse failure, unknown/unreadable
  path, invalid override glob, tokei failure.
- Audit every current `FAILURE` return and classify it.
- Document the codes in `README.md` alongside the existing usage section.
- Cover each class in `tests/cli.rs`; assert the code, not just non-zero.
