#import "@local/mindtape:0.2.0": *

#show: task.with(
  title: "Find the config above the working directory",
  status: proposed(2026, 10, 3),
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

- Walk up from the scanned path to the filesystem root (or a repo boundary, to
  decide) and use the nearest `.linecop.yaml`.
