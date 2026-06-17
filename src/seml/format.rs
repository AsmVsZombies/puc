//! Clean aligned-text output for each calculator's result JSON. Numbers are
//! derived the same way the standalone CSV drivers do (rates, averages, SEs);
//! the `std:` header toggles `± se` columns.

use serde_json::Value;
use std::collections::BTreeSet;

use super::types::Params;
use super::zombie;

// --- small helpers ----------------------------------------------------------

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

/// Binomial proportion (percent) and its standard error.
fn rate_pct(num: i64, den: i64) -> (f64, f64) {
    if den <= 0 {
        return (0.0, 0.0);
    }
    let p = num as f64 / den as f64;
    let se = (p * (1.0 - p) / den as f64).sqrt();
    (100.0 * p, 100.0 * se)
}

fn fmt_pct(mean: f64, se: f64, show_std: bool) -> String {
    if show_std {
        format!("{:.2}±{:.2}%", mean, se)
    } else {
        format!("{:.2}%", mean)
    }
}

fn fmt_prob(q: f64, total: i64, show_std: bool) -> String {
    if show_std {
        let se = if total > 0 {
            (q * (1.0 - q) / total as f64).sqrt()
        } else {
            0.0
        };
        format!("{:.6}±{:.6}", q, se)
    } else {
        format!("{:.6}", q)
    }
}

// --- pos --------------------------------------------------------------------

pub fn pos(v: &Value, p: &Params, compact: bool) {
    let time_mode = v.get("mode").and_then(|m| m.as_str()) == Some("time");
    let columns = arr(v, "columns");
    let stats = arr(v, "stats");

    outln!(
        "seml pos mode={} repeat={}",
        if time_mode { "time" } else { "pos" },
        p.repeat
            .map(|r| r.to_string())
            .unwrap_or_else(|| "default".into()),
    );
    if time_mode {
        outln!(
            "  {:<6} {:<8} {:<14} {:<10} {:<8} {:<8}",
            "wave",
            "zombie",
            t!("seml_arrival_rate"),
            t!("seml_arrival_count"),
            t!("seml_tick_min"),
            t!("seml_tick_max")
        );
    } else {
        outln!(
            "  {:<6} {:<8} {:<14} {:<10} {:<8} {:<8}",
            "wave",
            "zombie",
            t!("seml_alive_rate"),
            t!("seml_alive_count"),
            t!("seml_x_min"),
            t!("seml_x_max")
        );
    }

    for (col, st) in columns.iter().zip(stats.iter()) {
        let wave = i(col, "waveIdx") + 1;
        let zname = zombie::name_i18n(i(col, "zombieType") as i32);
        let total = i(st, "totalCount");
        if time_mode {
            let arrived = i(st, "arrivedCount");
            let (mean, se) = rate_pct(arrived, total);
            let min = f(st, "minTick")
                .map(|x| format!("{}", x as i64))
                .unwrap_or_else(|| "—".into());
            let max = f(st, "maxTick")
                .map(|x| format!("{}", x as i64))
                .unwrap_or_else(|| "—".into());
            outln!(
                "  {:<6} {:<8} {:<14} {:<10} {:<8} {:<8}",
                wave,
                zname,
                fmt_pct(mean, se, p.show_std),
                arrived,
                min,
                max
            );
        } else {
            let alive = i(st, "aliveCount");
            let (mean, se) = rate_pct(alive, total);
            let min = f(st, "minX")
                .map(|x| format!("{:.1}", x))
                .unwrap_or_else(|| "—".into());
            let max = f(st, "maxX")
                .map(|x| format!("{:.1}", x))
                .unwrap_or_else(|| "—".into());
            outln!(
                "  {:<6} {:<8} {:<14} {:<10} {:<8} {:<8}",
                wave,
                zname,
                fmt_pct(mean, se, p.show_std),
                alive,
                min,
                max
            );
        }
    }

    if !compact {
        print_pos_cumulative(time_mode, columns, stats, p.show_std);
    }
}

fn print_pos_cumulative(time_mode: bool, columns: &[Value], stats: &[Value], show_std: bool) {
    let mut keys = BTreeSet::new();
    for st in stats {
        if let Some(h) = hist(st) {
            for key in h.keys() {
                if let Ok(k) = key.parse::<i32>() {
                    keys.insert(k);
                }
            }
        }
    }
    if keys.is_empty() {
        return;
    }

    let labels: Vec<String> = columns
        .iter()
        .map(|col| {
            format!(
                "w{}{}",
                i(col, "waveIdx") + 1,
                zombie::name_i18n(i(col, "zombieType") as i32)
            )
        })
        .collect();
    let mut cumulative = vec![0_i64; stats.len()];

    outln!("\n  {}", t!("seml_cumulative_prob"));
    out!("  {:<8}", if time_mode { "tick" } else { "x" });
    for label in &labels {
        out!(" {:<16}", label);
    }
    outln!();

    for key in keys {
        out!("  {:<8}", key);
        for (idx, st) in stats.iter().enumerate() {
            if let Some(count) = hist(st)
                .and_then(|h| h.get(&key.to_string()))
                .and_then(|v| v.as_i64())
            {
                cumulative[idx] += count;
            }

            let total = i(st, "totalCount");
            let in_range = if time_mode {
                let arrived = i(st, "arrivedCount");
                let min = st.get("minTick").and_then(|x| x.as_i64());
                let max = st.get("maxTick").and_then(|x| x.as_i64());
                arrived > 0
                    && total > 0
                    && min.is_some_and(|m| key as i64 >= m)
                    && max.is_some_and(|m| key as i64 <= m)
            } else {
                let alive = i(st, "aliveCount");
                let min = f(st, "minX").map(|x| x as i32);
                let max = f(st, "maxX").map(|x| x as i32);
                alive > 0
                    && total > 0
                    && min.is_some_and(|m| key >= m)
                    && max.is_some_and(|m| key <= m)
            };

            if in_range {
                let q = cumulative[idx] as f64 / total as f64;
                out!(" {:<16}", fmt_prob(q, total, show_std));
            } else {
                out!(" {:<16}", "");
            }
        }
        outln!();
    }
}

// --- smash ------------------------------------------------------------------

pub fn smash(v: &Value, p: &Params, compact: bool) {
    let protect = arr(v, "protectPositions");
    let action_infos = arr(v, "actionInfos");
    let summary = arr(v, "summary");
    let table = arr(v, "table");

    let prot_str: Vec<String> = protect
        .iter()
        .map(|pp| {
            format!(
                "{}{}{}",
                i(pp, "row"),
                i(pp, "col"),
                if pp.get("isCob").and_then(|b| b.as_bool()).unwrap_or(false) {
                    ""
                } else {
                    "'"
                }
            )
        })
        .collect();
    outln!(
        "seml smash protect={}",
        if prot_str.is_empty() {
            "—".into()
        } else {
            prot_str.join(" ")
        }
    );

    // Per-wave summary.
    outln!(
        "  {:<6} {:<16} {:<10} {}",
        "wave",
        t!("seml_smash_rate"),
        t!("seml_smash_over_total"),
        t!("seml_by_row")
    );
    for s in summary {
        let wave = i(s, "wave");
        let smashed = i(s, "smashedGargCount");
        let total = i(s, "totalGargCount");
        let (mean, se) = rate_pct(smashed, total);
        let by_row: Vec<String> = arr(s, "smashedByRow")
            .iter()
            .map(|x| x.as_i64().unwrap_or(0).to_string())
            .collect();
        outln!(
            "  {:<6} {:<16} {:<10} {}",
            wave,
            fmt_pct(mean, se, p.show_std),
            format!("{}/{}", smashed, total),
            by_row.join(" "),
        );
    }

    // Per-op-state breakdown: columns align with actionInfos; OpState
    // 0=Dead 1=Hit 2=Miss 3=NotBorn. Display only effective ops: Hit.
    if !compact && !table.is_empty() && !action_infos.is_empty() {
        let actions: Vec<(i64, String)> = action_infos
            .iter()
            .map(|a| {
                (
                    i(a, "wave"),
                    a.get("desc")
                        .and_then(|d| d.as_str())
                        .unwrap_or("")
                        .to_string(),
                )
            })
            .collect();
        outln!("\n  {}", t!("seml_by_effective_ops"));
        for row in table {
            let ops = effective_smash_ops(&actions, arr(row, "opStates"));
            let smashed = i(row, "smashedGargCount");
            let total = i(row, "totalGargCount");
            let (mean, se) = rate_pct(smashed, total);
            outln!(
                "  [{}] {} ({}/{})",
                ops,
                fmt_pct(mean, se, p.show_std),
                smashed,
                total,
            );
        }
    }
}

fn effective_smash_ops(actions: &[(i64, String)], states: &[Value]) -> String {
    let mut groups: Vec<(i64, Vec<String>)> = Vec::new();
    for ((wave, desc), state) in actions.iter().zip(states.iter()) {
        if state.as_i64().unwrap_or(0) != 1 {
            continue;
        }
        if let Some((last_wave, labels)) = groups.last_mut() {
            if *last_wave == *wave {
                labels.push(desc.clone());
                continue;
            }
        }
        groups.push((*wave, vec![desc.clone()]));
    }

    if groups.is_empty() {
        "—".to_string()
    } else {
        groups
            .into_iter()
            .map(|(wave, labels)| format!("w{} {}", wave, labels.join(", ")))
            .collect::<Vec<_>>()
            .join("; ")
    }
}

// --- explode ----------------------------------------------------------------

pub fn explode(v: &Value, _p: &Params, compact: bool) {
    let repeat = i(v, "repeat").max(1) as f64;
    let waves = arr(v, "waves");

    outln!("seml explode repeat={}", i(v, "repeat"));
    for (wi, wave) in waves.iter().enumerate() {
        let start_tick = i(wave, "startTick");
        let actions: Vec<String> = arr(wave, "actions")
            .iter()
            .map(|a| a.as_str().unwrap_or("").to_string())
            .collect();
        outln!(
            "\n  wave={} start_tick={} actions={}",
            wi + 1,
            start_tick,
            if actions.is_empty() {
                "—".to_string()
            } else {
                actions.join(" ")
            }
        );
        outln!(
            "    {:<8} {:<10} {:<10} {:<10} {:<12}",
            "tick",
            t!("seml_dmg_upper"),
            t!("seml_dmg_same"),
            t!("seml_dmg_lower"),
            t!("seml_hp_loss")
        );

        let loss = arr(wave, "lossInfos");
        let end_tick = start_tick + loss.len().saturating_sub(1) as i64;
        for (idx, li) in loss.iter().enumerate() {
            let tick = start_tick + idx as i64;
            if compact && !sample_tick(tick, start_tick, end_tick) {
                continue;
            }
            let upper = i(li, "fromUpper") as f64 / repeat;
            let same = i(li, "fromSame") as f64 / repeat;
            let lower = i(li, "fromLower") as f64 / repeat;
            let hp = i(li, "hpLoss") as f64 / repeat;
            outln!(
                "    {:<8} {:<10.1} {:<10.1} {:<10.1} {:<12.1}",
                tick, upper, same, lower, hp,
            );
        }
    }
}

fn sample_tick(tick: i64, start: i64, end: i64) -> bool {
    tick == start || tick == end || tick % 50 == 0
}

// --- refresh ----------------------------------------------------------------

pub fn refresh(v: &Value, p: &Params, compact: bool) {
    let scene = v.get("scene").and_then(|s| s.as_str()).unwrap_or("?");
    let cols = arr(v, "cols");

    outln!("seml refresh scene={}", scene);
    for (ci, col) in cols.iter().enumerate() {
        let mean = f(col, "averageAccidentRate").unwrap_or(0.0) * 100.0;
        let se = f(col, "averageAccidentRateSe").unwrap_or(0.0) * 100.0;
        outln!(
            "  {}",
            t!("seml_refresh_group", n = ci + 1, rate = fmt_pct(mean, se, p.show_std))
        );
        if compact {
            continue;
        }
        for row in arr(col, "rows").iter().take(20) {
            let types: Vec<String> = arr(row, "types")
                .iter()
                .filter_map(|t| {
                    let id = t.as_i64().unwrap_or(0) as i32;
                    (id != zombie::YETI).then(|| zombie::name_i18n(id))
                })
                .collect();
            let rmean = f(row, "mean").unwrap_or(0.0) * 100.0;
            let rse = f(row, "se").unwrap_or(0.0) * 100.0;
            let label = if types.is_empty() {
                t!("seml_none").to_string()
            } else {
                types.join("")
            };
            outln!("    {:<12} {}", label, fmt_pct(rmean, rse, p.show_std));
        }
    }
}

// --- pogo -------------------------------------------------------------------

pub fn pogo(v: &Value, _p: &Params, compact: bool) {
    let start = i(v, "startTick");
    let wave_len = i(v, "waveLength");
    let ticks = arr(v, "ticks");

    outln!("seml pogo start_tick={} wave_length={}", start, wave_len);
    outln!(
        "  {:<8} {:<14} {:<14} {:<14}",
        "tick",
        t!("seml_row_upper"),
        t!("seml_row_same"),
        t!("seml_row_lower")
    );

    let fmt_range = |r: &Value| -> String {
        if r.is_null() {
            "ERR".to_string()
        } else {
            format!("{}~{}", i(r, "min"), i(r, "max"))
        }
    };

    let visible_ticks: Vec<&Value> = ticks
        .iter()
        .filter(|t| {
            let ranges = arr(t, "ranges");
            !ranges.iter().all(|r| r.is_null())
        })
        .collect();
    let first_tick = visible_ticks.first().map(|t| i(t, "tick")).unwrap_or(start);
    let last_tick = visible_ticks.last().map(|t| i(t, "tick")).unwrap_or(start);

    for t in visible_ticks {
        let tick = i(t, "tick");
        if compact && !sample_tick(tick, first_tick, last_tick) {
            continue;
        }
        let ranges = arr(t, "ranges");
        let cells: Vec<String> = (0..3)
            .map(|k| ranges.get(k).map(fmt_range).unwrap_or_else(|| "ERR".into()))
            .collect();
        outln!(
            "  {:<8} {:<14} {:<14} {:<14}",
            tick, cells[0], cells[1], cells[2]
        );
    }
}

// --- survive -----------------------------------------------------------------

pub fn survive(v: &Value, p: &Params, _compact: bool) {
    let columns = arr(v, "columns");
    let stats = arr(v, "stats");

    outln!("seml survive repeat={} hitThres={}", i(v, "repeat"), i(v, "hitThres"));
    outln!(
        "  {:<6} {:<8} {:<14} {:<10} {:<10} {:<10}",
        "wave",
        "zombie",
        t!("seml_hit_rate"),
        t!("seml_hit_count"),
        t!("seml_hit_avg_hp"),
        t!("seml_nothit_avg_hp")
    );

    for (col, st) in columns.iter().zip(stats.iter()) {
        let wave = i(col, "waveIdx") + 1;
        let zname = zombie::name_i18n(i(col, "zombieType") as i32);
        let total = i(st, "totalCount");
        let hit = i(st, "hitCount");
        let not_hit = total - hit;
        let (mean, se) = rate_pct(hit, total);
        // hp averages are over each bucket; an empty bucket shows 0.
        let hit_avg = if hit > 0 {
            f(st, "hitHpSum").unwrap_or(0.0) / hit as f64
        } else {
            0.0
        };
        let not_hit_avg = if not_hit > 0 {
            f(st, "notHitHpSum").unwrap_or(0.0) / not_hit as f64
        } else {
            0.0
        };
        outln!(
            "  {:<6} {:<8} {:<14} {:<10} {:<10.1} {:<10.1}",
            wave,
            zname,
            fmt_pct(mean, se, p.show_std),
            hit,
            hit_avg,
            not_hit_avg
        );
    }
}
