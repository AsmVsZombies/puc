//! Calculators ported from 万能表 (universal table): coord, time, extreme, ipp.
//! Each sheet is lookups over the `tables` position data plus geometry; we reproduce the
//! cell arithmetic directly. Functions print an aligned table to stdout and return
//! `Result<(), String>` (the `Err` message is printed as `error: ...` by `main`).

pub mod coord;
pub mod extreme;
pub mod ipp;
pub mod time;

use crate::tables::{FlagMode, Variant};
use clap::ValueEnum;

#[derive(Clone, Copy, PartialEq, ValueEnum)]
pub enum Wave {
    Normal,
    Flag,
}

#[derive(Clone, Copy, PartialEq, ValueEnum)]
pub enum ExplodeKind {
    Cob,
    Doom,
}

#[derive(Clone, Copy, PartialEq, ValueEnum)]
pub enum SceneArg {
    /// Front yard (前院, 5 rows)
    De,
    /// Backyard / pool (后院, 6 rows)
    Pe,
    /// Roof (屋顶, 5 rows)
    Re,
}

#[derive(Clone, Copy, PartialEq, ValueEnum)]
pub enum Equiv {
    /// 炮等效时间 (0 ice = perfect predictive ice)
    Cob,
    /// 卡等效时间 (1 ice = perfect predictive ice)
    Card,
}

/// Effective (position column, x offset, per-wave min cs) for a variant on `wave`.
/// Returns None if the variant does not exist on that wave.
pub fn wave_lookup(v: &Variant, wave: Wave) -> Option<(String, f64, Option<i32>)> {
    match wave {
        Wave::Normal => {
            // Normal-wave availability is driven solely by min_cs_normal; the flag-only rows
            // (flag / duck_flag) carry no normal min-cs and are filtered out here.
            v.min_cs_normal?;
            Some((v.pos_col.clone(), 0.0, v.min_cs_normal))
        }
        Wave::Flag => {
            v.min_cs_flag?;
            let (col, off) = match v.flag {
                FlagMode::Offset(o) => (v.pos_col.clone(), o as f64),
                FlagMode::Column(c) => (c.to_string(), 0.0),
                FlagMode::FlagOnly => (v.pos_col.clone(), 0.0),
            };
            Some((col, off, v.min_cs_flag))
        }
    }
}

/// MEDIAN(a,b,c) — clamps `c` into `[min(a,b), max(a,b)]`.
pub fn median(a: f64, b: f64, c: f64) -> f64 {
    a + b + c - a.min(b).min(c) - a.max(b).max(c)
}

/// Format a landing-column value the way the sheet does (multiples of 1/80), trimming zeros.
pub fn fmt_col(x: f64) -> String {
    let s = format!("{:.4}", x);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    s.to_string()
}

/// Format an inclusive landing-column range, or a marker when empty.
pub fn fmt_range(lo: f64, hi: f64) -> String {
    if lo > hi + 1e-9 {
        "—".to_string()
    } else {
        format!("{}~{}", fmt_col(lo), fmt_col(hi))
    }
}
