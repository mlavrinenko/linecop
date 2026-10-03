//! kvconf: `key = value` configuration files.

pub mod parser;
pub mod tables;

pub use parser::{Entry, ParseError, parse};
