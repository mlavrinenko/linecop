#import "@local/mindtape:0.2.0": *

#show: task.with(
  title: "Match override patterns relative to the config file",
  status: done(2026, 10, 3),
)

= Summary

An override glob is matched against the file path exactly as linecop prints it,
and that path carries whatever the `PATH` argument was. `linecop` with no
argument scans `.`, so every path starts with `./` and `src/tables.rs` matches
nothing. The same config matches `src/tables.rs` under `linecop src/` and
nothing under `linecop /abs/repo`. The README's own examples,
`src/generated_*.rs` and `RESEARCH.md`, never match a default run.

A config file describes the tree it sits in, so its patterns should read as
paths from its own directory, whatever the argument and working directory.

= Scope

- Match each override against the file's path relative to the config file's
  directory. A file outside that directory keeps matching its printed path.
- A leading `./` in a pattern means the config's directory, so existing
  `./CONTRIBUTING.md` patterns keep working.
- Reported paths do not change.
- End-to-end tests through the binary: one config, the same verdict under `.`,
  a subdirectory, an absolute path, and `--config` from another directory.
- Schema, README, landing page and changelog say what a pattern is relative
  to. The linecop skill drops its workaround.
