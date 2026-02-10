#![allow(dead_code)]

use caddyfile_rs::{Caddyfile, format, parse, parse_str, tokenize};

pub fn roundtrip(input: &str) {
    let tokens = tokenize(input).expect("tokenize failed");
    let cf = parse(&tokens).expect("parse failed");
    let output = format(&cf);
    assert_eq!(
        output, input,
        "round-trip mismatch:\n--- expected ---\n{input}\n--- got ---\n{output}"
    );
}

/// Helper: format an AST, parse it back, assert structural equality.
pub fn assert_ast_roundtrip(original: &Caddyfile) {
    let formatted = format(original);
    let parsed = parse_str(&formatted).unwrap_or_else(|e| {
        panic!(
            "failed to re-parse formatted output: {e}\n\
             --- formatted ---\n{formatted}"
        )
    });

    assert_eq!(
        original.global_options, parsed.global_options,
        "global_options mismatch\n--- formatted ---\n{formatted}"
    );
    assert_eq!(
        original.snippets, parsed.snippets,
        "snippets mismatch\n--- formatted ---\n{formatted}"
    );
    assert_eq!(
        original.named_routes, parsed.named_routes,
        "named_routes mismatch\n--- formatted ---\n{formatted}"
    );
    assert_eq!(
        original.sites, parsed.sites,
        "sites mismatch\n--- formatted ---\n{formatted}"
    );
}
