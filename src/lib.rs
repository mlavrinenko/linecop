pub mod checker;
pub mod config;
pub mod counter;
pub mod discovery;
pub mod init;
pub mod report;
pub mod schema;

#[cfg(test)]
pub(crate) mod test_helpers;

use std::io::Write;
use std::path::{Component, Path, PathBuf};

use anyhow::{Result, bail};

use crate::report::Format;

/// Options controlling a linecop run.
pub struct RunOptions<'a> {
    /// Explicit config path (from `--config`). When `None`, config is discovered automatically.
    pub config_path: Option<&'a Path>,
    /// How far config discovery walks up (from `--config-search`).
    pub config_search: discovery::ConfigSearch,
    /// Suppress all stdout output.
    pub quiet: bool,
    /// Output format.
    pub format: Format,
    /// Suppress the warning printed when no config file is found.
    pub no_config_warning: bool,
    /// Baseline percentage (1-100). Files at or above this percentage of their
    /// limit are reported. Default: 100 (only files strictly over the limit).
    pub baseline: u8,
    /// Scan dot-prefixed files and directories, overriding `include_hidden`
    /// from the config when set.
    pub hidden: bool,
}

/// Runs the full linecop check pipeline over `path`, or with `None` over the
/// config's `root`, else the working directory.
///
/// Returns `true` if there are violations (the CLI exits 1); errors mean the
/// check never ran (the CLI exits 2).
///
/// # Errors
///
/// Returns an error if the scan path does not exist, the config cannot be loaded,
/// counting fails, or output fails.
pub fn run(path: Option<&Path>, opts: &RunOptions<'_>) -> Result<bool> {
    let start = path.unwrap_or_else(|| Path::new("."));
    if !start.exists() {
        bail!("scan path does not exist: {}", start.display());
    }

    let start_abs = std::path::absolute(start)?;
    let cwd = std::env::current_dir()?;

    let mut cfg = if let Some(explicit) = opts.config_path {
        config::load(explicit)?
    } else if let Some(found) = discovery::find_config(&start_abs, &cwd, opts.config_search) {
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

    cfg.include_hidden |= opts.hidden;

    let scan = match (path, &cfg.root, &cfg.base_dir) {
        (None, Some(_), Some(root)) => relative_to(root, &std::fs::canonicalize(&cwd)?),
        _ => start.to_path_buf(),
    };
    let files = counter::count(&scan, &cfg)?;
    let violations = checker::check(&files, &cfg, opts.baseline);

    if !opts.quiet {
        let mut stdout = anstream::stdout().lock();
        report::print(&mut stdout, &violations, opts.format)?;
    }

    Ok(!violations.is_empty())
}

/// `target` as a path from `base`, both canonical, so a scan of the config's
/// `root` prints paths the way a scan of the same path typed in `base` would:
/// `.` for `base` itself, `src` below it, `..` above it. `target` unchanged
/// when the two share no root, as on two Windows drives.
fn relative_to(target: &Path, base: &Path) -> PathBuf {
    let shared = target
        .components()
        .zip(base.components())
        .take_while(|(aa, bb)| aa == bb)
        .count();
    if shared == 0 {
        return target.to_path_buf();
    }
    let mut rel: PathBuf = base
        .components()
        .skip(shared)
        .map(|_| Component::ParentDir)
        .collect();
    rel.extend(target.components().skip(shared));
    if rel.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        rel
    }
}

#[cfg(test)]
mod tests {
    use super::{RunOptions, relative_to, run};
    use crate::report::Format;
    use std::io::Write;
    use std::path::Path;

    fn opts_with_config(path: &Path) -> RunOptions<'_> {
        RunOptions {
            config_path: Some(path),
            config_search: crate::discovery::ConfigSearch::default(),
            quiet: true,
            format: Format::Text,
            no_config_warning: true,
            baseline: 100,
            hidden: false,
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

        let has_violations = run(Some(dir.path()), &opts_with_config(&cfg_path)).expect("run");
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

        let has_violations = run(Some(dir.path()), &opts_with_config(&cfg_path)).expect("run");
        assert!(has_violations);
    }

    #[test]
    fn run_missing_explicit_config() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cfg_path = dir.path().join(".linecop.yaml");
        let result = run(Some(dir.path()), &opts_with_config(&cfg_path));
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
            config_search: crate::discovery::ConfigSearch::default(),
            quiet: true,
            format: Format::Text,
            no_config_warning: true,
            baseline: 100,
            hidden: false,
        };
        let has_violations = run(Some(dir.path()), &opts).expect("run");
        assert!(!has_violations);
    }

    #[test]
    fn run_nonexistent_root_path() {
        let opts = RunOptions {
            config_path: None,
            config_search: crate::discovery::ConfigSearch::default(),
            quiet: true,
            format: Format::Text,
            no_config_warning: true,
            baseline: 100,
            hidden: false,
        };
        let result = run(Some(Path::new("/nonexistent/path")), &opts);
        let err = result.expect_err("should fail for nonexistent path");
        assert!(err.to_string().contains("scan path does not exist"));
    }

    #[test]
    fn hidden_option_overrides_config() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join(".just")).expect("mkdir");

        let rs_path = dir.path().join(".just/big.rs");
        let mut file = std::fs::File::create(&rs_path).expect("create");
        for ii in 0..5 {
            writeln!(file, "fn f{ii}() {{}}").expect("write");
        }

        let cfg_path = dir.path().join(".linecop.yaml");
        let mut cfg_file = std::fs::File::create(&cfg_path).expect("create");
        write!(cfg_file, "limits:\n  Rust: 3\n").expect("write");

        assert!(
            !run(Some(dir.path()), &opts_with_config(&cfg_path)).expect("run"),
            "config alone leaves the dot-directory unscanned"
        );

        let opts = RunOptions {
            hidden: true,
            ..opts_with_config(&cfg_path)
        };
        assert!(
            run(Some(dir.path()), &opts).expect("run"),
            "--hidden reaches it"
        );
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
            config_search: crate::discovery::ConfigSearch::default(),
            quiet: true,
            format: Format::Text,
            no_config_warning: true,
            baseline: 80,
            hidden: false,
        };
        let has_violations = run(Some(dir.path()), &opts).expect("run");
        assert!(has_violations, "4 lines >= 80% of 5 = 4");
    }

    #[test]
    fn relative_to_names_the_way_from_base() {
        let rel = |target: &str, base: &str| relative_to(Path::new(target), Path::new(base));
        assert_eq!(rel("/repo", "/repo"), Path::new("."));
        assert_eq!(rel("/repo/src", "/repo"), Path::new("src"));
        assert_eq!(rel("/repo", "/repo/docs/deep"), Path::new("../.."));
        assert_eq!(rel("/repo/src", "/repo/docs"), Path::new("../src"));
    }
}
