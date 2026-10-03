//! The linecop skill's evals run against `skills/linecop/evals/fixture` and
//! assume the state pinned here. An eval fixture no gate checks rots silently,
//! so a change that moves any file across a band fails here, not in an eval.

use assert_cmd::Command;

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/skills/linecop/evals/fixture");

fn paths_at(baseline: &str) -> assert_cmd::assert::Assert {
    Command::new(assert_cmd::cargo_bin!("linecop"))
        .current_dir(FIXTURE)
        .args([".", "--format", "paths", "--baseline", baseline])
        .assert()
}

#[test]
fn only_the_generated_table_breaches() {
    paths_at("100").code(1).stdout("./src/tables.rs\n");
}

#[test]
fn the_warning_band_adds_the_parser() {
    paths_at("85")
        .code(1)
        .stdout("./src/parser.rs\n./src/tables.rs\n");
}

#[test]
fn the_parser_looks_over_by_raw_lines() {
    // count_mode: code is the trap: `wc -l` says over, linecop says under.
    let parser = std::fs::read_to_string(format!("{FIXTURE}/src/parser.rs")).expect("read parser");
    assert!(
        parser.lines().count() > 80,
        "parser.rs must exceed 80 raw lines"
    );
}
