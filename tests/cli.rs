mod common;

use common::{linecop, write_config};
use predicates::prelude::predicate;

// --- Happy path ---

#[test]
fn no_violations_exits_zero() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cfg = write_config(dir.path(), "limits:\n  Rust: 500\n");

    let rs_path = dir.path().join("hello.rs");
    std::fs::write(&rs_path, "fn main() {}\n").expect("write");

    linecop()
        .arg(dir.path())
        .arg("--config")
        .arg(&cfg)
        .assert()
        .success()
        .stdout(predicate::str::contains("All files within size limits"));
}

#[test]
fn quiet_mode_suppresses_output() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cfg = write_config(dir.path(), "limits:\n  Rust: 500\n");

    linecop()
        .arg(dir.path())
        .arg("--config")
        .arg(&cfg)
        .arg("--quiet")
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

// --- Violations ---

#[test]
fn violation_exits_one() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cfg = write_config(dir.path(), "limits:\n  Rust: 2\n");

    let rs_path = dir.path().join("big.rs");
    std::fs::write(&rs_path, "fn a() {}\nfn b() {}\nfn c() {}\n").expect("write");

    linecop()
        .arg(dir.path())
        .arg("--config")
        .arg(&cfg)
        .assert()
        .code(1)
        .stdout(predicate::str::contains("file(s) reported"));
}

#[test]
fn json_format_outputs_valid_json() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cfg = write_config(dir.path(), "limits:\n  Rust: 2\n");

    let rs_path = dir.path().join("big.rs");
    std::fs::write(&rs_path, "fn a() {}\nfn b() {}\nfn c() {}\n").expect("write");

    let output = linecop()
        .arg(dir.path())
        .arg("--config")
        .arg(&cfg)
        .arg("--format")
        .arg("json")
        .assert()
        .code(1)
        .get_output()
        .stdout
        .clone();

    let parsed: serde_json::Value = serde_json::from_slice(&output).expect("valid json output");
    let arr = parsed.as_array().expect("array");
    assert!(!arr.is_empty());
}

// --- Error cases ---

#[test]
fn missing_explicit_config_exits_two() {
    linecop()
        .arg("--config")
        .arg("/nonexistent/config.yaml")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("failed to read config file"));
}

#[test]
fn nonexistent_scan_path_exits_two() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cfg = write_config(dir.path(), "limits:\n  Rust: 500\n");

    linecop()
        .arg("/nonexistent/scan/path")
        .arg("--config")
        .arg(&cfg)
        .assert()
        .code(2)
        .stderr(predicate::str::contains("scan path does not exist"));
}

/// A broken config must not read as ordinary violations, even when the tree
/// holds a file that would trip the limit.
fn assert_config_error(config: &str, stderr: &str) {
    let dir = tempfile::tempdir().expect("tempdir");
    let cfg = write_config(dir.path(), config);
    std::fs::write(
        dir.path().join("big.rs"),
        "fn a() {}\nfn b() {}\nfn c() {}\n",
    )
    .expect("write");

    linecop()
        .arg(dir.path())
        .arg("--config")
        .arg(&cfg)
        .assert()
        .code(2)
        .stderr(predicate::str::contains(stderr));
}

#[test]
fn malformed_config_exits_two() {
    assert_config_error("limits: [Rust: 2\n", "failed to parse config file");
}

#[test]
fn invalid_override_glob_exits_two() {
    assert_config_error(
        "limits:\n  Rust: 2\noverrides:\n  - pattern: \"src/[\"\n    limit: 1\n",
        "invalid glob pattern",
    );
}

#[test]
fn unknown_language_exits_two() {
    assert_config_error("limits:\n  NotALanguage: 2\n", "unknown language");
}

// --- Configless mode ---

#[test]
fn no_config_runs_with_default_limit_and_warning() {
    let dir = tempfile::tempdir().expect("tempdir");

    // Create a small file — should pass with 500-line default
    let rs_path = dir.path().join("hello.rs");
    std::fs::write(&rs_path, "fn main() {}\n").expect("write");

    linecop()
        .arg(dir.path())
        .assert()
        .success()
        .stderr(predicate::str::contains("no .linecop.yaml found"))
        .stdout(predicate::str::contains("All files within size limits"));
}

#[test]
fn no_config_warning_suppressed() {
    let dir = tempfile::tempdir().expect("tempdir");

    let rs_path = dir.path().join("hello.rs");
    std::fs::write(&rs_path, "fn main() {}\n").expect("write");

    linecop()
        .arg(dir.path())
        .arg("--no-config-warning")
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}

// --- Paths format ---

#[test]
fn paths_format_outputs_only_paths() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cfg = write_config(dir.path(), "limits:\n  Rust: 2\n");

    let rs_path = dir.path().join("big.rs");
    std::fs::write(&rs_path, "fn a() {}\nfn b() {}\nfn c() {}\n").expect("write");

    let output = linecop()
        .arg(dir.path())
        .arg("--config")
        .arg(&cfg)
        .arg("--format")
        .arg("paths")
        .assert()
        .code(1)
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).expect("utf8");
    assert!(output_str.contains("big.rs"));
    assert!(!output_str.contains("lines"));
    assert!(!output_str.contains("limit"));
}

#[test]
fn paths_format_no_violations_empty_output() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cfg = write_config(dir.path(), "limits:\n  Rust: 500\n");

    let rs_path = dir.path().join("hello.rs");
    std::fs::write(&rs_path, "fn main() {}\n").expect("write");

    linecop()
        .arg(dir.path())
        .arg("--config")
        .arg(&cfg)
        .arg("--format")
        .arg("paths")
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

// --- Baseline ---

#[test]
fn baseline_reports_near_limit_files() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cfg = write_config(dir.path(), "limits:\n  Rust: 5\n");

    let rs_path = dir.path().join("near.rs");
    std::fs::write(&rs_path, "fn a() {}\nfn b() {}\nfn c() {}\nfn d() {}\n").expect("write");

    linecop()
        .arg(dir.path())
        .arg("--config")
        .arg(&cfg)
        .arg("--baseline")
        .arg("80")
        .assert()
        .code(1)
        .stdout(predicate::str::contains("near.rs"));
}

#[test]
fn baseline_default_100_backward_compatible() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cfg = write_config(dir.path(), "limits:\n  Rust: 5\n");

    let rs_path = dir.path().join("exact.rs");
    std::fs::write(
        &rs_path,
        "fn a() {}\nfn b() {}\nfn c() {}\nfn d() {}\nfn e() {}\n",
    )
    .expect("write");

    linecop()
        .arg(dir.path())
        .arg("--config")
        .arg(&cfg)
        .assert()
        .success()
        .stdout(predicate::str::contains("All files within size limits"));
}

#[test]
#[allow(clippy::indexing_slicing)]
fn baseline_json_includes_baseline_limit() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cfg = write_config(dir.path(), "limits:\n  Rust: 5\n");

    let rs_path = dir.path().join("near.rs");
    std::fs::write(&rs_path, "fn a() {}\nfn b() {}\nfn c() {}\nfn d() {}\n").expect("write");

    let output = linecop()
        .arg(dir.path())
        .arg("--config")
        .arg(&cfg)
        .arg("--baseline")
        .arg("80")
        .arg("--format")
        .arg("json")
        .assert()
        .code(1)
        .get_output()
        .stdout
        .clone();

    let parsed: serde_json::Value = serde_json::from_slice(&output).expect("valid json");
    let arr = parsed.as_array().expect("array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["baseline-limit"], 4);
}

#[test]
fn baseline_rejects_invalid_values() {
    linecop()
        .arg("--baseline")
        .arg("0")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("invalid"));

    linecop()
        .arg("--baseline")
        .arg("101")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("invalid"));
}

// --- Hidden directories ---

/// Writes an oversized shell script under a dot-directory, mirroring the
/// `.just/scripts` layout that first exposed the blind spot. Returns the path
/// only after confirming the file landed — a gate assertion against a file that
/// was never written proves nothing.
fn write_hidden_probe(dir: &std::path::Path) -> std::path::PathBuf {
    let scripts = dir.join(".just").join("scripts");
    std::fs::create_dir_all(&scripts).expect("create hidden dir");
    let probe = scripts.join("probe.sh");
    std::fs::write(
        &probe,
        "#!/usr/bin/env bash\necho one\necho two\necho three\n",
    )
    .expect("write probe");
    assert!(
        probe.is_file(),
        "probe must exist before asserting the gate"
    );
    probe
}

#[test]
fn hidden_dir_unscanned_by_default() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cfg = write_config(dir.path(), "limits:\n  Shell: 2\n");
    write_hidden_probe(dir.path());

    linecop()
        .arg(dir.path())
        .arg("--config")
        .arg(&cfg)
        .assert()
        .success()
        .stdout(predicate::str::contains("All files within size limits"));
}

#[test]
fn include_hidden_config_reports_dot_dir_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cfg = write_config(dir.path(), "limits:\n  Shell: 2\ninclude_hidden: true\n");
    write_hidden_probe(dir.path());

    linecop()
        .arg(dir.path())
        .arg("--config")
        .arg(&cfg)
        .assert()
        .code(1)
        .stdout(predicate::str::contains("probe.sh"));
}

#[test]
fn hidden_flag_reports_dot_dir_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cfg = write_config(dir.path(), "limits:\n  Shell: 2\n");
    write_hidden_probe(dir.path());

    linecop()
        .arg(dir.path())
        .arg("--config")
        .arg(&cfg)
        .arg("--hidden")
        .assert()
        .code(1)
        .stdout(predicate::str::contains("probe.sh"));
}

// --- Byte cap ---

#[test]
fn file_under_line_limit_but_over_max_bytes_fails() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cfg = write_config(
        dir.path(),
        "limits:\n  Markdown: 200\noverrides:\n  - pattern: \"**/ESSAY.md\"\n    max_bytes: 100\n",
    );

    // Two lines, one of them a paragraph that grew into an essay.
    let essay = format!("# Essay\n{}\n", "word ".repeat(40));
    std::fs::write(dir.path().join("ESSAY.md"), &essay).expect("write");

    linecop()
        .arg(dir.path())
        .arg("--config")
        .arg(&cfg)
        .assert()
        .code(1)
        .stdout(predicate::str::contains(format!(
            "ESSAY.md: {} bytes (max_bytes: 100, +{} over)",
            essay.len(),
            essay.len() - 100
        )));
}
