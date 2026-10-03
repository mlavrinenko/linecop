#import "@local/mindtape:0.2.0": *

#show: task.with(
  title: "Find the config above the working directory",
  status: wip(2026, 10, 3),
)

= Summary

Config discovery walks up from the scanned path but stops at the working
directory, so running inside a subdirectory ignores the repo's config and
silently checks against the 500-line fallback:

```
$ cat .linecop.yaml
limits: {Rust: 2}
$ linecop --format paths; echo $?
./src/x.rs
1
$ cd src && linecop --format paths; echo $?
warning: no .linecop.yaml found, using default limit of 500 lines for all files
0
```

git, Prettier, ESLint, Ruff, rustfmt and EditorConfig all walk up past the
working directory to find their config.

= Scope

- Walk up from the scanned path (canonicalized, so `..` and symlinks resolve)
  and use the nearest `.linecop.yaml`, stopping after the repository root: the
  first directory holding `.git` (directory or file, for worktrees), `.jj`,
  `.hg` or `.svn`.
- Outside any repository, keep today's stop at the working directory.
- Opt-in `--config-search <repo|root>` (default `repo`): `root` walks on to the
  filesystem root, for the rare tree that is no repository.
- `--config` still bypasses discovery.
- End-to-end tests through the binary: from a subdirectory of a repo, from a
  subdirectory with `src` scanned, a config above the repo root is not found by
  default and is found with `--config-search root`, a nested repo stops at its
  own root.
- README, help text, changelog (0.7.0 section, still unreleased) and the
  linecop skill say where the config is found.
