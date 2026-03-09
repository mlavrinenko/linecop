pub mod checker;
pub mod config;
pub mod counter;
pub mod init;
pub mod report;
pub mod schema;

use std::path::Path;

use anyhow::{Result, bail};

use crate::report::Format;

/// Runs the full linecop check pipeline.
///
/// Returns `true` if there are violations (i.e. the process should exit non-zero).
///
/// # Errors
///
/// Returns an error if the root path does not exist, the config cannot be loaded,
/// counting fails, or output fails.
pub fn run(root: &Path, config_path: &Path, quiet: bool, format: Format) -> Result<bool> {
    if !root.exists() {
        bail!("scan path does not exist: {}", root.display());
    }
    let cfg = config::load(config_path)?;
    let files = counter::count(root, &cfg)?;
    let violations = checker::check(&files, &cfg);

    if !quiet {
        let mut stdout = anstream::stdout().lock();
        report::print(&mut stdout, &violations, format)?;
    }

    Ok(!violations.is_empty())
}

#[cfg(test)]
mod tests {
    use super::run;
    use crate::report::Format;
    use std::io::Write;

    #[test]
    fn run_with_valid_config_no_violations() {
        let dir = tempfile::tempdir().expect("tempdir");

        // Create a small Rust file
        let rs_path = dir.path().join("hello.rs");
        let mut file = std::fs::File::create(&rs_path).expect("create");
        writeln!(file, "fn main() {{}}").expect("write");

        // Create config
        let cfg_path = dir.path().join(".linecop.yaml");
        let mut cfg_file = std::fs::File::create(&cfg_path).expect("create");
        write!(cfg_file, "limits:\n  Rust: 500\n").expect("write");

        let has_violations = run(dir.path(), &cfg_path, true, Format::Text).expect("run");
        assert!(!has_violations);
    }

    #[test]
    fn run_with_violations() {
        let dir = tempfile::tempdir().expect("tempdir");

        // Create a Rust file with 5 lines
        let rs_path = dir.path().join("big.rs");
        let mut file = std::fs::File::create(&rs_path).expect("create");
        for ii in 0..5 {
            writeln!(file, "fn f{ii}() {{}}").expect("write");
        }

        // Config with limit of 3
        let cfg_path = dir.path().join(".linecop.yaml");
        let mut cfg_file = std::fs::File::create(&cfg_path).expect("create");
        write!(cfg_file, "limits:\n  Rust: 3\n").expect("write");

        let has_violations = run(dir.path(), &cfg_path, true, Format::Text).expect("run");
        assert!(has_violations);
    }

    #[test]
    fn run_missing_config() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cfg_path = dir.path().join(".linecop.yaml");
        let result = run(dir.path(), &cfg_path, true, Format::Text);
        assert!(result.is_err());
    }

    #[test]
    fn run_nonexistent_root_path() {
        let cfg_path = std::path::Path::new(".linecop.yaml");
        let result = run(std::path::Path::new("/nonexistent/path"), cfg_path, true, Format::Text);
        let err = result.expect_err("should fail for nonexistent path");
        assert!(err.to_string().contains("scan path does not exist"));
    }
}
