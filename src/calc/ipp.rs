//! `puc ipp` — 热过渡 (hot transition).
//!
//! Given a transition timing, compute the landing-column windows that hit both the
//! zomboni (冰车) and the digger (矿工) together — for back/front yard (收二/收三) and roof
//! per cob column.
//!
//! When the accelerated-wave length (`wave_len`) is supplied, also compute the garg
//! coordinate at the ice-adjusted effective time and the cob column for that "virtual"
//! position (炸虚落点); this uses card equivalence (卡等效: 1 ice = perfect-prejudge ice)
//! with the ice timing (default 1). Omitting `wave_len` skips the 炸虚落点 calculation.

use super::{fmt_col, fmt_range};
use crate::tables::{FAST, SLOW};

// back/front collect offsets: (zomboni_lo_off, zomboni_hi_off, miner_lo_off, miner_hi_off)
const BACK2: (i32, i32, i32, i32) = (107, 274, 57, 199);
const BACK3: (i32, i32, i32, i32) = (104, 271, 50, 192);
const FRONT2: (i32, i32, i32, i32) = (106, 273, 54, 196);
const FRONT3: (i32, i32, i32, i32) = (99, 266, 43, 185);

// roof cob horizontal damage distance per cob column (1,2,3,4,5,6,7/8), for hit above/same/below
const ROOF_ABOVE: [i32; 7] = [115, 115, 115, 115, 115, 113, 111];
const ROOF_SAME: [i32; 7] = [111, 114, 115, 115, 115, 115, 115];
const ROOF_BELOW: [i32; 7] = [21, 67, 88, 102, 110, 114, 114];

fn car_miner_range(
    car_fast: f32,
    car_slow: f32,
    miner_fast: f32,
    miner_slow: f32,
    off: (i32, i32, i32, i32),
) -> (f64, f64) {
    let (zlo, zhi, mlo, mhi) = off;
    let zomboni_lo = (car_slow.floor() as f64 - zlo as f64) / 80.0;
    let zomboni_hi = (car_fast.floor() as f64 + zhi as f64) / 80.0;
    let miner_lo = (miner_slow.floor() as f64 - mlo as f64) / 80.0;
    let miner_hi = (miner_fast.floor() as f64 + mhi as f64) / 80.0;
    (zomboni_lo.max(miner_lo), zomboni_hi.min(miner_hi))
}

pub fn run(transition: i32, wave_len: Option<i32>, ice: i32) -> Result<(), String> {
    if transition < 0 {
        return Err(t!("calc_bad_time").to_string());
    }

    // 炸虚落点 depends on the accelerated-wave length; skip it when wave_len is absent.
    // Times use card equivalence (1 ice = perfect-prejudge ice).
    let virtual_landing = match wave_len {
        Some(wave_len) => {
            if wave_len < 0 {
                return Err(t!("ipp_wave_len_nonnegative").to_string());
            }
            const EQUIV_V: i32 = 1;
            // Effective natural-speed garg time, accounting for freeze (399cs) + slow (counts half).
            let r3 = wave_len + ice - EQUIV_V; // ice moment
            let r4 = wave_len + transition; // transition moment
            let r5 = r4 - r3; // time after ice
            let r6 = (r5 - 399).max(0); // time after thaw
            let r7 = r6.min(1600); // slowed portion
            let r8 = r6 - r7; // normal-speed portion
            let r9 = if ice - EQUIV_V >= 0 {
                r3 as f64 + r7 as f64 / 2.0 + r8 as f64
            } else {
                r4 as f64
            };
            let garg_x = FAST.x_at("gargantuar", r9.trunc() as i32);
            let virtual_cob = (garg_x.floor() as f64 - 126.0) / 80.0;
            Some((wave_len, garg_x, virtual_cob))
        }
        None => None,
    };

    // ice-car / miner positions at the raw transition time (both move at natural speed here).
    let car_fast = FAST.x_at("zomboni", transition);
    let car_slow = SLOW.x_at("zomboni", transition);
    let miner_fast = FAST.x_at("digger", transition);
    let miner_slow = SLOW.x_at("digger", transition);

    match virtual_landing {
        Some((wave_len, garg_x, virtual_cob)) => outln!(
            "ipp transition={} wave_len={} ice={} garg_x={} cob_col={}",
            transition,
            wave_len,
            ice,
            fmt_col(garg_x as f64),
            fmt_col(virtual_cob),
        ),
        None => outln!("ipp transition={}", transition),
    }

    let (b2l, b2h) = car_miner_range(car_fast, car_slow, miner_fast, miner_slow, BACK2);
    let (b3l, b3h) = car_miner_range(car_fast, car_slow, miner_fast, miner_slow, BACK3);
    let (f2l, f2h) = car_miner_range(car_fast, car_slow, miner_fast, miner_slow, FRONT2);
    let (f3l, f3h) = car_miner_range(car_fast, car_slow, miner_fast, miner_slow, FRONT3);
    outln!(
        "  {:<6} {:<2}={:<12} {:<2}={}",
        t!("ipp_back"),
        t!("ipp_c2"),
        fmt_range(b2l, b2h),
        t!("ipp_c3"),
        fmt_range(b3l, b3h)
    );
    outln!(
        "  {:<6} {:<2}={:<12} {:<2}={}",
        t!("ipp_front"),
        t!("ipp_c2"),
        fmt_range(f2l, f2h),
        t!("ipp_c3"),
        fmt_range(f3l, f3h)
    );

    // roof: single cob-landing column to hit zomboni at the row above/same/below, per cob col.
    outln!(
        "  {} {:<8} {:<8} {}",
        t!("ipp_roof"), t!("ipp_above"), t!("ipp_same"), t!("ipp_below")
    );
    let labels = ["1", "2", "3", "4", "5", "6", "7/8"];
    for i in 0..7 {
        let col = |d: i32| fmt_col((car_fast.floor() as f64 - d as f64 + 7.0) / 80.0);
        outln!(
            "  {:<6} {:<8} {:<8} {}",
            labels[i],
            col(ROOF_ABOVE[i]),
            col(ROOF_SAME[i]),
            col(ROOF_BELOW[i])
        );
    }
    Ok(())
}
