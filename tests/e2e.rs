//! End-to-end tests ported from the Go reference implementation
//! at `github.com/caddyserver/caddy/caddyconfig/caddyfile/`.

use caddyfile_rs::{
    Address, Argument, Caddyfile, Directive, GlobalOptions, LexErrorKind, Matcher, NamedRoute,
    ParseErrorKind, Scheme, SiteBlock, Snippet, format, parse, parse_str, tokenize,
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

// -----------------------------------------------------------
// Complicated round-trip tests: real-world Caddy patterns
// that exercise deep nesting, multi-address blocks,
// env vars, heredocs, snippets, handle/route blocks, etc.
// -----------------------------------------------------------

#[test]
fn roundtrip_deep_nesting_three_levels() {
    // handle > route > reverse_proxy with sub-block
    roundtrip(
        "example.com {\n\
         \thandle /api/* {\n\
         \t\troute {\n\
         \t\t\treverse_proxy {\n\
         \t\t\t\tto app:3000\n\
         \t\t\t\thealth_uri /healthz\n\
         \t\t\t\thealth_interval 10s\n\
         \t\t\t}\n\
         \t\t}\n\
         \t}\n\
         }\n",
    );
}

#[test]
fn roundtrip_multiple_addresses_comma_separated() {
    roundtrip(
        "example.com, www.example.com {\n\
         \treverse_proxy app:3000\n\
         }\n",
    );
}

#[test]
fn roundtrip_scheme_host_port_path_address() {
    roundtrip(
        "https://example.com:8443 {\n\
         \treverse_proxy app:3000\n\
         }\n",
    );
}

#[test]
fn roundtrip_global_options_complex() {
    roundtrip(
        "{\n\
         \temail admin@example.com\n\
         \tacme_ca https://acme.example.com/directory\n\
         \torder authenticate before respond\n\
         \tservers {\n\
         \t\tprotocols h1 h2 h3\n\
         \t}\n\
         }\n\
         \n\
         example.com {\n\
         \tlog\n\
         }\n",
    );
}

#[test]
fn roundtrip_snippet_with_nested_block() {
    roundtrip(
        "(security) {\n\
         \theader {\n\
         \t\tX-Content-Type-Options \"nosniff\"\n\
         \t\tX-Frame-Options \"DENY\"\n\
         \t\tStrict-Transport-Security \
         \"max-age=31536000; includeSubDomains\"\n\
         \t}\n\
         }\n\
         \n\
         example.com {\n\
         \timport security\n\
         \treverse_proxy app:3000\n\
         }\n",
    );
}

#[test]
fn roundtrip_handle_and_respond_blocks() {
    roundtrip(
        "example.com {\n\
         \thandle /api/* {\n\
         \t\treverse_proxy api:8080\n\
         \t}\n\
         \n\
         \thandle {\n\
         \t\troot * /srv\n\
         \t\tfile_server\n\
         \t}\n\
         }\n",
    );
}

#[test]
fn roundtrip_reverse_proxy_with_lb_and_health() {
    roundtrip(
        "example.com {\n\
         \treverse_proxy {\n\
         \t\tto app1:3000\n\
         \t\tto app2:3000\n\
         \t\tto app3:3000\n\
         \t\tlb_policy round_robin\n\
         \t\thealth_uri /healthz\n\
         \t\thealth_interval 30s\n\
         \t\thealth_timeout 5s\n\
         \t\tfail_duration 30s\n\
         \t}\n\
         }\n",
    );
}

#[test]
fn roundtrip_tls_with_sub_block() {
    roundtrip(
        "example.com {\n\
         \ttls {\n\
         \t\tprotocols tls1.2 tls1.3\n\
         \t\tciphers TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384\n\
         \t\tcurves x25519\n\
         \t}\n\
         \n\
         \treverse_proxy app:3000\n\
         }\n",
    );
}

#[test]
fn roundtrip_log_with_output_and_format() {
    // Blank lines appear around directives with sub-blocks
    roundtrip(
        "example.com {\n\
         \tlog {\n\
         \t\toutput file /var/log/caddy/access.log {\n\
         \t\t\troll_size 100MiB\n\
         \t\t\troll_keep 10\n\
         \t\t\troll_keep_for 720h\n\
         \t\t}\n\
         \n\
         \t\tformat json {\n\
         \t\t\ttime_format iso8601\n\
         \t\t}\n\
         \n\
         \t\tlevel INFO\n\
         \t}\n\
         }\n",
    );
}

#[test]
fn roundtrip_matchers_with_handle() {
    roundtrip(
        "example.com {\n\
         \t@websocket {\n\
         \t\theader Connection *Upgrade*\n\
         \t\theader Upgrade websocket\n\
         \t}\n\
         \n\
         \treverse_proxy @websocket ws:8080\n\
         \treverse_proxy app:3000\n\
         }\n",
    );
}

#[test]
fn roundtrip_env_var_in_address_and_directive() {
    roundtrip(
        "{$DOMAIN:example.com} {\n\
         \treverse_proxy {$UPSTREAM:app:3000}\n\
         }\n",
    );
}

#[test]
fn roundtrip_redir_and_rewrite() {
    roundtrip(
        "example.com {\n\
         \tredir /old /new permanent\n\
         \trewrite /legacy/* /v2{path}\n\
         \treverse_proxy app:3000\n\
         }\n",
    );
}

#[test]
fn roundtrip_respond_with_heredoc() {
    roundtrip(
        "example.com {\n\
         \trespond /health <<EOF\n\
         {\"status\":\"ok\"}\n\
         EOF\n\
         }\n",
    );
}

#[test]
fn roundtrip_multiple_snippets_and_sites() {
    roundtrip(
        "(logging) {\n\
         \tlog {\n\
         \t\toutput stderr\n\
         \t\tformat console\n\
         \t}\n\
         }\n\
         \n\
         (compression) {\n\
         \tencode gzip zstd\n\
         }\n\
         \n\
         app.example.com {\n\
         \timport logging\n\
         \timport compression\n\
         \treverse_proxy app:3000\n\
         }\n\
         \n\
         api.example.com {\n\
         \timport logging\n\
         \treverse_proxy api:8080\n\
         }\n",
    );
}

#[test]
fn roundtrip_named_route_and_invoke() {
    roundtrip(
        "&(myauth) {\n\
         \tbasic_auth {\n\
         \t\tadmin $2a$14$hashedpassword\n\
         \t}\n\
         }\n\
         \n\
         admin.example.com {\n\
         \tinvoke myauth\n\
         \treverse_proxy admin:3001\n\
         }\n",
    );
}

#[test]
fn roundtrip_rate_limiting_pattern() {
    // Use quoted placeholder to avoid lexer splitting {remote_host}
    // into brace tokens (known limitation: bare Caddy placeholders
    // require quoting in the current lexer)
    roundtrip(
        "api.example.com {\n\
         \trate_limit {\n\
         \t\tzone api_zone {\n\
         \t\t\tkey \"{remote_host}\"\n\
         \t\t\tevents 100\n\
         \t\t\twindow 1m\n\
         \t\t}\n\
         \t}\n\
         \n\
         \treverse_proxy api:8080\n\
         }\n",
    );
}

#[test]
fn roundtrip_cors_headers_pattern() {
    roundtrip(
        "api.example.com {\n\
         \theader {\n\
         \t\tAccess-Control-Allow-Origin \"https://example.com\"\n\
         \t\tAccess-Control-Allow-Methods \"GET, POST, OPTIONS\"\n\
         \t\tAccess-Control-Allow-Headers \"Content-Type, Authorization\"\n\
         \t\tAccess-Control-Max-Age \"86400\"\n\
         \t}\n\
         \n\
         \treverse_proxy api:8080\n\
         }\n",
    );
}

#[test]
fn roundtrip_php_fastcgi_pattern() {
    // Blank line before and after directive with sub-block
    roundtrip(
        "example.com {\n\
         \troot * /srv/public\n\
         \n\
         \tphp_fastcgi unix//run/php/php-fpm.sock {\n\
         \t\troot /srv/public\n\
         \t\tindex index.php\n\
         \t}\n\
         \n\
         \tfile_server\n\
         }\n",
    );
}

#[test]
fn roundtrip_wildcard_and_on_demand_tls() {
    roundtrip(
        "*.example.com {\n\
         \ttls {\n\
         \t\ton_demand\n\
         \t}\n\
         \n\
         \treverse_proxy app:3000\n\
         }\n",
    );
}

// -----------------------------------------------------------
// Complicated builder tests: programmatic construction
// of real-world configurations.
// -----------------------------------------------------------

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
    // Use quoted_arg for Caddy placeholders to avoid lexer
    // splitting bare {path} into brace tokens
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
                    .block(vec![
                        Directive::new("time_format").arg("iso8601"),
                    ]),
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
                Directive::new("X-Content-Type-Options")
                    .quoted_arg("nosniff"),
                Directive::new("X-Frame-Options")
                    .quoted_arg("DENY"),
                Directive::new("Strict-Transport-Security")
                    .quoted_arg(
                        "max-age=31536000; includeSubDomains",
                    ),
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
                .directive(Directive::new("log").block(vec![
                        Directive::new("output")
                            .arg("file")
                            .arg("/var/log/caddy/example.log")
                            .block(vec![
                                Directive::new("roll_size")
                                    .arg("50MiB"),
                                Directive::new("roll_keep")
                                    .arg("5"),
                            ]),
                    ])),
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

    // Verify overall structure
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

    // Verify it round-trips cleanly
    let parsed = parse_str(&output).unwrap();
    assert!(parsed.global_options.is_some());
    assert_eq!(parsed.snippets.len(), 1);
    assert_eq!(parsed.sites.len(), 2);
    assert_eq!(parsed.sites[0].addresses.len(), 2);

    // Re-format and check stability
    let reformatted = format(&parsed);
    assert_eq!(output, reformatted);
}

// -----------------------------------------------------------
// AST direct construction and Display impls.
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
// parse_str error propagation.
// -----------------------------------------------------------

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

// -----------------------------------------------------------
// Go reference: inline placeholders (embedded in words survive
// the lexer because braces appear mid-word, not standalone).
// -----------------------------------------------------------

#[test]
fn roundtrip_inline_placeholder_in_rewrite() {
    // {uri} embedded in /v2{uri} is consumed as a single word
    roundtrip(
        "example.com {\n\
         \trewrite /old{uri} /new{uri}\n\
         }\n",
    );
}

#[test]
fn roundtrip_inline_placeholder_path_suffix() {
    roundtrip(
        "example.com {\n\
         \treverse_proxy /api{path} app:3000\n\
         }\n",
    );
}

#[test]
fn roundtrip_env_var_as_address() {
    // Env var with default used as address
    roundtrip(
        "{$SITE_ADDR:localhost:8080} {\n\
         \trespond \"hello\"\n\
         }\n",
    );
}

#[test]
fn roundtrip_multiple_env_vars() {
    roundtrip(
        "{$DOMAIN} {\n\
         \ttls {$TLS_EMAIL}\n\
         \treverse_proxy {$UPSTREAM}\n\
         }\n",
    );
}

// -----------------------------------------------------------
// Go reference: line continuations.
// -----------------------------------------------------------

#[test]
fn lex_line_continuation_crlf() {
    let tokens = tokenize("reverse_proxy \\\r\napp:3000").expect("tokenize");
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].text, "reverse_proxy");
    assert_eq!(tokens[1].text, "app:3000");
}

// -----------------------------------------------------------
// Go reference: BOM handling.
// -----------------------------------------------------------

#[test]
fn roundtrip_bom_input() {
    // BOM at start should be stripped by lexer
    let input = "\u{FEFF}example.com {\n\tlog\n}\n";
    let tokens = tokenize(input).expect("tokenize");
    let cf = parse(&tokens).expect("parse");
    assert_eq!(cf.sites[0].addresses[0].host, "example.com");
    let output = format(&cf);
    // Output won't include BOM, just verify it parses
    assert!(output.contains("example.com {"));
}

// -----------------------------------------------------------
// Complex nested configurations.
// -----------------------------------------------------------

#[test]
fn roundtrip_four_level_nesting() {
    // handle > route > reverse_proxy > transport
    roundtrip(
        "example.com {\n\
         \thandle /api/* {\n\
         \t\troute {\n\
         \t\t\treverse_proxy {\n\
         \t\t\t\ttransport http {\n\
         \t\t\t\t\ttls_insecure_skip_verify\n\
         \t\t\t\t\tread_timeout 30s\n\
         \t\t\t\t}\n\
         \n\
         \t\t\t\tto backend:8080\n\
         \t\t\t}\n\
         \t\t}\n\
         \t}\n\
         }\n",
    );
}

#[test]
fn roundtrip_complex_matcher_definition() {
    // Named matcher with path and header conditions
    roundtrip(
        "example.com {\n\
         \t@api {\n\
         \t\tpath /api/*\n\
         \t\theader Accept application/json\n\
         \t}\n\
         \n\
         \t@static {\n\
         \t\tpath /static/*\n\
         \t\tpath /assets/*\n\
         \t}\n\
         \n\
         \treverse_proxy @api api:8080\n\
         \tfile_server @static\n\
         }\n",
    );
}

#[test]
fn roundtrip_map_directive() {
    // Map directive with sub-block
    roundtrip(
        "example.com {\n\
         \tmap host subdomain {\n\
         \t\tapp.example.com app\n\
         \t\tapi.example.com api\n\
         \t\tdefault web\n\
         \t}\n\
         \n\
         \treverse_proxy app:3000\n\
         }\n",
    );
}

#[test]
fn roundtrip_respond_with_status() {
    roundtrip(
        "example.com {\n\
         \trespond /health 200\n\
         \trespond /ready 200\n\
         \trespond * 404\n\
         }\n",
    );
}

#[test]
fn roundtrip_abort_and_error() {
    roundtrip(
        "example.com {\n\
         \tabort /blocked\n\
         \terror /deprecated 410\n\
         }\n",
    );
}

// -----------------------------------------------------------
// Go reference: comments everywhere.
// -----------------------------------------------------------

#[test]
fn roundtrip_with_comments() {
    // Comments are stripped by parser, so parse->format won't
    // include them. Verify parsing works with comments present.
    let input = "\
# Main site configuration
example.com {
\t# Enable logging
\tlog
\t# Proxy to backend
\treverse_proxy app:3000
}
";
    let tokens = tokenize(input).expect("tokenize");
    let cf = parse(&tokens).expect("parse");
    assert_eq!(cf.sites[0].directives.len(), 2);
    assert_eq!(cf.sites[0].directives[0].name, "log");
    assert_eq!(cf.sites[0].directives[1].name, "reverse_proxy");
}

// -----------------------------------------------------------
// Complex global options patterns.
// -----------------------------------------------------------

#[test]
fn roundtrip_global_options_with_admin_off() {
    roundtrip(
        "{\n\
         \tadmin off\n\
         }\n\
         \n\
         :80 {\n\
         \trespond \"Hello, world!\"\n\
         }\n",
    );
}

#[test]
fn roundtrip_global_storage_and_logging() {
    roundtrip(
        "{\n\
         \temail admin@example.com\n\
         \tstorage file_system /data/caddy\n\
         \tlog {\n\
         \t\toutput stderr\n\
         \t\tformat console\n\
         \t\tlevel ERROR\n\
         \t}\n\
         }\n\
         \n\
         example.com {\n\
         \treverse_proxy app:3000\n\
         }\n",
    );
}

// -----------------------------------------------------------
// Multi-site with diverse features.
// -----------------------------------------------------------

#[test]
fn roundtrip_five_sites() {
    roundtrip(
        "a.com {\n\
         \tlog\n\
         }\n\
         \n\
         b.com {\n\
         \tlog\n\
         }\n\
         \n\
         c.com {\n\
         \tlog\n\
         }\n\
         \n\
         d.com {\n\
         \tlog\n\
         }\n\
         \n\
         e.com {\n\
         \tlog\n\
         }\n",
    );
}

#[test]
fn roundtrip_site_with_many_directives_mixed_blocks() {
    // Mix of block and non-block directives triggers
    // blank-line spacing logic
    roundtrip(
        "example.com {\n\
         \tencode gzip\n\
         \n\
         \theader {\n\
         \t\tX-Frame-Options \"DENY\"\n\
         \t\tX-Content-Type-Options \"nosniff\"\n\
         \t}\n\
         \n\
         \ttls internal\n\
         \n\
         \tlog {\n\
         \t\toutput stderr\n\
         \t}\n\
         \n\
         \treverse_proxy app:3000\n\
         }\n",
    );
}

// -----------------------------------------------------------
// Backtick strings.
// -----------------------------------------------------------

#[test]
fn roundtrip_backtick_in_respond() {
    roundtrip(
        "example.com {\n\
         \trespond `{\"status\":\"ok\"}`\n\
         }\n",
    );
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

// -----------------------------------------------------------
// Heredoc edge cases.
// -----------------------------------------------------------

#[test]
fn roundtrip_heredoc_html_page() {
    roundtrip(
        "example.com {\n\
         \trespond <<HTML\n\
         <!DOCTYPE html>\n\
         <html><body><h1>Hello</h1></body></html>\n\
         HTML\n\
         }\n",
    );
}

#[test]
fn lex_heredoc_with_marker_like_content() {
    // Content contains text that looks like marker but isn't
    // (because it's not on its own line)
    let input = "respond <<MARKER\nThis MARKER is inside content\nMARKER\n";
    let tokens = tokenize(input).expect("tokenize");
    assert_eq!(tokens[1].text, "This MARKER is inside content");
}

// -----------------------------------------------------------
// Address parsing edge cases.
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
fn roundtrip_bare_port_address() {
    roundtrip(
        ":8080 {\n\
         \trespond \"hello\"\n\
         }\n",
    );
}

// -----------------------------------------------------------
// Complex real-world patterns.
// -----------------------------------------------------------

#[test]
fn roundtrip_basicauth_with_acme_exclusion() {
    roundtrip(
        "example.com {\n\
         \t@notacme not path /.well-known/acme-challenge/*\n\
         \n\
         \tbasic_auth @notacme {\n\
         \t\tadmin $2a$14$Zkx19XLiW6VYouLHR5NmfOFU0z2GTNmpkT\n\
         \t}\n\
         \n\
         \treverse_proxy app:3000\n\
         }\n",
    );
}

#[test]
fn roundtrip_encode_multiple_algorithms() {
    roundtrip(
        "example.com {\n\
         \tencode zstd gzip\n\
         \treverse_proxy app:3000\n\
         }\n",
    );
}

#[test]
fn roundtrip_request_body_max_size() {
    roundtrip(
        "example.com {\n\
         \trequest_body {\n\
         \t\tmax_size 10MB\n\
         \t}\n\
         \n\
         \treverse_proxy app:3000\n\
         }\n",
    );
}

#[test]
fn roundtrip_header_delete_operations() {
    roundtrip(
        "example.com {\n\
         \theader {\n\
         \t\t-Server\n\
         \t\t-X-Powered-By\n\
         \t\tX-Robots-Tag \"noindex, nofollow\"\n\
         \t}\n\
         \n\
         \treverse_proxy app:3000\n\
         }\n",
    );
}

#[test]
fn roundtrip_route_with_multiple_handles() {
    roundtrip(
        "example.com {\n\
         \troute {\n\
         \t\t@api path /api/*\n\
         \n\
         \t\thandle @api {\n\
         \t\t\treverse_proxy api:8080\n\
         \t\t}\n\
         \n\
         \t\thandle {\n\
         \t\t\troot * /srv\n\
         \t\t\tfile_server\n\
         \t\t}\n\
         \t}\n\
         }\n",
    );
}

// -----------------------------------------------------------
// Builder: complex multi-snippet multi-site production config.
// -----------------------------------------------------------

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

    // Re-format and check stability
    let reformatted = format(&parsed);
    assert_eq!(output, reformatted);
}

// -----------------------------------------------------------
// Stability: format(parse(format(build(...)))) is idempotent.
// -----------------------------------------------------------

#[test]
fn idempotent_format_three_rounds() {
    let cf = Caddyfile::new()
        .global(GlobalOptions {
            directives: vec![
                Directive::new("email").arg("admin@example.com"),
                Directive::new("servers")
                    .block(vec![Directive::new("protocols").arg("h1").arg("h2")]),
            ],
        })
        .snippet(Snippet {
            name: "sec".to_string(),
            directives: vec![
                Directive::new("header")
                    .block(vec![Directive::new("X-Frame-Options").quoted_arg("DENY")]),
            ],
        })
        .site(
            SiteBlock::new("example.com")
                .address("www.example.com")
                .directive(Directive::new("import").arg("sec"))
                .directive(
                    Directive::new("handle")
                        .matcher(Matcher::Path("/api/*".to_string()))
                        .block(vec![Directive::new("reverse_proxy").arg("api:8080")]),
                )
                .directive(Directive::new("handle").block(vec![Directive::new("file_server")]))
                .log(),
        );

    let r1 = format(&cf);
    let r2 = format(&parse_str(&r1).unwrap());
    let r3 = format(&parse_str(&r2).unwrap());

    assert_eq!(r1, r2);
    assert_eq!(r2, r3);
}

// -----------------------------------------------------------
// Error edge cases.
// -----------------------------------------------------------

#[test]
fn parse_error_wrong_token_for_brace() {
    let tokens = tokenize("example.com log").expect("tokenize");
    let result = parse(&tokens);
    // No brace — parsed as site with no directives
    // (addresses consume tokens until { or newline)
    assert!(result.is_ok());
}

#[test]
fn lex_error_heredoc_marker_variety() {
    // Heredoc with a long marker name
    let input = "respond <<MYMARKER\nhello\nMYMARKER\n";
    let tokens = tokenize(input).expect("tokenize");
    assert_eq!(tokens[1].text, "hello");
    assert!(matches!(
        &tokens[1].kind,
        caddyfile_rs::TokenKind::Heredoc { marker }
        if marker == "MYMARKER"
    ));
}

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
