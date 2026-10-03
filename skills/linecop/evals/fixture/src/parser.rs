//! Line-oriented parser for `key = value` files.
//!
//! Blank lines and `#` comments are skipped. A value may be quoted to keep
//! leading or trailing spaces.

use std::fmt;

/// One `key = value` pair and the line it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub key: String,
    pub value: String,
    pub line: usize,
}

/// Why a line could not be parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// The line has no `=`.
    MissingEquals { line: usize },
    /// The key before `=` is empty.
    EmptyKey { line: usize },
    /// A quoted value never closes.
    UnclosedQuote { line: usize },
}

impl fmt::Display for ParseError {
    fn fmt(&self, ff: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingEquals { line } => write!(ff, "line {line}: expected `=`"),
            Self::EmptyKey { line } => write!(ff, "line {line}: empty key"),
            Self::UnclosedQuote { line } => write!(ff, "line {line}: unclosed quote"),
        }
    }
}

impl std::error::Error for ParseError {}

/// Parses a whole file, stopping at the first bad line.
pub fn parse(text: &str) -> Result<Vec<Entry>, ParseError> {
    let mut entries = Vec::new();
    for (idx, raw) in text.lines().enumerate() {
        let line = idx + 1;
        let trimmed = raw.trim();
        // Comments and blank lines carry no entry.
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        entries.push(parse_line(trimmed, line)?);
    }
    Ok(entries)
}

fn parse_line(trimmed: &str, line: usize) -> Result<Entry, ParseError> {
    let (key, value) = trimmed
        .split_once('=')
        .ok_or(ParseError::MissingEquals { line })?;
    let key = key.trim();
    if key.is_empty() {
        return Err(ParseError::EmptyKey { line });
    }
    let value = unquote(value.trim(), line)?;
    Ok(Entry {
        key: key.to_owned(),
        value,
        line,
    })
}

fn unquote(value: &str, line: usize) -> Result<String, ParseError> {
    match value.strip_prefix('"') {
        // A quoted value keeps its inner spaces verbatim.
        Some(rest) => rest
            .strip_suffix('"')
            .map(str::to_owned)
            .ok_or(ParseError::UnclosedQuote { line }),
        None => Ok(value.to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn skips_comments_and_blanks() {
        let entries = parse("# header\n\nname = kv\n").expect("parses");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].line, 3);
    }

    #[test]
    fn quoted_value_keeps_spaces() {
        let entries = parse("pad = \"  x  \"\n").expect("parses");
        assert_eq!(entries[0].value, "  x  ");
    }
}
