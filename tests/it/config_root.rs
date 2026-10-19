//! A config's `root` key names the tree it describes, so the file can sit in
//! `.config/` and still read its patterns from the repository root.

use crate::common::linecop;
use std::path::Path;

const BIG: &str = "fn a() {}\nfn b() {}\nfn c() {}\n";

fn write(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("mkdir");
    }
    std::fs::write(path, content).expect("write");
}

/// A repo with two over-limit files, `src/big.rs` and `src/other.rs`, and a
/// `.config/linecop.yaml` that opens with `head` and excludes `src/big.rs`.
fn repo(head: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir(dir.path().join(".git")).expect("mkdir");
    write(
        &dir.path().join(".config/linecop.yaml"),
        &format!(
            "{head}limits:\n  Rust: 2\noverrides:\n  - pattern: \"src/big.rs\"\n    exclude: true\n"
        ),
    );
    write(&dir.path().join("src/big.rs"), BIG);
    write(&dir.path().join("src/other.rs"), BIG);
    dir
}

/// The exit code and the `--format paths` report of a run in `cwd`.
fn paths(cwd: &Path, args: &[&str]) -> (Option<i32>, String) {
    let out = linecop()
        .current_dir(cwd)
        .args(["--format", "paths"])
        .args(args)
        .output()
        .expect("run");
    let stdout = String::from_utf8(out.stdout).expect("utf-8");
    (out.status.code(), stdout)
}

const CONFIG: [&str; 2] = ["-c", ".config/linecop.yaml"];

#[test]
fn without_root_patterns_stay_at_the_config_directory() {
    let dir = repo("");
    assert_eq!(
        paths(dir.path(), &CONFIG),
        (Some(1), "./src/big.rs\n./src/other.rs\n".to_owned())
    );
}

#[test]
fn root_anchors_the_override_patterns() {
    let dir = repo("root: ..\n");
    assert_eq!(
        paths(dir.path(), &CONFIG),
        (Some(1), "./src/other.rs\n".to_owned())
    );
}

#[test]
fn root_is_the_default_scan_root() {
    let dir = repo("root: ..\n");
    let docs = dir.path().join("docs");
    std::fs::create_dir(&docs).expect("mkdir");
    assert_eq!(
        paths(&docs, &["-c", "../.config/linecop.yaml"]),
        (Some(1), "../src/other.rs\n".to_owned())
    );
}

#[test]
fn a_root_below_the_config_scans_from_there() {
    let dir = tempfile::tempdir().expect("tempdir");
    write(
        &dir.path().join(".linecop.yaml"),
        "root: src\nlimits:\n  Rust: 2\noverrides:\n  - pattern: \"/big.rs\"\n    exclude: true\n",
    );
    write(&dir.path().join("src/big.rs"), BIG);
    write(&dir.path().join("src/other.rs"), BIG);
    write(&dir.path().join("top.rs"), BIG);
    assert_eq!(
        paths(dir.path(), &[]),
        (Some(1), "src/other.rs\n".to_owned())
    );
}

#[test]
fn a_path_argument_still_picks_the_scan() {
    let dir = repo("root: ..\n");
    write(&dir.path().join("lib/deep.rs"), BIG);
    assert_eq!(
        paths(dir.path(), &["-c", ".config/linecop.yaml", "src"]),
        (Some(1), "src/other.rs\n".to_owned())
    );
}

#[test]
fn a_root_that_is_no_directory_is_a_config_error() {
    let dir = repo("root: ../nowhere\n");
    linecop()
        .current_dir(dir.path())
        .args(CONFIG)
        .assert()
        .code(2)
        .stderr(predicates::str::contains("nowhere"));
}
