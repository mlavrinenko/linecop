# Linecop Review

Comprehensive review covering DX completeness, DRY-ness, and release readiness.
Reference project: [mindtape](../mindtape/) (recently released, same author conventions).

---

## 1. Release Readiness

### 1.1 Missing files

| What | Status | Notes |
|------|--------|-------|
| LICENSE | **missing** | MIT declared in Cargo.toml but no LICENSE file exists |
| CHANGELOG.md | **missing** | mindtape uses Keep a Changelog format — adopt the same |
| .github/workflows/ci.yml | **missing** | No CI at all |
| .github/workflows/release.yml | **missing** | No release automation |
| .github/workflows/pages.yml | **missing** | No GitHub Pages workflow |
| www/index.html | **missing** | No landing page (explicitly requested) |
| git remote | **missing** | No remote configured yet |

### 1.2 Cargo.toml gaps

Current metadata is minimal compared to mindtape:

```toml
# Missing fields:
repository = "https://github.com/mlavrinenko/linecop"   # required for crates.io
keywords = ["cli", "linter", "code-quality", "lines", "tokei"]
categories = ["command-line-utilities", "development-tools"]
rust-version = "1.85"  # or whatever MSRV — declare it
include = ["/src/", "/tests/", "/LICENSE", "/README.md"]

[profile.release]
strip = true
lto = true
codegen-units = 1
panic = "abort"
```

The empty `[workspace] members = []` is also noise — remove it unless sub-crates are actually planned.

### 1.3 CI workflow (create .github/workflows/ci.yml)

Model after mindtape's pattern:

- Format check (`just fmt-check`)
- Clippy + tests + file size (`just check`)
- Use `dtolnay/rust-toolchain@stable` + `Swatinem/rust-cache@v2`
- Trigger on push/PR to main
- Add `concurrency` group to cancel stale runs

### 1.4 Release workflow (create .github/workflows/release.yml)

Linecop is a single crate (simpler than mindtape's workspace), so the flow is:

1. Tag `v*` triggers workflow
2. Run `just check`
3. Matrix build (x86_64-linux-gnu, x86_64-linux-musl, aarch64-linux-gnu, x86_64-apple-darwin, aarch64-apple-darwin)
4. `cargo publish` with `CARGO_REGISTRY_TOKEN`
5. `softprops/action-gh-release` with binary tarballs

### 1.5 GitHub Pages (create .github/workflows/pages.yml + www/)

Deploy `www/` directory on push to main (paths: `www/**`). Same pattern as mindtape
but with a different design. Since linecop is a linter/code-quality tool, consider a
clean, minimal design with:

- Tool name + tagline
- Terminal demo showing a violation and a clean run
- Feature highlights (language-aware, overrides, JSON Schema, count modes)
- Install section (cargo, nix, binary releases)
- Links (GitHub, crates.io, changelog)

### 1.6 Justfile: add `release` recipe

Mindtape has a convenient `just release <version>` that tags and pushes. Add the same:

```just
release version:
    @echo "Tagging v{{version}}..."
    git tag -a "v{{version}}" -m "Release v{{version}}"
    git push origin "v{{version}}"
```

### 1.7 README.md overhaul

Current README is developer-oriented only. For release, it needs:

- CI badge + crates.io badge (like mindtape)
- "What" section — one-liner explaining the tool
- Install section (cargo install, nix run, binary releases)
- Usage examples (scan, init, schema, override, JSON output)
- Config format example
- Link to CONTRIBUTING.md
- License section

---

## 2. Developer Experience Completeness

### 2.1 What's solid

- `just check` as single entry point — good
- Dog-fooding: linecop checks itself via `just check-file-size` (`cargo run -q -- --quiet`)
- JSON Schema generation + yaml-language-server directive in init template
- CONTRIBUTING.md covers conventions well
- AGENT.md exists for AI-assisted development
- Comprehensive test suite (59+ tests, both unit and integration)
- `tarpaulin.toml` with 70% floor

### 2.2 Gaps

**`just check` doesn't run `fmt-check`.**
Mindtape's CI runs format check first. The `just check` recipe should include
`just fmt-check` as the first step so formatting issues are caught locally too.

**No `just default` recipe.**
Mindtape has `default: @just --list` so running bare `just` shows available recipes.
Add this for discoverability.

**Schema is committed but never regenerated.**
`linecop-schema.json` is checked into the repo but nothing ensures it stays in sync
with the Config struct. Options:
- Add a `just schema` recipe that regenerates it
- Add a test that asserts the committed schema matches `schema::generate()` output
- Or do both

**No `--help` example in README.**
The CLI has good `clap` descriptions but users can't discover them from the README.

**Coverage not in CI.**
Same gap as mindtape, but worth noting. At minimum, `just cover` should be documented
as a pre-release checklist item.

---

## 3. Implementation DRY-ness

### 3.1 Duplicated test helpers

Two separate `make_config` functions exist with overlapping logic:

- `checker.rs:86` — `fn make_config(limits, overrides, mode) -> Config`
- `counter.rs:91` — `fn make_config(limits) -> Config`

The counter version is a subset of the checker version. Similarly:
- `checker.rs:75` has `make_file(path, lang, code, comments, blanks) -> FileStats`
- `report.rs:91` has `make_violation(path, lang, lines, limit) -> Violation`

**Fix:** Create `src/test_helpers.rs` (or `#[cfg(test)] mod test_helpers` in `lib.rs`)
with shared builder functions. Both checker and counter tests would import from there.
This also prevents the helpers from drifting apart when fields are added.

### 3.2 main.rs match arms are repetitive

The `Init` and `Schema` subcommand handlers have identical Ok/Err structure:

```rust
match linecop::init::create(&cli.path) {
    Ok(path) => { println!("Created {path}"); return ExitCode::SUCCESS; }
    Err(err) => { eprintln!("error: {err:#}"); return ExitCode::FAILURE; }
}
// ...identical pattern for Schema...
```

This could be extracted into a small helper:

```rust
fn run_subcommand(result: Result<String>) -> ExitCode {
    match result {
        Ok(msg) => { println!("{msg}"); ExitCode::SUCCESS }
        Err(err) => { eprintln!("error: {err:#}"); ExitCode::FAILURE }
    }
}
```

Minor, but it's the pattern that `main.rs` would repeat for every future subcommand.

### 3.3 Glob recompilation in checker

`effective_limit()` recompiles glob patterns on every call via `Glob::new().compile_matcher()`.
For a project with 1000 files and 10 overrides, that's 10,000 glob compilations.
Patterns are already validated in `config::validate()`, so the compiled matchers could be
cached once (e.g. stored alongside the Config or pre-compiled at check start).

This is a correctness-adjacent issue too: `effective_limit` silently swallows `Glob::new`
errors via `if let Ok(glob)`, meaning a bad pattern would silently skip the override
rather than failing. The validation in `config.rs` catches this at load time, but the
silent fallthrough in the checker is fragile.

### 3.4 `write_config` in integration tests

`tests/cli.rs` defines `write_config()` which is yet another config-writing helper
alongside the test helpers in unit tests. If a `tests/common.rs` module is introduced
later, this should move there.

### 3.5 `thiserror` is a dependency but never used

`Cargo.toml` lists `thiserror = "2"` but no `#[derive(thiserror::Error)]` exists anywhere
in the codebase. All error handling uses `anyhow`. Either remove `thiserror` or define
structured error types for the library API (which would be better for downstream consumers).

### 3.6 `log` and `env_logger` are dependencies but never used

`main.rs` calls `env_logger::init()` but there are zero `log::debug!()`, `log::info!()`,
etc. calls anywhere in the codebase. Either remove both dependencies or add meaningful
log statements (e.g., logging which config was loaded, how many files were scanned, which
overrides matched).

---

## 4. Minor Issues

### 4.1 `serde_yml` version pinned to `0.0.12`

This is a `0.0.x` pre-release crate. Consider whether `serde_yaml` (the maintained fork)
would be more appropriate, or at least document why `serde_yml` was chosen.

### 4.2 No `--color` flag

`anstream` handles auto-detection, but there's no way to force `--color=always` or
`--color=never` from the CLI. Clap has built-in support for this.

### 4.3 init template has hardcoded schema path

The `init.rs` STARTER_CONFIG contains `$schema=linecop-schema.json` which is a relative
path. This only works if the schema file is in the same directory. For users installing
via cargo, the schema file won't exist. Consider:
- Hosting the schema at a URL (e.g., GitHub Pages)
- Making the schema path in init conditional or omitting it by default

---

## 5. Summary Checklist

**Release blockers:**
- [ ] Add LICENSE file
- [ ] Add CHANGELOG.md
- [ ] Set up git remote
- [ ] Add repository/keywords/categories to Cargo.toml
- [ ] Add release profile (strip, lto)
- [ ] Create .github/workflows/ci.yml
- [ ] Create .github/workflows/release.yml
- [ ] Rewrite README.md for users (install, usage, examples)

**High value improvements:**
- [ ] Create www/index.html + .github/workflows/pages.yml
- [ ] Add `fmt-check` to `just check`
- [ ] Add `just default` (list recipes)
- [ ] Add `just release` recipe
- [ ] Remove unused deps (thiserror, log, env_logger) or use them
- [ ] Extract shared test helpers to reduce duplication
- [ ] Pre-compile glob matchers in checker
- [ ] Add schema sync test or regeneration recipe

**Nice to have:**
- [ ] `--color` flag support
- [ ] Host JSON schema at a stable URL
- [ ] Add `include` field to Cargo.toml for clean publishes
