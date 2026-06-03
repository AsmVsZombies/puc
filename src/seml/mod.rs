//! `puc seml <type> <file>` — parse a SEML scenario, run it through the PvZ
//! emulator (`pvz-emulator-sys`), and print a clean aligned-text table.
//!
//! The SEML parser is a port of the `../seml` VSCode extension; markdown block
//! extraction is intentionally out of scope (the `<type>` comes from the CLI).

pub mod format;
pub mod parser;
pub mod plant;
pub mod string;
pub mod types;
pub mod zombie;

use std::path::Path;

use clap::ValueEnum;
use serde_json::{json, Value};

use types::Params;

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
    fn as_str(self) -> &'static str {
        match self {
            SemlType::Pos => "pos",
            SemlType::Smash => "smash",
            SemlType::Explode => "explode",
            SemlType::Refresh => "refresh",
            SemlType::Pogo => "pogo",
        }
    }
}

pub fn run(kind: SemlType, file: &Path, compact: bool) -> Result<(), String> {
    let text = std::fs::read_to_string(file)
        .map_err(|err| format!("无法读取文件 {}: {}", file.display(), err))?;

    let parsed = parser::parse(&text)?;

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
    Ok(())
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
