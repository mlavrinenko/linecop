# linecop

Patrols your code base to enforce line count limits.

## Development

Prerequisites: [Nix](https://nixos.org/) with flakes enabled.

```bash
# Enter dev shell
direnv allow
# or: nix develop

just check # clippy + tests + lines-limit-check
just build
just test
just cover

# Format
just fmt
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for coding conventions and contribution guidelines.

## Running

```bash
nix run          # build and run via nix
cargo run        # or via cargo
```

## License

MIT
