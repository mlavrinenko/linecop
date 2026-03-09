use std::path::Path;

use anyhow::{Result, bail};

const STARTER_CONFIG: &str = "\
limits:
  Rust: 500
  Markdown: 200
";

/// Writes a starter `.linecop.yaml` to the given directory.
///
/// # Errors
///
/// Returns an error if the config file already exists or cannot be written.
pub fn create(dir: &Path) -> Result<String> {
    let path = dir.join(".linecop.yaml");
    if path.exists() {
        bail!("{} already exists", path.display());
    }
    std::fs::write(&path, STARTER_CONFIG)?;
    Ok(format!("Created {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::create;

    #[test]
    fn creates_config_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let result = create(dir.path()).expect("create");
        assert!(result.contains(".linecop.yaml"));

        let contents = std::fs::read_to_string(dir.path().join(".linecop.yaml")).expect("read");
        assert!(contents.contains("limits:"));
        assert!(contents.contains("Rust: 500"));
    }

    #[test]
    fn refuses_to_overwrite() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join(".linecop.yaml"), "existing").expect("write");

        let err = create(dir.path()).expect_err("should fail");
        assert!(err.to_string().contains("already exists"));
    }
}
