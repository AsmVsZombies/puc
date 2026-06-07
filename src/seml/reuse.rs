//! `puc seml reuse <file>` — cob-cannon reuse scheduler.
//!
//! Given a SEML scenario plus the `ncobs:` (cannon pool size) and `loop:` headers,
//! reports which earlier shot's cannon each later shot reuses. Shots are ordered
//! globally by absolute time and cannons are assigned FIFO / round-robin, so the
//! shot at global index `i` reuses the cannon last used by shot `i - ncobs`; the
//! gap between them is the actual recharge interval (must be `>= 3475`cs to be
//! feasible). This is pure timing math — the emulator is not involved.

use super::parser;
use super::types::Action;

/// Cob cannon recharge time (cs).
const RECHARGE: i64 = 3475;

/// One cannon shot, placed on the absolute timeline.
struct Shot {
    wave: i32, // 1-based absolute wave number
    tw: i32,   // time within the wave
    abs: i64,  // absolute time across all preceding waves
}

pub fn run_text(text: &str, compact: bool, strict: bool) -> Result<(), String> {
    let parsed = parser::parse(text, strict)?;
    // `parse_int_arg` already guarantees ncobs > 0 when present.
    let ncobs = parsed.params.ncobs.ok_or("请提供炮数 (ncobs:N)")? as usize;
    let looping = parsed.params.r#loop.unwrap_or(false);

    // One cycle of shots. A PP/DD cob contributes one shot per landing position.
    // A cob's time is not bounded by its wave length (delayed cobs can spill into
    // the next wave's window), so sort by absolute time afterwards to get the true
    // global FIFO order rather than relying on wave-by-wave append order.
    let mut base: Vec<Shot> = Vec::new();
    let mut offset: i64 = 0;
    for (i, wave) in parsed.config.waves.iter().enumerate() {
        let wave_num = (i + 1) as i32;
        for action in &wave.actions {
            if let Action::Cob { time, positions, .. } = action {
                for _ in 0..positions.len() {
                    base.push(Shot {
                        wave: wave_num,
                        tw: *time,
                        abs: offset + *time as i64,
                    });
                }
            }
        }
        offset += wave.wave_length as i64;
    }
    base.sort_by_key(|s| s.abs); // stable: keeps same-time shots (PP/DD) adjacent
    let cycle_len = offset; // total length of one cycle
    let waves_per_cycle = parsed.config.waves.len() as i32;
    let per_cycle = base.len();
    if per_cycle == 0 {
        return Err("未找到用炮操作".to_string());
    }

    outln!("seml reuse ncobs={} loop={}", ncobs, looping);

    // In compact mode only failing reuses (recharge gap < 3475cs) are printed and a
    // lone `all ok` is shown when nothing fails; the `next:` line is kept either way.
    let mut any_fail = false;
    let mut next_line: Option<String> = None;

    if looping {
        // Steady state is cyclic: the reuse target of first-cycle shot `i` is global
        // FIFO index `i + ncobs`, which falls `k` whole cycles later at position
        // `j % per_cycle` within the cycle. Compute it directly so a large `ncobs`
        // never forces materializing all the intervening cycles.
        for i in 0..per_cycle {
            let j = i + ncobs;
            let k = (j / per_cycle) as i64;
            let dst_base = &base[j % per_cycle];
            let dst = Shot {
                wave: dst_base.wave + k as i32 * waves_per_cycle,
                tw: dst_base.tw,
                abs: dst_base.abs + k * cycle_len,
            };
            any_fail |= emit_line(&base[i], &dst, compact);
        }
    } else {
        let len = base.len();
        for i in 0..len {
            if i + ncobs < len {
                any_fail |= emit_line(&base[i], &base[i + ncobs], compact);
            }
        }
        // Per-cannon recharge remaining at sim end (= end of the last wave). FIFO:
        // cannon c last fires at the largest index i with i % ncobs == c. `None`
        // marks a cannon that never fired -> rendered as "-inf". Not clamped: a
        // cannon that recovered before sim end shows a negative number.
        let mut remaining: Vec<Option<i64>> = vec![None; ncobs];
        for (i, s) in base.iter().enumerate() {
            remaining[i % ncobs] = Some(RECHARGE - (cycle_len - s.abs));
        }
        remaining.sort(); // None < Some(_), Some ascending
        let cells: Vec<String> = remaining
            .iter()
            .map(|r| match r {
                Some(v) => v.to_string(),
                None => "-inf".to_string(),
            })
            .collect();
        next_line = Some(format!("next: {}", cells.join(" ")));
    }

    if compact && !any_fail {
        outln!("all ok");
    }
    if let Some(line) = next_line {
        outln!("{}", line);
    }

    Ok(())
}

/// Emit one reuse line and report whether it failed (recharge gap < 3475cs, flagged
/// `(!)`). In compact mode successful reuses are suppressed (nothing printed).
fn emit_line(src: &Shot, dst: &Shot, compact: bool) -> bool {
    let elapsed = dst.abs - src.abs;
    let fail = elapsed < RECHARGE;
    if !compact || fail {
        let flag = if fail { " (!)" } else { "" };
        outln!(
            "w{} {} -> w{} {}: {}cs{}",
            src.wave,
            src.tw,
            dst.wave,
            dst.tw,
            elapsed,
            flag
        );
    }
    fail
}
