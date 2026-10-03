# linecop

[![CI](https://github.com/mlavrinenko/linecop/actions/workflows/ci.yml/badge.svg)](https://github.com/mlavrinenko/linecop/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/linecop.svg)](https://crates.io/crates/linecop)

Language-aware file size linter. Patrols your code base to enforce per-language
line count limits using [tokei](https://github.com/XAMPPRocky/tokei) for
accurate counting.

## Installation

### Cargo

```bash
cargo install linecop
```

### Nix flake

```bash
nix run github:mlavrinenko/linecop
```

### Binary releases

Pre-built binaries for Linux and macOS are available on the
[releases page](https://github.com/mlavrinenko/linecop/releases).

## Quick start

```bash
linecop init        # creates .linecop.yaml with sensible defaults
linecop             # scan current directory
```

## Usage

```bash
linecop [PATH] [OPTIONS]
linecop [PATH] <COMMAND>
```

**Commands:**

| Command | Description |
|---------|-------------|
| `init` | Generate a starter `.linecop.yaml` |
| `schema` | Print JSON Schema for config validation |

**Options:**

| Option | Description |
|--------|-------------|
| `-c, --config <FILE>` | Config file path (default: auto-detected) |
| `--config-search <repo\|root>` | How far up to look for the config: `repo` (default) stops at the repository root, `root` walks to the filesystem root |
| `-q, --quiet` | Suppress output (exit code only) |
| `--format <text\|json\|paths>` | Output format (default: `text`) |
| `--baseline <PERCENT>` | Report files at or above this percentage of their limit, 1-100 (default: `100`) |
| `--hidden` | Scan dot-prefixed files and directories (overrides `include_hidden`) |
| `--color <auto\|always\|never>` | Control color output |
| `--no-config-warning` | Suppress the warning when no config file is found |

### Exit codes

| Code | Meaning |
|------|---------|
| `0` | No file exceeds its limit |
| `1` | Violations found — the codebase needs attention |
| `2` | Operational error — the check never ran: malformed or unreadable config, unknown language, invalid override glob, missing scan path, I/O failure, bad CLI arguments |

In a gate recipe, treat `2` as a broken gate rather than as violations.

### Examples

```bash
# Scan a specific directory
linecop src/

# Use a custom config
linecop --config my-config.yaml

# JSON output for CI
linecop --format json --quiet

# Report files at 90%+ of their limit (early warning)
linecop --baseline 90

# Paths only, one per line, for piping to other tools
linecop --baseline 90 --format paths | ejectest apply src/ --files-from -

# Include dot-directories such as .just/scripts or .github/scripts
linecop --hidden

# Generate JSON Schema for editor validation
linecop schema > linecop-schema.json
```

JSON output carries `lines`, `limit` and `baseline-limit` (the threshold after
`--baseline`) for each file, plus `bytes`, `max_bytes` and `baseline-max-bytes`
for a file whose override sets `max_bytes`.

## Configuration

Create a `.linecop.yaml` in your project root. linecop uses the nearest one found
walking up from the scanned path, and stops after the repository root (a directory
with `.git`, `.jj`, `.hg` or `.svn`), or at the working directory outside a
repository. `--config-search root` walks on to the filesystem root; `--config`
skips the search.

```yaml
limits:
  Rust: 500
  Markdown: 200
  Python: 400

count_mode: total  # total | code | code-comments

overrides:
  - pattern: "src/generated_*.rs"
    limit: 1000
  - pattern: "CONTRIBUTING.md"  # loaded whole by agents
    max_bytes: 14000
  - pattern: "RESEARCH.md"
    exclude: true

exclude_dirs:
  - target
  - node_modules

include_hidden: false  # scan .just/scripts, .github/scripts, ...
```

Language names follow [tokei conventions](https://github.com/XAMPPRocky/tokei#supported-languages).

An override pattern is a `.gitignore` line, read from the directory holding the
config file, so it matches the same files whatever you scan and wherever you run
linecop:

| Pattern | Matches |
|---------|---------|
| `RESEARCH.md` | that name at any depth |
| `src/*.rs` | `.rs` files directly in `src/` |
| `src/**/*.rs` | `.rs` files anywhere under `src/` |
| `/build.rs` | `build.rs` at the top only |
| `vendor/` | every file under any `vendor` directory |

The first matching override wins. `!` negation is not supported; order the
overrides instead.

An override sets `limit`, `max_bytes`, or both. A line limit does not bound a
file whose lines are paragraphs, so `max_bytes` caps its size too; the file
breaches when it exceeds either, and the report names which:
`--- CONTRIBUTING.md: 31204 bytes (max_bytes: 14000, +17204 over)`. An override
without `limit` keeps the language limit, and `--baseline` applies to both.

Dot-prefixed files and directories are skipped unless `include_hidden: true`
(or `--hidden`) is set. With it on, `.git`, `.hg`, `.svn` and `.jj` stay out of
the scan, and gitignored paths are still ignored — so a hidden build directory
gets limits without dragging repository metadata in.

## Agent skill

[`skills/linecop`](skills/linecop/SKILL.md) teaches a coding agent to check
headroom with `--baseline` before growing a file, instead of meeting the limit
when the gate fails. Copy or link it into your agent's skills directory, e.g.
`~/.claude/skills/linecop`.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for coding conventions and guidelines.

## License

[MIT](LICENSE)
