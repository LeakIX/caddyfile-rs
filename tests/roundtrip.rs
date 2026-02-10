//! Round-trip tests: parse then format should produce the same output.

mod common;

use common::roundtrip;

// -----------------------------------------------------------
// Basic round-trip tests.
// -----------------------------------------------------------

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
// Complex round-trip tests: real-world Caddy patterns.
// -----------------------------------------------------------

#[test]
fn roundtrip_deep_nesting_three_levels() {
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

#[test]
fn roundtrip_inline_placeholder_in_rewrite() {
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

#[test]
fn roundtrip_bom_input() {
    let input = "\u{FEFF}example.com {\n\tlog\n}\n";
    let tokens = caddyfile_rs::tokenize(input).expect("tokenize");
    let cf = caddyfile_rs::parse(&tokens).expect("parse");
    assert_eq!(cf.sites[0].addresses[0].host, "example.com");
    let output = caddyfile_rs::format(&cf);
    assert!(output.contains("example.com {"));
}

#[test]
fn roundtrip_four_level_nesting() {
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

#[test]
fn roundtrip_with_comments() {
    let input = "\
# Main site configuration
example.com {
\t# Enable logging
\tlog
\t# Proxy to backend
\treverse_proxy app:3000
}
";
    let tokens = caddyfile_rs::tokenize(input).expect("tokenize");
    let cf = caddyfile_rs::parse(&tokens).expect("parse");
    assert_eq!(cf.sites[0].directives.len(), 2);
    assert_eq!(cf.sites[0].directives[0].name, "log");
    assert_eq!(cf.sites[0].directives[1].name, "reverse_proxy");
}

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

#[test]
fn roundtrip_backtick_in_respond() {
    roundtrip(
        "example.com {\n\
         \trespond `{\"status\":\"ok\"}`\n\
         }\n",
    );
}

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
fn roundtrip_bare_port_address() {
    roundtrip(
        ":8080 {\n\
         \trespond \"hello\"\n\
         }\n",
    );
}

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
// Multi-level domain tests.
// -----------------------------------------------------------

#[test]
fn roundtrip_deep_subdomain() {
    roundtrip(
        "a.b.c.d.example.com {\n\
         \treverse_proxy app:3000\n\
         }\n",
    );
}

#[test]
fn roundtrip_three_level_subdomain() {
    roundtrip(
        "dev.api.example.com {\n\
         \treverse_proxy api:8080\n\
         }\n",
    );
}

#[test]
fn roundtrip_multi_level_with_wildcard() {
    roundtrip(
        "*.api.example.com {\n\
         \ttls {\n\
         \t\ton_demand\n\
         \t}\n\
         \n\
         \treverse_proxy api:8080\n\
         }\n",
    );
}

#[test]
fn roundtrip_multiple_subdomains_multi_site() {
    roundtrip(
        "api.v1.example.com {\n\
         \treverse_proxy api-v1:8080\n\
         }\n\
         \n\
         api.v2.example.com {\n\
         \treverse_proxy api-v2:8080\n\
         }\n\
         \n\
         staging.app.internal.example.com {\n\
         \treverse_proxy staging:3000\n\
         }\n",
    );
}

// -----------------------------------------------------------
// IPv6 round-trip tests.
// -----------------------------------------------------------

#[test]
fn roundtrip_ipv6_loopback_site() {
    roundtrip(
        "[::1]:8080 {\n\
         \trespond \"hello\"\n\
         }\n",
    );
}

#[test]
fn roundtrip_ipv6_full_address() {
    roundtrip(
        "[2001:db8::1]:443 {\n\
         \treverse_proxy app:3000\n\
         }\n",
    );
}

#[test]
fn roundtrip_ipv6_with_scheme() {
    roundtrip(
        "https://[::1]:8443 {\n\
         \treverse_proxy app:3000\n\
         }\n",
    );
}

#[test]
fn roundtrip_ipv6_bind_directive() {
    roundtrip(
        "example.com {\n\
         \tbind [::] 0.0.0.0\n\
         \treverse_proxy app:3000\n\
         }\n",
    );
}

#[test]
fn roundtrip_ipv6_and_ipv4_multi_address() {
    roundtrip(
        "[::1]:8080, 127.0.0.1:8080 {\n\
         \trespond \"hello\"\n\
         }\n",
    );
}

// -----------------------------------------------------------
// Idempotency: format(parse(format(build(...)))) is stable.
// -----------------------------------------------------------

#[test]
fn idempotent_format_three_rounds() {
    use caddyfile_rs::{
        Caddyfile, Directive, GlobalOptions, Matcher, SiteBlock, Snippet, format, parse_str,
    };

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
