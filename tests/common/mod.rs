use assert_cmd::Command;
use std::io::Write;

pub fn linecop() -> Command {
    Command::new(assert_cmd::cargo_bin!("linecop"))
}

pub fn write_config(dir: &std::path::Path, content: &str) -> std::path::PathBuf {
    let path = dir.join(".linecop.yaml");
    let mut file = std::fs::File::create(&path).expect("create config");
    write!(file, "{content}").expect("write config");
    path
}
