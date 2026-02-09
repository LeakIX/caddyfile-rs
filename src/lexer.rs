use std::fmt;

use crate::token::{Span, Token, TokenKind};

/// Classifies a lexer error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LexErrorKind {
    /// Unterminated double-quoted string.
    UnterminatedString,
    /// Unterminated backtick string.
    UnterminatedBacktick,
    /// Unterminated heredoc (closing marker never found).
    UnterminatedHeredoc { marker: String },
    /// Heredoc marker is empty (`<<` followed by whitespace).
    EmptyHeredocMarker,
    /// Byte that cannot start any token.
    UnexpectedCharacter(char),
}

impl fmt::Display for LexErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnterminatedString => {
                write!(f, "unterminated quoted string")
            }
            Self::UnterminatedBacktick => {
                write!(f, "unterminated backtick string")
            }
            Self::UnterminatedHeredoc { marker } => {
                write!(
                    f,
                    "unterminated heredoc, \
                     expected closing marker: {marker}"
                )
            }
            Self::EmptyHeredocMarker => {
                write!(f, "empty heredoc marker")
            }
            Self::UnexpectedCharacter(ch) => {
                write!(f, "unexpected character: {ch}")
            }
        }
    }
}

/// Error produced during lexing.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{kind} at line {}, column {}", span.line, span.column)]
pub struct LexError {
    pub kind: LexErrorKind,
    pub span: Span,
}

/// Tokenize a Caddyfile source string into a sequence of tokens.
///
/// # Errors
///
/// Returns `LexError` on unterminated strings, invalid heredocs,
/// or other lexical errors.
pub fn tokenize(input: &str) -> Result<Vec<Token>, LexError> {
    Lexer::new(input).tokenize()
}

struct Lexer<'a> {
    input: &'a [u8],
    pos: usize,
    line: usize,
    col: usize,
}

impl<'a> Lexer<'a> {
    fn new(input: &'a str) -> Self {
        let bytes = input.as_bytes();
        let start = if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
            3
        } else {
            0
        };
        Self {
            input: bytes,
            pos: start,
            line: 1,
            col: 1,
        }
    }

    fn tokenize(mut self) -> Result<Vec<Token>, LexError> {
        let mut tokens = Vec::new();

        while self.pos < self.input.len() {
            let ch = self.input[self.pos];

            match ch {
                b'\n' => {
                    tokens.push(self.make_token(TokenKind::Newline, "\n".to_string()));
                    self.advance();
                }
                b'\r' => {
                    self.advance();
                    if self.peek() == Some(b'\n') {
                        self.advance();
                    }
                    tokens.push(Self::make_token_at(
                        TokenKind::Newline,
                        "\n".to_string(),
                        self.line - 1,
                        self.col,
                    ));
                }
                b' ' | b'\t' => {
                    self.advance();
                }
                b'#' => {
                    tokens.push(self.read_comment());
                }
                b'{' => {
                    if self.try_read_env_var(&mut tokens) {
                        // consumed as env var
                    } else {
                        tokens.push(self.make_token(TokenKind::OpenBrace, "{".to_string()));
                        self.advance();
                    }
                }
                b'}' => {
                    tokens.push(self.make_token(TokenKind::CloseBrace, "}".to_string()));
                    self.advance();
                }
                b'"' => {
                    tokens.push(self.read_quoted_string()?);
                }
                b'`' => {
                    tokens.push(self.read_backtick_string()?);
                }
                b'\\' if self.peek_at(1) == Some(b'\n') => {
                    // line continuation
                    self.advance(); // skip backslash
                    self.advance(); // skip newline
                }
                b'\\' if self.peek_at(1) == Some(b'\r') => {
                    self.advance();
                    self.advance();
                    if self.peek() == Some(b'\n') {
                        self.advance();
                    }
                }
                _ => {
                    tokens.push(self.read_word()?);
                }
            }
        }

        Ok(tokens)
    }

    const fn span(&self) -> Span {
        Span {
            line: self.line,
            column: self.col,
        }
    }

    const fn make_token(&self, kind: TokenKind, text: String) -> Token {
        Token {
            kind,
            text,
            span: self.span(),
        }
    }

    const fn make_token_at(kind: TokenKind, text: String, line: usize, col: usize) -> Token {
        Token {
            kind,
            text,
            span: Span { line, column: col },
        }
    }

    fn peek(&self) -> Option<u8> {
        self.input.get(self.pos).copied()
    }

    fn peek_at(&self, offset: usize) -> Option<u8> {
        self.input.get(self.pos + offset).copied()
    }

    fn advance(&mut self) {
        if self.pos < self.input.len() {
            if self.input[self.pos] == b'\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
            self.pos += 1;
        }
    }

    fn read_comment(&mut self) -> Token {
        let start_line = self.line;
        let start_col = self.col;
        let start = self.pos;

        while self.pos < self.input.len() && self.input[self.pos] != b'\n' {
            self.pos += 1;
            self.col += 1;
        }

        let text = String::from_utf8_lossy(&self.input[start..self.pos]).into_owned();

        Token {
            kind: TokenKind::Comment,
            text,
            span: Span {
                line: start_line,
                column: start_col,
            },
        }
    }

    fn read_quoted_string(&mut self) -> Result<Token, LexError> {
        let start_line = self.line;
        let start_col = self.col;
        self.advance(); // skip opening quote

        let mut value = String::new();
        loop {
            match self.peek() {
                None => {
                    return Err(LexError {
                        kind: LexErrorKind::UnterminatedString,
                        span: Span {
                            line: start_line,
                            column: start_col,
                        },
                    });
                }
                Some(b'\\') => {
                    self.advance();
                    match self.peek() {
                        Some(b'n') => {
                            value.push('\n');
                            self.advance();
                        }
                        Some(b't') => {
                            value.push('\t');
                            self.advance();
                        }
                        Some(b'r') => {
                            value.push('\r');
                            self.advance();
                        }
                        Some(b'"') => {
                            value.push('"');
                            self.advance();
                        }
                        Some(b'\\') => {
                            value.push('\\');
                            self.advance();
                        }
                        Some(c) => {
                            value.push('\\');
                            value.push(char::from(c));
                            self.advance();
                        }
                        None => {
                            value.push('\\');
                        }
                    }
                }
                Some(b'"') => {
                    self.advance();
                    break;
                }
                Some(c) => {
                    if c == b'\n' {
                        // track newlines inside strings
                        self.advance();
                        value.push('\n');
                    } else {
                        value.push(char::from(c));
                        self.advance();
                    }
                }
            }
        }

        Ok(Token {
            kind: TokenKind::QuotedString,
            text: value,
            span: Span {
                line: start_line,
                column: start_col,
            },
        })
    }

    fn read_backtick_string(&mut self) -> Result<Token, LexError> {
        let start_line = self.line;
        let start_col = self.col;
        self.advance(); // skip opening backtick

        let mut value = String::new();
        loop {
            match self.peek() {
                None => {
                    return Err(LexError {
                        kind: LexErrorKind::UnterminatedBacktick,
                        span: Span {
                            line: start_line,
                            column: start_col,
                        },
                    });
                }
                Some(b'`') => {
                    self.advance();
                    break;
                }
                Some(c) => {
                    if c == b'\n' {
                        self.advance();
                        value.push('\n');
                    } else {
                        value.push(char::from(c));
                        self.advance();
                    }
                }
            }
        }

        Ok(Token {
            kind: TokenKind::BacktickString,
            text: value,
            span: Span {
                line: start_line,
                column: start_col,
            },
        })
    }

    fn try_read_env_var(&mut self, tokens: &mut Vec<Token>) -> bool {
        // Check for {$ pattern
        if self.peek_at(1) != Some(b'$') {
            return false;
        }

        let start_line = self.line;
        let start_col = self.col;
        let save_pos = self.pos;
        let save_line = self.line;
        let save_col = self.col;

        self.advance(); // skip {
        self.advance(); // skip $

        let name_start = self.pos;
        while self.pos < self.input.len()
            && self.input[self.pos] != b'}'
            && self.input[self.pos] != b':'
            && self.input[self.pos] != b'\n'
        {
            self.pos += 1;
            self.col += 1;
        }
        let name = String::from_utf8_lossy(&self.input[name_start..self.pos]).into_owned();

        let default = if self.peek() == Some(b':') {
            self.pos += 1;
            self.col += 1;
            let def_start = self.pos;
            while self.pos < self.input.len()
                && self.input[self.pos] != b'}'
                && self.input[self.pos] != b'\n'
            {
                self.pos += 1;
                self.col += 1;
            }
            Some(String::from_utf8_lossy(&self.input[def_start..self.pos]).into_owned())
        } else {
            None
        };

        if self.peek() != Some(b'}') {
            // Not a valid env var, restore position
            self.pos = save_pos;
            self.line = save_line;
            self.col = save_col;
            return false;
        }

        self.pos += 1;
        self.col += 1;

        let text = String::from_utf8_lossy(&self.input[save_pos..self.pos]).into_owned();

        tokens.push(Token {
            kind: TokenKind::EnvVar { name, default },
            text,
            span: Span {
                line: start_line,
                column: start_col,
            },
        });

        true
    }

    fn read_word(&mut self) -> Result<Token, LexError> {
        let start_line = self.line;
        let start_col = self.col;
        let start = self.pos;

        // Check for heredoc marker
        if self.input[self.pos] == b'<' && self.peek_at(1) == Some(b'<') {
            return self.read_heredoc(start_line, start_col);
        }

        while self.pos < self.input.len() {
            let ch = self.input[self.pos];
            match ch {
                b' ' | b'\t' | b'\n' | b'\r' => break,
                b'{' | b'}' => {
                    // check for {$ env var or placeholder
                    if ch == b'{' && self.peek_at(1) == Some(b'$') {
                        break;
                    }
                    // standalone brace at start means it's
                    // a brace token, not part of a word
                    if self.pos == start {
                        break;
                    }
                    // otherwise it could be a placeholder like
                    // {path} inside a word - consume it
                    self.pos += 1;
                    self.col += 1;
                }
                b'\\' => {
                    // escaped character
                    self.pos += 1;
                    self.col += 1;
                    if self.pos < self.input.len() {
                        self.pos += 1;
                        self.col += 1;
                    }
                }
                _ => {
                    self.pos += 1;
                    self.col += 1;
                }
            }
        }

        let text = String::from_utf8_lossy(&self.input[start..self.pos]).into_owned();

        if text.is_empty() {
            return Err(LexError {
                kind: LexErrorKind::UnexpectedCharacter(char::from(self.input[start])),
                span: Span {
                    line: start_line,
                    column: start_col,
                },
            });
        }

        Ok(Token {
            kind: TokenKind::Word,
            text,
            span: Span {
                line: start_line,
                column: start_col,
            },
        })
    }

    fn read_heredoc(&mut self, start_line: usize, start_col: usize) -> Result<Token, LexError> {
        self.advance(); // skip first <
        self.advance(); // skip second <

        // Read marker
        let marker_start = self.pos;
        while self.pos < self.input.len()
            && self.input[self.pos] != b'\n'
            && self.input[self.pos] != b'\r'
            && self.input[self.pos] != b' '
            && self.input[self.pos] != b'\t'
        {
            self.pos += 1;
            self.col += 1;
        }

        let marker = String::from_utf8_lossy(&self.input[marker_start..self.pos]).into_owned();

        if marker.is_empty() {
            return Err(LexError {
                kind: LexErrorKind::EmptyHeredocMarker,
                span: Span {
                    line: start_line,
                    column: start_col,
                },
            });
        }

        // Skip to next line
        if self.peek() == Some(b'\r') {
            self.advance();
        }
        if self.peek() == Some(b'\n') {
            self.advance();
        }

        // Read content until marker on its own line
        let content_start = self.pos;

        while self.pos < self.input.len() {
            let line_start = self.pos;
            // read one line
            while self.pos < self.input.len() && self.input[self.pos] != b'\n' {
                self.pos += 1;
                self.col += 1;
            }

            let line = String::from_utf8_lossy(&self.input[line_start..self.pos]);
            let trimmed = line.trim();

            if trimmed == marker {
                let content =
                    String::from_utf8_lossy(&self.input[content_start..line_start]).into_owned();
                // Remove trailing newline from content
                let content = content
                    .strip_suffix('\n')
                    .or_else(|| content.strip_suffix("\r\n"))
                    .unwrap_or(&content)
                    .to_string();

                if self.peek() == Some(b'\n') {
                    self.advance();
                }

                return Ok(Token {
                    kind: TokenKind::Heredoc { marker },
                    text: content,
                    span: Span {
                        line: start_line,
                        column: start_col,
                    },
                });
            }

            if self.peek() == Some(b'\n') {
                self.advance();
            }
        }

        Err(LexError {
            kind: LexErrorKind::UnterminatedHeredoc { marker },
            span: Span {
                line: start_line,
                column: start_col,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_words() {
        let tokens = tokenize("reverse_proxy app:3000").expect("should tokenize");
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].text, "reverse_proxy");
        assert_eq!(tokens[1].text, "app:3000");
    }

    #[test]
    fn braces_and_newlines() {
        let tokens = tokenize("example.com {\n    log\n}\n").expect("should tokenize");
        let kinds: Vec<_> = tokens.iter().map(|t| &t.kind).collect();
        assert!(matches!(kinds[0], TokenKind::Word));
        assert!(matches!(kinds[1], TokenKind::OpenBrace));
        assert!(matches!(kinds[2], TokenKind::Newline));
        assert!(matches!(kinds[3], TokenKind::Word));
        assert!(matches!(kinds[4], TokenKind::Newline));
        assert!(matches!(kinds[5], TokenKind::CloseBrace));
    }

    #[test]
    fn quoted_string() {
        let tokens = tokenize(r#"header "X-Frame-Options" "DENY""#).expect("should tokenize");
        assert_eq!(tokens.len(), 3);
        assert!(matches!(tokens[1].kind, TokenKind::QuotedString));
        assert_eq!(tokens[1].text, "X-Frame-Options");
        assert_eq!(tokens[2].text, "DENY");
    }

    #[test]
    fn quoted_string_with_escapes() {
        let tokens = tokenize(r#""hello \"world\"""#).expect("should tokenize");
        assert_eq!(tokens[0].text, r#"hello "world""#);
    }

    #[test]
    fn backtick_string() {
        let tokens = tokenize("`raw string`").expect("should tokenize");
        assert!(matches!(tokens[0].kind, TokenKind::BacktickString));
        assert_eq!(tokens[0].text, "raw string");
    }

    #[test]
    fn comment() {
        let tokens = tokenize("log # access log\nfile_server").expect("should tokenize");
        assert_eq!(tokens[1].kind, TokenKind::Comment);
        assert_eq!(tokens[1].text, "# access log");
    }

    #[test]
    fn env_var() {
        let tokens = tokenize("{$API_KEY}").expect("should tokenize");
        assert!(matches!(
            &tokens[0].kind,
            TokenKind::EnvVar { name, default: None }
            if name == "API_KEY"
        ));
    }

    #[test]
    fn env_var_with_default() {
        let tokens = tokenize("{$PORT:8080}").expect("should tokenize");
        assert!(matches!(
            &tokens[0].kind,
            TokenKind::EnvVar {
                name,
                default: Some(def)
            }
            if name == "PORT" && def == "8080"
        ));
    }

    #[test]
    fn heredoc() {
        let input = "respond <<EOF\nHello World\nEOF\n";
        let tokens = tokenize(input).expect("should tokenize");
        assert_eq!(tokens[0].text, "respond");
        assert!(matches!(
            &tokens[1].kind,
            TokenKind::Heredoc { marker }
            if marker == "EOF"
        ));
        assert_eq!(tokens[1].text, "Hello World");
    }

    #[test]
    fn unterminated_quote() {
        let result = tokenize("\"unclosed");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind, LexErrorKind::UnterminatedString);
    }

    #[test]
    fn line_continuation() {
        let tokens = tokenize("reverse_proxy \\\napp:3000").expect("should tokenize");
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].text, "reverse_proxy");
        assert_eq!(tokens[1].text, "app:3000");
    }

    #[test]
    fn bom_stripping() {
        let input = "\u{FEFF}example.com";
        let tokens = tokenize(input).expect("should tokenize");
        assert_eq!(tokens[0].text, "example.com");
    }

    #[test]
    fn escaped_braces() {
        let tokens = tokenize(r"respond \{hello\}").expect("should tokenize");
        assert_eq!(tokens[0].text, "respond");
        assert_eq!(tokens[1].text, r"\{hello\}");
    }

    #[test]
    fn span_tracking() {
        let tokens = tokenize("a\nb c").expect("should tokenize");
        assert_eq!(tokens[0].span.line, 1);
        assert_eq!(tokens[0].span.column, 1);
        // newline token
        assert_eq!(tokens[2].span.line, 2);
        assert_eq!(tokens[2].span.column, 1);
        assert_eq!(tokens[3].span.line, 2);
        assert_eq!(tokens[3].span.column, 3);
    }
}
