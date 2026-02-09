/// Source location for error reporting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub file: Option<String>,
    pub line: usize,
    pub column: usize,
}

/// Token kinds produced by the lexer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    /// Unquoted word.
    Word,
    /// Double-quoted string (`"..."`).
    QuotedString,
    /// Backtick-quoted string (`` `...` ``).
    BacktickString,
    /// Heredoc (`<<MARKER ... MARKER`).
    Heredoc { marker: String },
    /// Comment (`# ...`).
    Comment,
    /// Opening brace `{`.
    OpenBrace,
    /// Closing brace `}`.
    CloseBrace,
    /// Newline (line separator).
    Newline,
    /// Environment variable `{$VAR}` or `{$VAR:default}`.
    EnvVar {
        name: String,
        default: Option<String>,
    },
}

/// A single token with its kind, text, and source location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub text: String,
    pub span: Span,
}
