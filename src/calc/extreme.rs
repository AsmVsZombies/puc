//! `puc extreme` — 慢速 / 快速计算器 (extreme slow / fast).
//!
//! `slow`: given the walk-time (cs) of the slowest garg(s) stacked in a lane, sum their
//! advance distances to get the extreme (rightmost) coordinate, then the safe landing
//! columns — 全收两行 / 后院收三 / 前院收三. `fast`: given the walk-time of the fastest
//! garg (and optionally ladder / clown), the extreme (leftmost) coordinate and the
//! 正好不伤 (just-not-hitting) column.

use super::fmt_col;
#[cfg(feature = "en")]
use crate::lang::en::*;
#[cfg(feature = "zh")]
use crate::lang::zh::*;
use crate::tables::{FAST, SLOW};

const PLANT_DEF_RIGHT_SLOW: f64 = 854.0; // 植物防御域右限, 最慢巨人 (F2)
const PLANT_DEF_RIGHT_FAST: f64 = 845.0; // 最快巨人 (E3)

pub fn run_slow(walk: &[i32]) -> Result<(), String> {
    if walk.is_empty() {
        return Err(EXTREME_NEED_WALK.to_string());
    }
    if walk.iter().any(|&t| t < 0) {
        return Err(CALC_BAD_TIME.to_string());
    }
    // total advance distance = sum over gargs of (right_limit - slow_garg_x(walk_i))
    let total: f64 = walk
        .iter()
        .map(|&t| PLANT_DEF_RIGHT_SLOW - SLOW.x_at("gargantuar", t) as f64)
        .sum();
    let coord = PLANT_DEF_RIGHT_SLOW - total; // C15
    let c = coord.trunc();
    // safe landing columns (rightmost cob that still hits the row above): (coord - dist)/80
    let both_two = (c - 125.0) / 80.0; // 全收两行
    let back_three = (c - 118.0) / 80.0; // 后院收三
    let front_three = (c - 111.0) / 80.0; // 前院收三

    outln!(
        "extreme slow walk={} coord={} two_rows={} back_three={} front_three={}",
        walk.iter()
            .map(|t| t.to_string())
            .collect::<Vec<_>>()
            .join(","),
        fmt_col(coord),
        fmt_col(both_two),
        fmt_col(back_three),
        fmt_col(front_three),
    );
    Ok(())
}

pub fn run_fast(walk: &[i32], ladder: Option<i32>, clown: Option<i32>) -> Result<(), String> {
    if walk.is_empty() {
        return Err(EXTREME_NEED_WALK.to_string());
    }
    if walk
        .iter()
        .chain(ladder.iter())
        .chain(clown.iter())
        .any(|&t| t < 0)
    {
        return Err(CALC_BAD_TIME.to_string());
    }
    // fastest garg(s) stacked: coord = right_limit - sum(advance_i).
    let total: f64 = walk
        .iter()
        .map(|&t| PLANT_DEF_RIGHT_FAST - FAST.x_at("gargantuar", t) as f64)
        .sum();
    let garg_coord = PLANT_DEF_RIGHT_FAST - total;
    let just_safe = (garg_coord.trunc() - 126.0) / 80.0; // 正好不伤

    let mut line = format!(
        "extreme fast walk={} garg_coord={} just_safe_col={}",
        walk.iter()
            .map(|t| t.to_string())
            .collect::<Vec<_>>()
            .join(","),
        fmt_col(garg_coord),
        fmt_col(just_safe),
    );
    if let Some(l) = ladder {
        line.push_str(&format!(
            " ladder_coord={}",
            fmt_col(FAST.x_at("ladder", l) as f64)
        ));
    }
    if let Some(c) = clown {
        line.push_str(&format!(
            " clown_coord={}",
            fmt_col(FAST.x_at("jack", c) as f64)
        ));
    }
    outln!("{}", line);
    Ok(())
}
