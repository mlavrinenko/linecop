#import "@local/mindtape:0.2.0": *

#show: task.with(
  title: "Match override patterns with gitignore rules",
  status: proposed(2026, 10, 3),
)

= Summary

Override patterns compile with `globset::Glob::new` defaults, where `*` crosses
`/`, and a pattern without a slash matches only at the config's top level:

```
$ cat .linecop.yaml                     # src/top.rs, src/gen/deep.rs, docs/RESEARCH.md all over
overrides: [{pattern: "src/*.rs", exclude: true}, {pattern: "RESEARCH.md", exclude: true}]
$ linecop --format paths
./docs/RESEARCH.md                      # missed: no-slash pattern is top level only
                                        # src/gen/deep.rs hidden: `*` crossed `/`
```

Every tool that takes path patterns from a file (git, CODEOWNERS, EditorConfig,
Prettier) keeps `*` inside one segment and matches a slash-less pattern at any
depth. linecop should too, with the `ignore` crate's gitignore matcher rather
than its own rules.

= Scope

- Each override is one gitignore line rooted at the config's directory: `*`
  stays in a segment, `**` crosses, a slash-less pattern matches at any depth,
  a leading `/` or inner slash anchors, a trailing `/` covers a directory's
  files.
- A file outside the config's directory gets no override, as a `.gitignore`
  never reaches outside its own directory.
- A `!` pattern is a config error: overrides apply first-match, not gitignore's
  last-match negation.
- `globset` leaves the direct dependencies.
- Breaking: `src/*.rs` no longer reaches `src/gen/`; write `src/**/*.rs`. Minor
  version bump.

= Out of scope

- Config discovery (`find-the-config-above-the-working-directory`).
