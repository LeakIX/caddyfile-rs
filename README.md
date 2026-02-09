# caddyfile-rs

A Rust library for parsing, formatting, and building
[Caddyfile](https://caddyserver.com/docs/caddyfile) configuration files for
the [Caddy](https://caddyserver.com/) web server.

## Features

- **Lexer** - tokenize Caddyfile source text with full span tracking
- **Parser** - parse tokens into a typed AST
- **Formatter** - pretty-print AST back to valid Caddyfile syntax
- **Builder** - programmatic API for constructing Caddyfiles
- **Round-trip safe** - parse then format produces identical output
- Zero dependencies beyond `thiserror`

## Quick start

### Parse and re-format

```rust
use caddyfile_rs::{tokenize, parse, format};

let input = "example.com {\n\treverse_proxy app:3000\n}\n";
let tokens = tokenize(input).unwrap();
let caddyfile = parse(&tokens).unwrap();
let output = format(&caddyfile);
assert_eq!(output, input);
```

### Build programmatically

```rust
use caddyfile_rs::{Caddyfile, SiteBlock, Directive, format};

let cf = Caddyfile::new()
    .site(SiteBlock::new("example.com")
        .reverse_proxy("app:3000")
        .encode_gzip()
        .security_headers()
        .log());

let output = format(&cf);
println!("{output}");
```

Output:

```caddyfile
example.com {
	reverse_proxy app:3000
	encode gzip

	header {
		X-Content-Type-Options "nosniff"
		X-Frame-Options "DENY"
		X-XSS-Protection "1; mode=block"
		Referrer-Policy "strict-origin-when-cross-origin"
	}

	log
}
```

## Supported Caddyfile syntax

- Global options blocks
- Site blocks with multiple addresses
- Directives with arguments and sub-blocks
- Matchers (`*`, `/path`, `@name`)
- Snippets `(name) { ... }`
- Named routes `&(name) { ... }`
- Double-quoted strings with escape sequences
- Backtick-quoted literal strings
- Heredocs `<<MARKER ... MARKER`
- Comments `# ...`
- Environment variables `{$VAR}` and `{$VAR:default}`
- Line continuation with `\`
- BOM stripping

## License

MIT
