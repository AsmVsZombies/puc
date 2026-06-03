//! Precomputed zombie-position lookup tables (`原速极快` / `原速极慢`) exported from 万能表,
//! plus the per-variant metadata the coord/time calculators iterate over.
//!
//! `natural_fast.csv` holds each zombie's *most-advanced* x at centisecond `t` (no ice,
//! fastest random-walk realization); `natural_slow.csv` the *least-advanced* x. Each column
//! has its own length; lookups clamp `t` to the last valid centisecond, matching the sheet's
//! `OFFSET(..., MIN(lastCs, t))`.

use crate::zmc::{self, ZombieType};
use std::collections::HashMap;

pub struct PosTable {
    cols: HashMap<String, Vec<f32>>,
}

impl PosTable {
    fn from_csv(bytes: &[u8]) -> PosTable {
        let mut reader = csv::Reader::from_reader(bytes);
        let headers: Vec<String> = reader
            .headers()
            .unwrap()
            .iter()
            .map(|s| s.to_string())
            .collect();
        let mut cols: Vec<Vec<f32>> = vec![Vec::new(); headers.len()];
        for record in reader.records() {
            let record = record.unwrap();
            for (i, field) in record.iter().enumerate() {
                if field.is_empty() {
                    continue; // trailing blanks past a column's last valid cs
                }
                // Columns are dense from the top, so push directly (t == index).
                cols[i].push(field.parse::<f32>().unwrap());
            }
        }
        let cols = headers
            .into_iter()
            .zip(cols)
            .filter(|(name, _)| name != "t")
            .collect();
        PosTable { cols }
    }

    /// x-coordinate of `col` at centisecond `t`, clamped to the column's valid range.
    pub fn x_at(&self, col: &str, t: i32) -> f32 {
        let v = &self.cols[col];
        let idx = (t.max(0) as usize).min(v.len() - 1);
        v[idx]
    }

    /// Last centisecond tracked for `col` (the sheet's "最终cs").
    pub fn last_cs(&self, col: &str) -> i32 {
        self.cols[col].len() as i32 - 1
    }

    /// Raw position column (descending in `t`).
    pub fn raw(&self, col: &str) -> &[f32] {
        &self.cols[col]
    }
}

lazy_static::lazy_static! {
    pub static ref FAST: PosTable = PosTable::from_csv(include_bytes!("../assets/natural_fast.csv"));
    pub static ref SLOW: PosTable = PosTable::from_csv(include_bytes!("../assets/natural_slow.csv"));
}

/// How a zombie variant behaves on the flag (旗帜波) wave.
#[derive(Clone, Copy, PartialEq)]
pub enum FlagMode {
    /// Add `flag_offset` to the normal-wave position.
    Offset(i32),
    /// Use a dedicated `*_flag` position column (no offset).
    Column(&'static str),
    /// Only exists on the flag wave (e.g. the flag zombie itself).
    FlagOnly,
}

/// One row of the coord/time per-zombie table. There are ~28 of these because several
/// terrain/state variants reuse one position column with different defense geometry.
pub struct Variant {
    pub key: &'static str, // stable English key (also used by --zombies)
    pub pos_col: String,   // column in FAST/SLOW
    pub flag: FlagMode,
    pub min_cs_normal: Option<i32>,
    pub min_cs_flag: Option<i32>,
    pub dmg_range: (i32, i32), // leftmost / rightmost x still fully damageable
    pub def_x: (i32, i32),     // defense box x offset / right
    pub def_y: (i32, i32),     // defense box y offset / bottom
    pub h: i32,                // 僵尸h
}

// Override knobs for terrain/state variants that piggyback on a base zombie's position column.
struct Over {
    dmg_range: Option<(i32, i32)>,
    def_x: Option<(i32, i32)>,
    def_y: Option<(i32, i32)>,
    h: Option<i32>,
    min_cs: Option<(Option<i32>, Option<i32>)>,
    flag: Option<FlagMode>,
}
const NONE_OVER: Over = Over { dmg_range: None, def_x: None, def_y: None, h: None, min_cs: None, flag: None };

// (key, base ZombieType, flag mode for the *primary* row, override). Order matches the sheet.
struct Spec(&'static str, ZombieType, FlagMode, Over);

fn specs() -> Vec<Spec> {
    use FlagMode::*;
    use ZombieType::*;
    vec![
        Spec("regular", Regular, Offset(40), NONE_OVER),
        Spec("regular_dc_fast", DCFast, Offset(40), NONE_OVER),
        Spec("regular_dc_slow", DCSlow, Offset(40), NONE_OVER),
        Spec("pole", PoleVaulting, Offset(40), NONE_OVER),
        Spec("newspaper", Newspaper, Offset(40), NONE_OVER),
        Spec("door", ScreenDoor, Offset(40), NONE_OVER),
        Spec("football", Football, Offset(40), NONE_OVER),
        Spec("dancing", Dancing, Offset(40), NONE_OVER),
        Spec("snorkel", Snorkel, Column("snorkel_flag"), NONE_OVER),
        Spec("zomboni", Zomboni, Offset(40), NONE_OVER),
        Spec("dolphin", DolphinRider, Column("dolphin_flag"), NONE_OVER),
        Spec("jack", JackInTheBox, Offset(40), NONE_OVER),
        Spec("balloon", Balloon, Offset(40), NONE_OVER),
        Spec("digger", Digger, Column("digger_flag"), NONE_OVER),
        Spec("pogo", Pogo, Offset(40), NONE_OVER),
        Spec("ladder", Ladder, Offset(40), NONE_OVER),
        Spec("catapult", Catapult, Offset(40), NONE_OVER),
        Spec("gargantuar", Gargantuar, Offset(40), NONE_OVER),
        Spec("flag", Flag, FlagOnly, NONE_OVER),
        // Terrain / state variants reuse a base position column with tweaks; each keeps its
        // base zombie's flag behavior (availability is driven by min_cs per wave).
        Spec("duck", Regular, Offset(40), Over { h: Some(-40), ..NONE_OVER }),
        Spec("duck_dc_fast", DCFast, Offset(40), Over { h: Some(-40), ..NONE_OVER }),
        Spec("duck_dc_slow", DCSlow, Offset(40), Over { h: Some(-40), ..NONE_OVER }),
        Spec("snorkel_ashore", Snorkel, Column("snorkel_flag"), Over { h: Some(0), ..NONE_OVER }),
        Spec("digger_reverse", Digger, Column("digger_flag"), Over {
            dmg_range: Some((9, 758)), def_x: Some((42, 70)),
            min_cs: Some((Some(1358), Some(1419))), ..NONE_OVER }),
        Spec("duck_flag", Flag, FlagOnly, Over { h: Some(-40), ..NONE_OVER }),
        Spec("dolphin_swim", DolphinRider, Column("dolphin_flag"), Over {
            dmg_range: Some((-99, 780)), def_x: Some((20, 62)), ..NONE_OVER }),
        Spec("balloon_ground", Balloon, Offset(40), Over { h: Some(0), ..NONE_OVER }),
        Spec("pogo_walk", Pogo, Offset(40), Over {
            def_y: Some((17, 132)), h: Some(16), ..NONE_OVER }),
    ]
}

lazy_static::lazy_static! {
    pub static ref VARIANTS: Vec<Variant> = specs()
        .into_iter()
        .map(|Spec(key, t, flag, ov)| {
            let z = zmc::zombie_data(t);
            Variant {
                key,
                pos_col: z.pos_col.clone(), // pos_col always comes from the base zombie
                flag: ov.flag.unwrap_or(flag),
                min_cs_normal: ov.min_cs.map(|m| m.0).unwrap_or(z.min_cs_normal),
                min_cs_flag: ov.min_cs.map(|m| m.1).unwrap_or(z.min_cs_flag),
                dmg_range: ov.dmg_range.unwrap_or(z.dmg_range),
                def_x: ov.def_x.unwrap_or(z.def_x),
                def_y: ov.def_y.unwrap_or(z.def_y),
                h: ov.h.unwrap_or(z.coord_h),
            }
        })
        .collect();
}

/// Find variants matching a user `--zombies` filter (comma-separated keys); `None` = all.
pub fn select<'a>(filter: Option<&str>) -> Vec<&'a Variant> {
    match filter {
        None => VARIANTS.iter().collect(),
        Some(f) => {
            let wanted: Vec<&str> = f.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
            VARIANTS.iter().filter(|v| wanted.contains(&v.key)).collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn position_lookup_matches_sheet() {
        // 原速极快!W (gargantuar) values read off the workbook.
        assert!((FAST.x_at("gargantuar", 618) - 719.94).abs() < 0.01, "{}", FAST.x_at("gargantuar", 618));
        assert!((FAST.x_at("gargantuar", 685) - 718.947).abs() < 0.01, "{}", FAST.x_at("gargantuar", 685));
        assert_eq!(FAST.x_at("gargantuar", 0), 845.0);
        assert_eq!(SLOW.x_at("gargantuar", 0), 854.0);
    }

    #[test]
    fn variants_built() {
        assert_eq!(VARIANTS.len(), 28);
        let garg = VARIANTS.iter().find(|v| v.key == "gargantuar").unwrap();
        assert_eq!(garg.pos_col, "gargantuar");
        assert_eq!(garg.dmg_range, (-149, 817));
        assert_eq!(garg.def_x, (-17, 108));
    }
}
