//! End-to-end tests ported from the Go reference implementation
//! at `github.com/caddyserver/caddy/caddyconfig/caddyfile/`.

use caddyfile_rs::{
    Caddyfile, Directive, GlobalOptions, LexErrorKind, Matcher, ParseErrorKind, SiteBlock, format,
    parse, parse_str, tokenize,
};

// -----------------------------------------------------------
// Round-trip tests: parse then format should produce the
// same normalised output.
// -----------------------------------------------------------

fn roundtrip(input: &str) {
    let tokens = tokenize(input).expect("tokenize failed");
    let cf = parse(&tokens).expect("parse failed");
    let output = format(&cf);
    assert_eq!(
        output, input,
        "round-trip mismatch:\n--- expected ---\n{input}\n--- got ---\n{output}"
    );
}

#[test]
fn roundtrip_simple_site() {
    roundtrip("example.com {\n\tlog\n}\n");
}

#[test]
fn roundtrip_multiple_directives() {
    roundtrip("example.com {\n\treverse_proxy app:3000\n\tencode gzip\n\tlog\n}\n");
}

#[test]
fn roundtrip_global_options_and_site() {
    roundtrip("{\n\temail admin@example.com\n}\n\nexample.com {\n\tlog\n}\n");
}

#[test]
fn roundtrip_nested_block() {
    roundtrip("example.com {\n\theader {\n\t\tX-Frame-Options \"DENY\"\n\t}\n}\n");
}

#[test]
fn roundtrip_matcher() {
    roundtrip("example.com {\n\trespond /health 200\n}\n");
}

#[test]
fn roundtrip_multiple_sites() {
    roundtrip("a.com {\n\tlog\n}\n\nb.com {\n\tlog\n}\n");
}

#[test]
fn roundtrip_snippet() {
    roundtrip("(logging) {\n\tlog\n}\n\nexample.com {\n\timport logging\n}\n");
}

#[test]
fn roundtrip_named_route() {
    roundtrip("&(myroute) {\n\treverse_proxy app:3000\n}\n");
}

#[test]
fn roundtrip_address_with_scheme() {
    roundtrip("https://example.com {\n\tlog\n}\n");
}

#[test]
fn roundtrip_address_with_port() {
    roundtrip("example.com:8443 {\n\tlog\n}\n");
}

// -----------------------------------------------------------
// Builder round-trip: build, format, parse, compare.
// -----------------------------------------------------------

#[test]
fn builder_roundtrip_simple() {
    let cf = Caddyfile::new().site(
        SiteBlock::new("example.com")
            .reverse_proxy("app:3000")
            .encode_gzip()
            .log(),
    );
    let formatted = format(&cf);
    let tokens = tokenize(&formatted).expect("tokenize");
    let parsed = parse(&tokens).expect("parse");

    assert_eq!(parsed.sites.len(), 1);
    assert_eq!(
        parsed.sites[0].directives.len(),
        cf.sites[0].directives.len()
    );
}

#[test]
fn builder_roundtrip_global_options() {
    let cf = Caddyfile::new()
        .global(GlobalOptions {
            directives: vec![Directive::new("email").arg("admin@example.com")],
        })
        .site(SiteBlock::new("example.com").log());

    let formatted = format(&cf);
    let tokens = tokenize(&formatted).expect("tokenize");
    let parsed = parse(&tokens).expect("parse");

    assert!(parsed.global_options.is_some());
    assert_eq!(parsed.sites.len(), 1);
}

#[test]
fn builder_roundtrip_basic_auth() {
    let cf = Caddyfile::new().site(
        SiteBlock::new("example.com")
            .basic_auth("admin", "$2a$14$hash")
            .reverse_proxy("app:3000"),
    );
    let formatted = format(&cf);
    let tokens = tokenize(&formatted).expect("tokenize");
    let parsed = parse(&tokens).expect("parse");

    assert_eq!(
        parsed.sites[0].directives.len(),
        cf.sites[0].directives.len()
    );
}

#[test]
fn builder_roundtrip_security_headers() {
    let cf = Caddyfile::new().site(SiteBlock::new("example.com").security_headers());
    let formatted = format(&cf);

    assert!(formatted.contains("X-Content-Type-Options"));
    assert!(formatted.contains("X-Frame-Options"));
    assert!(formatted.contains("X-XSS-Protection"));
    assert!(formatted.contains("Referrer-Policy"));
}

// -----------------------------------------------------------
// Lexer edge cases from the Go reference.
// -----------------------------------------------------------

#[test]
fn lex_empty_input() {
    let tokens = tokenize("").expect("tokenize");
    assert!(tokens.is_empty());
}

#[test]
fn lex_only_whitespace() {
    let tokens = tokenize("   \t  \n\n  ").expect("tokenize");
    // Only newlines survive (spaces/tabs are skipped)
    assert!(
        tokens
            .iter()
            .all(|t| matches!(t.kind, caddyfile_rs::TokenKind::Newline))
    );
}

#[test]
fn lex_multiple_comments() {
    let tokens = tokenize("# comment 1\n# comment 2\n").expect("tokenize");
    let comments: Vec<_> = tokens
        .iter()
        .filter(|t| matches!(t.kind, caddyfile_rs::TokenKind::Comment))
        .collect();
    assert_eq!(comments.len(), 2);
}

#[test]
fn lex_quoted_with_newline() {
    let tokens = tokenize("\"line1\\nline2\"").expect("tokenize");
    assert_eq!(tokens[0].text, "line1\nline2");
}

#[test]
fn lex_backtick_preserves_backslash() {
    let tokens = tokenize("`hello\\nworld`").expect("tokenize");
    assert_eq!(tokens[0].text, "hello\\nworld");
}

#[test]
fn lex_env_var_in_context() {
    let tokens = tokenize("bind {$HOST:0.0.0.0}").expect("tokenize");
    assert_eq!(tokens.len(), 2);
    assert!(matches!(
        &tokens[1].kind,
        caddyfile_rs::TokenKind::EnvVar {
            name,
            default: Some(def)
        }
        if name == "HOST" && def == "0.0.0.0"
    ));
}

#[test]
fn lex_heredoc_multiline() {
    let input = "respond <<HTML\n<h1>Hello</h1>\n<p>World</p>\nHTML\n";
    let tokens = tokenize(input).expect("tokenize");
    assert_eq!(tokens[0].text, "respond");
    assert_eq!(tokens[1].text, "<h1>Hello</h1>\n<p>World</p>");
}

// -----------------------------------------------------------
// Parser error cases.
// -----------------------------------------------------------

#[test]
fn parse_error_unclosed_brace() {
    let tokens = tokenize("example.com {\n\tlog\n").expect("tokenize");
    let result = parse(&tokens);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(
        err.kind,
        ParseErrorKind::ExpectedCloseBrace { found: None }
    ));
}

#[test]
fn parse_error_nested_unclosed() {
    // Unclosed nested block should produce an error
    let tokens = tokenize("example.com {\n\theader {\n\t\tX-Test value\n}\n").expect("tokenize");
    let result = parse(&tokens);
    assert!(result.is_err());
}

#[test]
fn lex_error_unterminated_quote() {
    let err = tokenize("\"unclosed string").unwrap_err();
    assert_eq!(err.kind, LexErrorKind::UnterminatedString);
}

#[test]
fn lex_error_unterminated_backtick() {
    let err = tokenize("`unclosed backtick").unwrap_err();
    assert_eq!(err.kind, LexErrorKind::UnterminatedBacktick);
}

#[test]
fn lex_error_unterminated_heredoc() {
    let err = tokenize("<<EOF\nhello\n").unwrap_err();
    assert!(matches!(err.kind, LexErrorKind::UnterminatedHeredoc { .. }));
}

#[test]
fn lex_error_empty_heredoc_marker() {
    let err = tokenize("<<\nhello\n").unwrap_err();
    assert_eq!(err.kind, LexErrorKind::EmptyHeredocMarker);
}

#[test]
fn parse_str_convenience() {
    let cf = parse_str("example.com {\n\tlog\n}\n").unwrap();
    assert_eq!(cf.sites.len(), 1);
    assert_eq!(cf.sites[0].addresses[0].host, "example.com");
}

// -----------------------------------------------------------
// Formatter specifics.
// -----------------------------------------------------------

#[test]
fn format_trailing_newline() {
    let cf = Caddyfile::new();
    let output = format(&cf);
    assert!(output.ends_with('\n'));
}

#[test]
fn format_blank_line_between_sites() {
    let cf = Caddyfile::new()
        .site(SiteBlock::new("a.com").log())
        .site(SiteBlock::new("b.com").log());
    let output = format(&cf);
    assert!(output.contains("}\n\nb.com {"));
}

#[test]
fn format_tab_indentation() {
    let cf = Caddyfile::new()
        .site(SiteBlock::new("example.com").directive(
            Directive::new("header").block(vec![Directive::new("X-Test").arg("value")]),
        ));
    let output = format(&cf);
    assert!(output.contains("\theader {"));
    assert!(output.contains("\t\tX-Test value"));
}

#[test]
fn format_preserves_quoted_args() {
    let cf = Caddyfile::new().site(
        SiteBlock::new("example.com").directive(
            Directive::new("header")
                .arg("X-Custom")
                .quoted_arg("value with spaces"),
        ),
    );
    let output = format(&cf);
    assert!(output.contains("\"value with spaces\""));
}

#[test]
fn format_matcher_named() {
    let cf = Caddyfile::new().site(
        SiteBlock::new("example.com").directive(
            Directive::new("respond")
                .matcher(Matcher::Named("api".to_string()))
                .arg("200"),
        ),
    );
    let output = format(&cf);
    assert!(output.contains("respond @api 200"));
}

#[test]
fn format_matcher_all() {
    let cf = Caddyfile::new().site(
        SiteBlock::new("example.com")
            .directive(Directive::new("respond").matcher(Matcher::All).arg("200")),
    );
    let output = format(&cf);
    assert!(output.contains("respond * 200"));
}

// -----------------------------------------------------------
// Full Caddyfile integration tests.
// -----------------------------------------------------------

#[test]
fn full_caddyfile_with_all_features() {
    let cf = Caddyfile::new()
        .global(GlobalOptions {
            directives: vec![Directive::new("email").arg("admin@example.com")],
        })
        .site(
            SiteBlock::new("example.com")
                .basic_auth("admin", "$2a$14$hash")
                .reverse_proxy("app:3000")
                .encode_gzip()
                .security_headers()
                .log(),
        );

    let output = format(&cf);

    // Verify structure
    assert!(output.starts_with('{'));
    assert!(output.contains("email admin@example.com"));
    assert!(output.contains("example.com {"));
    assert!(output.contains("@protected"));
    assert!(output.contains("basic_auth @protected"));
    assert!(output.contains("reverse_proxy app:3000"));
    assert!(output.contains("encode gzip"));
    assert!(output.contains("X-Frame-Options"));
    assert!(output.contains("log"));

    // Verify it parses back cleanly
    let tokens = tokenize(&output).expect("tokenize");
    let parsed = parse(&tokens).expect("parse");
    assert!(parsed.global_options.is_some());
    assert_eq!(parsed.sites.len(), 1);
}

#[test]
fn complex_multisite_caddyfile() {
    let cf = Caddyfile::new()
        .site(
            SiteBlock::new("example.com")
                .reverse_proxy("web:3000")
                .encode_gzip()
                .log(),
        )
        .site(
            SiteBlock::new("api.example.com")
                .reverse_proxy("api:8080")
                .log(),
        )
        .site(
            SiteBlock::new("admin.example.com")
                .basic_auth("admin", "$2a$14$hash")
                .reverse_proxy("admin:3001")
                .security_headers(),
        );

    let output = format(&cf);
    let tokens = tokenize(&output).expect("tokenize");
    let parsed = parse(&tokens).expect("parse");

    assert_eq!(parsed.sites.len(), 3);
    assert_eq!(parsed.sites[0].addresses[0].host, "example.com");
    assert_eq!(parsed.sites[1].addresses[0].host, "api.example.com");
    assert_eq!(parsed.sites[2].addresses[0].host, "admin.example.com");
}
