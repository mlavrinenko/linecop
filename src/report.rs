use std::io::Write;

use anstyle::{AnsiColor, Style};
use anyhow::Result;
use serde::Serialize;

use crate::checker::{Gauge, Violation};

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
    /// Paths only, one per line (for piping to other tools).
    Paths,
}

#[derive(Serialize)]
struct JsonViolation {
    path: String,
    language: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    lines: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<u64>,
    #[serde(rename = "baseline-limit", skip_serializing_if = "Option::is_none")]
    baseline_limit: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_bytes: Option<u64>,
    #[serde(rename = "baseline-max-bytes", skip_serializing_if = "Option::is_none")]
    baseline_max_bytes: Option<u64>,
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
        Format::Paths => print_paths(writer, violations),
    }
}

/// A breached gauge with the words naming its unit and its cap.
struct Breach<'a> {
    gauge: Gauge,
    unit: &'a str,
    cap: &'a str,
}

fn breaches(vv: &Violation) -> impl Iterator<Item = Breach<'static>> {
    let lines = vv.lines.map(|gauge| Breach {
        gauge,
        unit: "lines",
        cap: "limit",
    });
    let bytes = vv.bytes.map(|gauge| Breach {
        gauge,
        unit: "bytes",
        cap: "max_bytes",
    });
    lines
        .into_iter()
        .chain(bytes)
        .filter(|breach| breach.gauge.breached)
}

fn total_over(violations: &[Violation], pick: fn(&Violation) -> Option<Gauge>) -> u64 {
    violations
        .iter()
        .filter_map(pick)
        .map(|gauge| gauge.count.saturating_sub(gauge.limit))
        .sum()
}

fn print_text(writer: &mut dyn Write, violations: &[Violation]) -> Result<()> {
    let reset = anstyle::Reset;
    for vv in violations {
        for Breach { gauge, unit, cap } in breaches(vv) {
            let (count, limit) = (gauge.count, gauge.limit);
            if count > limit {
                let over = count - limit;
                writeln!(
                    writer,
                    "{STYLE_RED}---{reset} {STYLE_BOLD}{}{reset}: {count} {unit} ({cap}: {limit}, {STYLE_YELLOW}+{over} over{reset})",
                    vv.path.display(),
                )?;
            } else {
                let pct = count * 100 / limit;
                writeln!(
                    writer,
                    "{STYLE_YELLOW}~~~{reset} {STYLE_BOLD}{}{reset}: {count} {unit} ({cap}: {limit}, {STYLE_YELLOW}{pct}% of limit{reset})",
                    vv.path.display(),
                )?;
            }
        }
    }
    if violations.is_empty() {
        writeln!(writer, "{STYLE_GREEN}All files within size limits.{reset}")?;
    } else {
        writeln!(
            writer,
            "\n{STYLE_RED}{} file(s) reported.{reset} Consider refactoring.",
            violations.len()
        )?;
        let lines_over = total_over(violations, |vv| vv.lines);
        if lines_over > 0 {
            writeln!(writer, "{STYLE_RED}+{lines_over} lines over limit.{reset}")?;
        }
        let bytes_over = total_over(violations, |vv| vv.bytes);
        if bytes_over > 0 {
            writeln!(writer, "{STYLE_RED}+{bytes_over} bytes over limit.{reset}")?;
        }
    }
    Ok(())
}

fn print_json(writer: &mut dyn Write, violations: &[Violation]) -> Result<()> {
    let json_violations: Vec<JsonViolation> = violations
        .iter()
        .map(|vv| JsonViolation {
            path: vv.path.to_string_lossy().into_owned(),
            language: vv.language.clone(),
            lines: vv.lines.map(|gg| gg.count),
            limit: vv.lines.map(|gg| gg.limit),
            baseline_limit: vv.lines.map(|gg| gg.baseline_limit),
            bytes: vv.bytes.map(|gg| gg.count),
            max_bytes: vv.bytes.map(|gg| gg.limit),
            baseline_max_bytes: vv.bytes.map(|gg| gg.baseline_limit),
        })
        .collect();
    serde_json::to_writer_pretty(&mut *writer, &json_violations)?;
    writeln!(writer)?;
    Ok(())
}

fn print_paths(writer: &mut dyn Write, violations: &[Violation]) -> Result<()> {
    for vv in violations {
        writeln!(writer, "{}", vv.path.display())?;
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)]
mod tests {
    use super::{Format, print};
    use crate::checker::{Gauge, Violation};
    use std::path::PathBuf;

    fn gauge(count: u64, limit: u64, baseline_limit: u64, breached: bool) -> Gauge {
        Gauge {
            count,
            limit,
            baseline_limit,
            breached,
        }
    }

    fn make_violation(path: &str, lang: &str, lines: u64, limit: u64) -> Violation {
        make_baseline_violation(path, lang, lines, limit, limit)
    }

    fn make_baseline_violation(
        path: &str,
        lang: &str,
        lines: u64,
        limit: u64,
        baseline_limit: u64,
    ) -> Violation {
        Violation {
            path: PathBuf::from(path),
            language: lang.to_owned(),
            lines: Some(gauge(lines, limit, baseline_limit, true)),
            bytes: None,
        }
    }

    /// A file within its line limit that breaches its byte cap.
    fn make_bytes_violation(path: &str, bytes: u64, max_bytes: u64) -> Violation {
        Violation {
            path: PathBuf::from(path),
            language: "Markdown".to_owned(),
            lines: Some(gauge(128, 142, 142, false)),
            bytes: Some(gauge(bytes, max_bytes, max_bytes, true)),
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
        assert!(output.contains("2 file(s) reported"));
        assert!(output.contains("+73 lines over limit"));
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
        assert_eq!(arr[0]["baseline-limit"], 500);
    }

    #[test]
    fn json_format_baseline_violation() {
        let violations = vec![make_baseline_violation(
            "src/near.rs",
            "Rust",
            480,
            500,
            450,
        )];
        let mut buf = Vec::new();
        print(&mut buf, &violations, Format::Json).expect("print");
        let output = String::from_utf8(buf).expect("utf8");
        let parsed: serde_json::Value = serde_json::from_str(&output).expect("valid json");
        let arr = parsed.as_array().expect("array");
        assert_eq!(arr[0]["baseline-limit"], 450);
    }

    #[test]
    fn paths_format_no_violations() {
        let mut buf = Vec::new();
        print(&mut buf, &[], Format::Paths).expect("print");
        let output = String::from_utf8(buf).expect("utf8");
        assert_eq!(output.trim(), "");
    }

    #[test]
    fn paths_format_with_violations() {
        let violations = vec![
            make_violation("src/query.rs", "Rust", 523, 500),
            make_violation("src/cli/render.rs", "Rust", 250, 200),
        ];
        let mut buf = Vec::new();
        print(&mut buf, &violations, Format::Paths).expect("print");
        let output = String::from_utf8(buf).expect("utf8");
        assert_eq!(output, "src/query.rs\nsrc/cli/render.rs\n");
    }

    #[test]
    fn text_format_baseline_violation_shows_percentage() {
        let violations = vec![make_baseline_violation(
            "src/near.rs",
            "Rust",
            480,
            500,
            450,
        )];
        let mut buf = Vec::new();
        print(&mut buf, &violations, Format::Text).expect("print");
        let output = strip_ansi(&String::from_utf8(buf).expect("utf8"));
        assert!(output.contains("~~~ src/near.rs: 480 lines (limit: 500, 96% of limit)"));
    }

    #[test]
    fn text_format_names_the_breached_byte_cap() {
        let violations = vec![make_bytes_violation("CONTRIBUTING.md", 31204, 14000)];
        let mut buf = Vec::new();
        print(&mut buf, &violations, Format::Text).expect("print");
        let output = strip_ansi(&String::from_utf8(buf).expect("utf8"));
        assert!(
            output.contains("--- CONTRIBUTING.md: 31204 bytes (max_bytes: 14000, +17204 over)")
        );
        assert!(
            !output.contains("128 lines"),
            "the unbreached line limit stays quiet"
        );
        assert!(output.contains("+17204 bytes over limit"));
        assert!(!output.contains("lines over limit"));
    }

    #[test]
    fn json_format_carries_bytes_for_capped_file() {
        let violations = vec![
            make_bytes_violation("CONTRIBUTING.md", 31204, 14000),
            make_violation("src/big.rs", "Rust", 523, 500),
        ];
        let mut buf = Vec::new();
        print(&mut buf, &violations, Format::Json).expect("print");
        let parsed: serde_json::Value = serde_json::from_slice(&buf).expect("valid json");
        assert_eq!(parsed[0]["lines"], 128);
        assert_eq!(parsed[0]["bytes"], 31204);
        assert_eq!(parsed[0]["max_bytes"], 14000);
        assert_eq!(parsed[0]["baseline-max-bytes"], 14000);
        assert!(parsed[1].get("bytes").is_none(), "no key, no bytes");
        assert!(parsed[1].get("max_bytes").is_none());
    }
}
