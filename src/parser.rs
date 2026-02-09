use std::fmt;

use crate::ast::{
    self, Argument, Caddyfile, Directive, GlobalOptions, Matcher, NamedRoute, SiteBlock, Snippet,
};
use crate::token::{Span, Token, TokenKind};

/// Classifies a parser error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseErrorKind {
    /// Expected `{`, found something else or EOF.
    ExpectedOpenBrace { found: Option<String> },
    /// Expected `}`, found something else or EOF.
    ExpectedCloseBrace { found: Option<String> },
}

impl fmt::Display for ParseErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ExpectedOpenBrace { found: None } => {
                write!(f, "expected '{{'")
            }
            Self::ExpectedOpenBrace { found: Some(t) } => {
                write!(f, "expected '{{', got '{t}'")
            }
            Self::ExpectedCloseBrace { found: None } => {
                write!(f, "expected '}}'")
            }
            Self::ExpectedCloseBrace { found: Some(t) } => {
                write!(f, "expected '}}', got '{t}'")
            }
        }
    }
}

/// Error produced during parsing.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{kind} at line {}, column {}", span.line, span.column)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub span: Span,
}

/// Parse a token stream into a `Caddyfile` AST.
///
/// # Errors
///
/// Returns `ParseError` on syntax errors such as unclosed
/// braces, unexpected tokens, or invalid structure.
pub fn parse(tokens: &[Token]) -> Result<Caddyfile, ParseError> {
    Parser::new(tokens).parse()
}

struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
}

impl<'a> Parser<'a> {
    const fn new(tokens: &'a [Token]) -> Self {
        Self { tokens, pos: 0 }
    }

    fn parse(mut self) -> Result<Caddyfile, ParseError> {
        let mut caddyfile = Caddyfile {
            global_options: None,
            snippets: Vec::new(),
            named_routes: Vec::new(),
            sites: Vec::new(),
        };

        self.skip_newlines_and_comments();

        // Check for global options block: { at start
        // (no addresses before it)
        if self.is_global_options_block() {
            caddyfile.global_options = Some(self.parse_global_options()?);
            self.skip_newlines_and_comments();
        }

        // Parse remaining blocks
        while self.pos < self.tokens.len() {
            self.skip_newlines_and_comments();
            if self.pos >= self.tokens.len() {
                break;
            }

            let token = &self.tokens[self.pos];

            // Snippet: (name) { ... }
            if token.text.starts_with('(') && token.text.ends_with(')') && token.text.len() > 2 {
                caddyfile.snippets.push(self.parse_snippet()?);
            }
            // Named route: &(name) { ... }
            else if token.text.starts_with("&(")
                && token.text.ends_with(')')
                && token.text.len() > 3
            {
                caddyfile.named_routes.push(self.parse_named_route()?);
            }
            // Site block
            else {
                caddyfile.sites.push(self.parse_site_block()?);
            }
        }

        Ok(caddyfile)
    }

    fn is_global_options_block(&self) -> bool {
        // Global options: first non-whitespace token is {
        self.pos < self.tokens.len() && self.tokens[self.pos].kind == TokenKind::OpenBrace
    }

    fn parse_global_options(&mut self) -> Result<GlobalOptions, ParseError> {
        self.expect_open_brace()?;
        let directives = self.parse_directives()?;
        self.expect_close_brace()?;
        Ok(GlobalOptions { directives })
    }

    fn parse_snippet(&mut self) -> Result<Snippet, ParseError> {
        let token = &self.tokens[self.pos];
        let name = token.text[1..token.text.len() - 1].to_string();
        self.pos += 1;
        self.skip_whitespace_tokens();
        self.expect_open_brace()?;
        let directives = self.parse_directives()?;
        self.expect_close_brace()?;
        Ok(Snippet { name, directives })
    }

    fn parse_named_route(&mut self) -> Result<NamedRoute, ParseError> {
        let token = &self.tokens[self.pos];
        let name = token.text[2..token.text.len() - 1].to_string();
        self.pos += 1;
        self.skip_whitespace_tokens();
        self.expect_open_brace()?;
        let directives = self.parse_directives()?;
        self.expect_close_brace()?;
        Ok(NamedRoute { name, directives })
    }

    fn parse_site_block(&mut self) -> Result<SiteBlock, ParseError> {
        let mut addresses = Vec::new();

        // Collect addresses until we hit {
        while self.pos < self.tokens.len() {
            let token = &self.tokens[self.pos];
            match &token.kind {
                TokenKind::OpenBrace => break,
                TokenKind::Newline => {
                    self.pos += 1;
                    // If next non-whitespace is { on same
                    // logical line, it's part of
                    // this site block
                    break;
                }
                TokenKind::Comment => {
                    self.pos += 1;
                }
                _ => {
                    // Handle comma-separated addresses
                    let text = token.text.trim_end_matches(',');
                    addresses.push(ast::parse_address(text));
                    self.pos += 1;
                }
            }
        }

        self.skip_newlines_and_comments();

        // Site block may be a single-line (no braces)
        if self.pos >= self.tokens.len() || self.tokens[self.pos].kind != TokenKind::OpenBrace {
            return Ok(SiteBlock {
                addresses,
                directives: Vec::new(),
            });
        }

        self.expect_open_brace()?;
        let directives = self.parse_directives()?;
        self.expect_close_brace()?;

        Ok(SiteBlock {
            addresses,
            directives,
        })
    }

    fn parse_directives(&mut self) -> Result<Vec<Directive>, ParseError> {
        let mut directives = Vec::new();

        loop {
            self.skip_newlines_and_comments();

            if self.pos >= self.tokens.len() {
                break;
            }

            // End of block
            if self.tokens[self.pos].kind == TokenKind::CloseBrace {
                break;
            }

            directives.push(self.parse_directive()?);
        }

        Ok(directives)
    }

    fn parse_directive(&mut self) -> Result<Directive, ParseError> {
        let name = self.tokens[self.pos].text.clone();
        self.pos += 1;

        // Check for matcher
        let matcher = self.try_parse_matcher();

        // Collect arguments until newline or {
        let mut arguments = Vec::new();
        while self.pos < self.tokens.len() {
            let tok = &self.tokens[self.pos];
            match &tok.kind {
                TokenKind::Newline => {
                    self.pos += 1;
                    break;
                }
                TokenKind::OpenBrace | TokenKind::CloseBrace => break,
                TokenKind::Comment => {
                    self.pos += 1;
                }
                _ => {
                    arguments.push(Self::token_to_argument(tok));
                    self.pos += 1;
                }
            }
        }

        // Check for sub-block
        let block =
            if self.pos < self.tokens.len() && self.tokens[self.pos].kind == TokenKind::OpenBrace {
                self.pos += 1; // skip {
                let sub = self.parse_directives()?;
                self.expect_close_brace()?;
                Some(sub)
            } else {
                None
            };

        Ok(Directive {
            name,
            matcher,
            arguments,
            block,
        })
    }

    fn try_parse_matcher(&mut self) -> Option<Matcher> {
        if self.pos >= self.tokens.len() {
            return None;
        }

        let tok = &self.tokens[self.pos];
        match &tok.kind {
            TokenKind::Newline
            | TokenKind::OpenBrace
            | TokenKind::CloseBrace
            | TokenKind::Comment => None,
            _ => {
                if tok.text == "*" {
                    self.pos += 1;
                    Some(Matcher::All)
                } else if tok.text.starts_with('@') {
                    let name = tok.text[1..].to_string();
                    self.pos += 1;
                    Some(Matcher::Named(name))
                } else if tok.text.starts_with('/') {
                    let path = tok.text.clone();
                    self.pos += 1;
                    Some(Matcher::Path(path))
                } else {
                    None
                }
            }
        }
    }

    fn token_to_argument(token: &Token) -> Argument {
        match &token.kind {
            TokenKind::QuotedString => Argument::Quoted(token.text.clone()),
            TokenKind::BacktickString => Argument::Backtick(token.text.clone()),
            TokenKind::Heredoc { marker } => Argument::Heredoc {
                marker: marker.clone(),
                content: token.text.clone(),
            },
            _ => Argument::Unquoted(token.text.clone()),
        }
    }

    fn skip_newlines_and_comments(&mut self) {
        while self.pos < self.tokens.len() {
            match self.tokens[self.pos].kind {
                TokenKind::Newline | TokenKind::Comment => {
                    self.pos += 1;
                }
                _ => break,
            }
        }
    }

    fn skip_whitespace_tokens(&mut self) {
        while self.pos < self.tokens.len() {
            if self.tokens[self.pos].kind == TokenKind::Newline {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn expect_open_brace(&mut self) -> Result<(), ParseError> {
        self.skip_newlines_and_comments();
        if self.pos >= self.tokens.len() {
            return Err(ParseError {
                kind: ParseErrorKind::ExpectedOpenBrace { found: None },
                span: self.eof_span(),
            });
        }
        if self.tokens[self.pos].kind != TokenKind::OpenBrace {
            return Err(ParseError {
                kind: ParseErrorKind::ExpectedOpenBrace {
                    found: Some(self.tokens[self.pos].text.clone()),
                },
                span: self.tokens[self.pos].span.clone(),
            });
        }
        self.pos += 1;
        Ok(())
    }

    fn expect_close_brace(&mut self) -> Result<(), ParseError> {
        self.skip_newlines_and_comments();
        if self.pos >= self.tokens.len() {
            return Err(ParseError {
                kind: ParseErrorKind::ExpectedCloseBrace { found: None },
                span: self.eof_span(),
            });
        }
        if self.tokens[self.pos].kind != TokenKind::CloseBrace {
            return Err(ParseError {
                kind: ParseErrorKind::ExpectedCloseBrace {
                    found: Some(self.tokens[self.pos].text.clone()),
                },
                span: self.tokens[self.pos].span.clone(),
            });
        }
        self.pos += 1;
        Ok(())
    }

    fn eof_span(&self) -> Span {
        self.tokens
            .last()
            .map_or(Span { line: 1, column: 1 }, |last| last.span.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Scheme;
    use crate::lexer::tokenize;

    fn parse_input(input: &str) -> Result<Caddyfile, ParseError> {
        let tokens = tokenize(input).expect("tokenize failed");
        parse(&tokens)
    }

    #[test]
    fn simple_site_block() {
        let cf =
            parse_input("example.com {\n    reverse_proxy app:3000\n}\n").expect("parse failed");
        assert_eq!(cf.sites.len(), 1);
        assert_eq!(cf.sites[0].addresses[0].host, "example.com");
        assert_eq!(cf.sites[0].directives.len(), 1);
        assert_eq!(cf.sites[0].directives[0].name, "reverse_proxy");
    }

    #[test]
    fn global_options() {
        let cf = parse_input(
            "{\n    email admin@example.com\n}\n\
             example.com {\n    log\n}\n",
        )
        .expect("parse failed");
        assert!(cf.global_options.is_some());
        let go = cf.global_options.as_ref().unwrap();
        assert_eq!(go.directives[0].name, "email");
        assert_eq!(cf.sites.len(), 1);
    }

    #[test]
    fn snippet() {
        let cf = parse_input(
            "(logging) {\n    log\n}\n\
             example.com {\n    import logging\n}\n",
        )
        .expect("parse failed");
        assert_eq!(cf.snippets.len(), 1);
        assert_eq!(cf.snippets[0].name, "logging");
    }

    #[test]
    fn named_route() {
        let cf =
            parse_input("&(myroute) {\n    reverse_proxy app:3000\n}\n").expect("parse failed");
        assert_eq!(cf.named_routes.len(), 1);
        assert_eq!(cf.named_routes[0].name, "myroute");
    }

    #[test]
    fn directive_with_sub_block() {
        let cf = parse_input(
            "example.com {\n\
             \theader {\n\
             \t\tX-Frame-Options DENY\n\
             \t}\n\
             }\n",
        )
        .expect("parse failed");
        let header = &cf.sites[0].directives[0];
        assert_eq!(header.name, "header");
        assert!(header.block.is_some());
        let sub = header.block.as_ref().unwrap();
        assert_eq!(sub[0].name, "X-Frame-Options");
    }

    #[test]
    fn matcher_all() {
        let cf = parse_input("example.com {\n    respond * 200\n}\n").expect("parse failed");
        assert_eq!(cf.sites[0].directives[0].matcher, Some(Matcher::All));
    }

    #[test]
    fn matcher_path() {
        let cf = parse_input("example.com {\n    respond /health 200\n}\n").expect("parse failed");
        assert_eq!(
            cf.sites[0].directives[0].matcher,
            Some(Matcher::Path("/health".to_string()))
        );
    }

    #[test]
    fn matcher_named() {
        let cf = parse_input(
            "example.com {\n\
             \tbasic_auth @protected {\n\
             \t\tadmin hash\n\
             \t}\n\
             }\n",
        )
        .expect("parse failed");
        assert_eq!(
            cf.sites[0].directives[0].matcher,
            Some(Matcher::Named("protected".to_string()))
        );
    }

    #[test]
    fn address_parsing() {
        let a = ast::parse_address("https://example.com:443/api");
        assert_eq!(a.scheme, Some(Scheme::Https));
        assert_eq!(a.host, "example.com");
        assert_eq!(a.port, Some(443));
        assert_eq!(a.path, Some("/api".to_string()));
    }

    #[test]
    fn address_simple() {
        let a = ast::parse_address("example.com");
        assert_eq!(a.scheme, None);
        assert_eq!(a.host, "example.com");
        assert_eq!(a.port, None);
        assert_eq!(a.path, None);
    }

    #[test]
    fn unclosed_brace() {
        let result = parse_input("example.com {\n    log\n");
        assert!(result.is_err());
    }

    #[test]
    fn multiple_sites() {
        let cf = parse_input("a.com {\n    log\n}\n\nb.com {\n    log\n}\n").expect("parse failed");
        assert_eq!(cf.sites.len(), 2);
    }
}
