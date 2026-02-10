//! Build a Caddyfile programmatically using the builder API.

use caddyfile_rs::{Caddyfile, Directive, GlobalOptions, Matcher, SiteBlock, Snippet};

fn main() {
    let cf = Caddyfile::new()
        .global(GlobalOptions {
            directives: vec![Directive::new("email").arg("admin@example.com")],
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
                .address("www.example.com")
                .directive(Directive::new("import").arg("security"))
                .directive(
                    Directive::new("handle")
                        .matcher(Matcher::Path("/api/*".to_string()))
                        .block(vec![Directive::new("reverse_proxy").arg("api:8080")]),
                )
                .directive(Directive::new("handle").block(vec![
                    Directive::new("root").matcher(Matcher::All).arg("/srv"),
                    Directive::new("file_server"),
                ]))
                .log(),
        );

    let output = caddyfile_rs::format(&cf);
    println!("{output}");
}
