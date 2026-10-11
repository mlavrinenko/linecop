#import "@local/mindtape:0.2.0": *

#show: task.with(
  title: "Fork and land worktrees with in-repo worktrunk hooks",
  status: wip(2026, 10, 11),
)

= Summary

`~/.nix` no longer ships worktrunk user hooks, so a worktree of this repo gets neither its dev shell nor a gate. Carry the tooling in the repo: `worktrunk` in the dev shell, project hooks in `.config/wt.toml`, and `just fork` / `just land` recipes.

= Problem

+ `wt switch --create scratch`, then `cd ../linecop.scratch && just check`: direnv refuses the new `.envrc` ("direnv: error ... is blocked"), so `just`, `mt` and `cargo` do not resolve until someone runs `direnv allow` by hand.
+ Commit there and run `wt merge`: it squashes the commits, dropping each `Refs:` footer, and fast-forwards main with no gate. Neither `just check` nor `mt check --commits` runs.
+ `worktrunk` is not in the dev shell of `flake.nix`, and no user hook supplies either step.

= Changes

- `flake.nix`: add `worktrunk` to the dev shell packages.
- `.config/wt.toml` (new): `pre-start` runs `direnv allow`; `pre-merge` runs `direnv exec . just check`, then `direnv exec . mt check --commits {{ target }}..HEAD`, whose tip rule refuses a branch that names a task left `wip`.
- `Justfile`: `fork SLUG *ARGS` runs `wt switch --create --yes`, `land *ARGS` runs `wt merge --no-squash --no-commit --yes`. The flags keep every `Refs:` footer and refuse a dirty tree.
- `AGENT.md`, `CONTRIBUTING.md`: delegate with `just fork <slug>`, land with `just land`, never a bare `wt merge`.

= Done when

- `just fork <slug>` makes a sibling worktree whose `.envrc` is allowed: `mt --version` resolves there at once.
- `just land` in the worktree runs `just check` and `mt check --commits main..HEAD` before the fast-forward, and a red gate stops the merge.
- Dogfood: `just fork scratch --base=@` from this worktree, a trivial commit, `wt hook pre-merge --yes --var target=main` runs the gate, `wt remove scratch -D --yes`.
- Out of scope: a `pre-remove` hook (no `.mmz/` records here) and an lcov carry (`just cover` rebuilds its tarpaulin report, nothing persists).
