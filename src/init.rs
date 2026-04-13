use std::path::Path;

use anyhow::{Result, bail};

const STARTER_CONFIG: &str = "\
limits:
  Rust: 500
  Markdown: 200
";

fn default_schema_url() -> String {
    let version = env!("CARGO_PKG_VERSION");
    format!(
        "https://raw.githubusercontent.com/mlavrinenko/linecop/refs/tags/v{version}/linecop-schema.json"
    )
}

/// Schema mode for the generated config file.
pub enum SchemaMode {
    /// Attach the default schema URL for the current version.
    Default,
    /// Attach a custom schema URL.
    Custom(String),
    /// Do not attach any schema comment.
    None,
}

/// Writes a starter `.linecop.yaml` to the given directory.
///
/// # Errors
///
/// Returns an error if the config file already exists or cannot be written.
pub fn create(dir: &Path, schema: &SchemaMode) -> Result<String> {
    let path = dir.join(".linecop.yaml");
    if path.exists() {
        bail!("{} already exists", path.display());
    }

    let header = match schema {
        SchemaMode::Default => {
            format!("# yaml-language-server: $schema={}\n", default_schema_url())
        }
        SchemaMode::Custom(url) => format!("# yaml-language-server: $schema={url}\n"),
        SchemaMode::None => String::new(),
    };

    let contents = format!("{header}{STARTER_CONFIG}");
    std::fs::write(&path, contents)?;
    Ok(format!("Created {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::{SchemaMode, create};

    #[test]
    fn creates_config_with_default_schema() {
        let dir = tempfile::tempdir().expect("tempdir");
        let result = create(dir.path(), &SchemaMode::Default).expect("create");
        assert!(result.contains(".linecop.yaml"));

        let contents = std::fs::read_to_string(dir.path().join(".linecop.yaml")).expect("read");
        assert!(contents.starts_with("# yaml-language-server: $schema=https://"));
        assert!(contents.contains(env!("CARGO_PKG_VERSION")));
        assert!(contents.contains("limits:"));
        assert!(contents.contains("Rust: 500"));
    }

    #[test]
    fn creates_config_with_custom_schema() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mode = SchemaMode::Custom("https://example.com/schema.json".to_owned());
        create(dir.path(), &mode).expect("create");

        let contents = std::fs::read_to_string(dir.path().join(".linecop.yaml")).expect("read");
        assert!(contents.contains("$schema=https://example.com/schema.json"));
    }

    #[test]
    fn creates_config_without_schema() {
        let dir = tempfile::tempdir().expect("tempdir");
        create(dir.path(), &SchemaMode::None).expect("create");

        let contents = std::fs::read_to_string(dir.path().join(".linecop.yaml")).expect("read");
        assert!(!contents.contains("yaml-language-server"));
        assert!(contents.starts_with("limits:"));
    }

    #[test]
    fn refuses_to_overwrite() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join(".linecop.yaml"), "existing").expect("write");

        let err = create(dir.path(), &SchemaMode::Default).expect_err("should fail");
        assert!(err.to_string().contains("already exists"));
    }
}
