//! `puc coord` — 落点计算器 (landing-point calculator).
//!
//! Given a wave + firing time, for each zombie variant look up its x-range `[fast, slow]`
//! at that time and report the cob/doom landing-column window (per hit-row-relation
//! above/same/below in the chosen scene) that fully damages it, plus a 全伤 flag.

use super::{fmt_col, fmt_range, median, wave_lookup, ExplodeKind, SceneArg, Wave};
use crate::tables::{self, FAST, SLOW};

// 屋顶爆心y per cob-tail column (1..=8); index 0 unused.
const ROOF_EXPLODE_Y: [f64; 9] = [0.0, 84.0, 104.0, 124.0, 144.0, 164.0, 184.0, 195.0, 195.0];

struct SceneGeom {
    explode_center: f64, // 爆炸y (back/roof) or 爆炸x-as-y (front)
    above_y: f64,
    spacing: f64,
}

fn scene_geom(scene: SceneArg, roof_tail: Option<i32>) -> Result<SceneGeom, String> {
    Ok(match scene {
        SceneArg::Pe => SceneGeom {
            explode_center: 205.0,
            above_y: 50.0,
            spacing: 85.0,
        },
        SceneArg::De => SceneGeom {
            explode_center: 220.0,
            above_y: 50.0,
            spacing: 100.0,
        },
        SceneArg::Re => {
            let tail = roof_tail.ok_or_else(|| t!("coord_need_roof_tail").to_string())?;
            if !(1..=8).contains(&tail) {
                return Err(t!("coord_bad_roof_tail").to_string());
            }
            SceneGeom {
                explode_center: ROOF_EXPLODE_Y[tail as usize],
                above_y: 40.0,
                spacing: 85.0,
            }
        }
    })
}

fn clamp01_10(x: f64) -> f64 {
    median(0.0, 10.0, x)
}

// flying balloon (空中气球) sits 30px higher than ground zombies.
fn zombie_y_shift(key: &str) -> f64 {
    if key == "balloon" {
        -30.0
    } else {
        0.0
    }
}

// per-row-relation extra equivalent-y offset (pogo's jump quirk).
fn pogo_dy(key: &str, rel: usize) -> f64 {
    if key == "pogo" {
        match rel {
            0 | 1 => -49.0, // above / same
            _ => 9.0,       // below
        }
    } else {
        0.0
    }
}

pub fn run(
    time: i32,
    wave: Wave,
    kind: ExplodeKind,
    scene: SceneArg,
    roof_tail: Option<i32>,
    x_override: Option<(i32, i32)>,
    zombies: Option<&str>,
) -> Result<(), String> {
    if time < 0 {
        return Err(t!("calc_bad_time").to_string());
    }
    if kind == ExplodeKind::Doom {
        return Err(t!("coord_doom_unsupported").to_string());
    }
    let geom = scene_geom(scene, roof_tail)?;
    let radius = 115.0_f64; // cob

    let wave_name = match wave {
        Wave::Normal => "normal",
        Wave::Flag => "flag",
    };
    outln!(
        "coord time={} wave={} scene={} kind=cob",
        time,
        wave_name,
        scene_key(scene)
    );
    outln!(
        "  {:<16} {:<13} {:<4} {:<13} {:<13} {}",
        t!("coord_hdr_zombie"),
        t!("coord_hdr_x"),
        t!("coord_hdr_full"),
        t!("coord_hdr_above"),
        t!("coord_hdr_same"),
        t!("coord_hdr_below")
    );

    for v in tables::select(zombies) {
        let Some((col, off, min_cs)) = wave_lookup(v, wave) else {
            continue;
        };
        let col = col.as_str();
        let last = FAST.last_cs(col).min(SLOW.last_cs(col));

        // coordinate range [min_x (most advanced), max_x (least advanced)].
        let (min_x, max_x) = match x_override {
            Some((lo, hi)) => (lo as f64, hi as f64),
            None => {
                if let Some(mc) = min_cs {
                    if time < mc || time > last {
                        continue; // 屏蔽: outside this variant's valid window
                    }
                }
                let tc = time.min(last);
                (
                    (FAST.x_at(col, tc) as f64 + off).trunc(),
                    (SLOW.x_at(col, tc) as f64 + off).trunc(),
                )
            }
        };

        let (lo, hi) = (v.dmg_range.0 as f64, v.dmg_range.1 as f64);
        let full = lo <= min_x && max_x <= hi;
        let min_coord = median(lo, hi, min_x); // B36: right-bound coordinate
        let max_coord = median(lo, hi, max_x); // B37: left-bound coordinate

        // landing-column range for each row-relation.
        let mut cols: [Option<(f64, f64)>; 3] = [None; 3];
        for (rel, cell) in cols.iter_mut().enumerate() {
            let zombie_y = geom.above_y + rel as f64 * geom.spacing + zombie_y_shift(v.key);
            let equiv_y = zombie_y - v.h as f64 + pogo_dy(v.key, rel);
            let def_top = equiv_y + v.def_y.0 as f64;
            let def_bot = equiv_y + v.def_y.1 as f64;
            let ydist = (def_top - geom.explode_center)
                .max(geom.explode_center - def_bot)
                .max(0.0);
            if ydist > radius {
                continue; // no horizontal reach -> can't hit this row
            }
            let reach = (radius * radius - ydist * ydist).sqrt().trunc();
            let left = clamp01_10((max_coord + v.def_x.0 as f64 - reach + 7.0) / 80.0);
            let right = clamp01_10((min_coord + v.def_x.1 as f64 + reach + 7.0) / 80.0);
            *cell = Some((left, right));
        }

        let fmt_rel = |c: Option<(f64, f64)>| match c {
            Some((l, r)) => fmt_range(l, r),
            None => "—".to_string(),
        };
        outln!(
            "  {:<16} {:<13} {:<4} {:<13} {:<13} {}",
            v.key,
            format!("{}~{}", fmt_col(min_x), fmt_col(max_x)),
            if full { "√" } else { "" },
            fmt_rel(cols[0]),
            fmt_rel(cols[1]),
            fmt_rel(cols[2]),
        );
    }
    Ok(())
}

fn scene_key(s: SceneArg) -> &'static str {
    match s {
        SceneArg::De => "de",
        SceneArg::Pe => "pe",
        SceneArg::Re => "re",
    }
}
