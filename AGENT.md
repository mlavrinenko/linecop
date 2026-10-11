# linecop

## Agent Rules

- Use `just` recipes instead of raw cargo commands (see `Justfile`)
- Use `-q` for cargo commands — only show errors/warnings
- After any code changes, run `just check` and fix all warnings
- If clippy suggests `--fix`, use `cargo clippy --fix --workspace --all-targets`
- Delegated work forks with `just fork <slug>` (a sibling worktree on a new
  branch, dev shell allowed) and lands with `just land` run in that worktree:
  it gates, checks no named task is left `wip`, fast-forwards main and removes
  the worktree. Both wrap [worktrunk](https://worktrunk.dev);
  [.config/wt.toml](.config/wt.toml) holds the hooks. Never run a bare
  `wt merge`: it squashes away the `Refs:` footers.

## Tasks

Work is tracked in-repo with MindTape (`mt`): one task per file under `tasks/`,
with the status legend in `.mindtape/config.toml`. `mt ls` shows what is open,
`mt add <title words>` files a new task, `mt wip <task>` starts one and `mt done <task>` closes it (a `proposed` task cannot jump straight to `done`). Drive the
CLI — `mt add` stamps the status and slugs the filename, and a hand-written task
file skips that. The task body is Typst and is yours to edit; run `mt check`
after.

Commits use Conventional Commits with a footer `Refs: <stem>` naming
the driving task. Commits that only file or edit task artifacts carry no `Refs:`
— a task commit refs nothing.

See [CONTRIBUTING.md](CONTRIBUTING.md) for project conventions and code standards.
