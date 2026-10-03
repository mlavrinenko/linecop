use std::path::{Path, PathBuf};

use ignore::gitignore::Gitignore;

use crate::config::{Config, CountMode, compile_pattern};
use crate::counter::FileStats;

/// A measured count held against its cap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gauge {
    /// The measured count (lines per the count mode, or bytes).
    pub count: u64,
    /// The configured cap.
    pub limit: u64,
    /// The effective threshold after applying the baseline percentage.
    pub baseline_limit: u64,
    /// Whether the count reaches the reporting threshold.
    pub breached: bool,
}

impl Gauge {
    /// At a baseline of 100 the count breaches only when it strictly exceeds
    /// the cap; below 100 it breaches at or above the threshold.
    fn new(count: u64, limit: u64, baseline: u8) -> Self {
        let baseline_limit = limit * u64::from(baseline) / 100;
        let breached = if baseline == 100 {
            count > limit
        } else {
            count >= baseline_limit
        };
        Self {
            count,
            limit,
            baseline_limit,
            breached,
        }
    }
}

/// A file that breaches its line limit, its byte cap, or both.
#[derive(Debug, Clone)]
pub struct Violation {
    /// Path to the offending file.
    pub path: PathBuf,
    /// Language of the file.
    pub language: String,
    /// Line count against the line limit; `None` when no line limit applies.
    pub lines: Option<Gauge>,
    /// Byte count against `max_bytes`; `None` when no override sets one.
    pub bytes: Option<Gauge>,
}

/// A compiled override rule ready for matching.
struct CompiledOverride {
    matcher: Gitignore,
    limit: Option<u64>,
    max_bytes: Option<u64>,
    exclude: bool,
}

impl CompiledOverride {
    fn compile_all(config: &Config) -> Vec<Self> {
        config
            .overrides
            .iter()
            .filter_map(|ovr| {
                compile_pattern(&ovr.pattern).ok().map(|matcher| Self {
                    matcher,
                    limit: ovr.limit,
                    max_bytes: ovr.max_bytes,
                    exclude: ovr.exclude,
                })
            })
            .collect()
    }

    fn matches(&self, path: &Path) -> bool {
        self.matcher
            .matched_path_or_any_parents(path, false)
            .is_ignore()
    }
}

/// The caps a file is held to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Caps {
    lines: Option<u64>,
    bytes: Option<u64>,
}

/// Selects the line count based on the configured count mode.
fn select_count(file: &FileStats, mode: CountMode) -> u64 {
    match mode {
        CountMode::Total => file.total,
        CountMode::Code => file.code,
        CountMode::CodeComments => file.code + file.comments,
    }
}

/// The path override patterns see: relative to the config file's directory.
/// `None` for a file outside it, which no override reaches, as a
/// `.gitignore` never reaches outside its own directory. Without a config
/// file, the path as scanned.
fn match_path(path: &Path, base: Option<&Path>) -> Option<PathBuf> {
    let Some(base) = base else {
        return path.is_relative().then(|| path.to_path_buf());
    };
    let canonical = path.canonicalize().ok()?;
    canonical.strip_prefix(base).ok().map(Path::to_path_buf)
}

/// Finds the caps for a file, or `None` when it is excluded. The first
/// matching override wins; a `limit` it leaves unset falls back to the
/// language limit.
fn effective_caps(
    file: &FileStats,
    config: &Config,
    compiled: &[CompiledOverride],
) -> Option<Caps> {
    let language_limit = || {
        config
            .limits
            .get(&file.language)
            .copied()
            .or(config.default_limit)
    };
    let path = match_path(&file.path, config.base_dir.as_deref());
    let matched = path.and_then(|path| compiled.iter().find(|ovr| ovr.matches(&path)));
    match matched {
        Some(ovr) if ovr.exclude => None,
        Some(ovr) => Some(Caps {
            lines: ovr.limit.or_else(language_limit),
            bytes: ovr.max_bytes,
        }),
        None => Some(Caps {
            lines: language_limit(),
            bytes: None,
        }),
    }
}

/// Checks all files against their caps and returns violations.
///
/// `baseline` is a percentage (1-100) applied to line limits and byte caps
/// alike. At 100, a file is reported only when it strictly exceeds a cap
/// (backward-compatible default). Below 100, files at or above `baseline`
/// percent of a cap are reported.
///
/// Glob patterns from overrides are compiled once upfront for efficiency.
/// Invalid patterns are skipped (they are validated at config load time).
pub fn check(files: &[FileStats], config: &Config, baseline: u8) -> Vec<Violation> {
    let compiled = CompiledOverride::compile_all(config);

    let mut violations = Vec::new();
    for file in files {
        let Some(caps) = effective_caps(file, config, &compiled) else {
            continue;
        };
        let lines = caps
            .lines
            .map(|limit| Gauge::new(select_count(file, config.count_mode), limit, baseline));
        let bytes = caps
            .bytes
            .map(|limit| Gauge::new(file.bytes, limit, baseline));
        if lines.iter().chain(&bytes).any(|gauge| gauge.breached) {
            violations.push(Violation {
                path: file.path.clone(),
                language: file.language.clone(),
                lines,
                bytes,
            });
        }
    }
    violations
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)]
mod tests {
    use super::{Caps, CompiledOverride, Gauge, Violation, check, effective_caps, select_count};
    use crate::config::{Config, CountMode, Override};
    use crate::test_helpers::{make_config, make_file};

    fn caps(file: &crate::counter::FileStats, config: &Config) -> Option<Caps> {
        effective_caps(file, config, &CompiledOverride::compile_all(config))
    }

    fn lines(violation: &Violation) -> Gauge {
        violation.lines.expect("line gauge")
    }

    #[test]
    fn no_violations_when_within_limits() {
        let files = vec![make_file("src/main.rs", "Rust", 100, 10, 5)];
        let config = make_config(&[("Rust", 500)], vec![], CountMode::Total);
        let violations = check(&files, &config, 100);
        assert!(violations.is_empty());
    }

    #[test]
    fn violation_when_exceeding_limit() {
        let files = vec![make_file("src/big.rs", "Rust", 400, 60, 50)];
        let config = make_config(&[("Rust", 500)], vec![], CountMode::Total);
        let violations = check(&files, &config, 100);
        assert_eq!(violations.len(), 1);
        assert_eq!(lines(&violations[0]).count, 510);
        assert_eq!(lines(&violations[0]).limit, 500);
    }

    #[test]
    fn exclude_override_skips_file() {
        let files = vec![make_file("RESEARCH.md", "Markdown", 300, 0, 10)];
        let overrides = vec![Override {
            pattern: "RESEARCH.md".into(),
            limit: None,
            max_bytes: None,
            exclude: true,
        }];
        let config = make_config(&[("Markdown", 200)], overrides, CountMode::Total);
        let violations = check(&files, &config, 100);
        assert!(violations.is_empty());
    }

    #[test]
    fn limit_override_replaces_language_limit() {
        let files = vec![make_file("src/generated.rs", "Rust", 900, 10, 10)];
        let overrides = vec![Override {
            pattern: "src/generated.rs".into(),
            limit: Some(1000),
            max_bytes: None,
            exclude: false,
        }];
        let config = make_config(&[("Rust", 500)], overrides, CountMode::Total);
        let violations = check(&files, &config, 100);
        assert!(violations.is_empty());
    }

    #[test]
    fn code_only_mode() {
        let files = vec![make_file("src/main.rs", "Rust", 400, 60, 50)];
        let config = make_config(&[("Rust", 500)], vec![], CountMode::Code);
        let violations = check(&files, &config, 100);
        assert!(violations.is_empty());
    }

    #[test]
    fn code_comments_mode() {
        let files = vec![make_file("src/main.rs", "Rust", 400, 60, 50)];
        let config = make_config(&[("Rust", 500)], vec![], CountMode::CodeComments);
        let violations = check(&files, &config, 100);
        assert!(violations.is_empty());
    }

    #[test]
    fn code_comments_mode_violation() {
        let files = vec![make_file("src/main.rs", "Rust", 450, 60, 50)];
        let config = make_config(&[("Rust", 500)], vec![], CountMode::CodeComments);
        let violations = check(&files, &config, 100);
        assert_eq!(violations.len(), 1);
        assert_eq!(lines(&violations[0]).count, 510);
    }

    #[test]
    fn file_without_matching_language_skipped() {
        let files = vec![make_file("script.py", "Python", 1000, 0, 0)];
        let config = make_config(&[("Rust", 500)], vec![], CountMode::Total);
        let violations = check(&files, &config, 100);
        assert!(violations.is_empty());
    }

    #[test]
    fn override_order_matters() {
        let files = vec![make_file("src/gen.rs", "Rust", 800, 0, 0)];
        let overrides = vec![
            Override {
                pattern: "src/gen.rs".into(),
                limit: Some(1000),
                max_bytes: None,
                exclude: false,
            },
            Override {
                pattern: "src/*.rs".into(),
                limit: None,
                max_bytes: None,
                exclude: true,
            },
        ];
        let config = make_config(&[("Rust", 500)], overrides, CountMode::Total);
        let violations = check(&files, &config, 100);
        assert!(violations.is_empty());
    }

    #[test]
    fn select_count_total() {
        let file = make_file("a.rs", "Rust", 10, 5, 3);
        assert_eq!(select_count(&file, CountMode::Total), 18);
    }

    #[test]
    fn select_count_code() {
        let file = make_file("a.rs", "Rust", 10, 5, 3);
        assert_eq!(select_count(&file, CountMode::Code), 10);
    }

    #[test]
    fn select_count_code_comments() {
        let file = make_file("a.rs", "Rust", 10, 5, 3);
        assert_eq!(select_count(&file, CountMode::CodeComments), 15);
    }

    #[test]
    fn effective_caps_with_no_overrides() {
        let file = make_file("src/main.rs", "Rust", 10, 0, 0);
        let config = make_config(&[("Rust", 500)], vec![], CountMode::Total);
        assert_eq!(
            caps(&file, &config),
            Some(Caps {
                lines: Some(500),
                bytes: None
            })
        );
    }

    #[test]
    fn effective_caps_exclude_returns_none() {
        let file = make_file("RESEARCH.md", "Markdown", 10, 0, 0);
        let overrides = vec![Override {
            pattern: "RESEARCH.md".into(),
            limit: None,
            max_bytes: None,
            exclude: true,
        }];
        let config = make_config(&[("Markdown", 200)], overrides, CountMode::Total);
        assert_eq!(caps(&file, &config), None);
    }

    #[test]
    fn default_limit_applies_to_unlisted_languages() {
        let files = vec![make_file("script.py", "Python", 600, 0, 0)];
        let mut config = make_config(&[], vec![], CountMode::Total);
        config.default_limit = Some(500);
        let violations = check(&files, &config, 100);
        assert_eq!(violations.len(), 1);
        assert_eq!(lines(&violations[0]).limit, 500);
    }

    #[test]
    fn default_limit_overridden_by_language_limit() {
        let files = vec![make_file("main.rs", "Rust", 350, 0, 0)];
        let mut config = make_config(&[("Rust", 300)], vec![], CountMode::Total);
        config.default_limit = Some(500);
        let violations = check(&files, &config, 100);
        assert_eq!(violations.len(), 1);
        assert_eq!(lines(&violations[0]).limit, 300);
    }

    #[test]
    fn no_default_limit_skips_unlisted_languages() {
        let files = vec![make_file("script.py", "Python", 1000, 0, 0)];
        let config = make_config(&[("Rust", 500)], vec![], CountMode::Total);
        let violations = check(&files, &config, 100);
        assert!(violations.is_empty());
    }

    #[test]
    fn glob_pattern_matching() {
        let files = vec![make_file("docs/RESEARCH.md", "Markdown", 300, 0, 0)];
        let overrides = vec![Override {
            pattern: "docs/*.md".into(),
            limit: None,
            max_bytes: None,
            exclude: true,
        }];
        let config = make_config(&[("Markdown", 200)], overrides, CountMode::Total);
        let violations = check(&files, &config, 100);
        assert!(violations.is_empty());
    }

    #[test]
    fn baseline_reports_near_limit_files() {
        let files = vec![make_file("src/near.rs", "Rust", 450, 0, 0)];
        let config = make_config(&[("Rust", 500)], vec![], CountMode::Total);
        let violations = check(&files, &config, 90);
        assert_eq!(violations.len(), 1);
        assert_eq!(lines(&violations[0]).count, 450);
        assert_eq!(lines(&violations[0]).limit, 500);
        assert_eq!(lines(&violations[0]).baseline_limit, 450);
    }

    #[test]
    fn baseline_below_threshold_no_violation() {
        let files = vec![make_file("src/ok.rs", "Rust", 440, 0, 0)];
        let config = make_config(&[("Rust", 500)], vec![], CountMode::Total);
        let violations = check(&files, &config, 90);
        assert!(violations.is_empty());
    }

    #[test]
    fn baseline_100_backward_compatible() {
        let files = vec![make_file("src/exact.rs", "Rust", 500, 0, 0)];
        let config = make_config(&[("Rust", 500)], vec![], CountMode::Total);
        let violations = check(&files, &config, 100);
        assert!(violations.is_empty(), "exactly at limit should not violate");
    }

    #[test]
    fn baseline_50_percent() {
        let files = vec![make_file("src/half.rs", "Rust", 250, 0, 0)];
        let config = make_config(&[("Rust", 500)], vec![], CountMode::Total);
        let violations = check(&files, &config, 50);
        assert_eq!(violations.len(), 1);
        assert_eq!(lines(&violations[0]).baseline_limit, 250);
    }

    fn byte_capped(pattern: &str, limit: Option<u64>, max_bytes: u64) -> Vec<Override> {
        vec![Override {
            pattern: pattern.into(),
            limit,
            max_bytes: Some(max_bytes),
            exclude: false,
        }]
    }

    #[test]
    fn max_bytes_breach_under_line_limit() {
        let mut file = make_file("CONTRIBUTING.md", "Markdown", 128, 0, 0);
        file.bytes = 30853;
        let overrides = byte_capped("CONTRIBUTING.md", Some(142), 14000);
        let config = make_config(&[("Markdown", 200)], overrides, CountMode::Total);
        let violations = check(&[file], &config, 100);
        assert_eq!(violations.len(), 1);
        assert!(!lines(&violations[0]).breached);
        let bytes = violations[0].bytes.expect("byte gauge");
        assert!(bytes.breached);
        assert_eq!((bytes.count, bytes.limit), (30853, 14000));
    }

    #[test]
    fn max_bytes_within_cap_no_violation() {
        let mut file = make_file("CONTRIBUTING.md", "Markdown", 128, 0, 0);
        file.bytes = 14000;
        let overrides = byte_capped("CONTRIBUTING.md", None, 14000);
        let config = make_config(&[("Markdown", 200)], overrides, CountMode::Total);
        assert!(check(&[file], &config, 100).is_empty());
    }

    #[test]
    fn max_bytes_only_override_keeps_language_limit() {
        let file = make_file("CONTRIBUTING.md", "Markdown", 10, 0, 0);
        let overrides = byte_capped("CONTRIBUTING.md", None, 14000);
        let config = make_config(&[("Markdown", 200)], overrides, CountMode::Total);
        assert_eq!(
            caps(&file, &config),
            Some(Caps {
                lines: Some(200),
                bytes: Some(14000)
            })
        );
    }

    #[test]
    fn baseline_applies_to_max_bytes() {
        let mut file = make_file("CONTRIBUTING.md", "Markdown", 10, 0, 0);
        file.bytes = 900;
        let overrides = byte_capped("CONTRIBUTING.md", None, 1000);
        let config = make_config(&[("Markdown", 200)], overrides, CountMode::Total);
        let violations = check(&[file], &config, 90);
        assert_eq!(violations.len(), 1);
        let bytes = violations[0].bytes.expect("byte gauge");
        assert_eq!(bytes.baseline_limit, 900);
        assert!(bytes.breached);
    }

    #[test]
    fn no_override_reaches_outside_the_config_directory() {
        let file = make_file("src/gen.rs", "Rust", 10, 0, 0);
        let overrides = vec![Override {
            pattern: "src/gen.rs".into(),
            limit: None,
            max_bytes: None,
            exclude: true,
        }];
        let mut config = make_config(&[("Rust", 5)], overrides, CountMode::Total);
        config.base_dir = Some("/nonexistent/repo".into());
        let lines = caps(&file, &config).and_then(|caps| caps.lines);
        assert_eq!(lines, Some(5));
    }
}
