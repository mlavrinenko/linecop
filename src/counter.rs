use std::path::{Path, PathBuf};
use std::str::FromStr as _;

use anyhow::{Result, anyhow};
use tokei::{CodeStats, LanguageType, Languages};

use crate::config::Config;

/// Line-count statistics for a single file.
#[derive(Debug, Clone)]
pub struct FileStats {
    /// Path to the file.
    pub path: PathBuf,
    /// Tokei language name (e.g. "Rust").
    pub language: String,
    /// Total lines (code + comments + blanks), including nested blobs.
    pub total: u64,
    /// Code lines only, including nested blobs.
    pub code: u64,
    /// Comment lines only, including nested blobs.
    pub comments: u64,
    /// Blank lines only, including nested blobs.
    pub blanks: u64,
}

/// Recursively sums code stats including nested language blobs.
fn summarise(stats: &CodeStats) -> (u64, u64, u64) {
    let mut code = stats.code as u64;
    let mut comments = stats.comments as u64;
    let mut blanks = stats.blanks as u64;
    for child in stats.blobs.values() {
        let (cc, cm, bl) = summarise(child);
        code += cc;
        comments += cm;
        blanks += bl;
    }
    (code, comments, blanks)
}

/// Counts lines for all files under `root` that match languages in the config.
/// When `limits` is empty (configless mode), all languages are scanned.
///
/// # Errors
///
/// Returns an error if a language name in the config is not recognized by tokei.
pub fn count(root: &Path, config: &Config) -> Result<Vec<FileStats>> {
    let types: Option<Vec<LanguageType>> = if config.limits.is_empty() {
        None
    } else {
        Some(
            config
                .limits
                .keys()
                .map(|name| {
                    LanguageType::from_str(name)
                        .map_err(|err| anyhow!("unknown language {name:?}: {err}"))
                })
                .collect::<Result<Vec<_>>>()?,
        )
    };

    let tokei_config = tokei::Config {
        types,
        ..tokei::Config::default()
    };

    let exclude_dirs: Vec<&str> = config.exclude_dirs.iter().map(String::as_str).collect();

    let mut languages = Languages::new();
    languages.get_statistics(&[root], &exclude_dirs, &tokei_config);

    let mut results = Vec::new();
    for (lang_type, language) in &languages {
        for report in &language.reports {
            let (code, comments, blanks) = summarise(&report.stats);
            results.push(FileStats {
                path: report.name.clone(),
                language: lang_type.name().to_owned(),
                total: code + comments + blanks,
                code,
                comments,
                blanks,
            });
        }
    }

    results.sort_by(|aa, bb| aa.path.cmp(&bb.path));
    Ok(results)
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)]
mod tests {
    use super::count;
    use crate::config::CountMode;
    use crate::test_helpers::make_config;
    use std::io::Write;

    #[test]
    fn count_rust_files() {
        let dir = tempfile::tempdir().expect("tempdir");
        let rs_path = dir.path().join("hello.rs");
        let mut file = std::fs::File::create(&rs_path).expect("create");
        writeln!(file, "fn main() {{").expect("write");
        writeln!(file, "    println!(\"hello\");").expect("write");
        writeln!(file, "}}").expect("write");

        let config = make_config(&[("Rust", 500)], vec![], CountMode::Total);
        let stats = count(dir.path(), &config).expect("count");

        assert_eq!(stats.len(), 1);
        assert_eq!(stats[0].language, "Rust");
        assert_eq!(stats[0].total, 3);
        assert_eq!(stats[0].code, 3);
    }

    #[test]
    fn count_empty_directory() {
        let dir = tempfile::tempdir().expect("tempdir");
        let config = make_config(&[("Rust", 500)], vec![], CountMode::Total);
        let stats = count(dir.path(), &config).expect("count");
        assert!(stats.is_empty());
    }

    #[test]
    fn count_with_comments_and_blanks() {
        let dir = tempfile::tempdir().expect("tempdir");
        let rs_path = dir.path().join("test.rs");
        let mut file = std::fs::File::create(&rs_path).expect("create");
        writeln!(file, "// a comment").expect("write");
        writeln!(file).expect("write");
        writeln!(file, "fn main() {{}}").expect("write");

        let config = make_config(&[("Rust", 500)], vec![], CountMode::Total);
        let stats = count(dir.path(), &config).expect("count");

        assert_eq!(stats.len(), 1);
        assert_eq!(stats[0].comments, 1);
        assert_eq!(stats[0].blanks, 1);
        assert_eq!(stats[0].code, 1);
        assert_eq!(stats[0].total, 3);
    }

    #[test]
    fn ignores_unlisted_languages() {
        let dir = tempfile::tempdir().expect("tempdir");
        // Create a Python file but only list Rust in config
        let py_path = dir.path().join("hello.py");
        let mut file = std::fs::File::create(&py_path).expect("create");
        writeln!(file, "print('hello')").expect("write");

        let config = make_config(&[("Rust", 500)], vec![], CountMode::Total);
        let stats = count(dir.path(), &config).expect("count");
        assert!(stats.is_empty());
    }

    #[test]
    fn custom_exclude_dirs() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join("vendor")).expect("mkdir");
        let rs_path = dir.path().join("vendor/lib.rs");
        let mut file = std::fs::File::create(&rs_path).expect("create");
        writeln!(file, "fn lib() {{}}").expect("write");

        let mut config = make_config(&[("Rust", 500)], vec![], CountMode::Total);
        // Default excludes "target" only, so vendor/lib.rs should be counted
        let stats = count(dir.path(), &config).expect("count");
        assert_eq!(stats.len(), 1);

        // Add "vendor" to exclude_dirs
        config.exclude_dirs.push("vendor".to_owned());
        let stats = count(dir.path(), &config).expect("count");
        assert!(stats.is_empty());
    }
}
