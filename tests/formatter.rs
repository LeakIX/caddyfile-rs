//! Formatter-specific tests.

use caddyfile_rs::{Caddyfile, Directive, Matcher, SiteBlock, format};

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
