#import "@local/mindtape:0.2.0": *

#show: task.with(
  title: "Resolve config paths from a root key",
  status: done(2026, 10, 19),
)

= Summary

A repo that keeps its tool configs in `.config/` cannot move `.linecop.yaml`
there. `--config` reads the file, but every override pattern is a path from the
config file's directory, so it stops reaching the tree the config describes,
and a `../` pattern matches nothing:

```
$ cat .config/linecop.yaml
limits: {Markdown: 2}
overrides:
  - pattern: "CHANGELOG.md"   # or "../CHANGELOG.md"
    exclude: true
$ linecop -c .config/linecop.yaml --format paths; echo $?
./CHANGELOG.md
1
```

Mindtape wants its config in `.config/linecop.yaml`, beside `.config/wt.toml`.

= Scope

- An optional top-level `root:`, a path from the config file's directory.
  Override patterns resolve from it, and with no `PATH` argument linecop scans
  it, printing paths relative to the working directory (`./` when they are one
  directory). A `root` that is no directory is a config error.
- Without `root`, nothing changes.
- Discovery looks for `.config/linecop.yaml` beside `.linecop.yaml` in each
  directory it walks; `.linecop.yaml` wins in the same directory. A found file
  reads exactly as under `--config`, so it needs `root: ..`.
- End-to-end tests through the binary: a `.config/` file without `root` keeps
  anchoring at `.config/`; with `root: ..` a pattern reaches the repo; the
  default scan root and its printed prefix; an explicit `PATH` still wins; a
  missing root exits 2; discovery and its precedence.
- Schema, README, landing page, skill and changelog describe the key.
- Dogfood on a copy of Mindtape: `.linecop.yaml` at the root, then moved to
  `.config/linecop.yaml` with `root: ..`, gives byte-identical output.

= Out of scope

- `exclude_dirs` holds directory names, which match at any depth, so `root`
  leaves it alone.
- `init` keeps writing `.linecop.yaml`.
