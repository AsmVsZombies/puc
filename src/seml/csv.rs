//! CSV export for each calculator, byte-compatible with the standalone C++
//! drivers (`crates/pvz-emulator-sys/vendor/{pos,smash,explode,refresh,pogo}_test.cpp`).
//!
//! These emitters are the spreadsheet counterpart to [`super::format`]: same
//! result JSON in, but laid out exactly like the reference `*_test.cpp` CSVs —
//! identical headers, column counts, blank cells, trailing commas, `ERR`/`[min]`
//! tokens, and `\n` line endings. Numbers mirror the C++ `std::fixed`/precision
//! choices and the adaptive `format_mean_std`; tiny FP/RNG deltas are expected.
//!
//! The file body returned here carries no BOM; the writer in [`super`] prepends
//! the UTF-8 BOM that `open_csv` did.

use serde_json::Value;
use std::collections::BTreeSet;

use super::types::Params;
use super::zombie;

// --- JSON accessors (mirrors super::format) ---------------------------------

fn f(v: &Value, key: &str) -> Option<f64> {
    v.get(key).and_then(|x| x.as_f64())
}
fn i(v: &Value, key: &str) -> i64 {
    v.get(key).and_then(|x| x.as_i64()).unwrap_or(0)
}
fn arr<'a>(v: &'a Value, key: &str) -> &'a [Value] {
    v.get(key)
        .and_then(|x| x.as_array())
        .map(|a| a.as_slice())
        .unwrap_or(&[])
}
fn hist(v: &Value) -> Option<&serde_json::Map<String, Value>> {
    v.get("histogram").and_then(|x| x.as_object())
}

// --- ported numeric helpers (vendor/common/test.h) --------------------------

/// `vendor/common/test.h::format_mean_std`: `mean ± sd` with the sd rounded to
/// two significant figures and the mean shown at the same decimal place.
fn format_mean_std(mean: f64, sd: f64, suffix: &str, fallback_precision: i32) -> String {
    if !sd.is_finite() || sd <= 0.0 {
        let p = fallback_precision.max(0) as usize;
        return format!("{:.*}{}±0{}", p, mean, suffix, suffix);
    }
    let round_to = |value: f64, dp: i32| {
        let scale = 10f64.powi(dp);
        (value * scale).round() / scale
    };
    let mag = sd.log10().floor() as i32;
    let mut decimals = 1 - mag; // decimal places that keep 2 significant figures
    let mut sd_rounded = round_to(sd, decimals);
    // Rounding may bump the magnitude up (e.g. 0.0999 -> 0.10): re-fit decimals.
    if sd_rounded > 0.0 && (sd_rounded.log10().floor() as i32) > mag {
        decimals -= 1;
        sd_rounded = round_to(sd, decimals);
    }
    let mean_rounded = round_to(mean, decimals);
    let p = decimals.max(0) as usize;
    format!("{:.*}{}±{:.*}{}", p, mean_rounded, suffix, p, sd_rounded, suffix)
}

/// `vendor/common/test.h::sample_standard_error`.
fn sample_standard_error(sum: f64, sum_sq: f64, count: i64) -> f64 {
    if count < 2 {
        return 0.0;
    }
    let n = count as f64;
    let mean = sum / n;
    let variance = (sum_sq - n * mean * mean) / (n - 1.0);
    if variance > 0.0 {
        (variance / n).sqrt()
    } else {
        0.0
    }
}

/// Binomial proportion (percent) and its standard error (percent).
fn rate_pct(num: i64, den: i64) -> (f64, f64) {
    if den <= 0 {
        return (0.0, 0.0);
    }
    let p = num as f64 / den as f64;
    let se = (p * (1.0 - p) / den as f64).sqrt();
    (100.0 * p, 100.0 * se)
}

// --- zombie naming (vendor/seml/refresh/name.h) -----------------------------

/// Zombie types that have a display name, in ascending id order — the iteration
/// `for i in 0..33 if ZOMBIE_NAMES.count(i)` from name.h.
const NAMED_TYPES: &[i32] = &[
    zombie::ZOMBIE,
    zombie::CONEHEAD,
    zombie::POLE_VAULTING,
    zombie::BUCKETHEAD,
    zombie::NEWSPAPER,
    zombie::SCREENDOOR,
    zombie::FOOTBALL,
    zombie::DANCING,
    zombie::SNORKEL,
    zombie::ZOMBONI,
    zombie::DOLPHIN_RIDER,
    zombie::JACK_IN_THE_BOX,
    zombie::BALLOON,
    zombie::DIGGER,
    zombie::POGO,
    zombie::BUNGEE,
    zombie::LADDER,
    zombie::CATAPULT,
    zombie::GARGANTUAR,
    zombie::GIGA_GARGANTUAR,
];

/// `zombie_types_to_names`: each named type rendered, present ones as their name
/// and absent ones as `fallback`; an empty input set renders as `无`.
fn zombie_types_to_names(types: &BTreeSet<i32>, fallback: &str) -> String {
    if types.is_empty() {
        return "无".to_string();
    }
    let mut s = String::new();
    for &t in NAMED_TYPES {
        if types.contains(&t) {
            s.push_str(&zombie::name(t));
        } else {
            s.push_str(fallback);
        }
    }
    s
}

/// `all_zombie_types_to_names`: every named type concatenated, in id order.
fn all_zombie_types_to_names() -> String {
    NAMED_TYPES.iter().map(|&t| zombie::name(t)).collect()
}

fn types_set(v: &Value) -> BTreeSet<i32> {
    arr(v, "types")
        .iter()
        .filter_map(|t| t.as_i64().map(|x| x as i32))
        .collect()
}

// --- pos --------------------------------------------------------------------

pub fn pos(v: &Value, p: &Params) -> String {
    let time_mode = v.get("mode").and_then(|m| m.as_str()) == Some("time");
    let columns = arr(v, "columns");
    let stats = arr(v, "stats");
    let show_std = p.show_std;
    let mut s = String::new();

    // Row 1: wave_length printed in the first column of each wave.
    let mut prev_wave = i64::MIN;
    for col in columns {
        s.push(',');
        let wi = i(col, "waveIdx");
        if wi != prev_wave {
            s.push_str(&i(col, "waveLength").to_string());
            prev_wave = wi;
        }
    }
    s.push('\n');

    // Row 2: zombie type names.
    s.push_str("僵尸类别");
    for col in columns {
        s.push(',');
        s.push_str(&zombie::name(i(col, "zombieType") as i32));
    }
    s.push('\n');

    // Rate row.
    s.push_str(if time_mode { "到达率" } else { "存活率" });
    for st in stats {
        let total = i(st, "totalCount");
        let num = if time_mode {
            i(st, "arrivedCount")
        } else {
            i(st, "aliveCount")
        };
        let (mean, se) = rate_pct(num, total);
        s.push(',');
        if show_std {
            s.push_str(&format_mean_std(mean, se, "%", 2));
        } else {
            s.push_str(&format!("{:.2}%", mean));
        }
    }
    s.push('\n');

    // Count row.
    s.push_str(if time_mode { "到达数" } else { "存活数" });
    for st in stats {
        s.push(',');
        let num = if time_mode {
            i(st, "arrivedCount")
        } else {
            i(st, "aliveCount")
        };
        s.push_str(&num.to_string());
    }
    s.push('\n');

    // Total row.
    s.push_str("总数");
    for st in stats {
        s.push(',');
        s.push_str(&i(st, "totalCount").to_string());
    }
    s.push('\n');

    // min / max rows.
    let (min_label, max_label) = if time_mode {
        ("时刻min", "时刻max")
    } else {
        ("坐标min", "坐标max")
    };
    for (label, key) in [(min_label, "min"), (max_label, "max")] {
        s.push_str(label);
        for st in stats {
            s.push(',');
            if time_mode {
                if i(st, "arrivedCount") > 0 {
                    let field = if key == "min" { "minTick" } else { "maxTick" };
                    s.push_str(&i(st, field).to_string());
                }
            } else if i(st, "aliveCount") > 0 {
                let field = if key == "min" { "minX" } else { "maxX" };
                s.push_str(&format!("{:.3}", f(st, field).unwrap_or(0.0)));
            }
        }
        s.push('\n');
    }

    // Blank separator row, then the 累积概率 marker row.
    for _ in columns {
        s.push(',');
    }
    s.push('\n');
    s.push_str("累积概率");
    for _ in columns {
        s.push(',');
    }
    s.push('\n');

    // Cumulative-probability histogram.
    let mut keys: BTreeSet<i64> = BTreeSet::new();
    for st in stats {
        if let Some(h) = hist(st) {
            for k in h.keys() {
                if let Ok(k) = k.parse::<i64>() {
                    keys.insert(k);
                }
            }
        }
    }
    let mut cumulative = vec![0_i64; stats.len()];
    for key in keys {
        s.push_str(&key.to_string());
        for (idx, st) in stats.iter().enumerate() {
            s.push(',');
            if let Some(c) = hist(st)
                .and_then(|h| h.get(&key.to_string()))
                .and_then(|x| x.as_i64())
            {
                cumulative[idx] += c;
            }
            let total = i(st, "totalCount");
            let in_range = if time_mode {
                let arrived = i(st, "arrivedCount");
                arrived > 0
                    && total > 0
                    && key >= i(st, "minTick")
                    && key <= i(st, "maxTick")
            } else {
                let alive = i(st, "aliveCount");
                let min = f(st, "minX").map(|x| x as i64);
                let max = f(st, "maxX").map(|x| x as i64);
                alive > 0
                    && total > 0
                    && min.is_some_and(|m| key >= m)
                    && max.is_some_and(|m| key <= m)
            };
            if in_range {
                let q = cumulative[idx] as f64 / total as f64;
                if show_std {
                    let se = (q * (1.0 - q) / total as f64).sqrt();
                    s.push_str(&format_mean_std(q, se, "", 6));
                } else {
                    s.push_str(&format!("{:.6}", q));
                }
            }
        }
        s.push('\n');
    }

    s
}

// --- smash ------------------------------------------------------------------

pub fn smash(v: &Value, scene: &str, p: &Params) -> String {
    let show_std = p.show_std;
    let protect = arr(v, "protectPositions");
    let action_infos = arr(v, "actionInfos");
    let summary = arr(v, "summary");
    let table = arr(v, "table");

    let protect_count = protect.len();
    // is_backyard(scene_type) == pool || fog; the seml scene mapper sends both
    // PE and FE to the "FE" bucket, so backyard <=> mapped scene "FE".
    let total_garg_rows = if scene == "FE" { 4 } else { 5 };
    let fmt_rate = |smashed: i64, total: i64| -> String {
        let k = 500.0 * protect_count as f64 / total_garg_rows as f64;
        let pp = if total > 0 {
            smashed as f64 / total as f64
        } else {
            0.0
        };
        let mean = k * pp;
        if show_std {
            let se = if total > 0 {
                k * (pp * (1.0 - pp) / total as f64).sqrt()
            } else {
                0.0
            };
            format_mean_std(mean, se, "%", 2)
        } else {
            format!("{:.2}%", mean)
        }
    };

    // Wave totals, keyed by wave number, for the op-state table denominator.
    let wave_total = |wave: i64| -> i64 {
        summary
            .iter()
            .find(|s| i(s, "wave") == wave)
            .map(|s| i(s, "totalGargCount"))
            .unwrap_or(0)
    };

    let mut s = String::new();

    // 单波砸率 header (wave columns).
    s.push_str("单波砸率,");
    for sm in summary {
        s.push_str(&format!("w{},", i(sm, "wave")));
    }
    s.push('\n');

    // 总和 row.
    s.push_str("总和,");
    for sm in summary {
        s.push_str(&fmt_rate(i(sm, "smashedGargCount"), i(sm, "totalGargCount")));
        s.push(',');
    }
    s.push('\n');

    // Per protect-position rows.
    for pp in protect {
        let row = i(pp, "row");
        let col = i(pp, "col");
        let is_cob = pp.get("isCob").and_then(|b| b.as_bool()).unwrap_or(false);
        s.push_str(&format!("{}路{}", row, col));
        s.push_str(if is_cob { "炮," } else { "普通," });
        for sm in summary {
            let by_row = arr(sm, "smashedByRow");
            let smashed = by_row
                .get((row - 1) as usize)
                .and_then(|x| x.as_i64())
                .unwrap_or(0);
            s.push_str(&fmt_rate(smashed, i(sm, "totalGargCount")));
            s.push(',');
        }
        s.push('\n');
    }

    // Op-state table: blank line + header, then one row per op-state combination.
    s.push_str("\n出生波数,单波砸率,砸炮数,总数,");
    let mut prev_wave = i64::MIN;
    for ai in action_infos {
        let wave = i(ai, "wave");
        if wave != prev_wave {
            s.push_str(&format!("[w{}] ", wave));
            prev_wave = wave;
        }
        s.push_str(ai.get("desc").and_then(|d| d.as_str()).unwrap_or(""));
        s.push(',');
    }
    for pp in protect {
        s.push_str(&format!("{}路,", i(pp, "row")));
    }
    s.push('\n');

    for row in table {
        let wave = i(row, "wave");
        let smashed = i(row, "smashedGargCount");
        let total = i(row, "totalGargCount");
        s.push_str(&format!("{},", wave));
        s.push_str(&fmt_rate(smashed, wave_total(wave)));
        s.push_str(&format!(",{},{},", smashed, total));
        for st in arr(row, "opStates") {
            s.push_str(op_state_to_string(st.as_i64().unwrap_or(0)));
            s.push(',');
        }
        let by_row = arr(row, "smashedByRow");
        for pp in protect {
            let r = i(pp, "row");
            let c = by_row
                .get((r - 1) as usize)
                .and_then(|x| x.as_i64())
                .unwrap_or(0);
            s.push_str(&format!("{},", c));
        }
        s.push('\n');
    }

    s
}

/// `vendor/seml/smash/data.h::op_state_to_string`. 0=Dead 1=Hit 2=Miss 3=NotBorn.
fn op_state_to_string(state: i64) -> &'static str {
    match state {
        1 => "HIT",
        2 => "MISS",
        _ => "", // Dead, NotBorn
    }
}

// --- explode ----------------------------------------------------------------

pub fn explode(v: &Value, p: &Params) -> String {
    let show_std = p.show_std;
    let repeat = i(v, "repeat").max(1);
    let waves = arr(v, "waves");
    let n_waves = waves.len();

    // Per-wave action descriptions, stacked vertically in the header.
    let headers: Vec<Vec<String>> = waves
        .iter()
        .map(|w| {
            arr(w, "actions")
                .iter()
                .map(|a| a.as_str().unwrap_or("").to_string())
                .collect()
        })
        .collect();
    let max_header_count = headers.iter().map(|h| h.len()).max().unwrap_or(0);

    // Tick range: min start to max (start + loss_count - 1) across waves.
    let mut first = i64::MAX;
    let mut last = i64::MIN;
    for w in waves {
        let start = i(w, "startTick");
        let len = arr(w, "lossInfos").len() as i64;
        first = first.min(start);
        if len > 0 {
            last = last.max(start + len - 1);
        }
    }
    if waves.is_empty() {
        first = 0;
        last = -1;
    }

    let mut s = String::new();

    // Section labels row: ",炮伤" + n_waves commas, repeated for 瞬伤 and 损伤.
    for label in ["炮伤", "瞬伤", "损伤"] {
        s.push(',');
        s.push_str(label);
        for _ in 0..n_waves {
            s.push(',');
        }
    }
    s.push('\n');

    // Stacked action-description header rows.
    for hi in 0..max_header_count {
        if hi == max_header_count - 1 {
            s.push_str("时刻");
        }
        s.push(',');
        for _ in 0..3 {
            for h in &headers {
                if hi < h.len() {
                    s.push_str(&h[hi]);
                }
                s.push(',');
            }
            s.push(',');
        }
        s.push('\n');
    }

    // Per-tick data rows.
    let to_string = |vals: &[Option<f64>], ses: &[Option<f64>]| -> String {
        let valid_count = vals.iter().filter(|x| x.is_some()).count();
        let mut min_idx: Option<usize> = None;
        if valid_count > 1 {
            let mut min: Option<f64> = None;
            for (idx, val) in vals.iter().enumerate() {
                if let Some(x) = val {
                    if min.map_or(true, |m| *x <= m) {
                        min = Some(*x);
                        min_idx = Some(idx);
                    }
                }
            }
        }
        let mut out = String::new();
        for (idx, val) in vals.iter().enumerate() {
            if let Some(x) = val {
                let cell = if show_std {
                    format_mean_std(*x, ses[idx].unwrap_or(0.0), "", 3)
                } else {
                    format!("{:.3}", x)
                };
                if min_idx == Some(idx) {
                    out.push('[');
                    out.push_str(&cell);
                    out.push(']');
                } else {
                    out.push_str(&cell);
                }
            }
            out.push(',');
        }
        out
    };

    let mut tick = first;
    while tick <= last {
        s.push_str(&format!("{},", tick));

        let mut loss = Vec::with_capacity(n_waves);
        let mut explode_loss = Vec::with_capacity(n_waves);
        let mut hp_loss = Vec::with_capacity(n_waves);
        let mut loss_se = Vec::with_capacity(n_waves);
        let mut explode_se = Vec::with_capacity(n_waves);
        let mut hp_se = Vec::with_capacity(n_waves);

        for w in waves {
            let start = i(w, "startTick");
            let infos = arr(w, "lossInfos");
            let sqs = arr(w, "lossSumSq");
            if tick < start || tick >= start + infos.len() as i64 {
                loss.push(None);
                explode_loss.push(None);
                hp_loss.push(None);
                loss_se.push(None);
                explode_se.push(None);
                hp_se.push(None);
            } else {
                let idx = (tick - start) as usize;
                let li = &infos[idx];
                let sq = sqs.get(idx);
                let explode = (i(li, "fromUpper") + i(li, "fromSame") + i(li, "fromLower")) as f64
                    * 300.0;
                let hp = i(li, "hpLoss") as f64;
                let r = repeat as f64;
                loss.push(Some((explode + hp) / r));
                explode_loss.push(Some(explode / r));
                hp_loss.push(Some(hp / r));
                let sq_explode = sq.map(|q| f(q, "explode").unwrap_or(0.0)).unwrap_or(0.0);
                let sq_hp = sq.map(|q| f(q, "hp").unwrap_or(0.0)).unwrap_or(0.0);
                let sq_total = sq.map(|q| f(q, "total").unwrap_or(0.0)).unwrap_or(0.0);
                loss_se.push(Some(sample_standard_error(explode + hp, sq_total, repeat)));
                explode_se.push(Some(sample_standard_error(explode, sq_explode, repeat)));
                hp_se.push(Some(sample_standard_error(hp, sq_hp, repeat)));
            }
        }

        s.push_str(&to_string(&loss, &loss_se));
        s.push(',');
        s.push_str(&to_string(&explode_loss, &explode_se));
        s.push(',');
        s.push_str(&to_string(&hp_loss, &hp_se));
        s.push(',');
        s.push('\n');

        tick += 1;
    }

    s
}

// --- refresh ----------------------------------------------------------------

pub fn refresh(v: &Value, p: &Params) -> String {
    let show_std = p.show_std;
    let scene = v.get("scene").and_then(|s| s.as_str()).unwrap_or("?");
    let headers = arr(v, "headers");
    let cols = arr(v, "cols");

    let huge = p.huge.unwrap_or(false);
    let activate = p.activate.unwrap_or(false);
    let dance = p.dance.unwrap_or(false);
    let natural = p.natural.unwrap_or(false);
    // get_dance_cheat: dance off => none; on => fast if activate else slow.
    let dance_str = if !dance {
        "无dance"
    } else if activate {
        "dance快"
    } else {
        "dance慢"
    };

    let max_header_count = headers
        .iter()
        .map(|h| h.as_array().map(|a| a.len()).unwrap_or(0))
        .max()
        .unwrap_or(0);
    let max_row_count = cols
        .iter()
        .map(|c| arr(c, "rows").len())
        .max()
        .unwrap_or(0);

    let require: BTreeSet<i32> = p
        .require
        .clone()
        .unwrap_or_default()
        .into_iter()
        .collect();
    let ban: BTreeSet<i32> = p.ban.clone().unwrap_or_default().into_iter().collect();

    let mut s = String::new();

    // Environment header lines.
    s.push_str(&format!(
        "测试环境: {} {} {} {} {}\n",
        scene,
        if huge { "旗帜波" } else { "普通波" },
        if activate { "激活" } else { "分离" },
        dance_str,
        if natural { "自然出怪" } else { "均匀出怪" },
    ));
    s.push_str(&format!(
        "必出类型: {}\n",
        zombie_types_to_names(&require, "")
    ));
    s.push_str(&format!("禁出类型: {}\n", zombie_types_to_names(&ban, "")));

    // Stacked action-description header rows (3 sub-columns per wave).
    for hi in 0..max_header_count {
        for h in headers {
            let cell = h
                .as_array()
                .and_then(|a| a.get(hi))
                .and_then(|x| x.as_str())
                .unwrap_or("");
            s.push(',');
            s.push_str(cell);
            s.push_str(",,");
        }
        s.push('\n');
    }

    // 平均意外率 row.
    for col in cols {
        s.push_str("平均意外率,");
        let mean = f(col, "averageAccidentRate").unwrap_or(0.0) * 100.0;
        if show_std {
            let se = f(col, "averageAccidentRateSe").unwrap_or(0.0) * 100.0;
            s.push_str(&format_mean_std(mean, se, "%", 3));
            s.push_str(",,");
        } else {
            s.push_str(&format!("{:.3}%,,", mean));
        }
    }
    s.push('\n');

    // all-zombie-types header.
    let all_names = all_zombie_types_to_names();
    for _ in cols {
        s.push_str(&all_names);
        s.push_str(",比照,,");
    }
    s.push('\n');

    // Per-row type/rate cells.
    for ri in 0..max_row_count {
        for col in cols {
            let rows = arr(col, "rows");
            if ri < rows.len() {
                let row = &rows[ri];
                s.push_str(&zombie_types_to_names(&types_set(row), "　"));
                s.push(',');
                let mean = f(row, "mean").unwrap_or(0.0) * 100.0;
                if show_std {
                    let se = f(row, "se").unwrap_or(0.0) * 100.0;
                    s.push_str(&format_mean_std(mean, se, "%", 3));
                    s.push(',');
                } else {
                    s.push_str(&format!("{:.3}%,", mean));
                }
            } else {
                s.push_str(",,");
            }
            s.push(',');
        }
        s.push('\n');
    }

    s
}

// --- pogo -------------------------------------------------------------------

pub fn pogo(v: &Value) -> String {
    let ticks = arr(v, "ticks");
    let mut s = String::new();

    s.push_str("时刻,收上行跳跳左,右,收本行跳跳左,右,收下行跳跳左,右,\n");
    for t in ticks {
        s.push_str(&i(t, "tick").to_string());
        s.push(',');
        let ranges = arr(t, "ranges");
        // The reference driver writes the rows in reverse index order (i=2,1,0).
        for k in (0..3).rev() {
            match ranges.get(k) {
                Some(r) if !r.is_null() => {
                    s.push_str(&format!("{},{},", i(r, "min"), i(r, "max")));
                }
                _ => s.push_str("ERR,ERR,"),
            }
        }
        s.push('\n');
    }

    s
}
