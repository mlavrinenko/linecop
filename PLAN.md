# LineCop MVP Implementation Plan

## Goal

Build an MVP that:
- Reads `.linecop.yaml` config with per-language limits and glob overrides
- Uses **tokei as a Rust library** to count lines per file
- Reports violations and exits non-zero
- Generates a JSON Schema for the config (bundled in binary, `linecop schema` subcommand)
- Replaces the current `check-file-size.jq` with linecop itself (dogfooding)
- Supports configurable counting mode (total, code-only, code+comments)

## Dependencies to Add

```toml
tokei = { version = "14", default-features = false }  # no CLI feature needed
serde_yaml = "0.9"
glob = "0.3"
schemars = "1"
# dev:
tempfile = "3"
```

Note: tokei does NOT have a `serialization` feature. Use `default-features = false`
to skip its CLI deps. Serde support is always available.

## Module Structure

```
src/
  main.rs      — thin clap CLI entry point
  lib.rs       — module re-exports + run() orchestrator
  config.rs    — Config/Override/CountMode types, YAML parsing, validation
  counter.rs   — tokei wrapper, produces Vec<FileStats> per file
  checker.rs   — applies limits to files, returns Vec<Violation>
  report.rs    — text and JSON output formatting
  schema.rs    — JSON Schema generation via schemars
```

## Step 1: Config module — `src/config.rs`

Types with serde + schemars derives:

```rust
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// What to count: "total" (default), "code", "code-comments"
    #[serde(default)]
    pub count_mode: CountMode,
    /// Per-language line limits (keys are tokei language names: "Rust", "Python", etc.)
    pub limits: BTreeMap<String, u64>,
    /// Per-path overrides
    #[serde(default)]
    pub overrides: Vec<Override>,
}

#[derive(Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum CountMode {
    #[default]
    Total,
    Code,
    CodeComments,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Override {
    pub pattern: String,
    #[serde(default)]
    pub limit: Option<u64>,
    #[serde(default)]
    pub exclude: bool,
}
```

Functions:
- `pub fn load(path: &Path) -> Result<Config>` — read and parse YAML
- `pub fn default_path() -> PathBuf` — returns `.linecop.yaml`
- Validate: language names via `LanguageType::from_str`
- Validate: overrides must have either `limit` or `exclude: true`

~80-100 lines.

## Step 2: Counter module — `src/counter.rs`

```rust
pub struct FileStats {
    pub path: PathBuf,
    pub language: String,
    pub total: u64,
    pub code: u64,
    pub comments: u64,
    pub blanks: u64,
}

pub fn count(root: &Path, config: &Config) -> Result<Vec<FileStats>>
```

Implementation:
1. Build `tokei::Config { types: Some(vec![...]) }` from config language keys
2. Call `Languages::get_statistics(&[root], &["target"], &tokei_config)`
3. Iterate languages → reports → build `FileStats` from `report.stats`
4. Handle nested `blobs` in `CodeStats` (Rust-in-Markdown etc.) by summing recursively
   (same logic as current `check-file-size.jq` `total_lines` function)

### Key tokei API reference

```rust
use tokei::{Config, Languages, LanguageType};

let mut languages = Languages::new();
languages.get_statistics(&[root], &["target"], &config);

// Per language:
//   language.reports -> Vec<Report> { name: PathBuf, stats: CodeStats }
//   CodeStats fields: .code, .comments, .blanks
//   CodeStats methods: .lines() = code + comments + blanks
//   CodeStats.blobs: BTreeMap<LanguageType, CodeStats> (nested languages)
//
// LanguageType: 329 variants, implements FromStr, Display, Serialize
```

~60-80 lines.

## Step 3: Checker module — `src/checker.rs`

```rust
pub struct Violation {
    pub path: PathBuf,
    pub language: String,
    pub lines: u64,
    pub limit: u64,
}

pub fn check(files: &[FileStats], config: &Config) -> Vec<Violation>
```

Per file:
1. Check glob overrides in order: `exclude: true` → skip; `limit` → use it
2. Fall back to `config.limits[language]`
3. Select count based on `config.count_mode` (total/code/code+comments)
4. If count > limit → Violation

Glob matching via `glob::Pattern::matches_path`.

~50-70 lines.

## Step 4: Report module — `src/report.rs`

```rust
pub enum Format { Text, Json }

pub fn print(violations: &[Violation], format: Format) -> Result<()>
```

- Text: `--- ./src/big.rs: 523 lines (limit: 500)` (matches current jq output)
- JSON: serialize violations array
- Summary line at end

~40-50 lines.

## Step 5: Schema module — `src/schema.rs`

```rust
pub fn generate() -> String {
    let schema = schema_for!(Config);
    serde_json::to_string_pretty(&schema).expect("schema serialization")
}
```

~15 lines.

## Step 6: Wire up `src/lib.rs`

```rust
pub mod checker;
pub mod config;
pub mod counter;
pub mod report;
pub mod schema;

pub fn run(path, config_path, quiet, format) -> Result<bool>
// Orchestrates: load config → count → check → report → return has_violations
```

~40 lines.

## Step 7: CLI in `src/main.rs`

```rust
#[derive(Parser)]
struct Cli {
    #[arg(default_value = ".")]
    path: PathBuf,
    #[arg(short, long, default_value = ".linecop.yaml")]
    config: PathBuf,
    #[arg(short, long)]
    quiet: bool,
    #[arg(long, default_value = "text", value_enum)]
    format: Format,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Print JSON Schema for .linecop.yaml
    Schema,
}
```

~35 lines.

## Step 8: Dogfooding

Create `.linecop.yaml`:
```yaml
# yaml-language-server: $schema=linecop-schema.json
limits:
  Rust: 500
  Markdown: 200

overrides:
  - pattern: "RESEARCH.md"
    exclude: true
```

Update `Justfile` `check-file-size` recipe:
```
cargo run -q -- --quiet
```

Delete `check-file-size.jq`.

Add `linecop-schema.json` to `.gitignore` (generated on demand).

## Step 9: Tests

Unit tests (in each module):
- `config.rs`: parse valid/invalid YAML, validate language names, validate overrides
- `counter.rs`: count temp directory with known content (tempfile crate)
- `checker.rs`: limit logic, override priority, exclude, count modes
- `report.rs`: text and JSON format

Integration tests:
- `assert_cmd` on test fixture directory
- Exit 0 when passing, exit 1 on violations
- `linecop schema` outputs valid JSON

## Step 10: Verification

1. `just fmt`
2. `just check` (clippy + tests + file size via linecop itself)
3. `just cover` (≥70%)
4. `cargo run -- .` on the linecop repo
5. `cargo run -- schema` to verify schema output

## Config schema support

The `.linecop.yaml` file uses `# yaml-language-server: $schema=<path-or-url>` comment
for editor schema validation (works in Zed, VS Code, etc.).

For now: `linecop schema > linecop-schema.json` generates locally, referenced
as relative path. Later: host the schema at a URL.

Zed docs on YAML schemas: schemas can be URLs, `./relative` paths, or `~/home` paths.
