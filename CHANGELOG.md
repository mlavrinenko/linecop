# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Added

- `max_bytes` on an override, beside `limit`: a file breaches when it exceeds
  either, and the report names which (`31204 bytes (max_bytes: 14000, +17204
  over)`). Bounds a file whose lines are paragraphs, which grows in bytes and
  not in lines. `--baseline` applies to it, and JSON output gains `bytes`,
  `max_bytes` and `baseline-max-bytes` for a file that has the key

## 0.5.0 - 2026-10-03

### Changed

- Operational errors (malformed or unreadable config, unknown language, invalid
  override glob, missing scan path, I/O failure, `init` refusing to overwrite)
  now exit `2` instead of `1`, so a broken gate no longer reads as violations.
  `1` still means violations found

## 0.4.0 - 2026-08-15

### Added

- `include_hidden` config key and `--hidden` flag: scan dot-prefixed files and
  directories (e.g. `.just/scripts`), which were previously invisible to every
  limit unless passed as an explicit scan root. Off by default; VCS metadata
  (`.git`, `.hg`, `.svn`, `.jj`) stays excluded and ignore files still apply

## 0.3.0 - 2026-06-04

### Added

- `--format=paths` output format: prints only violating file paths, one per line,
  for piping to other tools (e.g. `ejectest apply src/ --files-from -`)
- `--baseline=<PERCENT>` flag: reports files at or above the given percentage of
  their limit (1-100). Default is 100 (backward-compatible: only files strictly
  over the limit)
- JSON output now includes `baseline-limit` field for each violation

## 0.2.0 - 2026-04-13

### Added

- `init` now embeds a `yaml-language-server` schema comment pointing to the
  versioned JSON Schema on GitHub (e.g. `v0.2.0`)
- `init --schema <URL>` to specify a custom schema URL
- `init --no-schema` to omit the schema comment entirely

## 0.1.0 - 2026-03-10

### Added

- Language-aware file line counting powered by tokei
- Per-language line limits via `.linecop.yaml` configuration
- Per-path glob overrides with custom limits or exclusions
- Count modes: total, code-only, code+comments
- JSON and text output formats
- `init` subcommand to generate starter config
- `schema` subcommand to print JSON Schema for config validation
- `--quiet` mode for CI integration (exit code only)
- `--color` flag for explicit color control
- JSON Schema file with yaml-language-server support
- Configurable directory exclusions
- Configless mode: runs with 500-line default when no `.linecop.yaml` is found
- Upward config search: traverses from scan path to CWD to find `.linecop.yaml`
- `--no-config-warning` flag to suppress the missing-config warning
- Landing page (`www/index.html`) and logo (`www/logo.svg`)
- GitHub Pages deployment workflow (`.github/workflows/pages.yml`)
