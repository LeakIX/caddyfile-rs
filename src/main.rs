//! CLI tool to validate and format Caddyfile configuration files.

use std::fs;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 || args[1] == "--help" || args[1] == "-h" {
        eprintln!("Usage: caddyfile <command> [files...]");
        eprintln!();
        eprintln!("Commands:");
        eprintln!("  validate  Check if Caddyfile(s) are valid");
        eprintln!("  fmt       Format Caddyfile(s) and print to stdout");
        eprintln!("  check     Check if Caddyfile(s) are formatted");
        eprintln!();
        eprintln!("Examples:");
        eprintln!("  caddyfile validate Caddyfile");
        eprintln!("  caddyfile fmt Caddyfile");
        eprintln!("  caddyfile check Caddyfile");
        return ExitCode::from(2);
    }

    let command = args[1].as_str();
    let files = &args[2..];

    if files.is_empty() {
        eprintln!("Error: no files specified");
        return ExitCode::from(2);
    }

    let mut had_error = false;

    for path in files {
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("{path}: {e}");
                had_error = true;
                continue;
            }
        };

        match command {
            "validate" => match caddyfile_rs::parse_str(&content) {
                Ok(cf) => {
                    let sites = cf.sites.len();
                    let snippets = cf.snippets.len();
                    let named_routes = cf.named_routes.len();
                    let global = if cf.global_options.is_some() {
                        ", global options"
                    } else {
                        ""
                    };
                    eprintln!(
                        "{path}: valid ({sites} site(s), \
                         {snippets} snippet(s), \
                         {named_routes} named route(s){global})"
                    );
                }
                Err(e) => {
                    eprintln!("{path}: {e}");
                    had_error = true;
                }
            },
            "fmt" => match caddyfile_rs::parse_str(&content) {
                Ok(cf) => {
                    print!("{}", caddyfile_rs::format(&cf));
                }
                Err(e) => {
                    eprintln!("{path}: {e}");
                    had_error = true;
                }
            },
            "check" => match caddyfile_rs::parse_str(&content) {
                Ok(cf) => {
                    let formatted = caddyfile_rs::format(&cf);
                    if formatted == content {
                        eprintln!("{path}: formatted");
                    } else {
                        eprintln!("{path}: not formatted");
                        had_error = true;
                    }
                }
                Err(e) => {
                    eprintln!("{path}: {e}");
                    had_error = true;
                }
            },
            _ => {
                eprintln!("Unknown command: {command}");
                return ExitCode::from(2);
            }
        }
    }

    if had_error {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
