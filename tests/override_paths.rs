//! An override pattern reads as a path from the config file's directory, so
//! one config gives the same verdict whatever path is scanned and wherever
//! linecop runs from.

mod common;

use common::{linecop, write_config};
use std::path::Path;

/// A repo whose `src/big.rs` is over the Rust limit unless the override for
/// `pattern` reaches it.
fn repo(pattern: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    write_config(
        dir.path(),
        &format!("limits:\n  Rust: 2\noverrides:\n  - pattern: \"{pattern}\"\n    exclude: true\n"),
    );
    std::fs::create_dir(dir.path().join("src")).expect("mkdir");
    std::fs::write(
        dir.path().join("src/big.rs"),
        "fn a() {}\nfn b() {}\nfn c() {}\n",
    )
    .expect("write");
    dir
}

fn passes(cwd: &Path, args: &[&str]) {
    linecop().current_dir(cwd).args(args).assert().success();
}

#[test]
fn matches_when_scanning_the_current_directory() {
    let dir = repo("src/big.rs");
    passes(dir.path(), &[]);
    passes(dir.path(), &["."]);
}

#[test]
fn matches_when_scanning_a_subdirectory() {
    let dir = repo("src/big.rs");
    passes(dir.path(), &["src"]);
    passes(dir.path(), &["./src/"]);
}

#[test]
fn matches_when_scanning_an_absolute_path() {
    let dir = repo("src/big.rs");
    let abs = dir.path().to_str().expect("utf-8 path");
    passes(dir.path(), &[abs]);
}

#[test]
fn matches_with_an_explicit_config_from_another_directory() {
    let dir = repo("src/big.rs");
    passes(
        &dir.path().join("src"),
        &[".", "--config", "../.linecop.yaml"],
    );
}

#[test]
fn a_leading_dot_slash_means_the_config_directory() {
    let dir = repo("./src/big.rs");
    passes(dir.path(), &[]);
    passes(dir.path(), &["src"]);
}

#[test]
fn a_pattern_for_another_file_still_misses() {
    let dir = repo("tables.rs");
    linecop().current_dir(dir.path()).assert().code(1);
}

#[test]
fn reported_paths_keep_the_scanned_prefix() {
    let dir = repo("src/other.rs");
    linecop()
        .current_dir(dir.path())
        .args([".", "--format", "paths"])
        .assert()
        .code(1)
        .stdout("./src/big.rs\n");
}

// --- gitignore rules ---

fn reported(pattern: &str) -> String {
    let dir = repo(pattern);
    std::fs::create_dir(dir.path().join("src/gen")).expect("mkdir");
    std::fs::write(
        dir.path().join("src/gen/deep.rs"),
        "fn a() {}\nfn b() {}\nfn c() {}\n",
    )
    .expect("write");
    let out = linecop()
        .current_dir(dir.path())
        .args(["--format", "paths"])
        .output()
        .expect("run");
    String::from_utf8(out.stdout).expect("utf-8")
}

#[test]
fn a_star_stays_inside_one_directory() {
    assert_eq!(reported("src/*.rs"), "./src/gen/deep.rs\n");
}

#[test]
fn a_double_star_crosses_directories() {
    assert_eq!(reported("src/**/*.rs"), "");
}

#[test]
fn a_pattern_without_a_slash_matches_at_any_depth() {
    assert_eq!(reported("deep.rs"), "./src/big.rs\n");
    assert_eq!(reported("*.rs"), "");
}

#[test]
fn a_leading_slash_anchors_to_the_config_directory() {
    assert_eq!(reported("/big.rs"), "./src/big.rs\n./src/gen/deep.rs\n");
    assert_eq!(reported("/src/big.rs"), "./src/gen/deep.rs\n");
}

#[test]
fn a_trailing_slash_covers_a_directory() {
    assert_eq!(reported("gen/"), "./src/big.rs\n");
    assert_eq!(reported("src/"), "");
}

#[test]
fn a_negated_pattern_is_a_config_error() {
    let dir = repo("!src/big.rs");
    linecop()
        .current_dir(dir.path())
        .assert()
        .code(2)
        .stderr(predicates::str::contains("!src/big.rs"));
}
