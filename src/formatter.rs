use crate::ast::{
    Argument, Caddyfile, Directive, GlobalOptions, Matcher, NamedRoute, Scheme, SiteBlock, Snippet,
};

/// Format a `Caddyfile` AST into a valid Caddyfile string.
///
/// Uses tab-based indentation, blank lines between blocks,
/// and preserves quoting style from `Argument` variants.
#[must_use]
pub fn format(caddyfile: &Caddyfile) -> String {
    let mut out = String::new();
    let mut first_block = caddyfile.global_options.as_ref().is_none_or(|global| {
        format_global_options(&mut out, global);
        false
    });

    for snippet in &caddyfile.snippets {
        if !first_block {
            out.push('\n');
        }
        format_snippet(&mut out, snippet);
        first_block = false;
    }

    for route in &caddyfile.named_routes {
        if !first_block {
            out.push('\n');
        }
        format_named_route(&mut out, route);
        first_block = false;
    }

    for site in &caddyfile.sites {
        if !first_block {
            out.push('\n');
        }
        format_site_block(&mut out, site);
        first_block = false;
    }

    // Trailing newline
    if !out.ends_with('\n') {
        out.push('\n');
    }

    out
}

fn format_global_options(out: &mut String, global: &GlobalOptions) {
    out.push_str("{\n");
    format_directives(out, &global.directives, 1);
    out.push_str("}\n");
}

fn format_snippet(out: &mut String, snippet: &Snippet) {
    out.push('(');
    out.push_str(&snippet.name);
    out.push_str(") {\n");
    format_directives(out, &snippet.directives, 1);
    out.push_str("}\n");
}

fn format_named_route(out: &mut String, route: &NamedRoute) {
    out.push_str("&(");
    out.push_str(&route.name);
    out.push_str(") {\n");
    format_directives(out, &route.directives, 1);
    out.push_str("}\n");
}

fn format_site_block(out: &mut String, site: &SiteBlock) {
    // Addresses
    for (i, addr) in site.addresses.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        format_address(out, addr);
    }

    out.push_str(" {\n");
    format_directives_with_spacing(out, &site.directives, 1);
    out.push_str("}\n");
}

fn format_address(out: &mut String, addr: &super::ast::Address) {
    if let Some(scheme) = &addr.scheme {
        match scheme {
            Scheme::Http => out.push_str("http://"),
            Scheme::Https => out.push_str("https://"),
        }
    }
    out.push_str(&addr.host);
    if let Some(port) = addr.port {
        out.push(':');
        out.push_str(&port.to_string());
    }
    if let Some(path) = &addr.path {
        out.push_str(path);
    }
}

fn format_directives(out: &mut String, directives: &[Directive], indent: usize) {
    for directive in directives {
        format_directive(out, directive, indent);
    }
}

/// Format directives with blank lines between directives
/// that have sub-blocks.
fn format_directives_with_spacing(out: &mut String, directives: &[Directive], indent: usize) {
    let mut prev_had_block = false;

    for (i, directive) in directives.iter().enumerate() {
        let has_block = directive.block.is_some();

        // Blank line before directive with block, or after
        // one that had a block
        if i > 0 && (has_block || prev_had_block) {
            out.push('\n');
        }

        format_directive(out, directive, indent);
        prev_had_block = has_block;
    }
}

fn format_directive(out: &mut String, directive: &Directive, indent: usize) {
    let prefix = "\t".repeat(indent);
    out.push_str(&prefix);
    out.push_str(&directive.name);

    // Matcher
    if let Some(matcher) = &directive.matcher {
        out.push(' ');
        match matcher {
            Matcher::All => out.push('*'),
            Matcher::Path(p) => out.push_str(p),
            Matcher::Named(n) => {
                out.push('@');
                out.push_str(n);
            }
        }
    }

    // Arguments
    for arg in &directive.arguments {
        out.push(' ');
        format_argument(out, arg);
    }

    // Sub-block
    if let Some(block) = &directive.block {
        out.push_str(" {\n");
        format_directives_with_spacing(out, block, indent + 1);
        out.push_str(&prefix);
        out.push_str("}\n");
    } else {
        out.push('\n');
    }
}

fn format_argument(out: &mut String, arg: &Argument) {
    match arg {
        Argument::Unquoted(s) => out.push_str(s),
        Argument::Quoted(s) => {
            out.push('"');
            // Escape special characters
            for ch in s.chars() {
                match ch {
                    '"' => out.push_str("\\\""),
                    '\\' => out.push_str("\\\\"),
                    '\n' => out.push_str("\\n"),
                    '\t' => out.push_str("\\t"),
                    '\r' => out.push_str("\\r"),
                    _ => out.push(ch),
                }
            }
            out.push('"');
        }
        Argument::Backtick(s) => {
            out.push('`');
            out.push_str(s);
            out.push('`');
        }
        Argument::Heredoc { marker, content } => {
            out.push_str("<<");
            out.push_str(marker);
            out.push('\n');
            out.push_str(content);
            out.push('\n');
            out.push_str(marker);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Address;

    #[test]
    fn simple_site() {
        let cf = Caddyfile {
            global_options: None,
            snippets: Vec::new(),
            named_routes: Vec::new(),
            sites: vec![SiteBlock {
                addresses: vec![Address {
                    scheme: None,
                    host: "example.com".to_string(),
                    port: None,
                    path: None,
                }],
                directives: vec![Directive {
                    name: "log".to_string(),
                    matcher: None,
                    arguments: Vec::new(),
                    block: None,
                }],
            }],
        };

        let result = format(&cf);
        assert_eq!(result, "example.com {\n\tlog\n}\n");
    }

    #[test]
    fn directive_with_block_spacing() {
        let cf = Caddyfile {
            global_options: None,
            snippets: Vec::new(),
            named_routes: Vec::new(),
            sites: vec![SiteBlock {
                addresses: vec![Address {
                    scheme: None,
                    host: "example.com".to_string(),
                    port: None,
                    path: None,
                }],
                directives: vec![
                    Directive {
                        name: "encode".to_string(),
                        matcher: None,
                        arguments: vec![Argument::Unquoted("gzip".to_string())],
                        block: None,
                    },
                    Directive {
                        name: "header".to_string(),
                        matcher: None,
                        arguments: Vec::new(),
                        block: Some(vec![Directive {
                            name: "X-Frame-Options".to_string(),
                            matcher: None,
                            arguments: vec![Argument::Quoted("DENY".to_string())],
                            block: None,
                        }]),
                    },
                    Directive {
                        name: "log".to_string(),
                        matcher: None,
                        arguments: Vec::new(),
                        block: None,
                    },
                ],
            }],
        };

        let result = format(&cf);
        let expected = "\
example.com {
\tencode gzip

\theader {
\t\tX-Frame-Options \"DENY\"
\t}

\tlog
}
";
        assert_eq!(result, expected);
    }

    #[test]
    fn global_options_and_site() {
        let cf = Caddyfile {
            global_options: Some(GlobalOptions {
                directives: vec![Directive {
                    name: "email".to_string(),
                    matcher: None,
                    arguments: vec![Argument::Unquoted("admin@example.com".to_string())],
                    block: None,
                }],
            }),
            snippets: Vec::new(),
            named_routes: Vec::new(),
            sites: vec![SiteBlock {
                addresses: vec![Address {
                    scheme: None,
                    host: "example.com".to_string(),
                    port: None,
                    path: None,
                }],
                directives: vec![Directive {
                    name: "log".to_string(),
                    matcher: None,
                    arguments: Vec::new(),
                    block: None,
                }],
            }],
        };

        let result = format(&cf);
        let expected = "\
{
\temail admin@example.com
}

example.com {
\tlog
}
";
        assert_eq!(result, expected);
    }

    #[test]
    fn quoted_argument_escaping() {
        let cf = Caddyfile {
            global_options: None,
            snippets: Vec::new(),
            named_routes: Vec::new(),
            sites: vec![SiteBlock {
                addresses: vec![Address {
                    scheme: None,
                    host: "example.com".to_string(),
                    port: None,
                    path: None,
                }],
                directives: vec![Directive {
                    name: "respond".to_string(),
                    matcher: None,
                    arguments: vec![Argument::Quoted("hello \"world\"".to_string())],
                    block: None,
                }],
            }],
        };

        let result = format(&cf);
        assert!(result.contains("\"hello \\\"world\\\"\""));
    }

    #[test]
    fn address_with_scheme_and_port() {
        let cf = Caddyfile {
            global_options: None,
            snippets: Vec::new(),
            named_routes: Vec::new(),
            sites: vec![SiteBlock {
                addresses: vec![Address {
                    scheme: Some(Scheme::Https),
                    host: "example.com".to_string(),
                    port: Some(443),
                    path: None,
                }],
                directives: Vec::new(),
            }],
        };

        let result = format(&cf);
        assert!(result.contains("https://example.com:443"));
    }

    #[test]
    fn trailing_newline() {
        let cf = Caddyfile {
            global_options: None,
            snippets: Vec::new(),
            named_routes: Vec::new(),
            sites: Vec::new(),
        };

        let result = format(&cf);
        assert!(result.ends_with('\n'));
    }
}
