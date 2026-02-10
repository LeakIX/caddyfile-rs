//! Lexer edge cases and error tests.

use caddyfile_rs::{LexErrorKind, tokenize};

// -----------------------------------------------------------
// Basic lexer behaviour.
// -----------------------------------------------------------

#[test]
fn lex_empty_input() {
    let tokens = tokenize("").expect("tokenize");
    assert!(tokens.is_empty());
}

#[test]
fn lex_only_whitespace() {
    let tokens = tokenize("   \t  \n\n  ").expect("tokenize");
    assert!(
        tokens
            .iter()
            .all(|t| matches!(t.kind, caddyfile_rs::TokenKind::Newline))
    );
}

#[test]
fn lex_multiple_comments() {
    let tokens = tokenize("# comment 1\n# comment 2\n").expect("tokenize");
    let count = tokens
        .iter()
        .filter(|t| matches!(t.kind, caddyfile_rs::TokenKind::Comment))
        .count();
    assert_eq!(count, 2);
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

#[test]
fn lex_line_continuation_crlf() {
    let tokens = tokenize("reverse_proxy \\\r\napp:3000").expect("tokenize");
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].text, "reverse_proxy");
    assert_eq!(tokens[1].text, "app:3000");
}

#[test]
fn lex_backtick_with_special_chars() {
    let tokens = tokenize("`hello\\nworld`").expect("tokenize");
    assert_eq!(tokens[0].text, "hello\\nworld");
    assert!(matches!(
        tokens[0].kind,
        caddyfile_rs::TokenKind::BacktickString
    ));
}

#[test]
fn lex_heredoc_with_marker_like_content() {
    let input = "respond <<MARKER\nThis MARKER is inside content\nMARKER\n";
    let tokens = tokenize(input).expect("tokenize");
    assert_eq!(tokens[1].text, "This MARKER is inside content");
}

#[test]
fn lex_error_heredoc_marker_variety() {
    let input = "respond <<MYMARKER\nhello\nMYMARKER\n";
    let tokens = tokenize(input).expect("tokenize");
    assert_eq!(tokens[1].text, "hello");
    assert!(matches!(
        &tokens[1].kind,
        caddyfile_rs::TokenKind::Heredoc { marker }
        if marker == "MYMARKER"
    ));
}

// -----------------------------------------------------------
// Extended lexer edge cases.
// -----------------------------------------------------------

#[test]
fn lex_ascii_in_quoted_values() {
    let input = "example.com {\n\trespond \"caf\\u00e9-like value\"\n}\n";
    let tokens = tokenize(input).expect("tokenize");
    let respond_idx = tokens.iter().position(|t| t.text == "respond").unwrap();
    let value = &tokens[respond_idx + 1];
    assert!(value.text.contains("caf"));
}

#[test]
fn lex_ascii_in_hostname() {
    let input = "cafe123.example.com {\n\tlog\n}\n";
    let tokens = tokenize(input).expect("tokenize");
    assert_eq!(tokens[0].text, "cafe123.example.com");
}

#[test]
fn lex_heredoc_empty_content() {
    let input = "respond <<EOF\nEOF\n";
    let tokens = tokenize(input).expect("tokenize");
    assert_eq!(tokens[0].text, "respond");
    assert_eq!(tokens[1].text, "");
    assert!(matches!(
        &tokens[1].kind,
        caddyfile_rs::TokenKind::Heredoc { marker }
        if marker == "EOF"
    ));
}

#[test]
fn lex_multiple_heredocs() {
    let input = "\
respond /a <<A
content-a
A
respond /b <<B
content-b
B
";
    let tokens = tokenize(input).expect("tokenize");
    let heredocs: Vec<_> = tokens
        .iter()
        .filter(|t| matches!(t.kind, caddyfile_rs::TokenKind::Heredoc { .. }))
        .collect();
    assert_eq!(heredocs.len(), 2);
    assert_eq!(heredocs[0].text, "content-a");
    assert_eq!(heredocs[1].text, "content-b");
}

#[test]
fn lex_env_var_mid_word() {
    let tokens = tokenize("prefix{$VAR}suffix").expect("tokenize");
    let has_env = tokens.iter().any(|t| {
        matches!(
            &t.kind,
            caddyfile_rs::TokenKind::EnvVar { name, .. }
            if name == "VAR"
        )
    });
    assert!(has_env);
}

#[test]
fn lex_escaped_space_in_word() {
    let tokens = tokenize("hello\\ world").expect("tokenize");
    assert!(tokens.iter().any(|t| t.text.contains(' ')));
}

#[test]
fn lex_escaped_braces_in_word() {
    let tokens = tokenize("\\{literal\\}").expect("tokenize");
    assert!(tokens.iter().any(|t| t.text.contains('{')));
}

#[test]
fn lex_tab_indentation_preserved() {
    let input = "example.com {\n\t\t\tdeep\n}\n";
    let tokens = tokenize(input).expect("tokenize");
    let deep = tokens.iter().find(|t| t.text == "deep").unwrap();
    assert!(deep.span.column > 1);
}

#[test]
fn lex_crlf_line_endings() {
    let input = "example.com {\r\n\tlog\r\n}\r\n";
    let tokens = tokenize(input).expect("tokenize");
    assert!(tokens.iter().any(|t| t.text == "example.com"));
    assert!(tokens.iter().any(|t| t.text == "log"));
}

#[test]
fn lex_mixed_line_endings() {
    let input = "a {\n\tb\r\n\tc\n}\n";
    let tokens = tokenize(input).expect("tokenize");
    assert!(tokens.iter().any(|t| t.text == "b"));
    assert!(tokens.iter().any(|t| t.text == "c"));
}

#[test]
fn lex_quoted_string_with_newlines() {
    let tokens = tokenize("\"line1\\nline2\\nline3\"").expect("tokenize");
    assert_eq!(tokens[0].text.matches('\n').count(), 2);
}

#[test]
fn lex_quoted_string_all_escapes() {
    let tokens =
        tokenize("\"tab\\there\\nnewline\\rcarriage\\\\backslash\\\"quote\"").expect("tokenize");
    let text = &tokens[0].text;
    assert!(text.contains('\t'));
    assert!(text.contains('\n'));
    assert!(text.contains('\r'));
    assert!(text.contains('\\'));
    assert!(text.contains('"'));
}

#[test]
fn lex_backtick_multiline() {
    let input = "`line1\nline2\nline3`";
    let tokens = tokenize(input).expect("tokenize");
    assert_eq!(tokens[0].text, "line1\nline2\nline3");
    assert!(matches!(
        tokens[0].kind,
        caddyfile_rs::TokenKind::BacktickString
    ));
}

#[test]
fn lex_consecutive_braces() {
    let input = "a {\n\tb {\n\t\tc\n\t}\n}\n";
    let tokens = tokenize(input).expect("tokenize");
    let count = tokens
        .iter()
        .filter(|t| {
            matches!(
                t.kind,
                caddyfile_rs::TokenKind::OpenBrace | caddyfile_rs::TokenKind::CloseBrace
            )
        })
        .count();
    assert_eq!(count, 4);
}

#[test]
fn lex_multiple_spaces_between_tokens() {
    let input = "reverse_proxy    app:3000    other:4000";
    let tokens = tokenize(input).expect("tokenize");
    let count = tokens
        .iter()
        .filter(|t| matches!(t.kind, caddyfile_rs::TokenKind::Word))
        .count();
    assert_eq!(count, 3);
}

// -----------------------------------------------------------
// Lexer errors.
// -----------------------------------------------------------

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
fn lex_error_unterminated_heredoc_with_marker() {
    let err = tokenize("<<CUSTOM\nhello world\n").unwrap_err();
    assert!(matches!(
        &err.kind,
        LexErrorKind::UnterminatedHeredoc { marker }
        if marker == "CUSTOM"
    ));
}

#[test]
fn lex_error_display_includes_location() {
    let err = tokenize("example.com {\n\t\"unclosed").unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("line 2"));
}

#[test]
fn lex_error_span_multiline() {
    let err = tokenize("a\nb\n\"unclosed").unwrap_err();
    assert!(err.span.line >= 3);
}
