//! AST fidelity tests: build, format, parse, and compare structures.
//! Also covers Display impls and address parsing.

mod common;

use caddyfile_rs::{
    Address, Argument, Caddyfile, Directive, GlobalOptions, Matcher, NamedRoute, Scheme, SiteBlock,
    Snippet,
};
use common::assert_ast_roundtrip;

// -----------------------------------------------------------
// Display impls.
// -----------------------------------------------------------

#[test]
fn display_address_full() {
    let addr = Address {
        scheme: Some(Scheme::Https),
        host: "example.com".to_string(),
        port: Some(8443),
        path: Some("/api".to_string()),
    };
    assert_eq!(addr.to_string(), "https://example.com:8443/api");
}

#[test]
fn display_address_minimal() {
    let addr = Address {
        scheme: None,
        host: "localhost".to_string(),
        port: None,
        path: None,
    };
    assert_eq!(addr.to_string(), "localhost");
}

#[test]
fn display_matcher_variants() {
    assert_eq!(Matcher::All.to_string(), "*");
    assert_eq!(Matcher::Path("/health".to_string()).to_string(), "/health");
    assert_eq!(Matcher::Named("api".to_string()).to_string(), "@api");
}

#[test]
fn display_argument_variants() {
    assert_eq!(Argument::Unquoted("value".to_string()).to_string(), "value");
    assert_eq!(
        Argument::Quoted("hello world".to_string()).to_string(),
        "\"hello world\""
    );
    assert_eq!(
        Argument::Quoted("say \"hi\"".to_string()).to_string(),
        "\"say \\\"hi\\\"\""
    );
    assert_eq!(
        Argument::Backtick("raw\\n".to_string()).to_string(),
        "`raw\\n`"
    );
    assert_eq!(
        Argument::Heredoc {
            marker: "EOF".to_string(),
            content: "line1\nline2".to_string(),
        }
        .to_string(),
        "<<EOF\nline1\nline2\nEOF"
    );
}

#[test]
fn display_scheme() {
    assert_eq!(Scheme::Http.to_string(), "http");
    assert_eq!(Scheme::Https.to_string(), "https");
}

// -----------------------------------------------------------
// Address parsing.
// -----------------------------------------------------------

#[test]
fn parse_address_http_with_port() {
    let addr = caddyfile_rs::parse_address("http://localhost:8080");
    assert_eq!(addr.scheme, Some(Scheme::Http));
    assert_eq!(addr.host, "localhost");
    assert_eq!(addr.port, Some(8080));
    assert_eq!(addr.path, None);
}

#[test]
fn parse_address_bare_port() {
    let addr = caddyfile_rs::parse_address(":443");
    assert_eq!(addr.host, "");
    assert_eq!(addr.port, Some(443));
}

#[test]
fn parse_address_with_path_only() {
    let addr = caddyfile_rs::parse_address("example.com/blog");
    assert_eq!(addr.host, "example.com");
    assert_eq!(addr.path, Some("/blog".to_string()));
}

#[test]
fn parse_address_wildcard() {
    let addr = caddyfile_rs::parse_address("*.example.com");
    assert_eq!(addr.host, "*.example.com");
    assert_eq!(addr.scheme, None);
}

#[test]
fn parse_address_deep_subdomain() {
    let addr = caddyfile_rs::parse_address("a.b.c.d.example.com");
    assert_eq!(addr.host, "a.b.c.d.example.com");
    assert_eq!(addr.scheme, None);
    assert_eq!(addr.port, None);
}

#[test]
fn parse_address_deep_subdomain_with_port() {
    let addr = caddyfile_rs::parse_address("a.b.c.example.com:8443");
    assert_eq!(addr.host, "a.b.c.example.com");
    assert_eq!(addr.port, Some(8443));
}

#[test]
fn parse_address_deep_subdomain_with_scheme() {
    let addr = caddyfile_rs::parse_address("https://dev.api.staging.example.com");
    assert_eq!(addr.scheme, Some(Scheme::Https));
    assert_eq!(addr.host, "dev.api.staging.example.com");
}

// -----------------------------------------------------------
// IPv6 address parsing.
// -----------------------------------------------------------

#[test]
fn parse_address_ipv6_loopback_with_port() {
    let addr = caddyfile_rs::parse_address("[::1]:8080");
    assert_eq!(addr.host, "[::1]");
    assert_eq!(addr.port, Some(8080));
}

#[test]
fn parse_address_ipv6_loopback_no_port() {
    let addr = caddyfile_rs::parse_address("[::1]");
    assert_eq!(addr.host, "[::1]");
    assert_eq!(addr.port, None);
}

#[test]
fn parse_address_ipv6_full_with_port() {
    let addr = caddyfile_rs::parse_address("[2001:db8::1]:443");
    assert_eq!(addr.host, "[2001:db8::1]");
    assert_eq!(addr.port, Some(443));
}

#[test]
fn parse_address_ipv6_full_no_port() {
    let addr = caddyfile_rs::parse_address("[2001:db8::1]");
    assert_eq!(addr.host, "[2001:db8::1]");
    assert_eq!(addr.port, None);
}

#[test]
fn parse_address_ipv6_all_interfaces() {
    let addr = caddyfile_rs::parse_address("[::]");
    assert_eq!(addr.host, "[::]");
}

#[test]
fn parse_address_ipv6_with_scheme() {
    let addr = caddyfile_rs::parse_address("https://[::1]:8443");
    assert_eq!(addr.scheme, Some(Scheme::Https));
    assert_eq!(addr.host, "[::1]");
    assert_eq!(addr.port, Some(8443));
}

// -----------------------------------------------------------
// AST fidelity tests: build → format → parse → compare.
// -----------------------------------------------------------

#[test]
fn ast_fidelity_simple_site() {
    let cf = Caddyfile::new().site(
        SiteBlock::new("example.com")
            .reverse_proxy("app:3000")
            .log(),
    );
    assert_ast_roundtrip(&cf);
}

#[test]
fn ast_fidelity_global_options() {
    let cf = Caddyfile::new()
        .global(GlobalOptions {
            directives: vec![
                Directive::new("email").arg("admin@example.com"),
                Directive::new("admin").arg("off"),
            ],
        })
        .site(SiteBlock::new("example.com").log());
    assert_ast_roundtrip(&cf);
}

#[test]
fn ast_fidelity_global_with_nested_block() {
    let cf = Caddyfile::new()
        .global(GlobalOptions {
            directives: vec![
                Directive::new("email").arg("admin@example.com"),
                Directive::new("servers")
                    .block(vec![Directive::new("protocols").arg("h1").arg("h2")]),
            ],
        })
        .site(SiteBlock::new("example.com").log());
    assert_ast_roundtrip(&cf);
}

#[test]
fn ast_fidelity_snippet() {
    let cf = Caddyfile::new()
        .snippet(Snippet {
            name: "logging".to_string(),
            directives: vec![Directive::new("log").block(vec![
                Directive::new("output").arg("stderr"),
                Directive::new("format").arg("console"),
            ])],
        })
        .site(
            SiteBlock::new("example.com")
                .directive(Directive::new("import").arg("logging"))
                .log(),
        );
    assert_ast_roundtrip(&cf);
}

#[test]
fn ast_fidelity_named_route() {
    let cf = Caddyfile::new()
        .named_route(NamedRoute {
            name: "auth".to_string(),
            directives: vec![
                Directive::new("basic_auth")
                    .block(vec![Directive::new("admin").arg("$2a$14$hash")]),
            ],
        })
        .site(
            SiteBlock::new("admin.example.com")
                .directive(Directive::new("invoke").arg("auth"))
                .log(),
        );
    assert_ast_roundtrip(&cf);
}

#[test]
fn ast_fidelity_multiple_snippets_and_named_routes() {
    let cf = Caddyfile::new()
        .snippet(Snippet {
            name: "log".to_string(),
            directives: vec![Directive::new("log")],
        })
        .snippet(Snippet {
            name: "sec".to_string(),
            directives: vec![
                Directive::new("header")
                    .block(vec![Directive::new("X-Frame-Options").quoted_arg("DENY")]),
            ],
        })
        .named_route(NamedRoute {
            name: "auth".to_string(),
            directives: vec![
                Directive::new("basic_auth").block(vec![Directive::new("admin").arg("pass")]),
            ],
        })
        .site(SiteBlock::new("example.com").log());
    assert_ast_roundtrip(&cf);
}

#[test]
fn ast_fidelity_multi_address() {
    let cf = Caddyfile::new().site(
        SiteBlock::new("example.com")
            .address("www.example.com")
            .log(),
    );
    assert_ast_roundtrip(&cf);
}

#[test]
fn ast_fidelity_address_with_scheme_and_port() {
    let cf = Caddyfile::new().site(SiteBlock::new("https://example.com:8443").log());
    assert_ast_roundtrip(&cf);
}

#[test]
fn ast_fidelity_bare_port_address() {
    let cf = Caddyfile::new()
        .site(SiteBlock::new(":8080").directive(Directive::new("respond").quoted_arg("hello")));
    assert_ast_roundtrip(&cf);
}

#[test]
fn ast_fidelity_matcher_all() {
    let cf = Caddyfile::new().site(
        SiteBlock::new("example.com")
            .directive(Directive::new("root").matcher(Matcher::All).arg("/srv")),
    );
    assert_ast_roundtrip(&cf);
}

#[test]
fn ast_fidelity_matcher_path() {
    let cf = Caddyfile::new().site(
        SiteBlock::new("example.com").directive(
            Directive::new("handle")
                .matcher(Matcher::Path("/api/*".to_string()))
                .block(vec![Directive::new("reverse_proxy").arg("api:8080")]),
        ),
    );
    assert_ast_roundtrip(&cf);
}

#[test]
fn ast_fidelity_matcher_named() {
    let cf = Caddyfile::new().site(
        SiteBlock::new("example.com").directive(
            Directive::new("respond")
                .matcher(Matcher::Named("health".to_string()))
                .arg("200"),
        ),
    );
    assert_ast_roundtrip(&cf);
}

#[test]
fn ast_fidelity_quoted_arguments() {
    let cf = Caddyfile::new().site(
        SiteBlock::new("example.com").directive(
            Directive::new("header")
                .arg("X-Custom")
                .quoted_arg("value with spaces"),
        ),
    );
    assert_ast_roundtrip(&cf);
}

#[test]
fn ast_fidelity_quoted_with_escapes() {
    let cf = Caddyfile::new().site(
        SiteBlock::new("example.com")
            .directive(Directive::new("respond").quoted_arg("say \"hello\"")),
    );
    assert_ast_roundtrip(&cf);
}

#[test]
fn ast_fidelity_deep_nesting_three_levels() {
    let cf = Caddyfile::new().site(SiteBlock::new("example.com").directive(
        Directive::new("handle").block(vec![Directive::new("route").block(vec![
            Directive::new("reverse_proxy").block(vec![
                Directive::new("to").arg("app:3000"),
                Directive::new("lb_policy").arg("round_robin"),
            ]),
        ])]),
    ));
    assert_ast_roundtrip(&cf);
}

#[test]
fn ast_fidelity_four_level_nesting() {
    let cf = Caddyfile::new().site(SiteBlock::new("example.com").directive(
        Directive::new("handle").block(vec![Directive::new("route").block(vec![
            Directive::new("reverse_proxy").block(vec![Directive::new("transport")
                    .arg("http")
                    .block(vec![Directive::new("tls_insecure_skip_verify")])]),
        ])]),
    ));
    assert_ast_roundtrip(&cf);
}

#[test]
fn ast_fidelity_mixed_block_and_nonblock_directives() {
    let cf = Caddyfile::new().site(
        SiteBlock::new("example.com")
            .directive(Directive::new("encode").arg("gzip"))
            .directive(
                Directive::new("header")
                    .block(vec![Directive::new("X-Frame-Options").quoted_arg("DENY")]),
            )
            .directive(Directive::new("reverse_proxy").arg("app:3000")),
    );
    assert_ast_roundtrip(&cf);
}

#[test]
fn ast_fidelity_multiple_sites() {
    let cf = Caddyfile::new()
        .site(SiteBlock::new("a.com").log())
        .site(SiteBlock::new("b.com").log())
        .site(SiteBlock::new("c.com").log());
    assert_ast_roundtrip(&cf);
}

#[test]
fn ast_fidelity_full_production_config() {
    let cf = Caddyfile::new()
        .global(GlobalOptions {
            directives: vec![
                Directive::new("email").arg("ops@example.com"),
                Directive::new("servers")
                    .block(vec![Directive::new("protocols").arg("h1").arg("h2")]),
            ],
        })
        .snippet(Snippet {
            name: "security".to_string(),
            directives: vec![Directive::new("header").block(vec![
                Directive::new("X-Content-Type-Options").quoted_arg("nosniff"),
                Directive::new("X-Frame-Options").quoted_arg("DENY"),
            ])],
        })
        .named_route(NamedRoute {
            name: "auth".to_string(),
            directives: vec![
                Directive::new("basic_auth")
                    .block(vec![Directive::new("admin").arg("$2a$14$hash")]),
            ],
        })
        .site(
            SiteBlock::new("example.com")
                .address("www.example.com")
                .directive(Directive::new("import").arg("security"))
                .directive(
                    Directive::new("handle")
                        .matcher(Matcher::Path("/api/*".to_string()))
                        .block(vec![Directive::new("reverse_proxy").block(vec![
                            Directive::new("to").arg("api1:8080"),
                            Directive::new("to").arg("api2:8080"),
                            Directive::new("lb_policy").arg("round_robin"),
                        ])]),
                )
                .directive(Directive::new("handle").block(vec![
                    Directive::new("root").matcher(Matcher::All).arg("/srv"),
                    Directive::new("file_server"),
                ]))
                .log(),
        )
        .site(
            SiteBlock::new("admin.example.com")
                .directive(Directive::new("invoke").arg("auth"))
                .reverse_proxy("admin:3001")
                .log(),
        );
    assert_ast_roundtrip(&cf);
}

#[test]
fn ast_fidelity_empty_caddyfile() {
    let cf = Caddyfile::new();
    assert_ast_roundtrip(&cf);
}

#[test]
fn ast_fidelity_global_only() {
    let cf = Caddyfile::new().global(GlobalOptions {
        directives: vec![Directive::new("admin").arg("off")],
    });
    assert_ast_roundtrip(&cf);
}

#[test]
fn ast_fidelity_directive_no_args_no_block() {
    let cf = Caddyfile::new().site(
        SiteBlock::new("example.com")
            .directive(Directive::new("log"))
            .directive(Directive::new("file_server")),
    );
    assert_ast_roundtrip(&cf);
}

#[test]
fn ast_fidelity_directive_many_args() {
    let cf = Caddyfile::new().site(
        SiteBlock::new("example.com")
            .directive(Directive::new("tls").arg("cert.pem").arg("key.pem")),
    );
    assert_ast_roundtrip(&cf);
}

#[test]
fn ast_fidelity_empty_block() {
    let cf = Caddyfile::new()
        .site(SiteBlock::new("example.com").directive(Directive::new("handle").block(vec![])));
    assert_ast_roundtrip(&cf);
}

#[test]
fn ast_fidelity_wildcard_address() {
    let cf = Caddyfile::new().site(SiteBlock::new("*.example.com").log());
    assert_ast_roundtrip(&cf);
}

#[test]
fn ast_fidelity_ipv6_address() {
    let cf = Caddyfile::new().site(
        SiteBlock::new("[::1]:8080").directive(Directive::new("respond").quoted_arg("hello")),
    );
    assert_ast_roundtrip(&cf);
}

#[test]
fn ast_fidelity_deep_subdomain() {
    let cf = Caddyfile::new().site(
        SiteBlock::new("a.b.c.d.example.com")
            .reverse_proxy("app:3000")
            .log(),
    );
    assert_ast_roundtrip(&cf);
}
