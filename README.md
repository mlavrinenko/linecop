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
| `-q, --quiet` | Suppress output (exit code only) |
| `--format <text\|json\|paths>` | Output format (default: `text`) |
| `--baseline <PERCENT>` | Report files at or above this percentage of their limit, 1-100 (default: `100`) |
| `--color <auto\|always\|never>` | Control color output |
| `--no-config-warning` | Suppress the warning when no config file is found |

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

# Generate JSON Schema for editor validation
linecop schema > linecop-schema.json
```

With `--baseline` below 100, JSON output gains a `baseline-limit` field
(the effective threshold) alongside `lines` and `limit` for each file.

## Configuration

Create a `.linecop.yaml` in your project root:

```yaml
limits:
  Rust: 500
  Markdown: 200
  Python: 400

count_mode: total  # total | code | code-comments

overrides:
  - pattern: "src/generated_*.rs"
    limit: 1000
  - pattern: "RESEARCH.md"
    exclude: true

exclude_dirs:
  - target
  - node_modules
```

Language names follow [tokei conventions](https://github.com/XAMPPRocky/tokei#supported-languages).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for coding conventions and guidelines.

## License

[MIT](LICENSE)
