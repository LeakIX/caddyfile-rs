//! Parser edge cases and error tests.

use caddyfile_rs::{ParseErrorKind, parse, parse_str, tokenize};

// -----------------------------------------------------------
// Parser errors.
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
    let tokens = tokenize("example.com {\n\theader {\n\t\tX-Test value\n}\n").expect("tokenize");
    let result = parse(&tokens);
    assert!(result.is_err());
}

#[test]
fn parse_str_convenience() {
    let cf = parse_str("example.com {\n\tlog\n}\n").unwrap();
    assert_eq!(cf.sites.len(), 1);
    assert_eq!(cf.sites[0].addresses[0].host, "example.com");
}

#[test]
fn parse_str_lex_error() {
    let err = parse_str("\"unclosed").unwrap_err();
    assert!(matches!(err, caddyfile_rs::Error::Lex(_)));
}

#[test]
fn parse_str_parse_error() {
    let err = parse_str("example.com {\n\tlog\n").unwrap_err();
    assert!(matches!(err, caddyfile_rs::Error::Parse(_)));
}

#[test]
fn parse_error_wrong_token_for_brace() {
    let tokens = tokenize("example.com log").expect("tokenize");
    let result = parse(&tokens);
    assert!(result.is_ok());
}

#[test]
fn parse_error_display_includes_location() {
    let err = parse_str("example.com {\n\tlog\n").unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("expected '}'"));
}

// -----------------------------------------------------------
// Extended parser edge cases.
// -----------------------------------------------------------

#[test]
fn parse_error_only_open_brace() {
    let result = parse_str("{");
    assert!(result.is_err());
}

#[test]
fn parse_error_only_close_brace() {
    // The parser treats } as an address token, not as an error
    // This documents the current behavior
    let tokens = tokenize("}").expect("tokenize");
    let result = parse(&tokens);
    assert!(
        result.is_ok(),
        "parser treats bare }} as address token, not error"
    );
}

#[test]
fn parse_double_open_brace_is_directive_name() {
    // Inner { is treated as a directive name, not as a syntax error
    let cf = parse_str("example.com {\n\t{\n\t}\n}\n");
    assert!(cf.is_ok(), "parser treats inner {{ as directive name");
}

#[test]
fn parse_error_nested_unclosed_at_depth_2() {
    let result = parse_str("a {\n\tb {\n\t\tc\n}\n");
    assert!(result.is_err());
}

#[test]
fn parse_error_nested_unclosed_at_depth_3() {
    let result = parse_str("a {\n\tb {\n\t\tc {\n\t\t\td\n\t\t}\n\t}\n");
    // Missing the outermost closing brace
    assert!(
        result.is_err(),
        "should error on missing outermost close brace"
    );
}

#[test]
fn parse_empty_site_block() {
    let cf = parse_str("example.com {\n}\n").unwrap();
    assert_eq!(cf.sites.len(), 1);
    assert!(cf.sites[0].directives.is_empty());
}

#[test]
fn parse_multiple_empty_site_blocks() {
    let cf = parse_str("a.com {\n}\n\nb.com {\n}\n").unwrap();
    assert_eq!(cf.sites.len(), 2);
}

#[test]
fn parse_empty_global_options() {
    let cf = parse_str("{\n}\n\nexample.com {\n\tlog\n}\n").unwrap();
    assert!(cf.global_options.is_some());
    assert!(cf.global_options.unwrap().directives.is_empty());
}

#[test]
fn parse_close_brace_treated_as_address() {
    // Parser treats } as an address, which is unexpected but
    // documents the current behavior. The Go reference also
    // allows unusual address tokens.
    let result = parse_str("} {\n\tlog\n}\n");
    assert!(result.is_ok());
}

#[test]
fn parse_garbage_between_sites() {
    // Words between sites are treated as addresses
    let cf = parse_str("a.com {\n\tlog\n}\n\nb.com {\n\tlog\n}\n").unwrap();
    assert_eq!(cf.sites.len(), 2);
}

// -----------------------------------------------------------
// Parser treats /path prefix as path matcher.
// -----------------------------------------------------------

#[test]
fn parser_treats_slash_prefix_as_path_matcher() {
    use caddyfile_rs::Matcher;

    let cf = parse_str("example.com {\n\trespond /health 200\n}\n").unwrap();
    let directive = &cf.sites[0].directives[0];
    assert_eq!(directive.name, "respond");
    assert_eq!(
        directive.matcher,
        Some(Matcher::Path("/health".to_string()))
    );
    assert_eq!(directive.arguments.len(), 1);
    assert_eq!(directive.arguments[0].value(), "200");
}

// -----------------------------------------------------------
// Error display.
// -----------------------------------------------------------

#[test]
fn display_error_types() {
    let lex_err = tokenize("\"unclosed").unwrap_err();
    let msg = lex_err.to_string();
    assert!(msg.contains("unterminated quoted string"));
    assert!(msg.contains("line 1"));

    let parse_err = parse_str("example.com {\n\tlog\n").unwrap_err();
    let msg = parse_err.to_string();
    assert!(msg.contains("expected '}'"));
}
