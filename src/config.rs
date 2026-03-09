use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
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
    /// Glob pattern to match file paths.
    pub pattern: String,
    /// Custom line limit for matched files.
    #[serde(default)]
    pub limit: Option<u64>,
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
    /// Fallback limit for languages not listed in `limits`.
    /// Used when running without a config file.
    #[serde(default)]
    #[schemars(skip)]
    pub default_limit: Option<u64>,
}

impl Config {
    /// Creates a sensible default config (no per-language limits, 500-line
    /// default for every file).
    #[must_use]
    pub fn fallback() -> Self {
        Self {
            count_mode: CountMode::default(),
            limits: BTreeMap::new(),
            overrides: Vec::new(),
            exclude_dirs: default_exclude_dirs(),
            default_limit: Some(DEFAULT_LIMIT),
        }
    }
}

/// Searches for `.linecop.yaml` starting from `start` and traversing up,
/// stopping at `stop` (inclusive). Returns the first path found, or `None`.
#[must_use]
pub fn find_config(start: &Path, stop: &Path) -> Option<PathBuf> {
    let mut dir = start.to_path_buf();
    loop {
        let candidate = dir.join(".linecop.yaml");
        if candidate.is_file() {
            return Some(candidate);
        }
        if dir == stop {
            break;
        }
        if !dir.pop() {
            break;
        }
    }
    None
}

/// Loads and validates a config from the given YAML file.
///
/// # Errors
///
/// Returns an error if the file cannot be read, parsed, or contains invalid
/// language names or override rules.
pub fn load(path: &Path) -> Result<Config> {
    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read config file: {}", path.display()))?;
    let config: Config = serde_yml::from_str(&contents)
        .with_context(|| format!("failed to parse config file: {}", path.display()))?;
    validate(&config)?;
    Ok(config)
}

/// Validates that all language names are recognized by tokei and that
/// overrides have either a limit or exclude set.
///
/// # Errors
///
/// Returns an error if a language name is unknown or an override has
/// neither `limit` nor `exclude`.
pub fn validate(config: &Config) -> Result<()> {
    for lang_name in config.limits.keys() {
        if lang_name.parse::<LanguageType>().is_err() {
            bail!("unknown language in limits: {lang_name:?}");
        }
    }
    for (idx, ovr) in config.overrides.iter().enumerate() {
        if ovr.limit.is_none() && !ovr.exclude {
            bail!(
                "override #{} (pattern {:?}) must have either `limit` or `exclude: true`",
                idx + 1,
                ovr.pattern
            );
        }
        // Validate that the glob pattern compiles
        globset::Glob::new(&ovr.pattern).with_context(|| {
            format!(
                "invalid glob pattern in override #{}: {:?}",
                idx + 1,
                ovr.pattern
            )
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{Config, CountMode, Override, load, validate};
    use std::io::Write;

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
        assert!(err.to_string().contains("must have either"));
    }

    #[test]
    fn invalid_glob_pattern_rejected() {
        let yaml = r#"
limits:
  Rust: 500
overrides:
  - pattern: "[invalid"
    exclude: true
"#;
        let config: Config = serde_yml::from_str(yaml).expect("parse");
        let err = validate(&config).expect_err("should reject invalid glob");
        assert!(err.to_string().contains("invalid glob pattern"));
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

    #[test]
    fn find_config_in_same_dir() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cfg = dir.path().join(".linecop.yaml");
        std::fs::write(&cfg, "limits:\n  Rust: 500\n").expect("write");
        let found = super::find_config(dir.path(), dir.path());
        assert_eq!(found, Some(cfg));
    }

    #[test]
    fn find_config_traverses_up() {
        let dir = tempfile::tempdir().expect("tempdir");
        let sub = dir.path().join("a").join("b");
        std::fs::create_dir_all(&sub).expect("mkdir");
        let cfg = dir.path().join(".linecop.yaml");
        std::fs::write(&cfg, "limits:\n  Rust: 500\n").expect("write");
        let found = super::find_config(&sub, dir.path());
        assert_eq!(found, Some(cfg));
    }

    #[test]
    fn find_config_stops_at_boundary() {
        let dir = tempfile::tempdir().expect("tempdir");
        let parent = dir.path().join("parent");
        let child = parent.join("child");
        std::fs::create_dir_all(&child).expect("mkdir");
        // Place config above the stop boundary
        let cfg = dir.path().join(".linecop.yaml");
        std::fs::write(&cfg, "limits:\n  Rust: 500\n").expect("write");
        // Search from child, stop at parent — should NOT find config in dir
        let found = super::find_config(&child, &parent);
        assert!(found.is_none());
    }

    #[test]
    fn find_config_returns_none_when_missing() {
        let dir = tempfile::tempdir().expect("tempdir");
        let found = super::find_config(dir.path(), dir.path());
        assert!(found.is_none());
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
    fn override_with_exclude_true() {
        let ovr = Override {
            pattern: "*.md".into(),
            limit: None,
            exclude: true,
        };
        assert!(ovr.exclude);
        assert!(ovr.limit.is_none());
    }
}
