//! The config is found by walking up from the scanned path to the repository
//! root, so running inside a subdirectory checks against the repo's config.

mod common;

use common::{linecop, write_config};
use std::path::Path;

const STRICT: &str = "limits:\n  Rust: 2\n";
const BIG: &str = "fn a() {}\nfn b() {}\nfn c() {}\n";

fn mkdir(path: &Path) {
    std::fs::create_dir_all(path).expect("mkdir");
}

/// A repo (marked by a `.git` directory) with a strict config at its root and
/// an over-limit `src/x.rs`.
fn repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    mkdir(&dir.path().join(".git"));
    mkdir(&dir.path().join("src"));
    write_config(dir.path(), STRICT);
    std::fs::write(dir.path().join("src/x.rs"), BIG).expect("write");
    dir
}

fn fails(cwd: &Path, args: &[&str]) {
    linecop().current_dir(cwd).args(args).assert().code(1);
}

fn passes_warning_free(cwd: &Path, args: &[&str]) {
    linecop()
        .current_dir(cwd)
        .args(args)
        .assert()
        .success()
        .stderr("");
}

#[test]
fn finds_config_from_subdirectory_of_repo() {
    let dir = repo();
    fails(&dir.path().join("src"), &[]);
}

#[test]
fn finds_config_when_scanning_src_from_a_deeper_cwd() {
    let dir = repo();
    mkdir(&dir.path().join("src/deep"));
    fails(&dir.path().join("src/deep"), &["..", "--format", "paths"]);
}

#[test]
fn finds_config_through_dotdot_scan_path() {
    let dir = repo();
    mkdir(&dir.path().join("docs"));
    fails(&dir.path().join("docs"), &["../src"]);
}

#[test]
fn git_file_marks_a_repo_root() {
    let dir = repo();
    std::fs::remove_dir(dir.path().join(".git")).expect("rm");
    std::fs::write(dir.path().join(".git"), "gitdir: elsewhere\n").expect("write");
    fails(&dir.path().join("src"), &[]);
}

#[test]
fn other_vcs_markers_stop_the_walk() {
    for marker in [".jj", ".hg", ".svn"] {
        let outer = tempfile::tempdir().expect("tempdir");
        write_config(outer.path(), STRICT);
        let inner = outer.path().join("inner");
        mkdir(&inner.join(marker));
        mkdir(&inner.join("src"));
        std::fs::write(inner.join("src/x.rs"), BIG).expect("write");
        linecop()
            .current_dir(inner.join("src"))
            .assert()
            .success()
            .stderr(predicates::str::contains("no .linecop.yaml found"));
    }
}

#[test]
fn config_above_repo_root_is_not_found_by_default() {
    let outer = tempfile::tempdir().expect("tempdir");
    write_config(outer.path(), STRICT);
    let inner = outer.path().join("inner");
    mkdir(&inner.join(".git"));
    std::fs::write(inner.join("x.rs"), BIG).expect("write");
    warns_and_passes(&inner, &[]);
}

fn warns_and_passes(cwd: &Path, args: &[&str]) {
    linecop()
        .current_dir(cwd)
        .args(args)
        .assert()
        .success()
        .stderr(predicates::str::contains("no .linecop.yaml found"));
}

#[test]
fn config_search_root_finds_config_above_repo_root() {
    let outer = tempfile::tempdir().expect("tempdir");
    write_config(outer.path(), STRICT);
    let inner = outer.path().join("inner");
    mkdir(&inner.join(".git"));
    std::fs::write(inner.join("x.rs"), BIG).expect("write");
    fails(&inner, &["--config-search", "root"]);
}

#[test]
fn nested_repo_stops_at_its_own_root() {
    let dir = repo();
    let nested = dir.path().join("vendor/lib");
    mkdir(&nested.join(".git"));
    std::fs::write(nested.join("y.rs"), BIG).expect("write");
    warns_and_passes(&nested, &[]);
}

#[test]
fn nearest_config_wins() {
    let dir = repo();
    let sub = dir.path().join("src");
    write_config(&sub, "limits:\n  Rust: 100\n");
    passes_warning_free(&sub, &[]);
}

#[test]
fn explicit_config_bypasses_discovery() {
    let dir = repo();
    let lax = tempfile::tempdir().expect("tempdir");
    let cfg = write_config(lax.path(), "limits:\n  Rust: 100\n");
    let cfg = cfg.to_str().expect("utf8");
    passes_warning_free(&dir.path().join("src"), &["--config", cfg]);
}

#[test]
fn without_a_repo_the_walk_stops_at_the_working_directory() {
    let outer = tempfile::tempdir().expect("tempdir");
    write_config(outer.path(), STRICT);
    let sub = outer.path().join("sub");
    mkdir(&sub);
    std::fs::write(sub.join("x.rs"), BIG).expect("write");
    warns_and_passes(&sub, &[]);
    fails(outer.path(), &["sub"]);
}
