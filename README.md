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

## Documentation

See the full API reference on [docs.rs](https://docs.rs/caddyfile-rs).

## License

MIT
