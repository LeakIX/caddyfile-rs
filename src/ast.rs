/// Complete Caddyfile document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Caddyfile {
    pub global_options: Option<GlobalOptions>,
    pub snippets: Vec<Snippet>,
    pub named_routes: Vec<NamedRoute>,
    pub sites: Vec<SiteBlock>,
}

/// Global options block (first block, no keys).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalOptions {
    pub directives: Vec<Directive>,
}

/// Reusable snippet: `(name) { ... }`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snippet {
    pub name: String,
    pub directives: Vec<Directive>,
}

/// Named route: `&(name) { ... }`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedRoute {
    pub name: String,
    pub directives: Vec<Directive>,
}

/// Site block: one or more addresses + directives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SiteBlock {
    pub addresses: Vec<Address>,
    pub directives: Vec<Directive>,
}

/// Site address with parsed components.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Address {
    pub scheme: Option<Scheme>,
    pub host: String,
    pub port: Option<u16>,
    pub path: Option<String>,
}

/// URL scheme.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scheme {
    Http,
    Https,
}

/// A directive with optional matcher, arguments, and sub-block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Directive {
    pub name: String,
    pub matcher: Option<Matcher>,
    pub arguments: Vec<Argument>,
    pub block: Option<Vec<Self>>,
}

/// Matcher token after directive name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Matcher {
    /// Wildcard matcher `*`.
    All,
    /// Path matcher `/path`.
    Path(String),
    /// Named matcher `@name`.
    Named(String),
}

/// Argument value preserving its quoting style.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Argument {
    /// Unquoted value.
    Unquoted(String),
    /// Double-quoted value (`"..."`).
    Quoted(String),
    /// Backtick-quoted value (`` `...` ``).
    Backtick(String),
    /// Heredoc value (`<<MARKER ... MARKER`).
    Heredoc { marker: String, content: String },
}

/// Named matcher definition: `@name { ... }` or `@name <matcher>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedMatcher {
    pub name: String,
    pub matchers: Vec<Directive>,
}

impl Argument {
    /// Return the inner value regardless of quoting style.
    #[must_use]
    pub fn value(&self) -> &str {
        match self {
            Self::Unquoted(s) | Self::Quoted(s) | Self::Backtick(s) => s,
            Self::Heredoc { content, .. } => content,
        }
    }
}
