#import "@local/mindtape:0.2.0": *

#show: task.with(
  title: "Release pushes main, waits for its CI, then tags",
  status: done(2026, 10, 20),
)

= Summary

`just release VERSION` tags and pushes the tag, never main. During the 0.8.0
release main had to be pushed by hand, and nothing waited for CI before the tag
went out, so a red commit could be published.

= Problem

```
release version:
    git tag -a "v{{version}}" -m "Release v{{version}}"
    git push origin "v{{version}}"
```

- The tag can reach origin before the commit it names.
- Nothing checks that `ci.yml` is green on that commit.
- A tag that already exists, or a version already on crates.io, fails only in
  the `release.yml` run, which leaves the release half-done.

= Changes

Take the shape of `typst-world`'s `release` recipe, adapted to linecop:

- Checks that need no push run first: semver, `Cargo.toml` version, clean tree,
  on `main`, `origin/main` an ancestor of `HEAD`, no existing tag (local or
  remote), version not on crates.io, a dated `CHANGELOG.md` section
  (`## X.Y.Z - date`, no brackets), `just check`, `cargo publish --dry-run`.
- Then push `main` (fast-forward only), wait for the `ci.yml` run on that exact
  commit, and tag only if it is green.
- `--dry-run` runs the checks and says what it would push.
- Every refusal prints `error:` and a `hint:` with the way out; a failed
  release is finished with `gh workflow run release.yml -f tag=vX.Y.Z`.
- `CONTRIBUTING.md` gains a Releasing section.

= Done when

- `just release 0.8.1 --dry-run` on a scratch clone prints the checks' verdict and
  the push it would make, and pushes nothing.
- A tag that exists is refused with the hint.
- No real release is cut.
