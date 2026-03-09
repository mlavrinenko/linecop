# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
