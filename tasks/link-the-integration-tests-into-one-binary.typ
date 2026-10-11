#import "@local/mindtape:0.2.0": *

#show: task.with(
  title: "Link the integration tests into one binary",
  status: done(2026, 10, 11),
)

= Summary

Cargo builds one executable per file in `tests/`, and each one links every dependency again. linecop has five (`cli.rs`, `config_discovery.rs`, `override_paths.rs`, `skill_fixture.rs`, `subcommands.rs`), and four of them each compile their own copy of `tests/common/mod.rs`. Link them into one `tests/it` binary.

Measured 2026-10-11 in mindtape: 74 test files gave `target/debug/deps` 263 executables averaging 298 MB, 80 GB in all. `just cover` builds the whole set a second time, instrumented: 51 GB in one worktree, and the disk filled. canibump's 7 test files built 28 binaries of about 37 MB each with full debug info. Debug info size is handled host-wide in `~/.nix`; this task is the per-repo half.

= Steps

+ Record the baseline: `just count-tests` (it sums every `test result:` line of `cargo test --workspace`), and after `cargo test --no-run` the output of `find target/debug/deps -maxdepth 1 -type f -executable | wc -l`.
+ `git mv tests/*.rs tests/it/` and `git mv tests/common tests/it/common`, then add `tests/it/main.rs` with `mod common; mod cli; mod config_discovery; mod override_paths; mod skill_fixture; mod subcommands;`. Cargo autodiscovers `tests/it/main.rs` as the test target `it`.
+ Shared helper: `common` is declared once in `main.rs`. Delete the `mod common;` line from `cli.rs`, `config_discovery.rs`, `override_paths.rs` and `subcommands.rs`, and turn their `use common::{linecop, write_config};` into `use crate::common::{linecop, write_config};`.
+ Relative paths: no file uses `include_str!`, `include_bytes!` or `#[path]`. `skill_fixture.rs` builds its fixture path from `env!("CARGO_MANIFEST_DIR")`, which the move does not change.
+ `Cargo.toml` has no `[[test]]` entries; add none unless autodiscovery fails. Its `include = [..., "/tests/", ...]` still covers `tests/it/`.
+ Update what names the old file. `grep -rn "tests/" --exclude-dir=target --exclude-dir=.git .` finds these: `CHANGELOG.md:22`, `skills/linecop/evals/README.md:17` and `skills/linecop/evals/evals.json:4` all cite `tests/skill_fixture.rs` and become `tests/it/skill_fixture.rs`. The Justfile runs `cargo test --workspace` with no `--test` flag and `tarpaulin.toml` names no test file, so neither changes. Re-run the grep to confirm.

= Done when

- `just count-tests` prints the same total as before the move.
- `cargo test --no-run` builds one integration executable, `it`, not five.
- No file still names `tests/skill_fixture.rs`, and `just check` is green.
- Out of scope: inline `#[cfg(test)]` unit tests (they already share one binary per crate) and debug info settings.
