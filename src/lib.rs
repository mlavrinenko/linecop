pub mod checker;
pub mod config;
pub mod counter;
pub mod init;
pub mod report;
pub mod schema;

#[cfg(test)]
pub(crate) mod test_helpers;

use std::io::Write;
use std::path::Path;

use anyhow::{Result, bail};

use crate::report::Format;

/// Options controlling a linecop run.
pub struct RunOptions<'a> {
    /// Explicit config path (from `--config`). When `None`, config is discovered automatically.
    pub config_path: Option<&'a Path>,
    /// Suppress all stdout output.
    pub quiet: bool,
    /// Output format.
    pub format: Format,
    /// Suppress the warning printed when no config file is found.
    pub no_config_warning: bool,
    /// Baseline percentage (1-100). Files at or above this percentage of their
    /// limit are reported. Default: 100 (only files strictly over the limit).
    pub baseline: u8,
}

/// Runs the full linecop check pipeline.
///
/// Returns `true` if there are violations (i.e. the process should exit non-zero).
///
/// # Errors
///
/// Returns an error if the root path does not exist, the config cannot be loaded,
/// counting fails, or output fails.
pub fn run(root: &Path, opts: &RunOptions<'_>) -> Result<bool> {
    if !root.exists() {
        bail!("scan path does not exist: {}", root.display());
    }

    let root_abs = std::path::absolute(root)?;
    let cwd = std::env::current_dir()?;

    let cfg = if let Some(explicit) = opts.config_path {
        config::load(explicit)?
    } else if let Some(found) = config::find_config(&root_abs, &cwd) {
        config::load(&found)?
    } else {
        if !opts.quiet && !opts.no_config_warning {
            let mut stderr = anstream::stderr().lock();
            writeln!(
                stderr,
                "warning: no .linecop.yaml found, using default limit of {} lines for all files",
                config::DEFAULT_LIMIT
            )?;
        }
        config::Config::fallback()
    };

    let files = counter::count(root, &cfg)?;
    let violations = checker::check(&files, &cfg, opts.baseline);

    if !opts.quiet {
        let mut stdout = anstream::stdout().lock();
        report::print(&mut stdout, &violations, opts.format)?;
    }

    Ok(!violations.is_empty())
}

#[cfg(test)]
mod tests {
    use super::{RunOptions, run};
    use crate::report::Format;
    use std::io::Write;
    use std::path::Path;

    fn opts_with_config(path: &Path) -> RunOptions<'_> {
        RunOptions {
            config_path: Some(path),
            quiet: true,
            format: Format::Text,
            no_config_warning: true,
            baseline: 100,
        }
    }

    #[test]
    fn run_with_valid_config_no_violations() {
        let dir = tempfile::tempdir().expect("tempdir");

        let rs_path = dir.path().join("hello.rs");
        let mut file = std::fs::File::create(&rs_path).expect("create");
        writeln!(file, "fn main() {{}}").expect("write");

        let cfg_path = dir.path().join(".linecop.yaml");
        let mut cfg_file = std::fs::File::create(&cfg_path).expect("create");
        write!(cfg_file, "limits:\n  Rust: 500\n").expect("write");

        let has_violations = run(dir.path(), &opts_with_config(&cfg_path)).expect("run");
        assert!(!has_violations);
    }

    #[test]
    fn run_with_violations() {
        let dir = tempfile::tempdir().expect("tempdir");

        let rs_path = dir.path().join("big.rs");
        let mut file = std::fs::File::create(&rs_path).expect("create");
        for ii in 0..5 {
            writeln!(file, "fn f{ii}() {{}}").expect("write");
        }

        let cfg_path = dir.path().join(".linecop.yaml");
        let mut cfg_file = std::fs::File::create(&cfg_path).expect("create");
        write!(cfg_file, "limits:\n  Rust: 3\n").expect("write");

        let has_violations = run(dir.path(), &opts_with_config(&cfg_path)).expect("run");
        assert!(has_violations);
    }

    #[test]
    fn run_missing_explicit_config() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cfg_path = dir.path().join(".linecop.yaml");
        let result = run(dir.path(), &opts_with_config(&cfg_path));
        assert!(result.is_err());
    }

    #[test]
    fn run_no_config_uses_fallback() {
        let dir = tempfile::tempdir().expect("tempdir");

        // Small file — should pass with 500-line default
        let rs_path = dir.path().join("hello.rs");
        let mut file = std::fs::File::create(&rs_path).expect("create");
        writeln!(file, "fn main() {{}}").expect("write");

        let opts = RunOptions {
            config_path: None,
            quiet: true,
            format: Format::Text,
            no_config_warning: true,
            baseline: 100,
        };
        let has_violations = run(dir.path(), &opts).expect("run");
        assert!(!has_violations);
    }

    #[test]
    fn run_nonexistent_root_path() {
        let opts = RunOptions {
            config_path: None,
            quiet: true,
            format: Format::Text,
            no_config_warning: true,
            baseline: 100,
        };
        let result = run(Path::new("/nonexistent/path"), &opts);
        let err = result.expect_err("should fail for nonexistent path");
        assert!(err.to_string().contains("scan path does not exist"));
    }

    #[test]
    fn run_with_baseline() {
        let dir = tempfile::tempdir().expect("tempdir");

        let rs_path = dir.path().join("near.rs");
        let mut file = std::fs::File::create(&rs_path).expect("create");
        for ii in 0..4 {
            writeln!(file, "fn f{ii}() {{}}").expect("write");
        }

        let cfg_path = dir.path().join(".linecop.yaml");
        let mut cfg_file = std::fs::File::create(&cfg_path).expect("create");
        write!(cfg_file, "limits:\n  Rust: 5\n").expect("write");

        let opts = RunOptions {
            config_path: Some(&cfg_path),
            quiet: true,
            format: Format::Text,
            no_config_warning: true,
            baseline: 80,
        };
        let has_violations = run(dir.path(), &opts).expect("run");
        assert!(has_violations, "4 lines >= 80% of 5 = 4");
    }
}
