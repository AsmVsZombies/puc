use clap::{Parser as ClapParser, Subcommand};
use puc::calc::{self, Equiv, ExplodeKind, SceneArg, Wave};
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
    /// 落点计算器: per-zombie landing-column window at a given firing time
    Coord {
        /// Firing time (cs)
        time: i32,
        #[arg(long, value_enum, default_value_t = Wave::Normal)]
        wave: Wave,
        #[arg(long, value_enum, default_value_t = ExplodeKind::Cob)]
        kind: ExplodeKind,
        #[arg(long, value_enum, default_value_t = SceneArg::Pe)]
        scene: SceneArg,
        /// Roof cob-tail column (1..=8), required for --scene re
        #[arg(long)]
        roof_tail: Option<i32>,
        /// Override zombie x-range: "x" or "min,max"
        #[arg(long)]
        x: Option<String>,
        /// Filter to specific zombie keys, comma-separated
        #[arg(long)]
        zombies: Option<String>,
    },
    /// 时机计算器: firing-time window for a fixed cob/doom placement
    Time {
        #[arg(value_enum)]
        scene: SceneArg,
        #[arg(value_enum)]
        kind: ExplodeKind,
        /// Hit row
        row: i32,
        /// Landing column
        col: f64,
        #[arg(long, value_enum, default_value_t = Wave::Normal)]
        wave: Wave,
        #[arg(long)]
        roof_tail: Option<i32>,
        #[arg(long)]
        zombies: Option<String>,
    },
    /// 慢速/快速计算器: extreme garg coordinate + safe landing columns
    Extreme {
        #[command(subcommand)]
        mode: ExtremeMode,
    },
    /// 热过渡: garg coordinate + car/miner collect columns across the transition
    Ipp {
        /// Transition timing (cs)
        transition: i32,
        /// Accelerated-wave length (cs)
        #[arg(long)]
        wave_len: i32,
        /// Ice timing (cs)
        #[arg(long, default_value_t = 0)]
        ice: i32,
        #[arg(long, value_enum, default_value_t = Equiv::Cob)]
        equiv: Equiv,
    },
}

#[derive(Subcommand)]
enum ExtremeMode {
    /// Slowest garg(s): extreme coordinate + 全收两行/收三 columns
    Slow {
        /// Walk time(s) (cs); multiple = stacked gargs
        walk: Vec<i32>,
    },
    /// Fastest garg: extreme coordinate + 正好不伤 column
    Fast {
        /// Walk time(s) (cs); multiple = stacked gargs
        walk: Vec<i32>,
        #[arg(long)]
        ladder: Option<i32>,
        #[arg(long)]
        clown: Option<i32>,
    },
}

fn parse_x_override(s: &str) -> Result<(i32, i32), String> {
    let parts: Vec<&str> = s.split(',').collect();
    match parts.as_slice() {
        [a] => {
            let v = a.trim().parse::<i32>().map_err(|_| format!("bad --x: {}", s))?;
            Ok((v, v))
        }
        [a, b] => {
            let lo = a.trim().parse::<i32>().map_err(|_| format!("bad --x: {}", s))?;
            let hi = b.trim().parse::<i32>().map_err(|_| format!("bad --x: {}", s))?;
            if lo > hi {
                return Err(format!("bad --x: min > max ({})", s));
            }
            Ok((lo, hi))
        }
        _ => Err(format!("bad --x: {}", s)),
    }
}

fn run_calc(command: Command) -> Result<(), String> {
    match command {
        Command::Intercept { .. } => unreachable!(),
        Command::Coord { time, wave, kind, scene, roof_tail, x, zombies } => {
            let x_override = x.as_deref().map(parse_x_override).transpose()?;
            calc::coord::run(time, wave, kind, scene, roof_tail, x_override, zombies.as_deref())
        }
        Command::Time { scene, kind, row, col, wave, roof_tail, zombies } => {
            calc::time::run(scene, kind, row, col, wave, roof_tail, zombies.as_deref())
        }
        Command::Extreme { mode } => match mode {
            ExtremeMode::Slow { walk } => calc::extreme::run_slow(&walk),
            ExtremeMode::Fast { walk, ladder, clown } => calc::extreme::run_fast(&walk, ladder, clown),
        },
        Command::Ipp { transition, wave_len, ice, equiv } => {
            calc::ipp::run(transition, wave_len, ice, equiv)
        }
    }
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

    let command = match cli.command {
        Command::Intercept { command } => command,
        other => {
            return match run_calc(other) {
                Ok(()) => ExitCode::SUCCESS,
                Err(msg) => {
                    eprintln!("error: {}", msg);
                    ExitCode::from(1)
                }
            };
        }
    };

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
