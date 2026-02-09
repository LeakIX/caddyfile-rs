//! Caddyfile lexer, parser, formatter, and builder.
//!
//! A typed AST for Caddy's configuration file format with tools
//! to parse Caddyfiles from text, build them programmatically,
//! and format them back to valid syntax.
//!
//! # Quick start
//!
//! ## Parse and re-format a Caddyfile
//!
//! ```
//! use caddyfile_rs::{tokenize, parse, format};
//!
//! let input = "example.com {\n\treverse_proxy app:3000\n}\n";
//! let tokens = tokenize(input).unwrap();
//! let caddyfile = parse(&tokens).unwrap();
//! let output = format(&caddyfile);
//! assert_eq!(output, input);
//! ```
//!
//! ## Build a Caddyfile programmatically
//!
//! ```
//! use caddyfile_rs::{Caddyfile, SiteBlock, Directive, format};
//!
//! let cf = Caddyfile::new()
//!     .site(SiteBlock::new("example.com")
//!         .reverse_proxy("app:3000")
//!         .encode_gzip()
//!         .log());
//!
//! let output = format(&cf);
//! assert!(output.contains("reverse_proxy app:3000"));
//! ```

// Allow noisy pedantic lints that don't add value for
// a library crate.
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::module_name_repetitions
)]

pub mod ast;
pub mod builder;
pub mod formatter;
pub mod lexer;
pub mod parser;
pub mod token;

pub use ast::{
    Address, Argument, Caddyfile, Directive, GlobalOptions, Matcher, NamedRoute, Scheme, SiteBlock,
    Snippet,
};
pub use formatter::format;
pub use lexer::{LexError, LexErrorKind, tokenize};
pub use parser::{ParseError, ParseErrorKind, parse};
pub use token::{Span, Token, TokenKind};

/// Unified error type covering both lexing and parsing.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// A lexer error.
    #[error("{0}")]
    Lex(#[from] LexError),
    /// A parser error.
    #[error("{0}")]
    Parse(#[from] ParseError),
}

/// Tokenize and parse a Caddyfile source string in one step.
pub fn parse_str(input: &str) -> Result<Caddyfile, Error> {
    let tokens = tokenize(input)?;
    Ok(parse(&tokens)?)
}
