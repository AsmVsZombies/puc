use clap::{Parser as ClapParser, Subcommand};
use puc::calc::{self, Equiv, ExplodeKind, SceneArg, Wave};
use puc::parser;
use puc::seml::{self, SemlType};
use std::path::PathBuf;
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
    /// 慢速/快速计算器: extreme coordinate (+ garg safe landing columns)
    Extreme {
        /// Most-advanced realization (default).
        #[arg(long, conflicts_with = "slow")]
        fast: bool,
        /// Least-advanced realization.
        #[arg(long)]
        slow: bool,
        /// Stacked zombie type: garg (default), ladder, or jack.
        #[arg(long, value_enum, default_value_t = calc::extreme::ExtremeType::Garg)]
        r#type: calc::extreme::ExtremeType,
        /// Walk time(s) (cs); multiple = stacked segments
        walk: Vec<i32>,
    },
    /// seml: 解析 seml 文件并运行对应模拟器, 输出整洁表格
    Seml {
        /// 测试类型 (pos/smash/explode/refresh/pogo)
        #[arg(value_enum)]
        r#type: SemlType,
        /// seml 文件路径
        file: PathBuf,
        /// Compact output: omit verbose breakdowns; long tick tables show every 50cs plus endpoints
        #[arg(long)]
        compact: bool,
        /// Strict mode: error on unrecognized header lines instead of skipping them
        #[arg(long)]
        strict: bool,
        /// Also write a CSV export to TARGET (a file path, or a directory to use
        /// the default "<stem> (timestamp).csv" name). Off by default.
        #[arg(long, value_name = "TARGET")]
        csv: Option<PathBuf>,
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
    /// Run an MCP (Model Context Protocol) stdio server exposing every
    /// subcommand above as a tool.
    #[cfg(feature = "mcp")]
    McpServer,
}

fn run_calc(command: Command) -> Result<(), String> {
    match command {
        Command::Intercept { .. } => unreachable!(),
        #[cfg(feature = "mcp")]
        Command::McpServer => unreachable!(),
        Command::Coord {
            time,
            wave,
            kind,
            scene,
            roof_tail,
            x,
            zombies,
        } => {
            let x_override = x.as_deref().map(calc::parse_x_override).transpose()?;
            calc::coord::run(
                time,
                wave,
                kind,
                scene,
                roof_tail,
                x_override,
                zombies.as_deref(),
            )
        }
        Command::Time {
            scene,
            kind,
            row,
            col,
            wave,
            roof_tail,
            zombies,
        } => calc::time::run(scene, kind, row, col, wave, roof_tail, zombies.as_deref()),
        Command::Extreme {
            slow,
            r#type,
            walk,
            ..
        } => {
            let speed = if slow {
                calc::extreme::Speed::Slow
            } else {
                calc::extreme::Speed::Fast
            };
            calc::extreme::run(speed, r#type, &walk)
        }
        Command::Ipp {
            transition,
            wave_len,
            ice,
            equiv,
        } => calc::ipp::run(transition, wave_len, ice, equiv),
        Command::Seml {
            r#type,
            file,
            compact,
            strict,
            csv,
        } => seml::run(r#type, &file, compact, strict, csv.as_deref()),
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let command = match cli.command {
        Command::Intercept { command } => command,
        #[cfg(feature = "mcp")]
        Command::McpServer => {
            return match puc::mcp::serve() {
                Ok(()) => ExitCode::SUCCESS,
                Err(msg) => {
                    eprintln!("error: {}", msg);
                    ExitCode::from(1)
                }
            };
        }
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

    match parser::run_intercept(&command) {
        Ok(()) => ExitCode::SUCCESS,
        Err(()) => ExitCode::from(1),
    }
}
