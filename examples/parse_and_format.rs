//! Parse a Caddyfile string and re-format it.

fn main() {
    let input = "\
example.com {
\treverse_proxy app:3000
\tencode gzip
\tlog
}
";

    let caddyfile = caddyfile_rs::parse_str(input).expect("parse failed");

    println!("Sites: {}", caddyfile.sites.len());
    for site in &caddyfile.sites {
        for addr in &site.addresses {
            println!("  Address: {addr}");
        }
        for directive in &site.directives {
            println!("  Directive: {}", directive.name);
        }
    }

    let output = caddyfile_rs::format(&caddyfile);
    println!("\nFormatted output:\n{output}");
}
