#import "@local/mindtape:0.2.0": *

#show: task.with(
  title: "Coverage gate broken: tarpaulin aborts with ESRCH",
  status: done(2026, 8, 15),
)

= Summary

`just cover` fails before it prints a coverage number, so the 70% threshold
CONTRIBUTING.md claims to enforce is enforcing nothing.

```
thread panicked while processing panic. aborting.
ERROR cargo_tarpaulin: ESRCH: No such process
Error: "ESRCH: No such process"
error: Recipe `cover` failed on line 32 with exit code 1
```

Predates the `include_hidden` work — reproduced on a clean tree with that
change stashed, same abort at the same point. Tests themselves pass; the
tracer loses the process while a test panics, and tests that assert on panics
(`expect_err`, `#[should_panic]`-shaped paths) run under it.

= Scope

- Establish whether this is a tarpaulin/ptrace mismatch with the pinned
  toolchain or a hardened-kernel `ptrace_scope` restriction on the host.
- Try `--engine llvm` before pinning tarpaulin versions — the ptrace engine is
  the part failing, and llvm coverage sidesteps it.
- Whatever the fix, `just cover` must exit non-zero only for real threshold
  misses. A gate that always fails gets ignored, which is where it is now.
- `just check` does not call `cover`, so nothing caught this. Decide whether
  coverage joins the aggregate gate or CI, rather than relying on someone
  running it by hand.

= Resolution

Tarpaulin's ptrace engine, not the host. `/proc/sys/kernel/yama/ptrace_scope`
is 1, which still permits tracing descendants, and tarpaulin 0.35.2 covers a
throwaway crate with the same anyhow dependency and toolchain (rustc 1.93.0)
without complaint.

Narrowed to two tests: `report::tests::paths_format_no_violations` and
`paths_format_with_violations` abort on their own, every other test passes
alone. Both are the only callers of `print_paths`. Under ptrace the process
dies with a panic attributed to `anyhow/src/error.rs:919` (the unsafe vtable
read, linked in as dead code and never reached by that test), then double
panics and aborts, which is where the tracer's ESRCH comes from. Instrumenting
that code path is what breaks; the code itself is fine under `cargo test`.

`engine = "Llvm"` in `tarpaulin.toml` fixes it — source-based coverage, no
tracing. `just cover` reports 98.14% and exits 0; forcing `fail-under = 99`
exits 1 naming the real miss.

Two traps found on the way:

- `--engine llvm` on the command line is ignored when `tarpaulin.toml` exists,
  since the config file overrides CLI args. The engine has to be set in the
  file.
- The variant is spelled `Llvm`. A lowercase `"llvm"` makes the whole config
  unparseable, and tarpaulin only warns before running with its defaults —
  losing the threshold along with the engine. `just cover` now greps for that
  warning and fails on it, so the fallback cannot pass as a green gate.

Coverage runs as its own CI job rather than joining `just check`: coverage
builds carry different rustflags, so folding it into the aggregate gate would
rebuild the tree on every alternation between `just check` and `just cover`.
