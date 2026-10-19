#import "@local/mindtape:0.2.0": *

#show: task.with(
  title: "Silence the cd warning of just land",
  status: done(2026, 10, 20),
)

= Summary

`just land` ends with `Cannot change directory — shell integration not
installed`, because `wt merge` removes the worktree and writes a cd directive
that nothing receives.

= Changes

`land` and `fork` in the `Justfile` each create a file with `mktemp`, pass it to
`wt merge` or `wt switch` as `WORKTRUNK_DIRECTIVE_CD_FILE`, and remove it on exit
with a trap. The flags and the exit code stay as they are.

= Done when

- `just fork` and `just land` print no `shell integration` warning.
- A red gate still fails `just land` with `wt merge`'s exit code.
