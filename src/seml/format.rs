//! Clean aligned-text output for each calculator's result JSON. Numbers are
//! derived the same way the standalone CSV drivers do (rates, averages, SEs);
//! the `std:` header toggles `± se` columns.

use serde_json::Value;

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
    v.get(key).and_then(|x| x.as_array()).map(|a| a.as_slice()).unwrap_or(&[])
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

// --- pos --------------------------------------------------------------------

pub fn pos(v: &Value, p: &Params) {
    let time_mode = v.get("mode").and_then(|m| m.as_str()) == Some("time");
    let columns = arr(v, "columns");
    let stats = arr(v, "stats");

    println!(
        "seml pos mode={} repeat={}",
        if time_mode { "time" } else { "pos" },
        p.repeat.map(|r| r.to_string()).unwrap_or_else(|| "default".into()),
    );
    if time_mode {
        println!("  {:<6} {:<8} {:<14} {:<10} {:<8} {:<8}", "wave", "zombie", "到达率", "到达数", "时刻min", "时刻max");
    } else {
        println!("  {:<6} {:<8} {:<14} {:<10} {:<8} {:<8}", "wave", "zombie", "存活率", "存活数", "坐标min", "坐标max");
    }

    for (col, st) in columns.iter().zip(stats.iter()) {
        let wave = i(col, "waveIdx") + 1;
        let zname = zombie::name(i(col, "zombieType") as i32);
        let total = i(st, "totalCount");
        if time_mode {
            let arrived = i(st, "arrivedCount");
            let (mean, se) = rate_pct(arrived, total);
            let min = f(st, "minTick").map(|x| format!("{}", x as i64)).unwrap_or_else(|| "—".into());
            let max = f(st, "maxTick").map(|x| format!("{}", x as i64)).unwrap_or_else(|| "—".into());
            println!(
                "  {:<6} {:<8} {:<14} {:<10} {:<8} {:<8}",
                wave, zname, fmt_pct(mean, se, p.show_std), arrived, min, max
            );
        } else {
            let alive = i(st, "aliveCount");
            let (mean, se) = rate_pct(alive, total);
            let min = f(st, "minX").map(|x| format!("{:.1}", x)).unwrap_or_else(|| "—".into());
            let max = f(st, "maxX").map(|x| format!("{:.1}", x)).unwrap_or_else(|| "—".into());
            println!(
                "  {:<6} {:<8} {:<14} {:<10} {:<8} {:<8}",
                wave, zname, fmt_pct(mean, se, p.show_std), alive, min, max
            );
        }
    }
}

// --- smash ------------------------------------------------------------------

pub fn smash(v: &Value, p: &Params) {
    let protect = arr(v, "protectPositions");
    let action_infos = arr(v, "actionInfos");
    let summary = arr(v, "summary");
    let table = arr(v, "table");

    let prot_str: Vec<String> = protect
        .iter()
        .map(|pp| {
            format!("{}{}{}", i(pp, "row"), i(pp, "col"), if pp.get("isCob").and_then(|b| b.as_bool()).unwrap_or(false) { "" } else { "'" })
        })
        .collect();
    println!("seml smash protect={}", if prot_str.is_empty() { "—".into() } else { prot_str.join(" ") });

    // Per-wave summary.
    println!("  {:<6} {:<16} {:<10} {}", "wave", "砸率", "砸/总", "分行 (1..6)");
    for s in summary {
        let wave = i(s, "wave");
        let smashed = i(s, "smashedGargCount");
        let total = i(s, "totalGargCount");
        let (mean, se) = rate_pct(smashed, total);
        let by_row: Vec<String> = arr(s, "smashedByRow").iter().map(|x| x.as_i64().unwrap_or(0).to_string()).collect();
        println!(
            "  {:<6} {:<16} {:<10} {}",
            wave,
            fmt_pct(mean, se, p.show_std),
            format!("{}/{}", smashed, total),
            by_row.join(" "),
        );
    }

    // Per-op-state breakdown: columns align with actionInfos; OpState
    // 0=Dead 1=Hit 2=Miss 3=NotBorn.
    if !table.is_empty() && !action_infos.is_empty() {
        let labels: Vec<String> = action_infos.iter().map(|a| a.get("desc").and_then(|d| d.as_str()).unwrap_or("").to_string()).collect();
        println!("\n  按操作状态 ({}):", labels.join(", "));
        for row in table {
            let wave = i(row, "wave");
            let states: Vec<&str> = arr(row, "opStates")
                .iter()
                .map(|s| match s.as_i64().unwrap_or(0) {
                    1 => "中",
                    2 => "空",
                    3 => "未生成",
                    _ => "死",
                })
                .collect();
            let smashed = i(row, "smashedGargCount");
            let total = i(row, "totalGargCount");
            let (mean, se) = rate_pct(smashed, total);
            println!(
                "  w{} [{}] {} ({}/{})",
                wave,
                states.join(" "),
                fmt_pct(mean, se, p.show_std),
                smashed,
                total,
            );
        }
    }
}

// --- explode ----------------------------------------------------------------

pub fn explode(v: &Value, _p: &Params) {
    let repeat = i(v, "repeat").max(1) as f64;
    let waves = arr(v, "waves");

    println!("seml explode repeat={}", i(v, "repeat"));
    for (wi, wave) in waves.iter().enumerate() {
        let start_tick = i(wave, "startTick");
        let actions: Vec<String> = arr(wave, "actions").iter().map(|a| a.as_str().unwrap_or("").to_string()).collect();
        println!("\n  wave {} (起始 {}cs){}", wi + 1, start_tick, if actions.is_empty() { String::new() } else { format!(": {}", actions.join(" ")) });
        println!("    {:<8} {:<10} {:<10} {:<10} {:<12}", "tick", "炮伤↑", "炮伤=", "炮伤↓", "损伤");

        let loss = arr(wave, "lossInfos");
        for (idx, li) in loss.iter().enumerate() {
            let upper = i(li, "fromUpper") as f64 / repeat;
            let same = i(li, "fromSame") as f64 / repeat;
            let lower = i(li, "fromLower") as f64 / repeat;
            let hp = i(li, "hpLoss") as f64 / repeat;
            println!(
                "    {:<8} {:<10.1} {:<10.1} {:<10.1} {:<12.1}",
                start_tick + idx as i64,
                upper,
                same,
                lower,
                hp,
            );
        }
    }
}

// --- refresh ----------------------------------------------------------------

pub fn refresh(v: &Value, p: &Params) {
    let scene = v.get("scene").and_then(|s| s.as_str()).unwrap_or("?");
    let cols = arr(v, "cols");

    println!("seml refresh scene={}", scene);
    for (ci, col) in cols.iter().enumerate() {
        let mean = f(col, "averageAccidentRate").unwrap_or(0.0) * 100.0;
        let se = f(col, "averageAccidentRateSe").unwrap_or(0.0) * 100.0;
        println!("  组 {}: 平均意外率 {}", ci + 1, fmt_pct(mean, se, p.show_std));
        for row in arr(col, "rows") {
            let types: Vec<String> = arr(row, "types").iter().map(|t| zombie::name(t.as_i64().unwrap_or(0) as i32)).collect();
            let rmean = f(row, "mean").unwrap_or(0.0) * 100.0;
            let rse = f(row, "se").unwrap_or(0.0) * 100.0;
            let label = if types.is_empty() { "无".to_string() } else { types.join("") };
            println!("    {:<12} {}", label, fmt_pct(rmean, rse, p.show_std));
        }
    }
}

// --- pogo -------------------------------------------------------------------

pub fn pogo(v: &Value, _p: &Params) {
    let start = i(v, "startTick");
    let wave_len = i(v, "waveLength");
    let ticks = arr(v, "ticks");

    println!("seml pogo startTick={} waveLength={}", start, wave_len);
    println!("  {:<8} {:<14} {:<14} {:<14}", "tick", "炮行-1", "炮行+0", "炮行+1");

    let fmt_range = |r: &Value| -> String {
        if r.is_null() {
            "ERR".to_string()
        } else {
            format!("{}~{}", i(r, "min"), i(r, "max"))
        }
    };

    for t in ticks {
        let ranges = arr(t, "ranges");
        // Skip ticks where no row is constrained (all ERR).
        if ranges.iter().all(|r| r.is_null()) {
            continue;
        }
        let cells: Vec<String> = (0..3).map(|k| ranges.get(k).map(fmt_range).unwrap_or_else(|| "ERR".into())).collect();
        println!("  {:<8} {:<14} {:<14} {:<14}", i(t, "tick"), cells[0], cells[1], cells[2]);
    }
}
