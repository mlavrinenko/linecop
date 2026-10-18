#import "@local/mindtape:0.2.0": *

#show: task.with(
  title: "Print nothing when every file is within its limit",
  status: proposed(2026, 10, 18),
)

= Summary

A clean run prints `All files within size limits.` (`src/report.rs:112`). The
exit code already says so, and every gate runner that calls linecop has to
swallow the line. awesome-langnostic's `scripts/lint.sh` wraps each linter in
`out=$(… 2>&1) || print` just to hide it:

```
$ linecop; echo $?
All files within size limits.
0
```

typos and actionlint print nothing on a pass. linecop
should match, so a gate that passes stays silent.

= Scope

- Drop the success line in `src/report.rs`. A violation or a near-limit
  warning still prints as now.
- Flip the test at `src/report.rs:226` to assert empty output on a clean run.
- Mention it in the CHANGELOG as a behaviour change.

= Out of scope

- A `--verbose` flag that brings the line back. Add one only when someone asks.
