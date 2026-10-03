mod common;

use common::{linecop, write_config};
use predicates::prelude::predicate;

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
fn init_creates_config_file_with_schema() {
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
    assert!(contents.starts_with("# yaml-language-server: $schema=https://"));
    assert!(contents.contains("limits:"));
    assert!(contents.contains("Rust: 500"));
}

#[test]
fn init_no_schema_omits_header() {
    let dir = tempfile::tempdir().expect("tempdir");

    linecop()
        .arg(dir.path())
        .arg("init")
        .arg("--no-schema")
        .assert()
        .success();

    let contents = std::fs::read_to_string(dir.path().join(".linecop.yaml")).expect("read");
    assert!(!contents.contains("yaml-language-server"));
    assert!(contents.starts_with("limits:"));
}

#[test]
fn init_refuses_to_overwrite_existing() {
    let dir = tempfile::tempdir().expect("tempdir");
    write_config(dir.path(), "limits:\n  Rust: 100\n");

    linecop()
        .arg(dir.path())
        .arg("init")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("already exists"));
}
