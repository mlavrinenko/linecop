# linecop

## Agent Rules

- Use `just` recipes instead of raw cargo commands (see `Justfile`)
- Use `-q` for cargo commands — only show errors/warnings
- After any code changes, run `just check` and fix all warnings
- If clippy suggests `--fix`, use `cargo clippy --fix --workspace --all-targets`

See [CONTRIBUTING.md](CONTRIBUTING.md) for project conventions and code standards.
