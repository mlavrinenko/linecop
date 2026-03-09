use assert_cmd::Command;
use predicates::prelude::predicate;
use std::io::Write;

fn linecop() -> Command {
    Command::new(assert_cmd::cargo_bin!("linecop"))
}

fn write_config(dir: &std::path::Path, content: &str) -> std::path::PathBuf {
    let path = dir.join(".linecop.yaml");
    let mut file = std::fs::File::create(&path).expect("create config");
    write!(file, "{content}").expect("write config");
    path
}

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
fn violation_exits_nonzero() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cfg = write_config(dir.path(), "limits:\n  Rust: 2\n");

    let rs_path = dir.path().join("big.rs");
    std::fs::write(&rs_path, "fn a() {}\nfn b() {}\nfn c() {}\n").expect("write");

    linecop()
        .arg(dir.path())
        .arg("--config")
        .arg(&cfg)
        .assert()
        .failure()
        .stdout(predicate::str::contains("exceed size limits"));
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
        .failure()
        .get_output()
        .stdout
        .clone();

    let parsed: serde_json::Value = serde_json::from_slice(&output).expect("valid json output");
    let arr = parsed.as_array().expect("array");
    assert!(!arr.is_empty());
}

// --- Error cases ---

#[test]
fn missing_explicit_config_exits_nonzero() {
    linecop()
        .arg("--config")
        .arg("/nonexistent/config.yaml")
        .assert()
        .failure()
        .stderr(predicate::str::contains("failed to read config file"));
}

#[test]
fn nonexistent_scan_path_exits_nonzero() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cfg = write_config(dir.path(), "limits:\n  Rust: 500\n");

    linecop()
        .arg("/nonexistent/scan/path")
        .arg("--config")
        .arg(&cfg)
        .assert()
        .failure()
        .stderr(predicate::str::contains("scan path does not exist"));
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

// --- Subcommands ---

#[test]
fn schema_subcommand_outputs_json_schema() {
    linecop()
        .arg("schema")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"$schema\""))
        .stdout(predicate::str::contains("\"limits\""));
}

#[test]
fn config_resolved_relative_to_path() {
    let dir = tempfile::tempdir().expect("tempdir");
    // Place config inside the scan directory, not CWD
    write_config(dir.path(), "limits:\n  Rust: 500\n");

    let rs_path = dir.path().join("hello.rs");
    std::fs::write(&rs_path, "fn main() {}\n").expect("write");

    // Run without --config; it should find .linecop.yaml inside dir
    linecop()
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("All files within size limits"));
}

#[test]
fn version_flag_shows_version() {
    linecop()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("linecop"));
}

// --- Init subcommand ---

#[test]
fn init_creates_config_file() {
    let dir = tempfile::tempdir().expect("tempdir");

    linecop()
        .arg(dir.path())
        .arg("init")
        .assert()
        .success()
        .stdout(predicate::str::contains("Created"));

    let config_path = dir.path().join(".linecop.yaml");
    assert!(config_path.exists());
    let contents = std::fs::read_to_string(&config_path).expect("read");
    assert!(contents.contains("limits:"));
    assert!(contents.contains("Rust: 500"));
}

#[test]
fn init_refuses_to_overwrite_existing() {
    let dir = tempfile::tempdir().expect("tempdir");
    write_config(dir.path(), "limits:\n  Rust: 100\n");

    linecop()
        .arg(dir.path())
        .arg("init")
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));
}
