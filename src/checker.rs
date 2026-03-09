use std::path::PathBuf;

use globset::{Glob, GlobMatcher};

use crate::config::{Config, CountMode};
use crate::counter::FileStats;

/// A file that exceeds its line limit.
#[derive(Debug, Clone)]
pub struct Violation {
    /// Path to the offending file.
    pub path: PathBuf,
    /// Language of the file.
    pub language: String,
    /// Actual line count (based on count mode).
    pub lines: u64,
    /// The limit that was exceeded.
    pub limit: u64,
}

/// A compiled override rule ready for matching.
struct CompiledOverride {
    matcher: GlobMatcher,
    limit: Option<u64>,
    exclude: bool,
}

/// Selects the line count based on the configured count mode.
fn select_count(file: &FileStats, mode: CountMode) -> u64 {
    match mode {
        CountMode::Total => file.total,
        CountMode::Code => file.code,
        CountMode::CodeComments => file.code + file.comments,
    }
}

/// Finds the effective limit for a file, checking overrides first.
fn effective_limit(
    file: &FileStats,
    config: &Config,
    compiled: &[CompiledOverride],
) -> Option<u64> {
    for ovr in compiled {
        if ovr.matcher.is_match(&file.path) {
            if ovr.exclude {
                return None;
            }
            if let Some(limit) = ovr.limit {
                return Some(limit);
            }
        }
    }
    config
        .limits
        .get(&file.language)
        .copied()
        .or(config.default_limit)
}

/// Checks all files against their limits and returns violations.
///
/// Glob patterns from overrides are compiled once upfront for efficiency.
/// Invalid patterns are skipped (they are validated at config load time).
pub fn check(files: &[FileStats], config: &Config) -> Vec<Violation> {
    let compiled: Vec<CompiledOverride> = config
        .overrides
        .iter()
        .filter_map(|ovr| {
            Glob::new(&ovr.pattern).ok().map(|glob| CompiledOverride {
                matcher: glob.compile_matcher(),
                limit: ovr.limit,
                exclude: ovr.exclude,
            })
        })
        .collect();

    let mut violations = Vec::new();
    for file in files {
        if let Some(limit) = effective_limit(file, config, &compiled) {
            let lines = select_count(file, config.count_mode);
            if lines > limit {
                violations.push(Violation {
                    path: file.path.clone(),
                    language: file.language.clone(),
                    lines,
                    limit,
                });
            }
        }
    }
    violations
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)]
mod tests {
    use super::{CompiledOverride, check, effective_limit, select_count};
    use crate::config::{Config, CountMode, Override};
    use crate::test_helpers::{make_config, make_file};
    use globset::Glob;

    fn compile_overrides(config: &Config) -> Vec<CompiledOverride> {
        config
            .overrides
            .iter()
            .filter_map(|ovr| {
                Glob::new(&ovr.pattern).ok().map(|glob| CompiledOverride {
                    matcher: glob.compile_matcher(),
                    limit: ovr.limit,
                    exclude: ovr.exclude,
                })
            })
            .collect()
    }

    #[test]
    fn no_violations_when_within_limits() {
        let files = vec![make_file("src/main.rs", "Rust", 100, 10, 5)];
        let config = make_config(&[("Rust", 500)], vec![], CountMode::Total);
        let violations = check(&files, &config);
        assert!(violations.is_empty());
    }

    #[test]
    fn violation_when_exceeding_limit() {
        let files = vec![make_file("src/big.rs", "Rust", 400, 60, 50)];
        let config = make_config(&[("Rust", 500)], vec![], CountMode::Total);
        let violations = check(&files, &config);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].lines, 510);
        assert_eq!(violations[0].limit, 500);
    }

    #[test]
    fn exclude_override_skips_file() {
        let files = vec![make_file("RESEARCH.md", "Markdown", 300, 0, 10)];
        let overrides = vec![Override {
            pattern: "RESEARCH.md".into(),
            limit: None,
            exclude: true,
        }];
        let config = make_config(&[("Markdown", 200)], overrides, CountMode::Total);
        let violations = check(&files, &config);
        assert!(violations.is_empty());
    }

    #[test]
    fn limit_override_replaces_language_limit() {
        let files = vec![make_file("src/generated.rs", "Rust", 900, 10, 10)];
        let overrides = vec![Override {
            pattern: "src/generated.rs".into(),
            limit: Some(1000),
            exclude: false,
        }];
        let config = make_config(&[("Rust", 500)], overrides, CountMode::Total);
        let violations = check(&files, &config);
        assert!(violations.is_empty());
    }

    #[test]
    fn code_only_mode() {
        let files = vec![make_file("src/main.rs", "Rust", 400, 60, 50)];
        let config = make_config(&[("Rust", 500)], vec![], CountMode::Code);
        let violations = check(&files, &config);
        assert!(violations.is_empty());
    }

    #[test]
    fn code_comments_mode() {
        let files = vec![make_file("src/main.rs", "Rust", 400, 60, 50)];
        let config = make_config(&[("Rust", 500)], vec![], CountMode::CodeComments);
        let violations = check(&files, &config);
        assert!(violations.is_empty());
    }

    #[test]
    fn code_comments_mode_violation() {
        let files = vec![make_file("src/main.rs", "Rust", 450, 60, 50)];
        let config = make_config(&[("Rust", 500)], vec![], CountMode::CodeComments);
        let violations = check(&files, &config);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].lines, 510);
    }

    #[test]
    fn file_without_matching_language_skipped() {
        let files = vec![make_file("script.py", "Python", 1000, 0, 0)];
        let config = make_config(&[("Rust", 500)], vec![], CountMode::Total);
        let violations = check(&files, &config);
        assert!(violations.is_empty());
    }

    #[test]
    fn override_order_matters() {
        let files = vec![make_file("src/gen.rs", "Rust", 800, 0, 0)];
        let overrides = vec![
            Override {
                pattern: "src/gen.rs".into(),
                limit: Some(1000),
                exclude: false,
            },
            Override {
                pattern: "src/*.rs".into(),
                limit: None,
                exclude: true,
            },
        ];
        let config = make_config(&[("Rust", 500)], overrides, CountMode::Total);
        let violations = check(&files, &config);
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
    fn effective_limit_with_no_overrides() {
        let file = make_file("src/main.rs", "Rust", 10, 0, 0);
        let config = make_config(&[("Rust", 500)], vec![], CountMode::Total);
        let compiled = compile_overrides(&config);
        assert_eq!(effective_limit(&file, &config, &compiled), Some(500));
    }

    #[test]
    fn effective_limit_exclude_returns_none() {
        let file = make_file("RESEARCH.md", "Markdown", 10, 0, 0);
        let overrides = vec![Override {
            pattern: "RESEARCH.md".into(),
            limit: None,
            exclude: true,
        }];
        let config = make_config(&[("Markdown", 200)], overrides, CountMode::Total);
        let compiled = compile_overrides(&config);
        assert_eq!(effective_limit(&file, &config, &compiled), None);
    }

    #[test]
    fn default_limit_applies_to_unlisted_languages() {
        let files = vec![make_file("script.py", "Python", 600, 0, 0)];
        let mut config = make_config(&[], vec![], CountMode::Total);
        config.default_limit = Some(500);
        let violations = check(&files, &config);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].limit, 500);
    }

    #[test]
    fn default_limit_overridden_by_language_limit() {
        let files = vec![make_file("main.rs", "Rust", 350, 0, 0)];
        let mut config = make_config(&[("Rust", 300)], vec![], CountMode::Total);
        config.default_limit = Some(500);
        let violations = check(&files, &config);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].limit, 300);
    }

    #[test]
    fn no_default_limit_skips_unlisted_languages() {
        let files = vec![make_file("script.py", "Python", 1000, 0, 0)];
        let config = make_config(&[("Rust", 500)], vec![], CountMode::Total);
        let violations = check(&files, &config);
        assert!(violations.is_empty());
    }

    #[test]
    fn glob_pattern_matching() {
        let files = vec![make_file("docs/RESEARCH.md", "Markdown", 300, 0, 0)];
        let overrides = vec![Override {
            pattern: "docs/*.md".into(),
            limit: None,
            exclude: true,
        }];
        let config = make_config(&[("Markdown", 200)], overrides, CountMode::Total);
        let violations = check(&files, &config);
        assert!(violations.is_empty());
    }
}
