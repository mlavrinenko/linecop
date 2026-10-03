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

# Tag and push a release
release version:
    @echo "Tagging v{{version}}..."
    git tag -a "v{{version}}" -m "Release v{{version}}"
    git push origin "v{{version}}"
