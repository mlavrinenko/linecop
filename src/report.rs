use std::io::Write;

use anstyle::{AnsiColor, Style};
use anyhow::Result;
use serde::Serialize;

use crate::checker::Violation;

const STYLE_BOLD: Style = Style::new().bold();
const STYLE_RED: Style = AnsiColor::Red.on_default().bold();
const STYLE_GREEN: Style = AnsiColor::Green.on_default().bold();
const STYLE_YELLOW: Style = AnsiColor::Yellow.on_default();

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
    let reset = anstyle::Reset;
    for vv in violations {
        let over = vv.lines - vv.limit;
        writeln!(
            writer,
            "{STYLE_RED}---{reset} {STYLE_BOLD}{}{reset}: {} lines (limit: {}, {STYLE_YELLOW}+{over} over{reset})",
            vv.path.display(),
            vv.lines,
            vv.limit,
        )?;
    }
    if violations.is_empty() {
        writeln!(writer, "{STYLE_GREEN}All files within size limits.{reset}")?;
    } else {
        let total_over: u64 = violations.iter().map(|vv| vv.lines - vv.limit).sum();
        writeln!(
            writer,
            "\n{STYLE_RED}{} file(s) exceed size limits (+{total_over} lines over).{reset} Consider refactoring.",
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

    /// Strip ANSI escape sequences so tests can check semantic content.
    fn strip_ansi(input: &str) -> String {
        let mut out = String::with_capacity(input.len());
        let mut chars = input.chars();
        while let Some(ch) = chars.next() {
            if ch == '\x1b' {
                // Skip until 'm' (end of SGR sequence)
                for inner in chars.by_ref() {
                    if inner == 'm' {
                        break;
                    }
                }
            } else {
                out.push(ch);
            }
        }
        out
    }

    #[test]
    fn text_format_no_violations() {
        let mut buf = Vec::new();
        print(&mut buf, &[], Format::Text).expect("print");
        let output = strip_ansi(&String::from_utf8(buf).expect("utf8"));
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
        let output = strip_ansi(&String::from_utf8(buf).expect("utf8"));
        assert!(output.contains("--- src/big.rs: 523 lines (limit: 500, +23 over)"));
        assert!(output.contains("--- README.md: 250 lines (limit: 200, +50 over)"));
        assert!(output.contains("2 file(s) exceed size limits (+73 lines over)"));
    }

    #[test]
    fn text_format_includes_ansi_codes() {
        let violations = vec![make_violation("a.rs", "Rust", 10, 5)];
        let mut buf = Vec::new();
        print(&mut buf, &violations, Format::Text).expect("print");
        let raw = String::from_utf8(buf).expect("utf8");
        // Raw output should contain ANSI escape sequences
        assert!(raw.contains("\x1b["));
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
