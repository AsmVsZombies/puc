//! `puc time` — 时机计算器 (timing calculator). Inverse of coord: given a fixed cob/doom
//! placement (scene, kind, row, col), report for each hittable row the firing-time window
//! during which the cob collects each zombie variant in that row.

use super::{median, wave_lookup, ExplodeKind, SceneArg, Wave};
#[cfg(feature = "en")]
use crate::lang::en::*;
#[cfg(feature = "zh")]
use crate::lang::zh::*;
use crate::tables::{self, FAST, SLOW};

struct Explosion {
    x: f64,
    y: f64,
    radius: f64,
    hit_rows: Vec<i32>,
    start_y: f64, // 僵尸起始y
    spacing: f64, // 行距
}

fn max_row(scene: SceneArg) -> i32 {
    match scene {
        SceneArg::Pe => 6,
        SceneArg::De | SceneArg::Re => 5,
    }
}

// Port of the workbook's Worksheet_Change (computes 爆心x/y, radius, hittable rows).
fn explosion(
    scene: SceneArg,
    kind: ExplodeKind,
    row: i32,
    col: f64,
    roof_tail: Option<i32>,
) -> Result<Explosion, String> {
    let maxrow = max_row(scene);
    if row < 1 || row > maxrow {
        return Err(format!("{}: 1~{}", TIME_BAD_ROW, maxrow));
    }
    let row_height = if scene == SceneArg::De { 100.0 } else { 85.0 };
    let radius = match kind {
        ExplodeKind::Cob => 115.0,
        ExplodeKind::Doom => 250.0,
    };
    let span = match kind {
        ExplodeKind::Cob => 1,
        ExplodeKind::Doom => 3,
    };
    let hit_rows: Vec<i32> = ((row - span).max(1)..=(row + span).min(maxrow)).collect();

    let (x, y) = match (kind, scene) {
        (ExplodeKind::Doom, _) => (round_x(col), 120.0 + (row as f64 - 1.0) * row_height),
        (ExplodeKind::Cob, SceneArg::De) | (ExplodeKind::Cob, SceneArg::Pe) => {
            (cob_x(col), 120.0 + (row as f64 - 1.0) * row_height)
        }
        (ExplodeKind::Cob, SceneArg::Re) => {
            let tail = roof_tail.ok_or_else(|| TIME_NEED_ROOF_TAIL.to_string())?;
            if !(1..=8).contains(&tail) {
                return Err(TIME_BAD_ROOF_TAIL.to_string());
            }
            roof_cob_xy(col, row, tail)
        }
    };
    let start_y = if scene == SceneArg::Re { 40.0 } else { 50.0 };
    let spacing = if scene == SceneArg::De { 100.0 } else { 85.0 };
    Ok(Explosion {
        x,
        y,
        radius,
        hit_rows,
        start_y,
        spacing,
    })
}

fn round_x(col: f64) -> f64 {
    (col * 80.0).round()
}

// cob explosion x on ground (front/back): round(col*80) shifted left by 7 (or 6 if <7).
fn cob_x(col: f64) -> f64 {
    let x = round_x(col);
    if x >= 7.0 {
        x - 7.0
    } else {
        x - 6.0
    }
}

// roof cob explosion x/y (CobRow fixed at 3), ported from the workbook VBA.
fn roof_cob_xy(col: f64, row: i32, tail: i32) -> (f64, f64) {
    let cob_row = 3;
    let cob_col = if tail >= 7 { 7 } else { tail };
    let mut x = round_x(col) as i32;
    let mut y = 209 + (row - 1) * 85;

    let step1 = if x <= 206 {
        0
    } else if x >= 527 {
        5
    } else {
        (x - 127) / 80
    };
    y -= step1 * 20;

    let (left_edge, right_edge, step2_shift) = if cob_col == 1 {
        (87, 524, 0)
    } else if cob_col >= 7 {
        (510, 523, 5)
    } else {
        (80 * cob_col - 13, 524, 5)
    };
    let step2 = if x <= left_edge {
        0
    } else if x >= right_edge {
        (right_edge - left_edge + 3) / 4 - step2_shift
    } else {
        (x - left_edge + 3) / 4 - step2_shift
    };
    y -= step2;

    if x == left_edge && (2..=6).contains(&cob_col) {
        if (3..=5).contains(&cob_row) {
            y += 5;
        }
        if cob_row == 3 && cob_col == 6 {
            y -= 5;
        }
    }
    if y < 0 {
        y = 0;
    }
    x = if x >= 7 { x - 7 } else { x - 6 };
    (x as f64, y as f64)
}

// largest t with raw[t] >= thr (descending column); None if no entry qualifies.
fn latest_cs_ge(raw: &[f32], thr: f64) -> Option<i32> {
    (0..raw.len())
        .rev()
        .find(|&t| raw[t] as f64 >= thr)
        .map(|t| t as i32)
}
// smallest t with raw[t] < thr (descending column); len if none.
fn first_cs_lt(raw: &[f32], thr: f64) -> i32 {
    (0..raw.len())
        .find(|&t| (raw[t] as f64) < thr)
        .map(|t| t as i32)
        .unwrap_or(raw.len() as i32)
}

// per-lane equivalent-y offsets to evaluate (pogo jumps in two phases -> worst-case ydist).
fn equiv_offsets(key: &str) -> &'static [f64] {
    match key {
        "pogo" => &[-9.0, -49.0],
        _ => &[0.0],
    }
}

#[allow(clippy::too_many_arguments)]
pub fn run(
    scene: SceneArg,
    kind: ExplodeKind,
    row: i32,
    col: f64,
    wave: Wave,
    roof_tail: Option<i32>,
    zombies: Option<&str>,
) -> Result<(), String> {
    if !col.is_finite() {
        return Err(TIME_BAD_COL.to_string());
    }
    let ex = explosion(scene, kind, row, col, roof_tail)?;

    let wave_name = match wave {
        Wave::Normal => "normal",
        Wave::Flag => "flag",
    };
    let kind_name = match kind {
        ExplodeKind::Cob => "cob",
        ExplodeKind::Doom => "doom",
    };
    println!(
        "time scene={} kind={} row={} col={} wave={} cx={} cy={} rows={}",
        scene_key(scene),
        kind_name,
        row,
        super::fmt_col(col),
        wave_name,
        ex.x,
        ex.y,
        ex.hit_rows
            .iter()
            .map(|r| r.to_string())
            .collect::<Vec<_>>()
            .join(",")
    );
    print!("  {:<16}", TIME_HDR_ZOMBIE);
    for r in &ex.hit_rows {
        print!(" {:<13}", format!("{}{}", TIME_HDR_ROW, r));
    }
    println!();

    for v in tables::select(zombies) {
        let Some((col_name, off, min_cs)) = wave_lookup(v, wave) else {
            continue;
        };
        let fast = FAST.raw(&col_name);
        let slow = SLOW.raw(&col_name);
        let dmg_lo = v.dmg_range.0 as f64;
        let dmg_hi = v.dmg_range.1 as f64;

        let mut cells: Vec<String> = Vec::new();
        let mut any = false;
        for &lane in &ex.hit_rows {
            let zombie_y = ex.start_y + (lane as f64 - 1.0) * ex.spacing + balloon_shift(v.key);

            // worst-case horizontal reach over the variant's jump phases.
            let mut ydist_max: f64 = 0.0;
            for &dy in equiv_offsets(v.key) {
                let equiv_y = zombie_y - v.h as f64 + dy;
                let def_top = equiv_y + v.def_y.0 as f64;
                let def_bot = equiv_y + v.def_y.1 as f64;
                let ydist = (def_top - ex.y).max(ex.y - def_bot).max(0.0);
                ydist_max = ydist_max.max(ydist);
            }
            if ydist_max > ex.radius {
                cells.push("—".to_string());
                continue;
            }
            let reach = (ex.radius * ex.radius - ydist_max * ydist_max)
                .sqrt()
                .trunc();

            let coord_min_raw = ex.x - reach - v.def_x.1 as f64; // C32 (B8 - reach - defx_r)
            if coord_min_raw < dmg_lo {
                cells.push("—".to_string());
                continue;
            }
            let coord_min = median(dmg_lo, dmg_hi, coord_min_raw.max(dmg_lo)); // already >= dmg_lo
            let coord_max = (ex.x + reach - v.def_x.0 as f64).min(dmg_hi); // C38
            let cm = if coord_min < 0.0 {
                coord_min - 1.0
            } else {
                coord_min
            };
            let c_big = if coord_max < 0.0 {
                coord_max - 1.0
            } else {
                coord_max
            };

            let t_lo = first_cs_lt(slow, c_big + 0.9999 - off);
            let Some(t_hi) = latest_cs_ge(fast, cm - off) else {
                cells.push("—".to_string());
                continue;
            };
            // respect the variant's earliest valid cs on this wave
            let t_lo = match min_cs {
                Some(mc) => t_lo.max(mc),
                None => t_lo,
            };
            if t_lo <= t_hi {
                any = true;
                cells.push(format!("{}~{}", t_lo, t_hi));
            } else {
                cells.push("—".to_string());
            }
        }
        if !any {
            continue;
        }
        print!("  {:<16}", v.key);
        for c in &cells {
            print!(" {:<13}", c);
        }
        println!();
    }
    Ok(())
}

fn balloon_shift(key: &str) -> f64 {
    if key == "balloon" {
        -30.0
    } else {
        0.0
    }
}

fn scene_key(s: SceneArg) -> &'static str {
    match s {
        SceneArg::De => "de",
        SceneArg::Pe => "pe",
        SceneArg::Re => "re",
    }
}
