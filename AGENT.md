# linecop

## Agent Rules

- Use `just` recipes instead of raw cargo commands (see `Justfile`)
- Use `-q` for cargo commands — only show errors/warnings
- After any code changes, run `just check` and fix all warnings
- If clippy suggests `--fix`, use `cargo clippy --fix --workspace --all-targets`

## Tasks

Work is tracked in-repo with MindTape (`mt`): one task per file under `tasks/`,
with the status legend in `.mindtape/config.toml`. `mt ls` shows what is open,
`mt add <title words>` files a new task, `mt done <task>` closes one. Drive the
CLI — `mt add` stamps the status and slugs the filename, and a hand-written task
file skips that. The task body is Typst and is yours to edit; run `mt check`
after.

Commits use Conventional Commits with a footer `Refs: tasks/<stem>.typ` naming
the driving task. Commits that only file or edit task artifacts carry no `Refs:`
— a task commit refs nothing.

See [CONTRIBUTING.md](CONTRIBUTING.md) for project conventions and code standards.
