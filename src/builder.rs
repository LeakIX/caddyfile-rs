use crate::ast::{
    self, Argument, Caddyfile, Directive, GlobalOptions, Matcher, NamedRoute, SiteBlock, Snippet,
};

impl Caddyfile {
    /// Create a new empty Caddyfile.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            global_options: None,
            snippets: Vec::new(),
            named_routes: Vec::new(),
            sites: Vec::new(),
        }
    }

    /// Add a site block.
    #[must_use]
    pub fn site(mut self, block: SiteBlock) -> Self {
        self.sites.push(block);
        self
    }

    /// Set the global options block.
    #[must_use]
    pub fn global(mut self, opts: GlobalOptions) -> Self {
        self.global_options = Some(opts);
        self
    }

    /// Add a snippet.
    #[must_use]
    pub fn snippet(mut self, snippet: Snippet) -> Self {
        self.snippets.push(snippet);
        self
    }

    /// Add a named route.
    #[must_use]
    pub fn named_route(mut self, route: NamedRoute) -> Self {
        self.named_routes.push(route);
        self
    }
}

impl Default for Caddyfile {
    fn default() -> Self {
        Self::new()
    }
}

impl SiteBlock {
    /// Create a new site block with one address.
    #[must_use]
    pub fn new(address: &str) -> Self {
        Self {
            addresses: vec![ast::parse_address(address)],
            directives: Vec::new(),
        }
    }

    /// Add another address to this site block.
    #[must_use]
    pub fn address(mut self, addr: &str) -> Self {
        self.addresses.push(ast::parse_address(addr));
        self
    }

    /// Add a directive to this site block.
    #[must_use]
    pub fn directive(mut self, d: Directive) -> Self {
        self.directives.push(d);
        self
    }

    /// Add a `reverse_proxy` directive.
    #[must_use]
    pub fn reverse_proxy(self, upstream: &str) -> Self {
        self.directive(Directive::new("reverse_proxy").arg(upstream))
    }

    /// Add an `encode gzip` directive.
    #[must_use]
    pub fn encode_gzip(self) -> Self {
        self.directive(Directive::new("encode").arg("gzip"))
    }

    /// Add a `basic_auth` directive with ACME exclusion.
    #[must_use]
    pub fn basic_auth(self, user: &str, hash: &str) -> Self {
        let matcher_directive = Directive::new("@protected")
            .arg("not")
            .arg("path")
            .arg("/.well-known/acme-challenge/*");

        let auth_directive = Directive::new("basic_auth")
            .matcher(Matcher::Named("protected".to_string()))
            .block(vec![Directive::new(user).arg(hash)]);

        self.directive(matcher_directive).directive(auth_directive)
    }

    /// Add security headers.
    #[must_use]
    pub fn security_headers(self) -> Self {
        self.directive(Directive::new("header").block(vec![
            Directive::new("X-Content-Type-Options").quoted_arg("nosniff"),
            Directive::new("X-Frame-Options").quoted_arg("DENY"),
            Directive::new("X-XSS-Protection").quoted_arg("1; mode=block"),
            Directive::new("Referrer-Policy").quoted_arg("strict-origin-when-cross-origin"),
        ]))
    }

    /// Add a `tls` directive with arguments.
    #[must_use]
    pub fn tls(mut self, args: &[&str]) -> Self {
        let mut d = Directive::new("tls");
        for arg in args {
            d = d.arg(arg);
        }
        self.directives.push(d);
        self
    }

    /// Add a `log` directive.
    #[must_use]
    pub fn log(self) -> Self {
        self.directive(Directive::new("log"))
    }

    /// Add a `file_server` directive.
    #[must_use]
    pub fn file_server(self) -> Self {
        self.directive(Directive::new("file_server"))
    }
}

impl Directive {
    /// Create a new directive with the given name.
    #[must_use]
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            matcher: None,
            arguments: Vec::new(),
            block: None,
        }
    }

    /// Set a matcher on this directive.
    #[must_use]
    pub fn matcher(mut self, m: Matcher) -> Self {
        self.matcher = Some(m);
        self
    }

    /// Add an unquoted argument.
    #[must_use]
    pub fn arg(mut self, value: &str) -> Self {
        self.arguments.push(Argument::Unquoted(value.to_string()));
        self
    }

    /// Add a double-quoted argument.
    #[must_use]
    pub fn quoted_arg(mut self, value: &str) -> Self {
        self.arguments.push(Argument::Quoted(value.to_string()));
        self
    }

    /// Set a sub-block of directives.
    #[must_use]
    pub fn block(mut self, directives: Vec<Self>) -> Self {
        self.block = Some(directives);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formatter;

    #[test]
    fn build_simple_site() {
        let cf = Caddyfile::new().site(SiteBlock::new("example.com").reverse_proxy("app:3000"));

        let result = formatter::format(&cf);
        assert!(result.contains("example.com {"));
        assert!(result.contains("reverse_proxy app:3000"));
    }

    #[test]
    fn build_with_auth() {
        let cf =
            Caddyfile::new().site(SiteBlock::new("example.com").basic_auth("admin", "$2a$14$hash"));

        let result = formatter::format(&cf);
        assert!(result.contains("@protected"));
        assert!(result.contains("basic_auth @protected"));
        assert!(result.contains("admin $2a$14$hash"));
    }

    #[test]
    fn build_with_all_features() {
        let cf = Caddyfile::new().site(
            SiteBlock::new("example.com")
                .basic_auth("admin", "$2a$14$hash")
                .reverse_proxy("app:3000")
                .encode_gzip()
                .security_headers()
                .log(),
        );

        let result = formatter::format(&cf);
        assert!(result.contains("basic_auth"));
        assert!(result.contains("reverse_proxy"));
        assert!(result.contains("encode gzip"));
        assert!(result.contains("header {"));
        assert!(result.contains("X-Frame-Options"));
        assert!(result.contains("log"));
    }

    #[test]
    fn build_with_tls() {
        let cf = Caddyfile::new().site(SiteBlock::new("example.com").tls(&["internal"]));

        let result = formatter::format(&cf);
        assert!(result.contains("tls internal"));
    }

    #[test]
    fn build_file_server() {
        let cf = Caddyfile::new().site(SiteBlock::new("example.com").file_server());

        let result = formatter::format(&cf);
        assert!(result.contains("file_server"));
    }

    #[test]
    fn build_with_global_options() {
        let cf = Caddyfile::new()
            .global(GlobalOptions {
                directives: vec![Directive::new("email").arg("admin@example.com")],
            })
            .site(SiteBlock::new("example.com").log());

        let result = formatter::format(&cf);
        assert!(result.starts_with('{'));
        assert!(result.contains("email admin@example.com"));
        assert!(result.contains("example.com {"));
    }

    #[test]
    fn build_default() {
        let cf = Caddyfile::default();
        assert!(cf.global_options.is_none());
        assert!(cf.sites.is_empty());
    }
}
