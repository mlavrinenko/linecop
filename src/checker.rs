use std::path::PathBuf;

use globset::Glob;

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

/// Selects the line count based on the configured count mode.
fn select_count(file: &FileStats, mode: CountMode) -> u64 {
    match mode {
        CountMode::Total => file.total,
        CountMode::Code => file.code,
        CountMode::CodeComments => file.code + file.comments,
    }
}

/// Finds the effective limit for a file, checking overrides first.
fn effective_limit(file: &FileStats, config: &Config) -> Option<u64> {
    for ovr in &config.overrides {
        if let Ok(glob) = Glob::new(&ovr.pattern)
            && glob.compile_matcher().is_match(&file.path)
        {
            if ovr.exclude {
                return None;
            }
            if let Some(limit) = ovr.limit {
                return Some(limit);
            }
        }
    }
    config.limits.get(&file.language).copied()
}

/// Checks all files against their limits and returns violations.
pub fn check(files: &[FileStats], config: &Config) -> Vec<Violation> {
    let mut violations = Vec::new();
    for file in files {
        if let Some(limit) = effective_limit(file, config) {
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
    use super::{check, effective_limit, select_count};
    use crate::config::{Config, CountMode, Override};
    use crate::counter::FileStats;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    fn make_file(path: &str, lang: &str, code: u64, comments: u64, blanks: u64) -> FileStats {
        FileStats {
            path: PathBuf::from(path),
            language: lang.to_owned(),
            total: code + comments + blanks,
            code,
            comments,
            blanks,
        }
    }

    fn make_config(limits: &[(&str, u64)], overrides: Vec<Override>, mode: CountMode) -> Config {
        Config {
            count_mode: mode,
            limits: limits
                .iter()
                .map(|(kk, vv)| ((*kk).to_owned(), *vv))
                .collect::<BTreeMap<_, _>>(),
            overrides,
            exclude_dirs: vec!["target".to_owned()],
        }
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
        // 400 code + 60 comments + 50 blanks = 510 total, but code-only = 400
        let files = vec![make_file("src/main.rs", "Rust", 400, 60, 50)];
        let config = make_config(&[("Rust", 500)], vec![], CountMode::Code);
        let violations = check(&files, &config);
        assert!(violations.is_empty());
    }

    #[test]
    fn code_comments_mode() {
        // code + comments = 460, under 500
        let files = vec![make_file("src/main.rs", "Rust", 400, 60, 50)];
        let config = make_config(&[("Rust", 500)], vec![], CountMode::CodeComments);
        let violations = check(&files, &config);
        assert!(violations.is_empty());
    }

    #[test]
    fn code_comments_mode_violation() {
        // code + comments = 510, over 500
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
        // First override matches with limit 1000, file has 800 -> no violation
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
        assert_eq!(effective_limit(&file, &config), Some(500));
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
        assert_eq!(effective_limit(&file, &config), None);
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
