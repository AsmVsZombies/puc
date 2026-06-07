//! `puc seml <type> <file>` — parse a SEML scenario, run it through the PvZ
//! emulator (`pvz-emulator-sys`), and print a clean aligned-text table.
//!
//! The SEML parser is a port of the `../seml` VSCode extension; markdown block
//! extraction is intentionally out of scope (the `<type>` comes from the CLI).

pub mod csv;
pub mod format;
pub mod parser;
pub mod plant;
pub mod string;
pub mod types;
pub mod zombie;

use std::path::{Path, PathBuf};

use chrono::Local;
use clap::ValueEnum;
use serde_json::{json, Value};

use types::Params;

/// Where to write a CSV export. `path` is the user-supplied target: if it is an
/// existing directory the file is named `<default_stem> (<timestamp>) .csv`
/// inside it (the `open_csv` convention), otherwise it is written verbatim.
pub struct CsvTarget<'a> {
    pub path: &'a Path,
    pub default_stem: &'a str,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum SemlType {
    /// 坐标分布 (zombie x-coordinate / arrival-time distribution)
    Pos,
    /// 砸率 (gargantuar smash rate)
    Smash,
    /// 炮伤 (cob explosion damage)
    Explode,
    /// 刷新 (spawn refresh accident rate)
    Refresh,
    /// 跳跳 (pogo collect range)
    Pogo,
}

impl SemlType {
    pub fn as_str(self) -> &'static str {
        match self {
            SemlType::Pos => "pos",
            SemlType::Smash => "smash",
            SemlType::Explode => "explode",
            SemlType::Refresh => "refresh",
            SemlType::Pogo => "pogo",
        }
    }
}

pub fn run(
    kind: SemlType,
    file: &Path,
    compact: bool,
    strict: bool,
    csv: Option<&Path>,
) -> Result<(), String> {
    let text = std::fs::read_to_string(file)
        .map_err(|err| format!("无法读取文件 {}: {}", file.display(), err))?;
    // A directory CSV target names the file after the input scenario's stem.
    let stem = file
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| kind.as_str().to_string());
    let target = csv.map(|path| CsvTarget {
        path,
        default_stem: &stem,
    });
    run_text(kind, &text, compact, strict, target)
}

/// Same as [`run`] but takes the SEML source directly (no file I/O). Used by the
/// MCP server so a tool call can pass inline content instead of a path.
pub fn run_text(
    kind: SemlType,
    text: &str,
    compact: bool,
    strict: bool,
    csv: Option<CsvTarget>,
) -> Result<(), String> {
    let parsed = parser::parse(text, strict)?;

    let scenario =
        serde_json::to_string(&parsed.config).map_err(|err| format!("序列化场景失败: {}", err))?;
    let params_json = build_params(kind, &parsed.params).to_string();

    let result = pvz_emulator_sys::run(kind.as_str(), &scenario, &params_json)?;
    let value: Value =
        serde_json::from_str(&result).map_err(|err| format!("解析模拟结果失败: {}", err))?;

    match kind {
        SemlType::Pos => format::pos(&value, &parsed.params, compact),
        SemlType::Smash => format::smash(&value, &parsed.params, compact),
        SemlType::Explode => format::explode(&value, &parsed.params, compact),
        SemlType::Refresh => format::refresh(&value, &parsed.params, compact),
        SemlType::Pogo => format::pogo(&value, &parsed.params, compact),
    }

    if let Some(target) = csv {
        let body = match kind {
            SemlType::Pos => csv::pos(&value, &parsed.params),
            SemlType::Smash => csv::smash(&value, &parsed.config.setting.scene, &parsed.params),
            SemlType::Explode => csv::explode(&value, &parsed.params),
            SemlType::Refresh => csv::refresh(&value, &parsed.params),
            SemlType::Pogo => csv::pogo(&value),
        };
        let out_path = write_csv(&target, &body)?;
        outln!("CSV written to {}", out_path.display());
    }

    Ok(())
}

/// Resolves the CSV output path and writes the body with a UTF-8 BOM, matching
/// the `open_csv` convention (`"<stem> (<YYYY.MM.DD_HH.MM.SS>) .csv"`).
fn write_csv(target: &CsvTarget, body: &str) -> Result<PathBuf, String> {
    let path = if target.path.is_dir() {
        let ts = Local::now().format("%Y.%m.%d_%H.%M.%S");
        target.path.join(format!("{} ({}) .csv", target.default_stem, ts))
    } else {
        target.path.to_path_buf()
    };
    let mut bytes = Vec::with_capacity(body.len() + 3);
    bytes.extend_from_slice(&[0xEF, 0xBB, 0xBF]); // UTF-8 BOM
    bytes.extend_from_slice(body.as_bytes());
    std::fs::write(&path, bytes)
        .map_err(|err| format!("无法写入 CSV 文件 {}: {}", path.display(), err))?;
    Ok(path)
}

/// `disableCobDelay = !cobDelay` (header default false => disable true, the shim default).
fn disable_cob_delay(p: &Params) -> bool {
    !p.cob_delay.unwrap_or(false)
}

/// Build the per-calculator params JSON, including only the keys it reads.
/// Omitting `repeat` when absent lets the shim apply its per-calculator default.
fn build_params(kind: SemlType, p: &Params) -> Value {
    let mut obj = serde_json::Map::new();
    if let Some(r) = p.repeat {
        obj.insert("repeat".into(), json!(r));
    }
    match kind {
        SemlType::Pos => {
            obj.insert(
                "zombies".into(),
                json!(p.zombies.clone().unwrap_or_default()),
            );
            if let Some(x) = p.target_x {
                obj.insert("targetX".into(), json!(x));
            }
            obj.insert("disableCobDelay".into(), json!(disable_cob_delay(p)));
        }
        SemlType::Smash | SemlType::Explode => {
            obj.insert("disableCobDelay".into(), json!(disable_cob_delay(p)));
        }
        SemlType::Refresh => {
            obj.insert(
                "require".into(),
                json!(p.require.clone().unwrap_or_default()),
            );
            obj.insert("ban".into(), json!(p.ban.clone().unwrap_or_default()));
            obj.insert("huge".into(), json!(p.huge.unwrap_or(false)));
            obj.insert("activate".into(), json!(p.activate.unwrap_or(false)));
            obj.insert("dance".into(), json!(p.dance.unwrap_or(false)));
            obj.insert("natural".into(), json!(p.natural.unwrap_or(false)));
            obj.insert("disableCobDelay".into(), json!(disable_cob_delay(p)));
        }
        SemlType::Pogo => {}
    }
    Value::Object(obj)
}
