#import "@local/mindtape:0.2.0": *

#show: task.with(
  title: "Cap a language under any spelling its limits key parses as",
  status: proposed(2026, 10, 28),
)

= Summary

`validate` parses a `limits` key with tokei's case-insensitive `FromStr`, but
`check` looks the file's language up by tokei's display name, exactly. A key
that parses but is spelled otherwise passes validation and caps nothing:

```yaml
limits:
  Bash: 1   # accepted; a file tokei calls "BASH" is never held to it
```

Reported from mindtape, whose shebang-only `.githooks/` scripts are tokei's
`BASH` and went uncapped: the dead `Bash` key read as linecop not detecting
shebangs at all.

= Scope

- Load turns each `limits` key into tokei's display name, so `Bash`, `bash` and
  `BASH` all cap BASH files.
- Two keys naming one language are an error, as a duplicate key would be.
- A test through the binary with a lowercase or differently cased key.
