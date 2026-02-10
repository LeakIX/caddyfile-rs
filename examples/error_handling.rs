//! Demonstrate error handling for invalid Caddyfile input.

fn main() {
    // Unterminated quoted string
    match caddyfile_rs::parse_str("example.com {\n\trespond \"unclosed\n}\n") {
        Ok(_) => println!("Parsed OK (unexpected)"),
        Err(caddyfile_rs::Error::Lex(e)) => {
            println!("Lex error: {e}");
            println!("  Kind: {:?}", e.kind);
            println!("  Location: line {}, column {}", e.span.line, e.span.column);
        }
        Err(caddyfile_rs::Error::Parse(e)) => {
            println!("Parse error: {e}");
        }
    }

    println!();

    // Unclosed brace
    match caddyfile_rs::parse_str("example.com {\n\tlog\n") {
        Ok(_) => println!("Parsed OK (unexpected)"),
        Err(caddyfile_rs::Error::Lex(e)) => {
            println!("Lex error: {e}");
        }
        Err(caddyfile_rs::Error::Parse(e)) => {
            println!("Parse error: {e}");
            println!("  Kind: {:?}", e.kind);
            println!("  Location: line {}, column {}", e.span.line, e.span.column);
        }
    }
}
