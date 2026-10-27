use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tokei::LanguageType;

/// The default line limit applied to all languages when no config file is found.
pub const DEFAULT_LIMIT: u64 = 500;

/// What lines to count when checking limits.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum CountMode {
    /// Count all lines (code + comments + blanks).
    #[default]
    Total,
    /// Count only code lines.
    Code,
    /// Count code + comment lines (exclude blanks).
    CodeComments,
}

/// A per-path override rule.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Override {
    /// A `.gitignore` line, relative to `root` (default: the config file's
    /// directory): `*` stays within a directory, `**` crosses them, and a
    /// pattern without a slash matches at any depth. `!` negation is not
    /// supported.
    pub pattern: String,
    /// Custom line limit for matched files.
    #[serde(default)]
    pub limit: Option<u64>,
    /// Byte cap for matched files, checked beside the line limit. Catches a
    /// file whose lines are paragraphs: it grows in bytes, not in lines.
    #[serde(default)]
    pub max_bytes: Option<u64>,
    /// If true, matched files are excluded from checking.
    #[serde(default)]
    pub exclude: bool,
}

/// Returns the default set of directories to exclude from scanning.
fn default_exclude_dirs() -> Vec<String> {
    vec!["target".to_owned()]
}

/// Top-level configuration for linecop.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// The directory this config describes, as a path from the config file's
    /// directory (default: that directory). Override patterns resolve from
    /// it, and linecop scans it when no path is given. A config kept in
    /// `.config/` sets `root: ..`.
    #[serde(default)]
    pub root: Option<PathBuf>,
    /// What to count: "total" (default), "code", "code-comments".
    #[serde(default)]
    pub count_mode: CountMode,
    /// Per-language line limits. Keys are tokei language names (e.g. "Rust", "Python").
    #[schemars(schema_with = "crate::schema::language_limits_schema")]
    pub limits: BTreeMap<String, u64>,
    /// Per-path overrides applied in order.
    #[serde(default)]
    pub overrides: Vec<Override>,
    /// Directory names to exclude from scanning (default: ["target"]).
    #[serde(default = "default_exclude_dirs")]
    pub exclude_dirs: Vec<String>,
    /// Scan dot-prefixed files and directories such as `.just/scripts`
    /// (default: false). VCS metadata directories stay excluded, and
    /// gitignored paths are still skipped.
    #[serde(default)]
    pub include_hidden: bool,
    /// Fallback limit for languages not listed in `limits`.
    /// Used when running without a config file.
    #[serde(default)]
    #[schemars(skip)]
    pub default_limit: Option<u64>,
    /// The resolved `root`, canonical: the directory override patterns are
    /// relative to. `None` matches them against paths as scanned.
    #[serde(skip)]
    #[schemars(skip)]
    pub base_dir: Option<PathBuf>,
}

impl Config {
    /// Creates a sensible default config (no per-language limits, 500-line
    /// default for every file).
    #[must_use]
    pub fn fallback() -> Self {
        Self {
            root: None,
            count_mode: CountMode::default(),
            limits: BTreeMap::new(),
            overrides: Vec::new(),
            exclude_dirs: default_exclude_dirs(),
            include_hidden: false,
            default_limit: Some(DEFAULT_LIMIT),
            base_dir: None,
        }
    }
}

/// Loads and validates a config from the given YAML file.
///
/// # Errors
///
/// Returns an error if the file cannot be read, parsed, or contains invalid
/// language names or override rules, or if its `root` is no directory.
pub fn load(path: &Path) -> Result<Config> {
    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read config file: {}", path.display()))?;
    let mut config: Config = serde_yml::from_str(&contents)
        .with_context(|| format!("failed to parse config file: {}", path.display()))?;
    validate(&config)?;
    config.limits = canonical_limits(&config.limits)?;
    let canonical = path
        .canonicalize()
        .with_context(|| format!("failed to resolve config file: {}", path.display()))?;
    let config_dir = canonical.parent().map(Path::to_path_buf);
    config.base_dir = match (&config.root, config_dir) {
        (Some(root), Some(dir)) => Some(
            dir.join(root)
                .canonicalize()
                .ok()
                .filter(|resolved| resolved.is_dir())
                .with_context(|| format!("root {root:?} in {} is no directory", path.display()))?,
        ),
        (_, config_dir) => config_dir,
    };
    Ok(config)
}

/// Validates that all language names are recognized by tokei and that
/// overrides set a limit, a byte cap or exclude.
///
/// # Errors
///
/// Returns an error if a language name is unknown or an override sets none
/// of `limit`, `max_bytes` and `exclude`.
pub fn validate(config: &Config) -> Result<()> {
    for lang_name in config.limits.keys() {
        if lang_name.parse::<LanguageType>().is_err() {
            bail!("unknown language in limits: {lang_name:?}");
        }
    }
    for (idx, ovr) in config.overrides.iter().enumerate() {
        if ovr.limit.is_none() && ovr.max_bytes.is_none() && !ovr.exclude {
            bail!(
                "override #{} (pattern {:?}) must set `limit`, `max_bytes` or `exclude: true`",
                idx + 1,
                ovr.pattern
            );
        }
        compile_pattern(&ovr.pattern).with_context(|| {
            format!(
                "invalid pattern in override #{}: {:?}",
                idx + 1,
                ovr.pattern
            )
        })?;
    }
    Ok(())
}

/// Rekeys `limits` by tokei's display name, the name a counted file carries,
/// so a key holds its language under any spelling tokei parses: `Bash`,
/// `bash` and `BASH` all cap the files tokei calls `BASH`.
///
/// # Errors
///
/// Returns an error for a key tokei does not know, or for two keys that
/// name one language.
pub fn canonical_limits(limits: &BTreeMap<String, u64>) -> Result<BTreeMap<String, u64>> {
    let mut canonical = BTreeMap::new();
    for (key, limit) in limits {
        let Ok(language) = key.parse::<LanguageType>() else {
            bail!("unknown language in limits: {key:?}");
        };
        if canonical
            .insert(language.name().to_owned(), *limit)
            .is_some()
        {
            bail!("limits name {} twice, once as {key:?}", language.name());
        }
    }
    Ok(canonical)
}

/// Compiles an override pattern as one `.gitignore` line, matched against
/// paths relative to the config's `root`. A leading `./` anchors it
/// there, as a leading `/` does.
///
/// # Errors
///
/// Returns an error for a pattern gitignore rejects, a `!` negation (overrides
/// apply first-match, so order them instead), or a line that is no pattern.
pub fn compile_pattern(pattern: &str) -> Result<Gitignore> {
    if pattern.starts_with('!') {
        bail!("`!` negation is not supported; the first matching override wins");
    }
    let line = pattern
        .strip_prefix("./")
        .map_or_else(|| pattern.to_owned(), |rest| format!("/{rest}"));
    let mut builder = GitignoreBuilder::new("");
    builder.add_line(None, &line)?;
    let matcher = builder.build()?;
    if matcher.num_ignores() == 0 {
        bail!("matches nothing: empty, or a `#` comment");
    }
    Ok(matcher)
}

#[cfg(test)]
mod tests {
    use super::{Config, CountMode, Override, load, validate};
    use std::io::Write;
    use std::path::PathBuf;

    #[test]
    fn parse_valid_config() {
        let yaml = r#"
count_mode: code
limits:
  Rust: 500
  Markdown: 200
overrides:
  - pattern: "RESEARCH.md"
    exclude: true
  - pattern: "src/generated_*.rs"
    limit: 1000
"#;
        let config: Config = serde_yml::from_str(yaml).expect("parse");
        assert_eq!(config.count_mode, CountMode::Code);
        assert_eq!(config.limits.len(), 2);
        assert_eq!(config.overrides.len(), 2);
        validate(&config).expect("valid");
    }

    #[test]
    fn parse_minimal_config() {
        let yaml = "limits:\n  Rust: 500\n";
        let config: Config = serde_yml::from_str(yaml).expect("parse");
        assert_eq!(config.count_mode, CountMode::Total);
        assert!(config.overrides.is_empty());
        validate(&config).expect("valid");
    }

    #[test]
    fn unknown_language_rejected() {
        let yaml = "limits:\n  FakeLang: 100\n";
        let config: Config = serde_yml::from_str(yaml).expect("parse");
        let err = validate(&config).expect_err("should reject unknown language");
        assert!(err.to_string().contains("unknown language"));
    }

    #[test]
    fn override_without_limit_or_exclude_rejected() {
        let yaml = r#"
limits:
  Rust: 500
overrides:
  - pattern: "*.rs"
"#;
        let config: Config = serde_yml::from_str(yaml).expect("parse");
        let err = validate(&config).expect_err("should reject override without limit or exclude");
        assert!(
            err.to_string()
                .contains("must set `limit`, `max_bytes` or `exclude: true`")
        );
    }

    #[test]
    fn override_with_only_max_bytes_accepted() {
        let yaml = r#"
limits:
  Markdown: 200
overrides:
  - pattern: "./CONTRIBUTING.md"
    max_bytes: 14000
"#;
        let config: Config = serde_yml::from_str(yaml).expect("parse");
        let ovr = config.overrides.first().expect("override");
        assert_eq!(ovr.max_bytes, Some(14000));
        validate(&config).expect("valid");
    }

    #[test]
    fn invalid_glob_pattern_rejected() {
        let yaml = r#"
limits:
  Rust: 500
overrides:
  - pattern: "[z-a]"
    exclude: true
"#;
        let config: Config = serde_yml::from_str(yaml).expect("parse");
        let err = validate(&config).expect_err("should reject invalid glob");
        assert!(err.to_string().contains("invalid pattern"));
    }

    #[test]
    fn deny_unknown_fields() {
        let yaml = "limits:\n  Rust: 500\nunknown_field: true\n";
        let result = serde_yml::from_str::<Config>(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn load_from_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join(".linecop.yaml");
        let mut file = std::fs::File::create(&path).expect("create");
        write!(file, "limits:\n  Rust: 500\n").expect("write");
        let config = load(&path).expect("load");
        assert_eq!(config.limits.get("Rust").copied(), Some(500));
    }

    #[test]
    fn load_missing_file() {
        let result = load(std::path::Path::new("/nonexistent/.linecop.yaml"));
        assert!(result.is_err());
    }

    /// The `base_dir` of a `.config/linecop.yaml` holding `head`, and the
    /// canonical repo directory above it.
    fn base_dir_under_dot_config(head: &str) -> (Option<PathBuf>, PathBuf) {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join(".config/linecop.yaml");
        std::fs::create_dir(dir.path().join(".config")).expect("mkdir");
        std::fs::write(&path, format!("{head}limits:\n  Rust: 500\n")).expect("write");
        let config = load(&path).expect("load");
        (config.base_dir, dir.path().canonicalize().expect("canon"))
    }

    #[test]
    fn load_without_root_anchors_at_the_config_directory() {
        let (base, repo) = base_dir_under_dot_config("");
        assert_eq!(base, Some(repo.join(".config")));
    }

    #[test]
    fn load_resolves_root_from_the_config_directory() {
        let (base, repo) = base_dir_under_dot_config("root: ..\n");
        assert_eq!(base, Some(repo));
    }

    #[test]
    fn fallback_config_has_default_limit() {
        let cfg = Config::fallback();
        assert!(cfg.limits.is_empty());
        assert_eq!(cfg.default_limit, Some(super::DEFAULT_LIMIT));
    }

    #[test]
    fn count_mode_default_is_total() {
        let yaml = "limits:\n  Rust: 500\n";
        let config: Config = serde_yml::from_str(yaml).expect("parse");
        assert_eq!(config.count_mode, CountMode::Total);
    }

    #[test]
    fn count_mode_code_comments() {
        let yaml = "count_mode: code-comments\nlimits:\n  Rust: 500\n";
        let config: Config = serde_yml::from_str(yaml).expect("parse");
        assert_eq!(config.count_mode, CountMode::CodeComments);
    }

    #[test]
    fn exclude_dirs_defaults_to_target() {
        let yaml = "limits:\n  Rust: 500\n";
        let config: Config = serde_yml::from_str(yaml).expect("parse");
        assert_eq!(config.exclude_dirs, vec!["target"]);
    }

    #[test]
    fn exclude_dirs_custom() {
        let yaml = "limits:\n  Rust: 500\nexclude_dirs:\n  - vendor\n  - dist\n";
        let config: Config = serde_yml::from_str(yaml).expect("parse");
        assert_eq!(config.exclude_dirs, vec!["vendor", "dist"]);
    }

    #[test]
    fn include_hidden_defaults_to_false() {
        let yaml = "limits:\n  Rust: 500\n";
        let config: Config = serde_yml::from_str(yaml).expect("parse");
        assert!(!config.include_hidden);
    }

    #[test]
    fn include_hidden_parsed() {
        let yaml = "limits:\n  Rust: 500\ninclude_hidden: true\n";
        let config: Config = serde_yml::from_str(yaml).expect("parse");
        assert!(config.include_hidden);
        validate(&config).expect("valid");
    }

    #[test]
    fn fallback_config_excludes_hidden() {
        assert!(!Config::fallback().include_hidden);
    }

    #[test]
    fn override_with_exclude_true() {
        let ovr = Override {
            pattern: "*.md".into(),
            limit: None,
            max_bytes: None,
            exclude: true,
        };
        assert!(ovr.exclude);
        assert!(ovr.limit.is_none());
    }
}
