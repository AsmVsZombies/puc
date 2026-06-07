//! `puc extreme` — 慢速 / 快速计算器 (extreme slow / fast).
//!
//! Given the walk-time(s) (cs) of stacked zombies of one `--type` (garg / ladder / jack) in a
//! lane, sum their advance distances to get the extreme coordinate. Each segment advances from
//! its type's spawn x `R = x_at(type, 0)`, so `coord = R − Σ(R − x_at(type, tᵢ))`; multiple walk
//! times = same-lane stacked segments. `--slow` uses the least-advanced table and (for garg)
//! reports 全收两行 / 后院收三 / 前院收三; `--fast` the most-advanced table and (for garg) the
//! 正好不伤 (just-not-hitting) column. ladder / jack report the coordinate only.

use super::fmt_col;
#[cfg(feature = "en")]
use crate::lang::en::*;
#[cfg(feature = "zh")]
use crate::lang::zh::*;
use crate::tables::{FAST, SLOW};
use clap::ValueEnum;

/// Slowest (least-advanced) vs fastest (most-advanced) realization.
#[derive(Clone, Copy, PartialEq, ValueEnum)]
pub enum Speed {
    Slow,
    Fast,
}

/// Which zombie type is stacked in the lane.
#[derive(Clone, Copy, PartialEq, ValueEnum)]
pub enum ExtremeType {
    Garg,
    Ladder,
    Jack,
}

impl ExtremeType {
    /// Position column in FAST/SLOW.
    fn col(self) -> &'static str {
        match self {
            ExtremeType::Garg => "gargantuar",
            ExtremeType::Ladder => "ladder",
            ExtremeType::Jack => "jack",
        }
    }

    fn name(self) -> &'static str {
        match self {
            ExtremeType::Garg => "garg",
            ExtremeType::Ladder => "ladder",
            ExtremeType::Jack => "jack",
        }
    }
}

pub fn run(speed: Speed, ty: ExtremeType, walk: &[i32]) -> Result<(), String> {
    if walk.is_empty() {
        return Err(EXTREME_NEED_WALK.to_string());
    }
    if walk.iter().any(|&t| t < 0) {
        return Err(CALC_BAD_TIME.to_string());
    }
    let (table, speed_name) = match speed {
        Speed::Slow => (&*SLOW, "slow"),
        Speed::Fast => (&*FAST, "fast"),
    };
    let col = ty.col();
    // Each segment advances from its type's spawn x R = x_at(type, 0); stacking sums advances.
    let r = table.x_at(col, 0) as f64;
    let total: f64 = walk.iter().map(|&t| r - table.x_at(col, t) as f64).sum();
    let coord = r - total;

    let mut line = format!(
        "extreme {} type={} walk={} coord={}",
        speed_name,
        ty.name(),
        walk.iter()
            .map(|t| t.to_string())
            .collect::<Vec<_>>()
            .join(","),
        fmt_col(coord),
    );
    // Safe landing columns are garg-specific defense geometry; ladder/jack get coordinate only.
    if ty == ExtremeType::Garg {
        let c = coord.trunc();
        match speed {
            Speed::Slow => {
                // (rightmost cob that still hits the row above): (coord - dist)/80
                line.push_str(&format!(
                    " two_rows={} back_three={} front_three={}",
                    fmt_col((c - 125.0) / 80.0), // 全收两行
                    fmt_col((c - 118.0) / 80.0), // 后院收三
                    fmt_col((c - 111.0) / 80.0), // 前院收三
                ));
            }
            Speed::Fast => {
                line.push_str(&format!(" just_safe={}", fmt_col((c - 126.0) / 80.0))); // 正好不伤
            }
        }
    }
    outln!("{}", line);
    Ok(())
}
