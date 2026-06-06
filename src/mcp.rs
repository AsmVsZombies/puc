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

use crate::calc::{self, Equiv, ExplodeKind, SceneArg, Wave};
use crate::seml::{self, SemlType};

/// Server-level orientation shown to MCP clients on connect.
const INSTRUCTIONS: &str = "PvZ's Ultimate Calculator — cob/doom landing-point and timing \
    calculators, extreme garg coordinates, hot-transition collect columns, the SEML scenario \
    emulator, and the stateful interception command interpreter. Times are centiseconds (cs); \
    columns are in cob units (1 col = 80 px). Tool output is plain aligned text; failures come \
    back as isError results with the diagnostic message. Full grammar references are exposed as \
    resources: read puc://docs/intercept for the puc_intercept command grammar and \
    puc://docs/seml for the SEML syntax.";

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
    /// Semicolon-separated interception commands, e.g.
    /// "pe; wave 1 400 800; delay 8.8".
    command: String,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct CoordParams {
    /// Firing time in centiseconds (>= 0).
    time: i32,
    /// Wave type: "normal" or "flag". Default "normal".
    #[serde(default)]
    wave: Option<String>,
    /// Explosion kind: "cob" or "doom". Default "cob" (doom is unsupported here).
    #[serde(default)]
    kind: Option<String>,
    /// Scene: "de" (front yard), "pe" (pool/back), "re" (roof). Default "pe".
    #[serde(default)]
    scene: Option<String>,
    /// Roof cob-tail column 1..=8. Required when scene = "re".
    #[serde(default)]
    roof_tail: Option<i32>,
    /// Override zombie x-range: "x" (single) or "min,max".
    #[serde(default)]
    x: Option<String>,
    /// Filter to specific zombie keys, comma-separated. Valid keys: regular,
    /// regular_dc_fast, regular_dc_slow, pole, newspaper, door, football,
    /// dancing, snorkel, zomboni, dolphin, jack, balloon, digger,
    /// digger_reverse, pogo, ladder, catapult, gargantuar, duck, duck_dc_fast,
    /// duck_dc_slow, snorkel_ashore, dolphin_swim, balloon_ground, pogo_walk
    /// (available keys vary by scene/tool).
    #[serde(default)]
    zombies: Option<String>,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct TimeParams {
    /// Scene: "de" (front yard), "pe" (pool/back), "re" (roof).
    scene: String,
    /// Explosion kind: "cob" or "doom".
    kind: String,
    /// Hit row (1..=5, or 1..=6 for pe).
    row: i32,
    /// Landing column.
    col: f64,
    /// Wave type: "normal" or "flag". Default "normal".
    #[serde(default)]
    wave: Option<String>,
    /// Roof cob-tail column 1..=8. Required when scene = "re".
    #[serde(default)]
    roof_tail: Option<i32>,
    /// Filter to specific zombie keys, comma-separated. Valid keys: regular,
    /// regular_dc_fast, regular_dc_slow, pole, newspaper, door, football,
    /// dancing, snorkel, zomboni, dolphin, jack, balloon, digger,
    /// digger_reverse, pogo, ladder, catapult, gargantuar, duck, duck_dc_fast,
    /// duck_dc_slow, snorkel_ashore, dolphin_swim, balloon_ground, pogo_walk
    /// (available keys vary by scene/tool).
    #[serde(default)]
    zombies: Option<String>,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct ExtremeSlowParams {
    /// Walk time(s) in centiseconds; multiple values = stacked gargs.
    walk: Vec<i32>,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct ExtremeFastParams {
    /// Walk time(s) in centiseconds; multiple values = stacked gargs.
    walk: Vec<i32>,
    /// Optional ladder timing (cs).
    #[serde(default)]
    ladder: Option<i32>,
    /// Optional clown (jack) timing (cs).
    #[serde(default)]
    clown: Option<i32>,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct IppParams {
    /// Transition timing in centiseconds (>= 0).
    transition: i32,
    /// Accelerated-wave length in centiseconds (>= 0).
    wave_len: i32,
    /// Ice timing (cs). Default 0.
    #[serde(default)]
    ice: Option<i32>,
    /// Equivalence mode: "cob" or "card". Default "cob".
    #[serde(default)]
    equiv: Option<String>,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct SemlParams {
    /// Test type: "pos", "smash", "explode", "refresh", or "pogo".
    r#type: String,
    /// Path to a SEML file. Provide this OR `content`.
    #[serde(default)]
    file: Option<String>,
    /// Inline SEML source. Takes precedence over `file` when both are given.
    #[serde(default)]
    content: Option<String>,
    /// Compact output (omit verbose breakdowns). Default false.
    #[serde(default)]
    compact: bool,
    /// Strict mode: error on unrecognized header lines instead of skipping them. Default false.
    #[serde(default)]
    strict: bool,
}

// --- tools ------------------------------------------------------------------

#[tool_router]
impl PucServer {
    #[tool(
        name = "puc_intercept",
        description = "拦截计算器: run semicolon-separated interception commands (scene, wave, delay, doom, hit/nohit, max, imp) against a stateful parser, e.g. \"pe; wave 1 400 800; delay 8.8\"."
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
        description = "落点计算器 (landing-point): for a firing time, the per-zombie cob landing-column window at each row relation (above/same/below)."
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
        description = "时机计算器 (timing): for a fixed cob/doom placement (scene, kind, row, col), the firing-time window that collects each zombie per hittable row."
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
        name = "puc_extreme_slow",
        description = "慢速计算器: for the slowest garg(s)' walk time(s), the extreme (rightmost) coordinate and safe landing columns (全收两行 / 后院收三 / 前院收三)."
    )]
    fn puc_extreme_slow(&self, Parameters(p): Parameters<ExtremeSlowParams>) -> CallToolResult {
        finish(crate::capture(|| calc::extreme::run_slow(&p.walk)))
    }

    #[tool(
        name = "puc_extreme_fast",
        description = "快速计算器: for the fastest garg(s)' walk time(s) (plus optional ladder/clown), the extreme (leftmost) coordinate and the 正好不伤 column."
    )]
    fn puc_extreme_fast(&self, Parameters(p): Parameters<ExtremeFastParams>) -> CallToolResult {
        finish(crate::capture(|| calc::extreme::run_fast(&p.walk, p.ladder, p.clown)))
    }

    #[tool(
        name = "puc_ipp",
        description = "热过渡 (hot transition): garg coordinate, virtual cob column, and ice-car/miner collect columns across the transition for back/front yard and roof."
    )]
    fn puc_ipp(&self, Parameters(p): Parameters<IppParams>) -> CallToolResult {
        let equiv = match opt_enum::<Equiv>(p.equiv.as_deref(), Equiv::Cob, "equiv") {
            Ok(v) => v,
            Err(e) => return bad_args(e),
        };
        let ice = p.ice.unwrap_or(0);
        finish(crate::capture(|| calc::ipp::run(p.transition, p.wave_len, ice, equiv)))
    }

    #[tool(
        name = "puc_seml",
        description = "seml: parse a SEML scenario, run the PvZ emulator, and print a clean table. Provide `content` (inline SEML) or `file` (path); `content` wins if both are set."
    )]
    fn puc_seml(&self, Parameters(p): Parameters<SemlParams>) -> CallToolResult {
        let kind = match req_enum::<SemlType>(&p.r#type, "type") {
            Ok(v) => v,
            Err(e) => return bad_args(e),
        };
        let compact = p.compact;
        let strict = p.strict;
        if let Some(text) = p.content.clone() {
            finish(crate::capture(|| seml::run_text(kind, &text, compact, strict)))
        } else if let Some(path) = p.file.clone() {
            finish(crate::capture(move || {
                seml::run(kind, std::path::Path::new(&path), compact, strict)
            }))
        } else {
            bad_args("provide either `content` (inline SEML) or `file` (path)".to_string())
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
