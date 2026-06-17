//! `puc mcp-server` — expose every other subcommand as a tool over an MCP
//! (Model Context Protocol) stdio server, so an LLM agent can call the
//! calculators directly.
//!
//! Each tool runs the corresponding library entry point inside
//! [`crate::capture`], collecting its stdout/stderr output into the tool result
//! instead of letting it corrupt the stdio JSON-RPC stream. Tool-level failures
//! are returned as `isError` results (model-visible), not protocol errors.

use clap::ValueEnum;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{
    AnnotateAble, CallToolResult, Content, Implementation, ListResourcesResult,
    PaginatedRequestParams, RawResource, ReadResourceRequestParams, ReadResourceResult,
    ResourceContents, ServerCapabilities, ServerInfo,
};
use rmcp::schemars;
use rmcp::service::RequestContext;
use rmcp::transport::stdio;
use rmcp::{tool, tool_handler, tool_router, ErrorData, RoleServer, ServerHandler, ServiceExt};
use serde::Deserialize;

use crate::calc::{self, ExplodeKind, SceneArg, Wave};
use crate::seml::{self, SemlType};

/// Server-level orientation shown to MCP clients on connect.
const INSTRUCTIONS: &str = "植物大战僵尸终极计算器 —— 玉米炮/核武落点与时机计算器、极限巨人坐标、\
    热过渡落点计算、SEML 场景模拟器，以及有状态的拦截指令解释器。时间单位为厘秒 (cs)；列以炮为基准\
    （x 列灰烬 = x+0.0875列炮）。工具输出纯文本；失败通过 isError 返回并附带诊断信息。完整语法参考以\
    资源形式暴露：读取 puc://docs/intercept 获取 puc_intercept 指令语法，读取 puc://docs/seml \
    获取 SEML 语法。";

const INTERCEPT_URI: &str = "puc://docs/intercept";
const SEML_URI: &str = "puc://docs/seml";

/// Grammar docs are baked into the binary so the server is self-contained.
const INTERCEPT_DOC: &str = include_str!("../doc/intercept.md");
const SEML_DOC: &str = include_str!("../doc/seml.md");

/// Run the MCP stdio server, blocking until the client disconnects.
pub fn serve() -> Result<(), String> {
    let rt = tokio::runtime::Runtime::new().map_err(|e| format!("tokio runtime: {e}"))?;
    rt.block_on(async {
        let service = PucServer
            .serve(stdio())
            .await
            .map_err(|e| format!("serve: {e}"))?;
        service.waiting().await.map_err(|e| format!("waiting: {e}"))?;
        Ok(())
    })
}

#[derive(Clone)]
struct PucServer;

// --- parameter structs ------------------------------------------------------
// Enum-valued arguments (scene/kind/wave/equiv/type) are taken as strings and
// converted via the existing clap `ValueEnum`s, so the schema stays decoupled
// from the calculator enums; allowed values are documented per field.

#[derive(Deserialize, schemars::JsonSchema)]
struct InterceptParams {
    /// 分号分隔的拦截指令，例如
    /// "pe; wave 1 400 800; delay 8.8"。
    command: String,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct CoordParams {
    /// 开火时机，单位厘秒（>= 0）。
    time: i32,
    /// 波类型："normal" 或 "flag"。默认 "normal"。
    #[serde(default)]
    wave: Option<String>,
    /// 爆炸类型："cob" 或 "doom"。默认 "cob"。doom 命中范围更广，输出可达 7 个相对行
    /// （收上3…收本…收下3），落点列以整数格 1..9 显示（屋顶 doom 无需 roof_tail）。
    #[serde(default)]
    kind: Option<String>,
    /// 场地："de"（前院）、"pe"（泳池/后院）、"re"（屋顶）。默认 "pe"。
    #[serde(default)]
    scene: Option<String>,
    /// 屋顶炮尾列 1..=8。scene = "re" 时必填。
    #[serde(default)]
    roof_tail: Option<i32>,
    /// 覆盖僵尸 x 范围："x"（单值）或 "min,max"。
    #[serde(default)]
    x: Option<String>,
    /// 筛选指定僵尸键，逗号分隔。有效键：regular, regular_dc_fast,
    /// regular_dc_slow, pole, newspaper, door, football, dancing, snorkel,
    /// zomboni, dolphin, jack, balloon, digger, digger_reverse, pogo, ladder,
    /// catapult, gargantuar, duck, duck_dc_fast, duck_dc_slow, snorkel_ashore,
    /// dolphin_swim, balloon_ground, pogo_walk（可用键随场地/工具而异）。
    #[serde(default)]
    zombies: Option<String>,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct TimeParams {
    /// 场地："de"（前院）、"pe"（泳池/后院）、"re"（屋顶）。
    scene: String,
    /// 爆炸类型："cob" 或 "doom"。
    kind: String,
    /// 命中行（1..=5，pe 为 1..=6）。
    row: i32,
    /// 落点列。
    col: f64,
    /// 波类型："normal" 或 "flag"。默认 "normal"。
    #[serde(default)]
    wave: Option<String>,
    /// 屋顶炮尾列 1..=8。scene = "re" 时必填。
    #[serde(default)]
    roof_tail: Option<i32>,
    /// 筛选指定僵尸键，逗号分隔。有效键：regular, regular_dc_fast,
    /// regular_dc_slow, pole, newspaper, door, football, dancing, snorkel,
    /// zomboni, dolphin, jack, balloon, digger, digger_reverse, pogo, ladder,
    /// catapult, gargantuar, duck, duck_dc_fast, duck_dc_slow, snorkel_ashore,
    /// dolphin_swim, balloon_ground, pogo_walk（可用键随场地/工具而异）。
    #[serde(default)]
    zombies: Option<String>,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct ExtremeParams {
    /// 行走时间，单位厘秒；多个值表示多段行走时间。
    walk: Vec<i32>,
    /// 实现："fast"（默认，最靠前）或 "slow"（最靠后）。
    #[serde(default)]
    speed: Option<String>,
    /// 僵尸类型："garg"（默认）、"ladder" 或 "jack"。
    #[serde(default)]
    r#type: Option<String>,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct IppParams {
    /// 过渡时机，单位厘秒（>= 0）。
    transition: i32,
    /// 加速波波长，单位厘秒（>= 0）。省略则跳过炸虚落点计算。
    #[serde(default)]
    wave_len: Option<i32>,
    /// 用冰时机 (cs)。默认 1。
    #[serde(default)]
    ice: Option<i32>,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct SemlParams {
    /// 测试类型："pos"、"smash"、"explode"、"refresh"、"pogo"、"survive" 或 "reuse"。
    r#type: String,
    /// SEML 文件路径。提供此项或 `content`（不可同时给出）。
    #[serde(default)]
    file: Option<String>,
    /// 内联 SEML 源。提供此项或 `file`（不可同时给出）。
    #[serde(default)]
    content: Option<String>,
    /// 精简输出（省略冗长明细）。默认 false。
    #[serde(default)]
    compact: bool,
    /// 严格模式：遇到无法识别的头部行时报错而非跳过。默认 false。
    #[serde(default)]
    strict: bool,
    /// 同时导出 CSV。值为文件路径，或目录则使用默认
    /// "<stem> (timestamp).csv" 名。省略则跳过 CSV（默认）。
    #[serde(default)]
    csv: Option<String>,
}

// --- tools ------------------------------------------------------------------

#[tool_router]
impl PucServer {
    #[tool(
        name = "puc_intercept",
        description = "拦截计算器：针对有状态解析器运行分号分隔的拦截指令（scene、wave、delay、doom、hit/nohit、max、imp），例如 \"pe; wave 1 400 800; delay 8.8\"。"
    )]
    fn puc_intercept(&self, Parameters(p): Parameters<InterceptParams>) -> CallToolResult {
        let (res, out, diag) = crate::capture(|| crate::parser::run_intercept(&p.command));
        match res {
            Ok(()) => success(out, diag),
            // run_intercept reports the error through the diagnostic stream.
            Err(()) => CallToolResult::error(vec![Content::text(combine(out, diag))]),
        }
    }

    #[tool(
        name = "puc_coord",
        description = "落点计算器：给定开火时机，对每种僵尸类型计算收上/本/下行的落点列范围。"
    )]
    fn puc_coord(&self, Parameters(p): Parameters<CoordParams>) -> CallToolResult {
        let wave = match opt_enum::<Wave>(p.wave.as_deref(), Wave::Normal, "wave") {
            Ok(v) => v,
            Err(e) => return bad_args(e),
        };
        let kind = match opt_enum::<ExplodeKind>(p.kind.as_deref(), ExplodeKind::Cob, "kind") {
            Ok(v) => v,
            Err(e) => return bad_args(e),
        };
        let scene = match opt_enum::<SceneArg>(p.scene.as_deref(), SceneArg::Pe, "scene") {
            Ok(v) => v,
            Err(e) => return bad_args(e),
        };
        let x_override = match p.x.as_deref().map(calc::parse_x_override).transpose() {
            Ok(v) => v,
            Err(e) => return bad_args(e),
        };
        let zombies = p.zombies.clone();
        finish(crate::capture(|| {
            calc::coord::run(p.time, wave, kind, scene, p.roof_tail, x_override, zombies.as_deref())
        }))
    }

    #[tool(
        name = "puc_time",
        description = "时机计算器：给定固定炮/核落点（scene、kind、row、col），计算可全收各行僵尸的开火时机窗口。"
    )]
    fn puc_time(&self, Parameters(p): Parameters<TimeParams>) -> CallToolResult {
        let scene = match req_enum::<SceneArg>(&p.scene, "scene") {
            Ok(v) => v,
            Err(e) => return bad_args(e),
        };
        let kind = match req_enum::<ExplodeKind>(&p.kind, "kind") {
            Ok(v) => v,
            Err(e) => return bad_args(e),
        };
        let wave = match opt_enum::<Wave>(p.wave.as_deref(), Wave::Normal, "wave") {
            Ok(v) => v,
            Err(e) => return bad_args(e),
        };
        let zombies = p.zombies.clone();
        finish(crate::capture(|| {
            calc::time::run(scene, kind, p.row, p.col, wave, p.roof_tail, zombies.as_deref())
        }))
    }

    #[tool(
        name = "puc_extreme",
        description = "慢速/快速计算器：给定僵尸类型（garg/ladder/jack；默认 garg）和多段行走时间，计算极限坐标。speed=\"fast\"（默认）给出最靠前坐标 + 巨人正好不伤列；speed=\"slow\" 给出最靠后坐标 + 巨人安全落点列（全收两行 / 后院收三 / 前院收三）。ladder/jack 仅给出坐标。"
    )]
    fn puc_extreme(&self, Parameters(p): Parameters<ExtremeParams>) -> CallToolResult {
        let speed = match opt_enum::<calc::extreme::Speed>(
            p.speed.as_deref(),
            calc::extreme::Speed::Fast,
            "speed",
        ) {
            Ok(v) => v,
            Err(e) => return bad_args(e),
        };
        let ty = match opt_enum::<calc::extreme::ExtremeType>(
            p.r#type.as_deref(),
            calc::extreme::ExtremeType::Garg,
            "type",
        ) {
            Ok(v) => v,
            Err(e) => return bad_args(e),
        };
        finish(crate::capture(|| calc::extreme::run(speed, ty, &p.walk)))
    }

    #[tool(
        name = "puc_ipp",
        description = "热过渡：计算炸虚落点和同收冰车/矿工的落点范围。"
    )]
    fn puc_ipp(&self, Parameters(p): Parameters<IppParams>) -> CallToolResult {
        let ice = p.ice.unwrap_or(1);
        finish(crate::capture(|| calc::ipp::run(p.transition, p.wave_len, ice)))
    }

    #[tool(
        name = "puc_seml",
        description = "seml：解析 SEML 场景并运行对应测试。类型 `pos`/`smash`/`explode`/`refresh`/`pogo`/`survive` 运行 PvZ 模拟器并打印整洁表格（`survive` 对每波放入 5× 各场景允许的僵尸类型，报告各类型受击率与受击/未受击均血；受击定义为死亡或受创 ≥ `hitThres`，默认 1800）；类型 `reuse` 为炮复用计算器（纯时机计算，不经模拟器，参数详见 puc://docs/seml）。提供 `content`（内联 SEML）或 `file`（路径），两者不可同时给出。设置 `csv` 可同时导出 CSV（文件路径，或目录则使用 `<stem> (timestamp).csv` 名）。"
    )]
    fn puc_seml(&self, Parameters(p): Parameters<SemlParams>) -> CallToolResult {
        let kind = match req_enum::<SemlType>(&p.r#type, "type") {
            Ok(v) => v,
            Err(e) => return bad_args(e),
        };
        let compact = p.compact;
        let strict = p.strict;
        let csv = p.csv.clone();
        if p.content.is_some() && p.file.is_some() {
            return bad_args(
                "`content`（内联 SEML）与 `file`（路径）不能同时提供".to_string(),
            );
        }
        if let Some(text) = p.content.clone() {
            // Inline content has no file, so a directory CSV target is named after
            // the calculator type.
            finish(crate::capture(move || {
                let target = csv.as_deref().map(|c| seml::CsvTarget {
                    path: std::path::Path::new(c),
                    default_stem: kind.as_str(),
                });
                seml::run_text(kind, &text, compact, strict, target)
            }))
        } else if let Some(path) = p.file.clone() {
            finish(crate::capture(move || {
                let csv_path = csv.as_deref().map(std::path::Path::new);
                seml::run(kind, std::path::Path::new(&path), compact, strict, csv_path)
            }))
        } else {
            bad_args("请提供 `content`（内联 SEML）或 `file`（路径）".to_string())
        }
    }
}

// `#[tool_handler]` injects `call_tool`/`list_tools`/`get_tool`. We supply our
// own `get_info` (so resource capability is advertised alongside tools) and the
// resource read/list methods, exposing the grammar docs as MCP resources.
#[tool_handler]
impl ServerHandler for PucServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
        )
        .with_server_info(Implementation::new("puc", env!("CARGO_PKG_VERSION")))
        .with_instructions(INSTRUCTIONS.to_string())
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        Ok(ListResourcesResult::with_all_items(vec![
            RawResource::new(INTERCEPT_URI, "intercept-grammar")
                .with_title("拦截指令语法")
                .with_description(
                    "puc_intercept 的指令语法：分号分隔的有状态指令串（scene/wave/delay/\
                     doom/hit/nohit/max/imp）、取值范围与输出约定。",
                )
                .with_mime_type("text/markdown")
                .no_annotation(),
            RawResource::new(SEML_URI, "seml-grammar")
                .with_title("SEML 语法")
                .with_description(
                    "puc_seml 的 SEML 场景描述语法：测试参数、波长、用炮 / 用垫 / 用卡操作。",
                )
                .with_mime_type("text/markdown")
                .no_annotation(),
        ]))
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, ErrorData> {
        let text = match request.uri.as_str() {
            INTERCEPT_URI => INTERCEPT_DOC,
            SEML_URI => SEML_DOC,
            other => {
                return Err(ErrorData::resource_not_found(
                    format!("unknown resource: {other}"),
                    None,
                ))
            }
        };
        Ok(ReadResourceResult::new(vec![ResourceContents::text(
            text,
            request.uri,
        )
        .with_mime_type("text/markdown")]))
    }
}

// --- helpers ----------------------------------------------------------------

fn opt_enum<T: ValueEnum>(s: Option<&str>, default: T, field: &str) -> Result<T, String> {
    match s {
        None => Ok(default),
        Some(v) => req_enum(v, field),
    }
}

fn req_enum<T: ValueEnum>(s: &str, field: &str) -> Result<T, String> {
    T::from_str(s, true).map_err(|_| format!("invalid {field}: {s:?}"))
}

fn bad_args(msg: String) -> CallToolResult {
    CallToolResult::error(vec![Content::text(msg)])
}

/// Turn a captured `(Result, stdout, stderr)` into a tool result.
fn finish((res, out, diag): (Result<(), String>, String, String)) -> CallToolResult {
    match res {
        Ok(()) => success(out, diag),
        Err(msg) => {
            let mut text = combine(out, diag);
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(&msg);
            CallToolResult::error(vec![Content::text(text)])
        }
    }
}

fn success(out: String, diag: String) -> CallToolResult {
    let mut text = out;
    if !diag.trim().is_empty() {
        if !text.is_empty() && !text.ends_with('\n') {
            text.push('\n');
        }
        text.push_str("\n--- warnings ---\n");
        text.push_str(&diag);
    }
    if text.is_empty() {
        text.push_str("(no output)");
    }
    CallToolResult::success(vec![Content::text(text)])
}

fn combine(out: String, diag: String) -> String {
    let mut text = out;
    if !diag.is_empty() {
        if !text.is_empty() && !text.ends_with('\n') {
            text.push('\n');
        }
        text.push_str(&diag);
    }
    text
}
