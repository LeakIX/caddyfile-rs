use std::fmt;

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

impl fmt::Display for Scheme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Http => f.write_str("http"),
            Self::Https => f.write_str("https"),
        }
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(scheme) = &self.scheme {
            write!(f, "{scheme}://")?;
        }
        f.write_str(&self.host)?;
        if let Some(port) = self.port {
            write!(f, ":{port}")?;
        }
        if let Some(path) = &self.path {
            f.write_str(path)?;
        }
        Ok(())
    }
}

impl fmt::Display for Matcher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::All => f.write_str("*"),
            Self::Path(p) => f.write_str(p),
            Self::Named(n) => write!(f, "@{n}"),
        }
    }
}

impl fmt::Display for Argument {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unquoted(s) => f.write_str(s),
            Self::Quoted(s) => {
                f.write_str("\"")?;
                for ch in s.chars() {
                    match ch {
                        '"' => f.write_str("\\\"")?,
                        '\\' => f.write_str("\\\\")?,
                        '\n' => f.write_str("\\n")?,
                        '\t' => f.write_str("\\t")?,
                        '\r' => f.write_str("\\r")?,
                        _ => write!(f, "{ch}")?,
                    }
                }
                f.write_str("\"")
            }
            Self::Backtick(s) => write!(f, "`{s}`"),
            Self::Heredoc { marker, content } => {
                write!(f, "<<{marker}\n{content}\n{marker}")
            }
        }
    }
}

/// Parse an address string into its components.
#[must_use]
pub fn parse_address(addr: &str) -> Address {
    let mut remaining = addr;
    let mut scheme = None;

    if let Some(rest) = remaining.strip_prefix("https://") {
        scheme = Some(Scheme::Https);
        remaining = rest;
    } else if let Some(rest) = remaining.strip_prefix("http://") {
        scheme = Some(Scheme::Http);
        remaining = rest;
    }

    let (host_port, path) = remaining.find('/').map_or((remaining, None), |pos| {
        (&remaining[..pos], Some(remaining[pos..].to_string()))
    });

    let (host, port) = host_port.rfind(':').map_or_else(
        || (host_port.to_string(), None),
        |pos| {
            let potential = &host_port[pos + 1..];
            potential.parse::<u16>().map_or_else(
                |_| (host_port.to_string(), None),
                |p| (host_port[..pos].to_string(), Some(p)),
            )
        },
    );

    Address {
        scheme,
        host,
        port,
        path,
    }
}
