use clap::{Parser as ClapParser, Subcommand};
use puc::parser::{ParseResult, Parser};
use std::process::ExitCode;

#[derive(ClapParser)]
#[command(name = "puc", version, about = "PvZ's Ultimate Calculator")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run interception commands (semicolon-separated for chaining)
    Intercept {
        /// Command string, e.g. "pe; wave 1 400 800; delay 8.8"
        command: String,
    },
}

fn dispatch(parser: &mut Parser, input: &str) -> ParseResult {
    let dispatchers: [fn(&mut Parser, &str) -> ParseResult; 7] = [
        Parser::parse_scene,
        Parser::parse_wave,
        Parser::parse_delay,
        Parser::parse_doom,
        Parser::parse_hit_or_nohit,
        Parser::parse_find_max_delay,
        Parser::parse_imp,
    ];
    for d in dispatchers {
        match d(parser, input) {
            ParseResult::Unmatched => continue,
            other => return other,
        }
    }
    ParseResult::Unmatched
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let Command::Intercept { command } = cli.command;

    let mut parser = Parser::default();
    for segment in command.split(';') {
        let line = segment.trim().to_lowercase();
        if line.is_empty() {
            continue;
        }
        match dispatch(&mut parser, &line) {
            ParseResult::Ok => continue,
            ParseResult::Err => return ExitCode::from(1),
            ParseResult::Unmatched => {
                eprintln!("error: unknown command (got: {})", line);
                return ExitCode::from(1);
            }
        }
    }
    ExitCode::SUCCESS
}
