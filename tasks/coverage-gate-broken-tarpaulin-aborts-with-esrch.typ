#import "@local/mindtape:0.2.0": *

#show: task.with(
  title: "Coverage gate broken: tarpaulin aborts with ESRCH",
  status: proposed(2026, 8, 15),
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
