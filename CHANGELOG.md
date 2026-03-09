# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
