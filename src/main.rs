use clap::{Parser as ClapParser, Subcommand};
use puc::calc::{self, ExplodeKind, SceneArg, Wave};
use puc::parser;
use puc::seml::{self, SemlType};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(ClapParser)]
#[command(name = "puc", version, about = "植物大战僵尸终极计算器")]
struct Cli {
    /// 输出语言（覆盖 PUC_LANG 环境变量；默认：zh）
    #[arg(long, global = true, value_parser = ["zh", "en"])]
    lang: Option<String>,
    #[command(subcommand)]
    command: Command,
}

/// Resolve the output locale: `--lang` flag > `PUC_LANG` env > default "zh".
/// Only `zh`/`en` are recognized; anything else falls back to the default.
fn resolve_locale(flag: Option<&str>) -> &'static str {
    let pick = |s: &str| match s {
        "en" => Some("en"),
        "zh" => Some("zh"),
        _ => None,
    };
    flag.and_then(pick)
        .or_else(|| std::env::var("PUC_LANG").ok().as_deref().and_then(pick))
        .unwrap_or("zh")
}

#[derive(Subcommand)]
enum Command {
    /// 运行拦截指令（分号分隔可串联）
    Intercept {
        /// 指令字符串，例如 "pe; wave 1 400 800; delay 8.8"
        command: String,
    },
    /// 落点计算器：给定开火时机，对每种僵尸类型计算收上/本/下行的落点列范围
    Coord {
        /// 开火时机 (cs)
        time: i32,
        #[arg(long, value_enum, default_value_t = Wave::Normal)]
        wave: Wave,
        #[arg(long, value_enum, default_value_t = ExplodeKind::Cob)]
        kind: ExplodeKind,
        #[arg(long, value_enum, default_value_t = SceneArg::Pe)]
        scene: SceneArg,
        /// 屋顶炮尾列 (1..=8)，--scene re 时必填
        #[arg(long)]
        roof_tail: Option<i32>,
        /// 覆盖僵尸 x 范围："x" 或 "min,max"
        #[arg(long)]
        x: Option<String>,
        /// 筛选指定僵尸键，逗号分隔
        #[arg(long)]
        zombies: Option<String>,
    },
    /// 时机计算器：给定固定炮/核落点，计算可全收各行僵尸的开火时机窗口。
    Time {
        #[arg(value_enum)]
        scene: SceneArg,
        #[arg(value_enum)]
        kind: ExplodeKind,
        /// 落点行
        row: i32,
        /// 落点列
        col: f64,
        #[arg(long, value_enum, default_value_t = Wave::Normal)]
        wave: Wave,
        #[arg(long)]
        roof_tail: Option<i32>,
        #[arg(long)]
        zombies: Option<String>,
    },
    /// 慢速/快速计算器：给定僵尸类型和多段行走时间，计算极限坐标
    Extreme {
        /// 计算最快僵尸坐标（默认）
        #[arg(long, conflicts_with = "slow")]
        fast: bool,
        /// 计算最慢僵尸坐标（与 fast 互斥）
        #[arg(long)]
        slow: bool,
        /// 僵尸类型：garg（默认）、ladder 或 jack
        #[arg(long, value_enum, default_value_t = calc::extreme::ExtremeType::Garg)]
        r#type: calc::extreme::ExtremeType,
        /// 行走时间 (cs)；多个值表示多段行走时间
        walk: Vec<i32>,
    },
    /// seml: 解析 seml 文件并运行对应模拟器, 输出整洁表格 (reuse 为用炮复用计算)
    Seml {
        /// 测试类型 (pos/smash/explode/refresh/pogo/survive/reuse)
        #[arg(value_enum)]
        r#type: SemlType,
        /// seml 文件路径
        file: PathBuf,
        /// 精简输出：省略冗长明细；长时刻表每 50cs 显示一行并含端点
        #[arg(long)]
        compact: bool,
        /// 严格模式：遇到无法识别的头部行时报错而非跳过
        #[arg(long)]
        strict: bool,
        /// 同时将 CSV 导出到 TARGET（文件路径，或目录则使用默认
        /// "<stem> (timestamp).csv" 名）。默认关闭。
        #[arg(long, value_name = "TARGET")]
        csv: Option<PathBuf>,
    },
    /// 热过渡：计算炸虚落点和同收冰车/矿工的落点范围
    Ipp {
        /// 热过渡时机 (cs)
        transition: i32,
        /// 加速波波长 (cs)。省略则跳过炸虚落点计算。
        #[arg(long)]
        wave_len: Option<i32>,
        /// 用冰时机 (cs)
        #[arg(long, default_value_t = 1)]
        ice: i32,
    },
    /// 运行 MCP（模型上下文协议）stdio 服务器，将以上每个子命令暴露为工具。
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
        } => calc::ipp::run(transition, wave_len, ice),
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
    rust_i18n::set_locale(resolve_locale(cli.lang.as_deref()));

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
