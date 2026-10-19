# Development recipes

set quiet

# List available recipes
default:
    @just --list

# Run all checks (format + clippy + tests + file size)
# Silent on success; on a test failure prints the full test log.
check:
    #!/usr/bin/env bash
    set -euo pipefail
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -q -- -D warnings
    log=$(mktemp)
    trap 'rm -f "$log"' EXIT
    cargo test --workspace -q >"$log" 2>&1 || { cat "$log"; exit 1; }
    cargo run -q -- --quiet || { cargo run -q --; exit 1; }

# Run tests only
test *ARGS:
    cargo test --workspace {{ ARGS }}

# Run clippy only
clippy:
    cargo clippy --workspace --all-targets -q -- -D warnings

# Auto-fix clippy warnings
clippy-fix:
    cargo clippy --fix --workspace --all-targets -- -D warnings

# Build the project
build:
    cargo build --workspace -q

# Run coverage with tarpaulin (settings live in tarpaulin.toml)
cover:
    #!/usr/bin/env bash
    set -eo pipefail
    log=$(mktemp)
    trap 'rm -f "$log"' EXIT
    cargo tarpaulin --workspace --skip-clean 2>&1 | tee "$log"
    # Tarpaulin only warns on a rejected tarpaulin.toml and silently falls back
    # to its defaults — which drops both the engine and the threshold.
    if grep -q "Invalid config file" "$log"; then
        echo "error: tarpaulin.toml was rejected, coverage ran unconfigured" >&2
        exit 1
    fi

# Format code
fmt:
    cargo fmt --all

# Format check (CI-friendly)
fmt-check:
    cargo fmt --all -- --check

# Regenerate the JSON Schema file
schema:
    cargo run -q -- schema > linecop-schema.json

# Count tests across workspace
count-tests:
    #!/usr/bin/env bash
    cargo test --workspace 2>&1 | grep "test result:" | awk '{sum += $4} END {print sum " tests"}'

# Show top 20 files by line count
file-sizes:
    #!/usr/bin/env bash
    find . -type f \( -name '*.rs' -o -name '*.md' \) ! -path './target/*' -exec wc -l {} + | sort -rn | head -20

# Check for oversized files (fails if any exceed limits)
check-file-size:
    cargo run -q -- --quiet

# Fork a sibling worktree on a new branch (`just fork <slug>`); extra flags go to
# `wt switch`, e.g. `--base=@` to branch from HEAD, not main. The `pre-start`
# hook in .config/wt.toml allows the new worktree's `.envrc`. `wt` writes its cd
# directive to a temp file nobody reads, which keeps it from warning that shell
# integration is missing.
[positional-arguments]
fork SLUG *ARGS:
    #!/usr/bin/env bash
    set -euo pipefail
    cd_file=$(mktemp)
    trap 'rm -f "$cd_file"' EXIT
    WORKTRUNK_DIRECTIVE_CD_FILE="$cd_file" wt switch --create --yes "$@"

# Gate, fast-forward the branch onto main and remove its worktree; run it in the
# worktree. `--no-squash --no-commit` keeps each commit's `Refs:` footer and
# refuses a dirty tree instead of committing it; the `pre-merge` hooks in
# .config/wt.toml run the gate and the wip check before the fast-forward. The cd
# directive goes to a temp file nobody reads, as in `fork`; `wt`'s exit code is
# the recipe's.
[positional-arguments]
land *ARGS:
    #!/usr/bin/env bash
    set -euo pipefail
    cd_file=$(mktemp)
    trap 'rm -f "$cd_file"' EXIT
    WORKTRUNK_DIRECTIVE_CD_FILE="$cd_file" wt merge --no-squash --no-commit --yes "$@"

# The tag push is the only publish path: never `cargo publish` by hand. One
# command does the whole release, so the push of main can't be forgotten and no
# tag lands on a red commit: every check that needs no push runs first; then
# main is pushed (fast-forward only), the recipe waits for the `ci.yml` run on
# that exact commit, and tags only if it is green. Each refusal prints `error:`
# plus a `hint:` with the way out. `--dry-run` runs the checks and says what it
# would push. A tag whose workflow failed is finished on the same tag, never
# re-tagged: `gh run rerun <run> --failed` on its `release.yml` run.
# Push main, wait for its CI, then tag (usage: just release 0.1.0 [--dry-run])
[no-exit-message]
release VERSION *FLAGS:
    #!/usr/bin/env bash
    set -euo pipefail
    version='{{ VERSION }}'
    tag="v$version"
    dry_run=false
    refuse() { echo "error: $1" >&2; shift; for h in "$@"; do echo "hint: $h" >&2; done; exit 1; }
    for flag in {{ FLAGS }}; do
        case "$flag" in
            --dry-run) dry_run=true ;;
            *) refuse "unknown flag '$flag'" "just release VERSION [--dry-run]" ;;
        esac
    done
    [[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]] \
        || refuse "'$version' is not a semver version" "give it without the v: just release 1.2.3"
    cargo_version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
    [ "$version" = "$cargo_version" ] \
        || refuse "requested '$tag' but Cargo.toml is '$cargo_version'" "bump Cargo.toml to '$version' and commit it first"
    for tool in git cargo curl gh; do
        command -v "$tool" >/dev/null \
            || refuse "'$tool' is not on PATH" "run inside the dev shell"
    done
    gh auth status >/dev/null 2>&1 || refuse "gh is not logged in" "gh auth login"
    crate=$(sed -n 's/^name = "\(.*\)"/\1/p' Cargo.toml | head -1)
    repo_url=$(sed -n 's/^repository = "\(.*\)"/\1/p' Cargo.toml | head -1)
    [ -z "$(git status --porcelain)" ] \
        || refuse "the working tree has uncommitted changes" "commit or discard them, then re-run"
    branch=$(git symbolic-ref --short -q HEAD || true)
    [ "$branch" = main ] || refuse "HEAD is on '${branch:-a detached commit}', not main" "git switch main"
    git fetch --quiet origin main || refuse "can't fetch 'origin main'" "check the network and the origin remote"
    git merge-base --is-ancestor origin/main HEAD \
        || refuse "origin/main has commits that main lacks" "git pull --ff-only origin main, then re-run"
    if git rev-parse -q --verify "refs/tags/$tag" >/dev/null \
        || git ls-remote --exit-code --tags origin "refs/tags/$tag" >/dev/null; then
        refuse "'$tag' already exists" "a pushed tag never moves; finish a failed release with: gh run list --workflow release.yml --limit 1, then gh run rerun <run> --failed"
    fi
    status=$(curl -sS -o /dev/null -w '%{http_code}' -A "$repo_url" \
        "https://crates.io/api/v1/crates/$crate/$version" || true)
    case "$status" in
        404) ;;
        200) refuse "'$crate $version' is already on crates.io" "a published version is never re-published; bump to the next version" ;;
        *) refuse "crates.io answered HTTP '${status:-nothing}' for '$crate $version'" "retry once crates.io is reachable" ;;
    esac
    grep -Eq "^## ${version//./\\.} - [0-9]{4}-[0-9]{2}-[0-9]{2}" CHANGELOG.md \
        || refuse "CHANGELOG.md has no dated section for '$version'" "rename '## Unreleased' to '## $version - $(date +%F)' and commit"
    just check || refuse "just check failed on this tree" "fix what it reports above, then commit"
    cargo publish --dry-run --locked --quiet || refuse "cargo publish --dry-run failed" "fix the packaging error above"
    head=$(git rev-parse HEAD)
    short=$(git rev-parse --short HEAD)
    ahead=$(git rev-list --count origin/main..HEAD)
    if [ "$ahead" -gt 0 ]; then
        plan="push $ahead commit(s) to origin main ('$(git rev-parse --short origin/main)'..'$short')"
    else
        plan="push nothing (origin main is already '$short')"
    fi
    if $dry_run; then
        echo "dry run: every check passed"
        echo "would $plan, wait for ci.yml on '$short', then tag and push '$tag'"
        exit 0
    fi
    if [ "$ahead" -gt 0 ]; then
        git push --quiet origin "$head:refs/heads/main" \
            || refuse "origin rejected the push of main" "git pull --ff-only origin main, then re-run"
    fi
    run=""
    deadline=$((SECONDS + 300))
    while :; do
        run=$(gh run list --commit "$head" --workflow ci.yml --limit 1 \
            --json databaseId --jq '.[0].databaseId // empty') \
            || refuse "can't list ci.yml runs for '$short'" "check gh auth status and that .github/workflows/ci.yml exists"
        [ -n "$run" ] && break
        [ "$SECONDS" -lt "$deadline" ] \
            || refuse "no ci.yml run showed up for '$short' in 5 minutes, so '$tag' was not made" "main is pushed; once ci.yml is green on it, re-run just release $version"
        sleep 5
    done
    echo "waiting for ci.yml run '$run' on '$short'"
    if ! gh run watch "$run" --exit-status --compact --interval 15 >/dev/null; then
        conclusion=$(gh run view "$run" --json conclusion --jq .conclusion 2>/dev/null || true)
        refuse "ci.yml ended '${conclusion:-unknown}' on '$short', so '$tag' was not made" \
            "see why: gh run view $run --log-failed" \
            "fix main and re-run just release $version, or retry a flake with: gh run rerun $run --failed"
    fi
    git tag -a "$tag" -m "Release $tag" "$head"
    git push --quiet origin "refs/tags/$tag" \
        || { git tag -d "$tag" >/dev/null; refuse "origin rejected the push of '$tag'" "re-run just release $version"; }
    echo "pushed '$tag'; follow it with: gh run list --workflow release.yml --limit 1"
