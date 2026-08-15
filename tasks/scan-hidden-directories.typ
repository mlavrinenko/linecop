#import "@local/mindtape:0.2.0": *

#show: task.with(
  title: "Scan hidden directories",
  status: wip(2026, 8, 15),
)

= Summary

The walker never descends into dot-directories, so files under `.just/scripts`,
`.config/`, `.github/scripts/` and friends are invisible to every limit. The
config schema has no key to change that — `limits`, `overrides`, `exclude_dirs`,
`count_mode` is the whole surface.

Reported from mindtape, measured there with a 401-line script at
`.just/scripts/_probe.sh` against a `Shell: 300` limit:

```
$ linecop            # what the size gate runs
EXIT=0               # probe not reported at all

$ linecop .just
--- .just/scripts/_probe.sh: 401 lines (limit: 300, +101 over)
EXIT=1
```

The cap works when the hidden directory is the explicit scan root, and only
then. Nothing is wrong with the limit; the walker never reaches the files.

That project can paper over it with a second scan root (`linecop && linecop
.just`), but every project that moves build clutter under a dot-directory hits
the same wall. Fix belongs here.

= Scope

- `include_hidden` config key, default `false`. Opt-in, because turning it on
  by default would silently start walking `.git`, `.venv` and every other
  dot-directory in existing users' repos.
- `--hidden` CLI flag forcing it on for one run, for configless scans and gate
  one-liners.
- VCS metadata directories (`.git`, `.hg`, `.svn`, `.jj`) stay skipped when
  hidden scanning is on — nobody wants a limit enforced against packfiles.
  Gitignored dot-directories (`.direnv`, `.venv`) stay skipped by the existing
  ignore-file handling.
- Regression test that bites end-to-end: an oversized file under a dot-directory
  must be absent from the report by default and must fail the run with
  `include_hidden: true`, asserted through the binary, not the library.
- Schema, README, landing page and changelog all name the new key.
