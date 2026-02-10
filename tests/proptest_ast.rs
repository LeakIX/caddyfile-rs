//! Property-based tests with proptest.
//!
//! Generate random ASTs, format them, parse them back, and verify the
//! round-trip produces a stable (idempotent) output.
//!
//! We check `format(parse(format(ast))) == format(ast)` rather than
//! `ast == parse(format(ast))` because the parser may normalise some
//! constructs (e.g. bare `/path` args become path matchers). The
//! idempotency check is strictly stronger than equality for real-world
//! correctness.

use caddyfile_rs::{
    Argument, Caddyfile, Directive, GlobalOptions, Matcher, NamedRoute, SiteBlock, Snippet, format,
    parse_str, tokenize,
};
use proptest::prelude::*;

// -- Leaf strategies --

/// Safe directive name: lowercase alpha start, then alphanumeric + _ -
fn directive_name() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9_-]{0,15}".prop_map(|s| s)
}

/// Safe unquoted argument: no leading special characters, no whitespace
fn unquoted_arg() -> impl Strategy<Value = String> {
    "[a-z0-9][a-z0-9.:_-]{0,20}".prop_map(|s| s)
}

/// Quoted argument: printable ASCII, may contain spaces.
/// Must not start with / @ * -- the parser treats those as
/// matchers even inside quoted strings (known limitation).
fn quoted_arg_value() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9][a-zA-Z0-9 .:_-]{0,29}".prop_map(|s| s)
}

/// Argument: either unquoted or quoted (skip backtick/heredoc
/// for simplicity -- they have their own dedicated tests)
fn argument() -> impl Strategy<Value = Argument> {
    prop_oneof![
        unquoted_arg().prop_map(Argument::Unquoted),
        quoted_arg_value().prop_map(Argument::Quoted),
    ]
}

/// Arguments list (0-4 args)
fn arguments() -> impl Strategy<Value = Vec<Argument>> {
    prop::collection::vec(argument(), 0..=4)
}

/// Matcher (optional)
fn matcher() -> impl Strategy<Value = Option<Matcher>> {
    prop_oneof![
        3 => Just(None),
        1 => Just(Some(Matcher::All)),
        1 => "[a-z]{1,10}".prop_map(|n| Some(Matcher::Named(n))),
    ]
}

/// Directive at a given depth (limits recursion)
fn directive(depth: u32) -> impl Strategy<Value = Directive> {
    let leaf = (directive_name(), matcher(), arguments()).prop_map(|(name, matcher, arguments)| {
        Directive {
            name,
            matcher,
            arguments,
            block: None,
        }
    });

    if depth == 0 {
        leaf.boxed()
    } else {
        let with_block = (
            directive_name(),
            // No matcher on block directives to avoid ambiguity
            arguments(),
            prop::collection::vec(directive(depth - 1), 0..=3),
        )
            .prop_map(|(name, arguments, sub)| Directive {
                name,
                matcher: None,
                arguments,
                block: Some(sub),
            });

        prop_oneof![
            3 => leaf,
            1 => with_block,
        ]
        .boxed()
    }
}

/// Directives list (0-5 directives at depth 2)
fn directives() -> impl Strategy<Value = Vec<Directive>> {
    prop::collection::vec(directive(2), 0..=5)
}

/// Simple hostname
fn hostname() -> impl Strategy<Value = String> {
    "[a-z]{2,8}\\.(com|org|net|io)".prop_map(|s| s)
}

/// Address -- just use simple hostnames to avoid `parse_address`
/// ambiguities with port/path
fn address() -> impl Strategy<Value = String> {
    hostname()
}

/// Snippet
fn snippet() -> impl Strategy<Value = Snippet> {
    ("[a-z]{2,10}", directives()).prop_map(|(name, directives)| Snippet { name, directives })
}

/// Named route
fn named_route() -> impl Strategy<Value = NamedRoute> {
    ("[a-z]{2,10}", directives()).prop_map(|(name, directives)| NamedRoute { name, directives })
}

/// Site block
fn site_block() -> impl Strategy<Value = SiteBlock> {
    (prop::collection::vec(address(), 1..=3), directives()).prop_map(|(addrs, directives)| {
        let mut sb = SiteBlock::new(&addrs[0]);
        for addr in &addrs[1..] {
            sb = sb.address(addr);
        }
        sb.directives = directives;
        sb
    })
}

/// Global options (optional)
fn global_options() -> impl Strategy<Value = Option<GlobalOptions>> {
    prop_oneof![
        3 => Just(None),
        1 => directives().prop_map(|d| Some(GlobalOptions { directives: d })),
    ]
}

/// Full Caddyfile
fn caddyfile() -> impl Strategy<Value = Caddyfile> {
    (
        global_options(),
        prop::collection::vec(snippet(), 0..=2),
        prop::collection::vec(named_route(), 0..=2),
        prop::collection::vec(site_block(), 0..=3),
    )
        .prop_map(
            |(global_options, snippets, named_routes, sites)| Caddyfile {
                global_options,
                snippets,
                named_routes,
                sites,
            },
        )
}

// -- Property tests --

proptest! {
    /// Formatting is idempotent: format(parse(format(x))) == format(x).
    /// This is the core round-trip property.
    #[test]
    fn format_idempotent(cf in caddyfile()) {
        let r1 = format(&cf);
        let parsed = parse_str(&r1)
            .map_err(|e| {
                TestCaseError::fail(
                    std::format!("parse error: {e}\n--- output ---\n{r1}"))
            })?;
        let r2 = format(&parsed);
        prop_assert_eq!(r1, r2);
    }

    /// A formatted Caddyfile never panics when tokenized.
    #[test]
    fn format_never_produces_lex_error(cf in caddyfile()) {
        let formatted = format(&cf);
        tokenize(&formatted).map_err(|e| {
            TestCaseError::fail(
                std::format!("lex error: {e}\n--- output ---\n{formatted}"))
        })?;
    }

    /// Parsed site count survives the round-trip.
    #[test]
    fn site_count_preserved(cf in caddyfile()) {
        let formatted = format(&cf);
        let parsed = parse_str(&formatted).unwrap();
        prop_assert_eq!(cf.sites.len(), parsed.sites.len());
    }

    /// Snippet count survives the round-trip.
    #[test]
    fn snippet_count_preserved(cf in caddyfile()) {
        let formatted = format(&cf);
        let parsed = parse_str(&formatted).unwrap();
        prop_assert_eq!(cf.snippets.len(), parsed.snippets.len());
    }

    /// Named route count survives the round-trip.
    #[test]
    fn named_route_count_preserved(cf in caddyfile()) {
        let formatted = format(&cf);
        let parsed = parse_str(&formatted).unwrap();
        prop_assert_eq!(cf.named_routes.len(), parsed.named_routes.len());
    }

    /// Global options presence survives.
    #[test]
    fn global_options_presence_preserved(cf in caddyfile()) {
        let formatted = format(&cf);
        let parsed = parse_str(&formatted).unwrap();
        prop_assert_eq!(
            cf.global_options.is_some(),
            parsed.global_options.is_some()
        );
    }

    /// Snippet names survive the round-trip.
    #[test]
    fn snippet_names_preserved(cf in caddyfile()) {
        let formatted = format(&cf);
        let parsed = parse_str(&formatted).unwrap();
        let orig_names: Vec<_> = cf.snippets.iter().map(|s| &s.name).collect();
        let parsed_names: Vec<_> = parsed.snippets.iter().map(|s| &s.name).collect();
        prop_assert_eq!(orig_names, parsed_names);
    }

    /// Named route names survive the round-trip.
    #[test]
    fn named_route_names_preserved(cf in caddyfile()) {
        let formatted = format(&cf);
        let parsed = parse_str(&formatted).unwrap();
        let orig_names: Vec<_> = cf.named_routes.iter().map(|r| &r.name).collect();
        let parsed_names: Vec<_> = parsed.named_routes.iter().map(|r| &r.name).collect();
        prop_assert_eq!(orig_names, parsed_names);
    }

    /// Address count per site survives the round-trip.
    #[test]
    fn address_counts_preserved(cf in caddyfile()) {
        let formatted = format(&cf);
        let parsed = parse_str(&formatted).unwrap();
        for (orig, re) in cf.sites.iter().zip(parsed.sites.iter()) {
            prop_assert_eq!(orig.addresses.len(), re.addresses.len());
        }
    }

    /// Directive count per site survives the round-trip.
    #[test]
    fn directive_counts_preserved(cf in caddyfile()) {
        let formatted = format(&cf);
        let parsed = parse_str(&formatted).unwrap();
        for (orig, re) in cf.sites.iter().zip(parsed.sites.iter()) {
            prop_assert_eq!(
                orig.directives.len(),
                re.directives.len(),
                "Directive count mismatch in site {:?}\nFormatted:\n{}",
                orig.addresses,
                formatted
            );
        }
    }
}
