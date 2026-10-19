#import "@local/mindtape:0.2.0": *

#show: task.with(
  title: "Silence the cd warning of just land",
  status: proposed(2026, 10, 20),
)

= Summary

`just land` ends with `Cannot change directory — shell integration not
installed`, because `wt merge` removes the worktree and writes a cd directive
that nothing receives.

= Changes

`land` in the `Justfile` creates a file with `mktemp`, passes it to `wt merge` as
`WORKTRUNK_DIRECTIVE_CD_FILE`, and removes it on exit with a trap. The flags and
the exit code stay as they are.

= Done when

- `just land` prints no `shell integration` warning.
- A red gate still fails `just land` with `wt merge`'s exit code.
