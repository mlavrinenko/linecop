#import "@local/mindtape:0.2.0": *

#show: task.with(
  title: "Let an override limit stand without a language base",
  status: proposed(2026, 10, 28),
)

= Summary

`count` hands tokei only the languages named in `limits`, so a file whose
language has no entry there is never counted, and an override's `limit:` or
`max_bytes:` on it does nothing. Reported from mindtape, which caps
`docs/src/**/*.typ` by override and has to carry a `Typst: 1000` base whose
only job is to make tokei count Typst files:

```yaml
limits:
  Rust: 500
overrides:
  - pattern: "docs/*.typ"
    limit: 10       # inert: no Typst file is ever counted
```

= Scope

- When any override sets `limit` or `max_bytes`, scan every language, so the
  override's caps apply to whatever file it matches. A file no cap reaches
  stays unreported, as an unlisted language is today.
- A test through the binary: an override-only cap on a language with no base
  reports the file.
- README and changelog say an override needs no language base.
