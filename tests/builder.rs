//! Builder API tests: build ASTs programmatically, format, and verify.

use caddyfile_rs::{
    Caddyfile, Directive, GlobalOptions, Matcher, NamedRoute, Scheme, SiteBlock, Snippet, format,
    parse, parse_str, tokenize,
};

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

    assert!(output.starts_with('{'));
    assert!(output.contains("email admin@example.com"));
    assert!(output.contains("example.com {"));
    assert!(output.contains("@protected"));
    assert!(output.contains("basic_auth @protected"));
    assert!(output.contains("reverse_proxy app:3000"));
    assert!(output.contains("encode gzip"));
    assert!(output.contains("X-Frame-Options"));
    assert!(output.contains("log"));

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

#[test]
fn builder_production_reverse_proxy_with_lb() {
    let cf = Caddyfile::new().site(SiteBlock::new("example.com").directive(
        Directive::new("reverse_proxy").block(vec![
            Directive::new("to").arg("app1:3000"),
            Directive::new("to").arg("app2:3000"),
            Directive::new("to").arg("app3:3000"),
            Directive::new("lb_policy").arg("round_robin"),
            Directive::new("health_uri").arg("/healthz"),
            Directive::new("health_interval").arg("30s"),
        ]),
    ));

    let output = format(&cf);
    let parsed = parse_str(&output).unwrap();
    let rp = &parsed.sites[0].directives[0];
    assert_eq!(rp.name, "reverse_proxy");
    let block = rp.block.as_ref().unwrap();
    assert_eq!(block.len(), 6);
    assert_eq!(block[3].name, "lb_policy");
    assert_eq!(block[3].arguments[0].value(), "round_robin");
}

#[test]
fn builder_handle_blocks_spa_and_api() {
    let cf = Caddyfile::new().site(
        SiteBlock::new("example.com")
            .directive(
                Directive::new("handle")
                    .matcher(Matcher::Path("/api/*".to_string()))
                    .block(vec![Directive::new("reverse_proxy").arg("api:8080")]),
            )
            .directive(Directive::new("handle").block(vec![
                Directive::new("root").matcher(Matcher::All).arg("/srv"),
                Directive::new("try_files")
                    .quoted_arg("{path}")
                    .arg("/index.html"),
                Directive::new("file_server"),
            ])),
    );

    let output = format(&cf);
    assert!(output.contains("handle /api/*"));
    assert!(output.contains("reverse_proxy api:8080"));
    assert!(output.contains("handle {"));
    assert!(output.contains("root * /srv"));
    assert!(output.contains("try_files \"{path}\" /index.html"));
    assert!(output.contains("file_server"));

    let parsed = parse_str(&output).unwrap();
    assert_eq!(parsed.sites[0].directives.len(), 2);
    let api_handle = &parsed.sites[0].directives[0];
    assert_eq!(
        api_handle.matcher,
        Some(Matcher::Path("/api/*".to_string()))
    );
}

#[test]
fn builder_snippets_and_import() {
    let cf = Caddyfile::new()
        .snippet(Snippet {
            name: "logging".to_string(),
            directives: vec![Directive::new("log").block(vec![
                Directive::new("output").arg("stderr"),
                Directive::new("format").arg("console"),
            ])],
        })
        .snippet(Snippet {
            name: "security".to_string(),
            directives: vec![Directive::new("header").block(vec![
                Directive::new("X-Content-Type-Options").quoted_arg("nosniff"),
                Directive::new("X-Frame-Options").quoted_arg("DENY"),
            ])],
        })
        .site(
            SiteBlock::new("example.com")
                .directive(Directive::new("import").arg("logging"))
                .directive(Directive::new("import").arg("security"))
                .reverse_proxy("app:3000"),
        );

    let output = format(&cf);
    assert!(output.contains("(logging) {"));
    assert!(output.contains("(security) {"));
    assert!(output.contains("import logging"));
    assert!(output.contains("import security"));

    let parsed = parse_str(&output).unwrap();
    assert_eq!(parsed.snippets.len(), 2);
    assert_eq!(parsed.snippets[0].name, "logging");
    assert_eq!(parsed.snippets[1].name, "security");
}

#[test]
fn builder_named_route_with_auth() {
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
                .reverse_proxy("admin:3001"),
        );

    let output = format(&cf);
    assert!(output.contains("&(auth) {"));
    assert!(output.contains("basic_auth {"));
    assert!(output.contains("invoke auth"));

    let parsed = parse_str(&output).unwrap();
    assert_eq!(parsed.named_routes.len(), 1);
    assert_eq!(parsed.named_routes[0].name, "auth");
}

#[test]
fn builder_multi_address_site() {
    let cf = Caddyfile::new().site(
        SiteBlock::new("example.com")
            .address("www.example.com")
            .address("https://example.org")
            .reverse_proxy("app:3000")
            .log(),
    );

    let output = format(&cf);
    assert!(output.contains("example.com, www.example.com, https://example.org {"));

    let parsed = parse_str(&output).unwrap();
    assert_eq!(parsed.sites[0].addresses.len(), 3);
    assert_eq!(parsed.sites[0].addresses[0].host, "example.com");
    assert_eq!(parsed.sites[0].addresses[1].host, "www.example.com");
    assert_eq!(parsed.sites[0].addresses[2].scheme, Some(Scheme::Https));
    assert_eq!(parsed.sites[0].addresses[2].host, "example.org");
}

#[test]
fn builder_deep_nesting_log_output() {
    let cf = Caddyfile::new().site(SiteBlock::new("example.com").directive(
        Directive::new("log").block(vec![
            Directive::new("output")
                .arg("file")
                .arg("/var/log/caddy/access.log")
                .block(vec![
                    Directive::new("roll_size").arg("100MiB"),
                    Directive::new("roll_keep").arg("10"),
                    Directive::new("roll_keep_for").arg("720h"),
                ]),
            Directive::new("format")
                .arg("json")
                .block(vec![Directive::new("time_format").arg("iso8601")]),
            Directive::new("level").arg("INFO"),
        ]),
    ));

    let output = format(&cf);
    assert!(output.contains("\t\t\troll_size 100MiB"));
    assert!(output.contains("\t\t\ttime_format iso8601"));

    let parsed = parse_str(&output).unwrap();
    let log = &parsed.sites[0].directives[0];
    let log_block = log.block.as_ref().unwrap();
    assert_eq!(log_block.len(), 3);
    let file_output = &log_block[0];
    assert_eq!(file_output.name, "output");
    let file_block = file_output.block.as_ref().unwrap();
    assert_eq!(file_block.len(), 3);
    assert_eq!(file_block[0].arguments[0].value(), "100MiB");
}

#[test]
fn builder_full_production_config() {
    let cf = Caddyfile::new()
        .global(GlobalOptions {
            directives: vec![
                Directive::new("email").arg("ops@example.com"),
                Directive::new("acme_ca").arg("https://acme.example.com/directory"),
                Directive::new("servers")
                    .block(vec![Directive::new("protocols").arg("h1").arg("h2")]),
            ],
        })
        .snippet(Snippet {
            name: "security_headers".to_string(),
            directives: vec![Directive::new("header").block(vec![
                Directive::new("X-Content-Type-Options").quoted_arg("nosniff"),
                Directive::new("X-Frame-Options").quoted_arg("DENY"),
                Directive::new("Strict-Transport-Security")
                    .quoted_arg("max-age=31536000; includeSubDomains"),
                Directive::new("Referrer-Policy")
                    .quoted_arg("strict-origin-when-cross-origin"),
            ])],
        })
        .site(
            SiteBlock::new("example.com")
                .address("www.example.com")
                .directive(Directive::new("import").arg("security_headers"))
                .directive(
                    Directive::new("handle")
                        .matcher(Matcher::Path("/api/*".to_string()))
                        .block(vec![Directive::new("reverse_proxy").block(vec![
                            Directive::new("to").arg("api1:8080"),
                            Directive::new("to").arg("api2:8080"),
                            Directive::new("lb_policy").arg("round_robin"),
                            Directive::new("health_uri").arg("/healthz"),
                        ])]),
                )
                .directive(Directive::new("handle").block(vec![
                    Directive::new("root")
                        .matcher(Matcher::All)
                        .arg("/srv/public"),
                    Directive::new("file_server"),
                ]))
                .directive(Directive::new("log").block(vec![Directive::new("output")
                    .arg("file")
                    .arg("/var/log/caddy/example.log")
                    .block(vec![
                        Directive::new("roll_size").arg("50MiB"),
                        Directive::new("roll_keep").arg("5"),
                    ])])),
        )
        .site(
            SiteBlock::new("admin.example.com")
                .directive(Directive::new("import").arg("security_headers"))
                .directive(
                    Directive::new("basic_auth")
                        .block(vec![Directive::new("admin").arg("$2a$14$hash")]),
                )
                .reverse_proxy("admin:3001")
                .log(),
        );

    let output = format(&cf);

    assert!(output.starts_with('{'));
    assert!(output.contains("email ops@example.com"));
    assert!(output.contains("(security_headers) {"));
    assert!(output.contains("example.com, www.example.com {"));
    assert!(output.contains("admin.example.com {"));
    assert!(output.contains("handle /api/*"));
    assert!(output.contains("lb_policy round_robin"));
    assert!(output.contains("root * /srv/public"));
    assert!(output.contains("file_server"));
    assert!(output.contains("roll_size 50MiB"));
    assert!(output.contains("basic_auth {"));

    let parsed = parse_str(&output).unwrap();
    assert!(parsed.global_options.is_some());
    assert_eq!(parsed.snippets.len(), 1);
    assert_eq!(parsed.sites.len(), 2);
    assert_eq!(parsed.sites[0].addresses.len(), 2);

    let reformatted = format(&parsed);
    assert_eq!(output, reformatted);
}

#[test]
fn builder_microservices_gateway() {
    let cf = Caddyfile::new()
        .global(GlobalOptions {
            directives: vec![
                Directive::new("email").arg("devops@example.com"),
                Directive::new("admin").arg("off"),
            ],
        })
        .snippet(Snippet {
            name: "common".to_string(),
            directives: vec![
                Directive::new("encode").arg("gzip").arg("zstd"),
                Directive::new("log").block(vec![
                    Directive::new("output").arg("stderr"),
                    Directive::new("format").arg("json"),
                ]),
                Directive::new("header").block(vec![
                    Directive::new("-Server"),
                    Directive::new("X-Content-Type-Options").quoted_arg("nosniff"),
                ]),
            ],
        })
        .site(
            SiteBlock::new("api.example.com")
                .directive(Directive::new("import").arg("common"))
                .directive(Directive::new("reverse_proxy").block(vec![
                    Directive::new("to").arg("api1:8080"),
                    Directive::new("to").arg("api2:8080"),
                    Directive::new("lb_policy").arg("least_conn"),
                    Directive::new("health_uri").arg("/healthz"),
                ])),
        )
        .site(
            SiteBlock::new("web.example.com")
                .directive(Directive::new("import").arg("common"))
                .directive(Directive::new("root").matcher(Matcher::All).arg("/srv/web"))
                .file_server(),
        )
        .site(
            SiteBlock::new("admin.example.com")
                .directive(Directive::new("import").arg("common"))
                .directive(
                    Directive::new("basic_auth")
                        .block(vec![Directive::new("admin").arg("$2a$14$hash")]),
                )
                .reverse_proxy("admin:3001"),
        );

    let output = format(&cf);
    let parsed = parse_str(&output).unwrap();

    assert!(parsed.global_options.is_some());
    assert_eq!(parsed.snippets.len(), 1);
    assert_eq!(parsed.snippets[0].name, "common");
    assert_eq!(parsed.sites.len(), 3);
    assert_eq!(parsed.sites[0].addresses[0].host, "api.example.com");
    assert_eq!(parsed.sites[1].addresses[0].host, "web.example.com");
    assert_eq!(parsed.sites[2].addresses[0].host, "admin.example.com");

    let reformatted = format(&parsed);
    assert_eq!(output, reformatted);
}
