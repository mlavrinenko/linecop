use std::io::Write;

use anyhow::Result;
use serde::Serialize;

use crate::checker::Violation;

/// Output format for violation reports.
#[derive(Debug, Clone, Copy, Default, clap::ValueEnum)]
pub enum Format {
    /// Human-readable text output.
    #[default]
    Text,
    /// JSON array of violations.
    Json,
}

#[derive(Serialize)]
struct JsonViolation {
    path: String,
    language: String,
    lines: u64,
    limit: u64,
}

/// Prints violations to the given writer.
///
/// # Errors
///
/// Returns an error if writing to the output fails.
pub fn print(writer: &mut dyn Write, violations: &[Violation], format: Format) -> Result<()> {
    match format {
        Format::Text => print_text(writer, violations),
        Format::Json => print_json(writer, violations),
    }
}

fn print_text(writer: &mut dyn Write, violations: &[Violation]) -> Result<()> {
    for vv in violations {
        writeln!(
            writer,
            "--- {}: {} lines (limit: {})",
            vv.path.display(),
            vv.lines,
            vv.limit
        )?;
    }
    if violations.is_empty() {
        writeln!(writer, "All files within size limits.")?;
    } else {
        writeln!(
            writer,
            "\n{} file(s) exceed size limits. Consider refactoring.",
            violations.len()
        )?;
    }
    Ok(())
}

fn print_json(writer: &mut dyn Write, violations: &[Violation]) -> Result<()> {
    let json_violations: Vec<JsonViolation> = violations
        .iter()
        .map(|vv| JsonViolation {
            path: vv.path.to_string_lossy().into_owned(),
            language: vv.language.clone(),
            lines: vv.lines,
            limit: vv.limit,
        })
        .collect();
    serde_json::to_writer_pretty(&mut *writer, &json_violations)?;
    writeln!(writer)?;
    Ok(())
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)]
mod tests {
    use super::{Format, print};
    use crate::checker::Violation;
    use std::path::PathBuf;

    fn make_violation(path: &str, lang: &str, lines: u64, limit: u64) -> Violation {
        Violation {
            path: PathBuf::from(path),
            language: lang.to_owned(),
            lines,
            limit,
        }
    }

    #[test]
    fn text_format_no_violations() {
        let mut buf = Vec::new();
        print(&mut buf, &[], Format::Text).expect("print");
        let output = String::from_utf8(buf).expect("utf8");
        assert!(output.contains("All files within size limits"));
    }

    #[test]
    fn text_format_with_violations() {
        let violations = vec![
            make_violation("src/big.rs", "Rust", 523, 500),
            make_violation("README.md", "Markdown", 250, 200),
        ];
        let mut buf = Vec::new();
        print(&mut buf, &violations, Format::Text).expect("print");
        let output = String::from_utf8(buf).expect("utf8");
        assert!(output.contains("--- src/big.rs: 523 lines (limit: 500)"));
        assert!(output.contains("--- README.md: 250 lines (limit: 200)"));
        assert!(output.contains("2 file(s) exceed size limits"));
    }

    #[test]
    fn json_format_no_violations() {
        let mut buf = Vec::new();
        print(&mut buf, &[], Format::Json).expect("print");
        let output = String::from_utf8(buf).expect("utf8");
        assert_eq!(output.trim(), "[]");
    }

    #[test]
    fn json_format_with_violations() {
        let violations = vec![make_violation("src/big.rs", "Rust", 523, 500)];
        let mut buf = Vec::new();
        print(&mut buf, &violations, Format::Json).expect("print");
        let output = String::from_utf8(buf).expect("utf8");
        let parsed: serde_json::Value = serde_json::from_str(&output).expect("valid json");
        let arr = parsed.as_array().expect("array");
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["lines"], 523);
        assert_eq!(arr[0]["limit"], 500);
    }
}
