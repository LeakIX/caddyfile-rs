# caddyfile-rs

[![crates.io](https://img.shields.io/crates/v/caddyfile-rs.svg)](https://crates.io/crates/caddyfile-rs)
[![docs](https://img.shields.io/badge/docs-rustdoc-blue)](https://leakix.github.io/caddyfile-rs/)

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

## CLI

Install the `caddyfile` command-line tool:

```sh
cargo install --git https://github.com/LeakIX/caddyfile-rs
```

### Validate

```sh
caddyfile validate Caddyfile
```

### Format

```sh
caddyfile fmt Caddyfile
```

### Check formatting

```sh
caddyfile check Caddyfile
```

### GitHub Actions

Add a workflow to validate your Caddyfile on every push
(see [validate-caddyfile.yaml](.github/workflows/validate-caddyfile.yaml)):

```yaml
name: Validate Caddyfile

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  validate:
    name: Validate Caddyfile
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - name: Install caddyfile CLI
        run: cargo install --git https://github.com/LeakIX/caddyfile-rs
      - name: Validate Caddyfile
        run: caddyfile validate Caddyfile
      - name: Check formatting
        run: caddyfile check Caddyfile
```

## Documentation

See the full API reference on [leakix.github.io/caddyfile-rs](https://leakix.github.io/caddyfile-rs/).

## License

MIT
