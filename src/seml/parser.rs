//! Faithful port of `../seml/src/parser.ts`. Parses a plain SEML document into a
//! [`Config`] (scenario for the emulator reader) plus header-derived [`Params`].
//! Error strings keep the source's Chinese wording, prefixed with the line number.

use std::collections::HashSet;

use super::plant;
use super::string::{chop_prefix, chop_suffix, find_closest_string, parse_decimal, parse_natural};
use super::types::{
    Action, CardPos, CobPos, Config, Fodder, Params, ProtectKind, ProtectPos, Wave,
};
use super::zombie;

pub struct Parsed {
    pub config: Config,
    pub params: Params,
}

type R<T> = Result<T, String>;

fn e(line_num: usize, msg: &str, src: &str) -> String {
    if src.is_empty() {
        t!("seml_line_prefix", line = line_num, msg = msg).to_string()
    } else {
        t!("seml_line_prefix_src", line = line_num, msg = msg, src = src).to_string()
    }
}

fn max_rows(scene: &str) -> i32 {
    if scene == "FE" {
        6
    } else {
        5
    }
}

struct Line {
    line_num: usize,
    line: String,
}

/// Parse a full SEML document. When `strict` is false, unrecognized header-style
/// lines (whose first token contains `:`) are silently skipped instead of erroring.
pub fn parse(text: &str, strict: bool) -> R<Parsed> {
    let raw: Vec<String> = text
        .split('\n')
        .map(|l| l.trim_end_matches('\r').to_string())
        .collect();
    let lines = expand_lines(&raw)?;

    let mut config = Config::default();
    let mut params = Params::default();
    let mut variables: Vec<(String, f64)> = Vec::new();
    let mut seen: HashSet<&'static str> = HashSet::new();
    // Determined up front so wave parsing sees it regardless of header order; the
    // loop below still parses `avzTime:` for value validation and duplicate checks.
    let avz_time = lines.iter().any(|Line { line, .. }| {
        let symbol = line.split(' ').next().unwrap_or("");
        symbol.starts_with("avzTime:") && arg_value(line).eq_ignore_ascii_case("true")
    });

    parse_scene(&mut config, &lines)?;

    for Line { line_num, line } in &lines {
        let line_num = *line_num;
        if line.is_empty() || line.starts_with("scene:") {
            continue;
        }
        let line = replace_variables(&variables, line);
        let symbol = line.split(' ').next().unwrap_or("");

        if symbol.starts_with("protect:") {
            parse_protect(&mut config, line_num, &line)?;
        } else if symbol.starts_with("repeat:") {
            let v = parse_int_arg(&mut seen, "repeat", line_num, &line)?;
            params.repeat = Some(v);
        } else if symbol.starts_with("require:") {
            let original = config.setting.original_scene.clone().unwrap_or_default();
            let prev = params.ban.clone();
            let v = parse_zombie_type_arg(
                &mut seen,
                "require",
                &original,
                line_num,
                &line,
                prev.as_deref(),
                true,
            )?;
            params.require = Some(v);
        } else if symbol.starts_with("ban:") {
            let original = config.setting.original_scene.clone().unwrap_or_default();
            let prev = params.require.clone();
            let v = parse_zombie_type_arg(
                &mut seen,
                "ban",
                &original,
                line_num,
                &line,
                prev.as_deref(),
                true,
            )?;
            params.ban = Some(v);
        } else if symbol.starts_with("huge:") {
            if let Some(b) = parse_bool_arg(&mut seen, "huge", line_num, &line)? {
                params.huge = Some(b);
            }
        } else if symbol.starts_with("activate:") {
            if let Some(b) = parse_bool_arg(&mut seen, "activate", line_num, &line)? {
                params.activate = Some(b);
            }
        } else if symbol.starts_with("dance:") {
            if let Some(b) = parse_bool_arg(&mut seen, "dance", line_num, &line)? {
                params.dance = Some(b);
            }
        } else if symbol.starts_with("natural:") {
            if let Some(b) = parse_bool_arg(&mut seen, "natural", line_num, &line)? {
                params.natural = Some(b);
            }
        } else if symbol.starts_with("cobDelay:") {
            if let Some(b) = parse_bool_arg(&mut seen, "cobDelay", line_num, &line)? {
                params.cob_delay = Some(b);
            }
        } else if symbol.starts_with("std:") {
            if let Some(b) = parse_bool_arg(&mut seen, "std", line_num, &line)? {
                params.show_std = b;
            }
        } else if symbol.starts_with("hitThres:") {
            let v = parse_int_arg(&mut seen, "hitThres", line_num, &line)?;
            params.hit_thres = Some(v);
        } else if symbol.starts_with("types:") {
            let original = config.setting.original_scene.clone().unwrap_or_default();
            let v =
                parse_zombie_type_arg(&mut seen, "types", &original, line_num, &line, None, false)?;
            params.zombies = Some(v);
        } else if symbol.starts_with("targetPos:") {
            let v = parse_int_arg(&mut seen, "targetPos", line_num, &line)?;
            params.target_x = Some(v);
        } else if symbol.starts_with("avzTime:") {
            // Value/duplicate validation only; `avz_time` is computed up front.
            parse_bool_arg(&mut seen, "avzTime", line_num, &line)?;
        } else if symbol.starts_with("ncobs:") {
            let v = parse_int_arg(&mut seen, "ncobs", line_num, &line)?;
            params.ncobs = Some(v);
        } else if symbol.starts_with("loop:") {
            if let Some(b) = parse_bool_arg(&mut seen, "loop", line_num, &line)? {
                params.r#loop = Some(b);
            }
        } else if symbol.starts_with('w') {
            parse_wave(&mut config, line_num, &line, avz_time)?;
        } else if let Some(cob_num) = cob_kind(&symbol.to_uppercase()) {
            parse_cob(&mut config, line_num, &line, cob_num)?;
        } else if symbol == "C" || symbol == "C_POS" || symbol == "C_NUM" {
            parse_fodder(&mut config, line_num, &line)?;
        } else if symbol == "G" {
            parse_fixed_card(&mut config, line_num, &line, plant::GARLIC)?;
        } else if symbol == "A" {
            parse_fixed_card(&mut config, line_num, &line, plant::CHERRY_BOMB)?;
        } else if symbol == "J" {
            parse_fixed_card(&mut config, line_num, &line, plant::JALAPENO)?;
        } else if symbol == "a" || symbol == "W" {
            parse_fixed_card(&mut config, line_num, &line, plant::SQUASH)?;
        } else if symbol == "N" {
            parse_fixed_card(&mut config, line_num, &line, plant::DOOMSHROOM)?;
        } else if symbol == "A_NUM" {
            parse_smart_card(&mut config, line_num, &line, plant::CHERRY_BOMB)?;
        } else if symbol == "J_NUM" {
            parse_smart_card(&mut config, line_num, &line, plant::JALAPENO)?;
        } else if symbol == "a_NUM" || symbol == "W_NUM" {
            parse_smart_card(&mut config, line_num, &line, plant::SQUASH)?;
        } else if symbol == "SET" {
            parse_set(&mut variables, line_num, &line)?;
        } else if strict || !symbol.contains(':') {
            // Unknown line. In lenient mode (default), silently skip header-style
            // lines (first token contains ':'); bare unknown symbols still error.
            return Err(e(
                line_num,
                &t!("seml_unknown_symbol"),
                &t!("seml_help_hint", symbol = symbol),
            ));
        }
    }

    if avz_time {
        // AvZ time base sits 1 cs later than SEML's (perfect prejudge ice is 1 in
        // AvZ, 0 in SEML). Shift the wave's ice times and the instant cards back by
        // 1 to get SEML time. Fodder (C and its variants) and garlic (G) are left
        // uncorrected.
        for wave in &mut config.waves {
            for ice in &mut wave.ice_times {
                *ice -= 1;
            }
            for action in &mut wave.actions {
                match action {
                    Action::FixedCard { symbol, time, .. }
                        if matches!(symbol.as_str(), "A" | "J" | "a" | "N" | "W") =>
                    {
                        *time -= 1;
                    }
                    Action::SmartCard { symbol, time, .. }
                        if matches!(symbol.as_str(), "A_NUM" | "J_NUM" | "a_NUM" | "W_NUM") =>
                    {
                        *time -= 1;
                    }
                    _ => {}
                }
            }
        }
    }

    for wave in &mut config.waves {
        wave.actions.sort_by_key(|a| a.time());
    }

    Ok(Parsed { config, params })
}

// --- line preprocessing -----------------------------------------------------

fn expand_lines(raw: &[String]) -> R<Vec<Line>> {
    // Strip comments, collapse whitespace.
    let original: Vec<Line> = raw
        .iter()
        .enumerate()
        .map(|(i, l)| {
            let no_comment = l.split('#').next().unwrap_or("").trim();
            let collapsed = collapse_ws(no_comment);
            Line {
                line_num: i + 1,
                line: collapsed,
            }
        })
        .collect();

    let mut out: Vec<Line> = Vec::new();
    let mut cur = 0;
    while cur < original.len() {
        let Line { line_num, line } = &original[cur];
        let line_num = *line_num;
        let symbol = line.split(' ').next().unwrap_or("");

        if !(symbol.starts_with('w') && symbol.contains('~')) {
            out.push(Line {
                line_num,
                line: line.clone(),
            });
        } else {
            let tilde = symbol.find('~').unwrap();
            let start_wave = parse_natural(&symbol[1..tilde]);
            let end_wave = parse_natural(&symbol[tilde + 1..]);
            let (start_wave, end_wave) = match (start_wave, end_wave) {
                (Some(s), Some(en)) => (s, en),
                _ => return Err(e(line_num, &t!("seml_wave_num_positive_int"), symbol)),
            };
            if start_wave > end_wave {
                return Err(e(line_num, &t!("seml_start_wave_gt_end"), symbol));
            }

            let prev_cur = cur;
            while cur + 1 < original.len() && !original[cur + 1].line.starts_with('w') {
                cur += 1;
            }
            for wave_num in start_wave..=end_wave {
                for item in original.iter().take(cur + 1).skip(prev_cur) {
                    out.push(Line {
                        line_num: item.line_num,
                        line: populate_line_with_wave(&item.line, wave_num),
                    });
                }
            }
        }
        cur += 1;
    }
    Ok(out)
}

fn collapse_ws(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_space = false;
    for c in s.chars() {
        if c == ' ' || c == '\t' {
            if !prev_space {
                out.push(' ');
            }
            prev_space = true;
        } else {
            out.push(c);
            prev_space = false;
        }
    }
    out
}

fn populate_line_with_wave(line: &str, wave_num: i32) -> String {
    if line.starts_with('w') {
        let rest: Vec<&str> = line.split(' ').skip(1).collect();
        format!("w{} {}", wave_num, rest.join(" "))
            .trim()
            .to_string()
    } else {
        line.to_string()
    }
}

fn replace_variables(vars: &[(String, f64)], line: &str) -> String {
    if vars.is_empty() {
        return line.to_string();
    }
    let reserved = if line.starts_with("SET") { 2 } else { 1 };
    let toks: Vec<&str> = line.split(' ').collect();
    let head = toks
        .iter()
        .take(reserved)
        .cloned()
        .collect::<Vec<_>>()
        .join(" ");
    let mut tail = toks
        .iter()
        .skip(reserved)
        .cloned()
        .collect::<Vec<_>>()
        .join(" ");
    for (name, val) in vars {
        tail = tail.replace(name, &fmt_num(*val));
    }
    format!("{} {}", head, tail).trim().to_string()
}

fn fmt_num(x: f64) -> String {
    if x.fract() == 0.0 && x.is_finite() {
        format!("{}", x as i64)
    } else {
        format!("{}", x)
    }
}

// --- scene + settings -------------------------------------------------------

fn is_scene(v: &str) -> bool {
    matches!(v, "DE" | "NE" | "PE" | "FE" | "RE" | "ME")
}

fn parse_scene(config: &mut Config, lines: &[Line]) -> R<()> {
    let mut set = false;
    for Line { line_num, line } in lines {
        if line.starts_with("scene:") {
            if set {
                return Err(e(*line_num, &t!("seml_dup_param"), "scene"));
            }
            let scene: String = line.splitn(2, ':').nth(1).unwrap_or("").trim().to_string();
            let upper = scene.to_uppercase();
            if is_scene(&upper) {
                config.setting.original_scene = Some(upper.clone());
                let mapped = match upper.as_str() {
                    "DE" | "NE" => "NE",
                    "PE" | "FE" => "FE",
                    _ => "ME", // RE | ME
                };
                config.setting.scene = mapped.to_string();
                set = true;
            } else {
                return Err(e(
                    *line_num,
                    &t!("seml_unknown_scene"),
                    &t!("seml_supported_scenes", scene = scene),
                ));
            }
        }
    }
    if !set {
        config.setting.scene = "FE".to_string();
        config.setting.original_scene = Some("FE".to_string());
    }
    Ok(())
}

fn parse_protect(config: &mut Config, line_num: usize, line: &str) -> R<()> {
    if config.setting.protect.is_some() {
        return Err(e(line_num, &t!("seml_dup_param"), "protect"));
    }
    let value = line.splitn(2, ':').nth(1).unwrap_or("").trim().to_string();
    if value.is_empty() {
        return Err(e(line_num, &t!("seml_protect_value_empty"), line));
    }

    let mut protect: Vec<ProtectPos> = Vec::new();
    for pos_token in value.split(' ') {
        let is_normal = pos_token.ends_with('\'');
        let chopped = chop_suffix(pos_token, "'");
        let chars: Vec<char> = chopped.chars().collect();
        if chars.len() < 2 {
            return Err(e(line_num, &t!("seml_need_protect_row_col"), line));
        }
        let row = parse_natural(&chars[0].to_string());
        let col = parse_natural(&chars[1].to_string());

        let row = match row {
            Some(r) if r >= 1 && r <= max_rows(&config.setting.scene) => r,
            _ => {
                return Err(e(
                    line_num,
                    &t!("seml_protect_row_range", max = max_rows(&config.setting.scene)),
                    &chars[0].to_string(),
                ))
            }
        };
        let min_col = if is_normal { 1 } else { 2 };
        let col = match col {
            Some(c) if c >= min_col && c <= 9 => c,
            _ => {
                return Err(e(
                    line_num,
                    &t!(
                        "seml_col_range",
                        who = if is_normal { t!("seml_normal_plant") } else { t!("seml_cob") },
                        min = min_col
                    ),
                    &chars[1].to_string(),
                ))
            }
        };

        let kind = if is_normal {
            ProtectKind::Normal
        } else {
            ProtectKind::Cob
        };
        // overlap check against previous positions in the same row
        for prev in &protect {
            if prev.row == row {
                let prev_cols: Vec<i32> = if prev.kind == ProtectKind::Normal {
                    vec![prev.col]
                } else {
                    vec![prev.col - 1, prev.col]
                };
                let new_cols: Vec<i32> = if kind == ProtectKind::Normal {
                    vec![col]
                } else {
                    vec![col - 1, col]
                };
                if prev_cols.iter().any(|pc| new_cols.contains(pc)) {
                    return Err(e(line_num, &t!("seml_protect_overlap"), pos_token));
                }
            }
        }
        protect.push(ProtectPos { kind, row, col });
    }
    config.setting.protect = Some(protect);
    Ok(())
}

// --- argument headers -------------------------------------------------------

fn arg_value(line: &str) -> String {
    line.splitn(2, ':').nth(1).unwrap_or("").trim().to_string()
}

fn parse_int_arg(
    seen: &mut HashSet<&'static str>,
    name: &'static str,
    line_num: usize,
    line: &str,
) -> R<i32> {
    if seen.contains(name) {
        return Err(e(line_num, &t!("seml_dup_param"), name));
    }
    let value = arg_value(line);
    match parse_natural(&value) {
        Some(v) if v > 0 => {
            seen.insert(name);
            Ok(v)
        }
        _ => Err(e(
            line_num,
            &t!("seml_value_positive_int", name = name),
            &value,
        )),
    }
}

fn parse_bool_arg(
    seen: &mut HashSet<&'static str>,
    name: &'static str,
    line_num: usize,
    line: &str,
) -> R<Option<bool>> {
    if seen.contains(name) {
        return Err(e(line_num, &t!("seml_dup_param"), name));
    }
    let value = arg_value(line).to_lowercase();
    match value.as_str() {
        "true" => {
            seen.insert(name);
            Ok(Some(true))
        }
        "false" => Ok(None),
        _ => Err(e(
            line_num,
            &t!("seml_value_bool", name = name),
            &value,
        )),
    }
}

#[allow(clippy::too_many_arguments)]
fn parse_zombie_type_arg(
    seen: &mut HashSet<&'static str>,
    name: &'static str,
    original_scene: &str,
    line_num: usize,
    line: &str,
    prev_types: Option<&[i32]>,
    check_acceptable: bool,
) -> R<Vec<i32>> {
    if seen.contains(name) {
        return Err(e(line_num, &t!("seml_dup_param"), name));
    }
    let abbrs = arg_value(line);
    let contain_chinese = !abbrs
        .chars()
        .all(|c| c.is_ascii_alphabetic() || c.is_whitespace());
    let prev: Vec<i32> = prev_types.map(|p| p.to_vec()).unwrap_or_default();
    let mut types: Vec<i32> = Vec::new();

    let tokens: Vec<String> = if contain_chinese {
        abbrs.chars().map(|c| c.to_string()).collect()
    } else {
        abbrs.split(' ').map(|s| s.to_string()).collect()
    };

    for abbr in tokens {
        let lower = abbr.to_lowercase();
        let zombie_type = if contain_chinese {
            match zombie::cn_lookup(&abbr) {
                Some(z) => z,
                None => {
                    let keys: Vec<&str> = zombie::CN_ABBR.iter().map(|(k, _)| *k).collect();
                    return Err(e(
                        line_num,
                        &t!("seml_unknown_zombie"),
                        &t!("seml_supported_zombies", abbr = abbr, keys = keys.join(",")),
                    ));
                }
            }
        } else {
            match zombie::en_lookup(&lower) {
                Some(z) => z,
                None => {
                    let keys = zombie::en_keys();
                    let mut src = abbr.clone();
                    if let Some(closest) = find_closest_string(&lower, &keys) {
                        src = t!("seml_did_you_mean", abbr = abbr, closest = closest).to_string();
                    }
                    return Err(e(line_num, &t!("seml_unknown_zombie"), &src));
                }
            }
        };

        if check_acceptable && !zombie::ACCEPTABLE.contains(&zombie_type) {
            return Err(e(line_num, &t!("seml_cannot_specify_zombie"), &abbr));
        }
        if types.contains(&zombie_type) || prev.contains(&zombie_type) {
            return Err(e(line_num, &t!("seml_dup_zombie"), &abbr));
        }
        if zombie::banned(original_scene).contains(&zombie_type) {
            return Err(e(
                line_num,
                &t!("seml_scene_cannot_zombie", scene = original_scene),
                &abbr,
            ));
        }
        types.push(zombie_type);
    }

    seen.insert(name);
    Ok(types)
}

// --- waves ------------------------------------------------------------------

fn parse_wave(config: &mut Config, line_num: usize, line: &str, avz_time: bool) -> R<()> {
    let tokens: Vec<&str> = line.split(' ').collect();
    if tokens.len() < 2 {
        return Err(e(line_num, &t!("seml_need_wave_length"), line));
    }
    let wave_num_token = tokens[0];
    let ice_time_tokens = &tokens[1..tokens.len() - 1];
    let wave_range_token = tokens[tokens.len() - 1];

    let prev_wave_num = config.waves.len() as i32;
    let wave_num = if wave_num_token == "w" {
        prev_wave_num + 1
    } else {
        match parse_natural(chop_prefix(wave_num_token, "w")) {
            Some(n) if (1..=99).contains(&n) => n,
            _ => return Err(e(line_num, &t!("seml_wave_num_positive_int"), wave_num_token)),
        }
    };
    if (wave_num - 1) < prev_wave_num {
        return Err(e(line_num, &t!("seml_dup_wave_num"), wave_num_token));
    }
    if prev_wave_num + 1 != wave_num {
        return Err(e(
            line_num,
            &t!("seml_need_set_wave_n", n = prev_wave_num + 1),
            wave_num_token,
        ));
    }

    // Under avzTime every ice time is shifted back by 1 (perfect prejudge is 1, not
    // 0), so 0 would underflow; require a positive value up front.
    let min_ice = if avz_time { 1 } else { 0 };
    let mut ice_times: Vec<i32> = Vec::new();
    for t in ice_time_tokens {
        match parse_natural(t) {
            Some(v) if v >= min_ice => ice_times.push(v),
            _ if avz_time => return Err(e(line_num, &t!("seml_avztime_ice_positive"), t)),
            _ => return Err(e(line_num, &t!("seml_ice_nonneg"), t)),
        }
    }

    let (start_tick, wave_length) = parse_wave_range(line_num, wave_range_token)?;

    if let Some(last_ice) = ice_times.last() {
        if wave_length < *last_ice {
            return Err(e(line_num, &t!("seml_wave_len_ge_last_ice"), line));
        }
    }

    config.waves.push(Wave {
        ice_times,
        wave_length,
        start_tick,
        actions: Vec::new(),
    });
    Ok(())
}

fn parse_wave_range(line_num: usize, token: &str) -> R<(Option<i32>, i32)> {
    let (start_token, length_token) = match token.split_once('~') {
        Some((s, l)) => (Some(s), l),
        None => (None, token),
    };
    let wave_length = match parse_natural(length_token) {
        Some(v) if v > 0 => v,
        _ => return Err(e(line_num, &t!("seml_wave_len_positive"), token)),
    };
    let start_tick = match start_token {
        Some(s) => match parse_natural(s) {
            Some(v) if v <= wave_length => Some(v),
            _ => return Err(e(line_num, &t!("seml_start_le_wavelen"), s)),
        },
        None => None,
    };
    Ok((start_tick, wave_length))
}

// --- time helpers -----------------------------------------------------------

fn parse_time(line_num: usize, token: &str, prev_time: Option<i32>) -> R<i32> {
    let is_delay = token.starts_with('+');
    let chopped = chop_prefix(token, "+");
    let time = match parse_natural(chopped) {
        Some(v) if v >= 0 => v,
        _ => return Err(e(line_num, &t!("seml_time_nonneg"), chopped)),
    };
    if !is_delay {
        Ok(time)
    } else {
        match prev_time {
            Some(p) => Ok(p + time),
            None => Err(e(line_num, &t!("seml_no_delay_base"), token)),
        }
    }
}

/// Parses `time`, `time~shovel`, or `time+shovel` (shovel may itself be relative).
fn parse_card_time_and_shovel(
    line_num: usize,
    token: &str,
    prev_action_time: Option<i32>,
) -> R<(i32, Option<i32>)> {
    let plus = token.rfind('+').map(|i| i as isize).unwrap_or(-1);
    let tilde = token.rfind('~').map(|i| i as isize).unwrap_or(-1);
    let delim = plus.max(tilde);

    let (card_token, shovel_token) = if delim <= 0 {
        (token, None)
    } else {
        let d = delim as usize;
        (&token[..d], Some(chop_prefix(&token[d..], "~").to_string()))
    };

    let card_time = parse_time(line_num, card_token, prev_action_time)?;
    match shovel_token {
        None => Ok((card_time, None)),
        Some(st) => {
            let shovel_time = parse_time(line_num, &st, Some(card_time))?;
            if shovel_time < card_time {
                return Err(e(line_num, &t!("seml_shovel_before_card"), &st));
            }
            Ok((card_time, Some(shovel_time)))
        }
    }
}

// --- cob --------------------------------------------------------------------

/// Returns the cob count (1 or 2) if `up` (uppercased symbol) is a cob symbol.
fn cob_kind(up: &str) -> Option<i32> {
    let c: Vec<char> = up.chars().collect();
    if !c.is_empty() && matches!(c[0], 'B' | 'P' | 'D') {
        if c.len() == 1 || (c.len() == 2 && c[1].is_ascii_digit()) {
            return Some(1);
        }
    }
    if c.len() >= 2 && matches!((c[0], c[1]), ('B', 'B') | ('P', 'P') | ('D', 'D')) {
        if c.len() == 2 || (c.len() == 3 && c[2].is_ascii_digit()) {
            return Some(2);
        }
    }
    None
}

fn parse_cob(config: &mut Config, line_num: usize, line: &str, cob_num: i32) -> R<()> {
    let scene = config.setting.scene.clone();
    let prev_time = current_wave(config, line_num, line)?
        .actions
        .last()
        .map(|a| a.time());

    let tokens: Vec<&str> = line.split(' ').collect();
    let symbol = tokens[0];
    let time_token = tokens.get(1);
    let rows_token = tokens.get(2);
    let col_token = tokens.get(3);
    let tail = tokens.get(4..).map(|s| s.join(" ")).unwrap_or_default();

    let (time_token, rows_token, col_token) = match (time_token, rows_token, col_token) {
        (Some(t), Some(r), Some(c)) => (*t, *r, *c),
        _ => return Err(e(line_num, &t!("seml_need_cob_args"), line)),
    };
    if !tail.is_empty() {
        return Err(e(line_num, &t!("seml_remove_extra_args"), &tail));
    }

    let mut cob_col: Option<i32> = None;
    if symbol
        .chars()
        .last()
        .map(|c| c.is_ascii_digit())
        .unwrap_or(false)
    {
        if scene != "ME" {
            return Err(e(line_num, &t!("seml_cob_tail_roof_only"), symbol));
        }
        let last = symbol.chars().last().unwrap().to_string();
        match parse_natural(&last) {
            Some(v) if (1..=8).contains(&v) => cob_col = Some(v),
            _ => return Err(e(line_num, &t!("seml_cob_tail_range"), &last)),
        }
    } else if scene == "ME" {
        return Err(e(line_num, &t!("seml_roof_need_col"), symbol));
    }

    let time = parse_time(line_num, time_token, prev_time)?;
    let rows = parse_cob_rows(line_num, rows_token, cob_num, &scene)?;
    let col = match parse_decimal(col_token) {
        Some(v) if (0.0..=10.0).contains(&v) => v,
        _ => return Err(e(line_num, &t!("seml_landing_col_range"), col_token)),
    };

    let positions: Vec<CobPos> = rows.iter().map(|&row| CobPos { row, col }).collect();
    let symbol = symbol.to_string();
    current_wave(config, line_num, line)?
        .actions
        .push(Action::Cob {
            symbol,
            time,
            positions,
            cob_col,
        });
    Ok(())
}

fn parse_cob_rows(line_num: usize, token: &str, cob_num: i32, scene: &str) -> R<Vec<i32>> {
    let chars: Vec<char> = token.chars().collect();
    if chars.len() != cob_num as usize {
        return Err(e(
            line_num,
            &t!("seml_need_n_landing_rows", n = cob_num),
            token,
        ));
    }
    let mut rows: Vec<i32> = Vec::new();
    for c in chars {
        match parse_natural(&c.to_string()) {
            Some(r) if r >= 1 && r <= max_rows(scene) => rows.push(r),
            _ => {
                return Err(e(
                    line_num,
                    &t!("seml_landing_row_range", max = max_rows(scene)),
                    &c.to_string(),
                ))
            }
        }
    }
    rows.sort_unstable();
    Ok(rows)
}

// --- fodder -----------------------------------------------------------------

fn parse_fodder(config: &mut Config, line_num: usize, line: &str) -> R<()> {
    let scene = config.setting.scene.clone();
    let curr_wave_num = config.waves.len() as i32;
    let prev_time = current_wave(config, line_num, line)?
        .actions
        .last()
        .map(|a| a.time());

    let tokens: Vec<&str> = line.split(' ').collect();
    let symbol = tokens[0];
    let time_token = tokens.get(1);
    let rows_token = tokens.get(2);
    let col_token = tokens.get(3);
    let fodder_arg_tokens: Vec<&str> = tokens.get(4..).map(|s| s.to_vec()).unwrap_or_default();

    let (time_token, rows_token, col_token) = match (time_token, rows_token, col_token) {
        (Some(t), Some(r), Some(c)) => (*t, *r, *c),
        _ => return Err(e(line_num, &t!("seml_need_card_args"), line)),
    };

    let (time, shovel_time) = parse_card_time_and_shovel(line_num, time_token, prev_time)?;
    let rows = parse_fodder_rows(line_num, rows_token, &scene)?;
    let col = parse_card_col(line_num, col_token)?;

    let fodders: Vec<Fodder> = rows.iter().map(|(_, card)| *card).collect();
    let positions: Vec<CardPos> = rows
        .iter()
        .map(|(row, _)| CardPos { row: *row, col })
        .collect();

    if symbol == "C" {
        let symbol = symbol.to_string();
        current_wave(config, line_num, line)?
            .actions
            .push(Action::FixedFodder {
                symbol,
                time,
                shovel_time,
                fodders,
                positions,
            });
    } else {
        let rows_len = rows_token.chars().count();
        if rows_len < 2 {
            return Err(e(line_num, &t!("seml_need_2_card_rows"), rows_token));
        }
        let (choose, waves) = parse_fodder_args(
            line_num,
            &fodder_arg_tokens,
            rows.len() as i32,
            symbol == "C_POS",
            curr_wave_num,
        )?;
        let symbol = symbol.to_string();
        current_wave(config, line_num, line)?
            .actions
            .push(Action::SmartFodder {
                symbol,
                time,
                shovel_time,
                fodders,
                positions,
                choose,
                waves,
            });
    }
    Ok(())
}

fn parse_fodder_rows(line_num: usize, token: &str, scene: &str) -> R<Vec<(i32, Fodder)>> {
    let chars: Vec<char> = token.chars().collect();
    let mut rows: Vec<(i32, Fodder)> = Vec::new();
    let mut skip = false;
    for i in 0..chars.len() {
        if skip {
            skip = false;
            continue;
        }
        let row = match parse_natural(&chars[i].to_string()) {
            Some(r) if r >= 1 && r <= max_rows(scene) => r,
            _ => {
                return Err(e(
                    line_num,
                    &t!("seml_card_row_range", max = max_rows(scene)),
                    &chars[i].to_string(),
                ))
            }
        };
        if rows.iter().any(|(r, _)| *r == row) {
            return Err(e(line_num, &t!("seml_dup_card_row"), &chars[i].to_string()));
        }
        let mut card = Fodder::Normal;
        if let Some(next) = chars.get(i + 1) {
            if *next == '\'' {
                card = Fodder::Puff;
                skip = true;
            } else if *next == '"' {
                card = Fodder::Pot;
                skip = true;
            }
        }
        rows.push((row, card));
    }
    rows.sort_by_key(|(r, _)| *r);
    Ok(rows)
}

fn parse_card_col(line_num: usize, token: &str) -> R<i32> {
    match parse_natural(token) {
        Some(c) if (1..=9).contains(&c) => Ok(c),
        _ => Err(e(line_num, &t!("seml_card_col_range"), token)),
    }
}

fn parse_card_row(line_num: usize, token: &str, scene: &str) -> R<i32> {
    match parse_natural(token) {
        Some(r) if r >= 1 && r <= max_rows(scene) => Ok(r),
        _ => Err(e(
            line_num,
            &t!("seml_card_row_range", max = max_rows(scene)),
            token,
        )),
    }
}

fn parse_fodder_args(
    line_num: usize,
    tokens: &[&str],
    card_num: i32,
    must_provide_choose: bool,
    curr_wave_num: i32,
) -> R<(i32, Vec<i32>)> {
    let mut choose: Option<i32> = None;
    let mut waves: Option<Vec<i32>> = None;
    let mut seen: HashSet<&str> = HashSet::new();

    for token in tokens {
        let (key, value) = match token.split_once(':') {
            Some(kv) => kv,
            None => return Err(e(line_num, &t!("seml_param_format"), token)),
        };
        if key.is_empty() {
            return Err(e(line_num, &t!("seml_param_empty"), token));
        }
        if value.is_empty() {
            return Err(e(line_num, &t!("seml_value_empty"), token));
        }
        if seen.contains(key) {
            return Err(e(line_num, &t!("seml_dup_param"), key));
        }
        seen.insert(key);

        if key == "choose" {
            match parse_natural(value) {
                Some(n) if n >= 1 && n <= card_num => choose = Some(n),
                _ => {
                    return Err(e(
                        line_num,
                        &t!("seml_choose_range", max = card_num),
                        value,
                    ))
                }
            }
        } else if key == "waves" {
            let mut ws: Vec<i32> = Vec::new();
            for wt in value.split(',') {
                match parse_natural(wt) {
                    Some(n) if n >= 1 && n <= curr_wave_num => {
                        if ws.contains(&n) {
                            return Err(e(line_num, &t!("seml_dup_waves"), &n.to_string()));
                        }
                        ws.push(n);
                    }
                    _ => {
                        return Err(e(
                            line_num,
                            &t!("seml_waves_range", max = curr_wave_num),
                            value,
                        ))
                    }
                }
            }
            waves = Some(ws);
        } else {
            return Err(e(
                line_num,
                &t!("seml_unknown_param"),
                &t!("seml_supported_params", key = key),
            ));
        }
    }

    if must_provide_choose && choose.is_none() {
        return Err(e(line_num, &t!("seml_need_choose"), ""));
    }
    Ok((choose.unwrap_or(card_num), waves.unwrap_or_default()))
}

// --- cards ------------------------------------------------------------------

fn parse_fixed_card(config: &mut Config, line_num: usize, line: &str, plant_type: i32) -> R<()> {
    let scene = config.setting.scene.clone();
    let prev_time = current_wave(config, line_num, line)?
        .actions
        .last()
        .map(|a| a.time());

    let tokens: Vec<&str> = line.split(' ').collect();
    let symbol = tokens[0];
    let time_token = tokens.get(1);
    let row_token = tokens.get(2);
    let col_token = tokens.get(3);
    let tail = tokens.get(4..).map(|s| s.join(" ")).unwrap_or_default();

    let (time_token, row_token, col_token) = match (time_token, row_token, col_token) {
        (Some(t), Some(r), Some(c)) => (*t, *r, *c),
        _ => return Err(e(line_num, &t!("seml_need_card_args"), line)),
    };
    if !tail.is_empty() {
        return Err(e(line_num, &t!("seml_remove_extra_args"), &tail));
    }

    let (time, shovel_time) = if symbol == "G" {
        parse_card_time_and_shovel(line_num, time_token, prev_time)?
    } else {
        (parse_time(line_num, time_token, prev_time)?, None)
    };

    let row = parse_card_row(line_num, row_token, &scene)?;
    let col = parse_card_col(line_num, col_token)?;
    let symbol = symbol.to_string();

    current_wave(config, line_num, line)?
        .actions
        .push(Action::FixedCard {
            symbol,
            time,
            shovel_time,
            plant_type,
            position: CardPos { row, col },
        });
    Ok(())
}

fn parse_smart_card(config: &mut Config, line_num: usize, line: &str, plant_type: i32) -> R<()> {
    let scene = config.setting.scene.clone();
    let prev_time = current_wave(config, line_num, line)?
        .actions
        .last()
        .map(|a| a.time());

    let tokens: Vec<&str> = line.split(' ').collect();
    let symbol = tokens[0];
    let time_token = tokens.get(1);
    let rows_token = tokens.get(2);
    let col_token = tokens.get(3);
    let tail = tokens.get(4..).map(|s| s.join(" ")).unwrap_or_default();

    let (time_token, rows_token, col_token) = match (time_token, rows_token, col_token) {
        (Some(t), Some(r), Some(c)) => (*t, *r, *c),
        _ => return Err(e(line_num, &t!("seml_need_card_args"), line)),
    };
    if !tail.is_empty() {
        return Err(e(line_num, &t!("seml_remove_extra_args"), &tail));
    }

    let rows = parse_smart_card_rows(line_num, rows_token, &scene)?;
    let time = parse_time(line_num, time_token, prev_time)?;
    let col = parse_card_col(line_num, col_token)?;
    let positions: Vec<CardPos> = rows.iter().map(|&row| CardPos { row, col }).collect();
    let symbol = symbol.to_string();

    current_wave(config, line_num, line)?
        .actions
        .push(Action::SmartCard {
            symbol,
            time,
            plant_type,
            positions,
        });
    Ok(())
}

fn parse_smart_card_rows(line_num: usize, token: &str, scene: &str) -> R<Vec<i32>> {
    let chars: Vec<char> = token.chars().collect();
    if chars.len() < 2 {
        return Err(e(line_num, &t!("seml_need_2_card_rows"), token));
    }
    let mut rows: Vec<i32> = Vec::new();
    for c in chars {
        let row = match parse_natural(&c.to_string()) {
            Some(r) if r >= 1 && r <= max_rows(scene) => r,
            _ => {
                return Err(e(
                    line_num,
                    &t!("seml_card_row_range", max = max_rows(scene)),
                    &c.to_string(),
                ))
            }
        };
        if rows.contains(&row) {
            return Err(e(line_num, &t!("seml_dup_card_row"), &c.to_string()));
        }
        rows.push(row);
    }
    rows.sort_unstable();
    Ok(rows)
}

// --- SET --------------------------------------------------------------------

fn parse_set(variables: &mut Vec<(String, f64)>, line_num: usize, line: &str) -> R<()> {
    let tokens: Vec<&str> = line.split(' ').collect();
    let var_name = match tokens.get(1) {
        Some(v) => *v,
        None => return Err(e(line_num, &t!("seml_need_var_expr"), line)),
    };
    let expr = tokens.get(2..).map(|s| s.join(" ")).unwrap_or_default();

    if var_name.is_empty() {
        return Err(e(line_num, &t!("seml_var_name_empty"), line));
    }
    if !var_name.is_empty() && var_name.bytes().all(|b| b.is_ascii_digit()) {
        return Err(e(line_num, &t!("seml_var_name_not_numeric"), var_name));
    }
    if expr.is_empty() {
        return Err(e(line_num, &t!("seml_expr_empty"), line));
    }
    if !expr
        .bytes()
        .all(|b| matches!(b, b'0'..=b'9' | b'+' | b'-' | b'*' | b'/' | b'(' | b')'))
    {
        return Err(e(line_num, &t!("seml_expr_chars"), &expr));
    }
    // The grammar above forbids '.', so every literal is an integer; +/-/* match
    // JS exactly. (Division-with-remainder would differ — evalexpr does integer
    // division — but no SEML uses it.)
    let val = match evalexpr::eval(&expr).ok().and_then(|v| v.as_number().ok()) {
        Some(v) if v.is_finite() => v,
        _ => return Err(e(line_num, &t!("seml_expr_invalid"), &expr)),
    };

    if let Some(entry) = variables.iter_mut().find(|(n, _)| n == var_name) {
        entry.1 = val;
    } else {
        variables.push((var_name.to_string(), val));
    }
    Ok(())
}

// --- helpers ----------------------------------------------------------------

fn current_wave<'a>(config: &'a mut Config, line_num: usize, line: &str) -> R<&'a mut Wave> {
    config
        .waves
        .last_mut()
        .ok_or_else(|| e(line_num, &t!("seml_set_wave_first"), line))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn pos_template_serializes_to_expected_scenario() {
        let text = "\
scene:PE
types:红
repeat:20000

w 601 900
C 445+200 1256 9
";
        let parsed = parse(text, false).expect("parse ok");
        let scenario = serde_json::to_value(&parsed.config).unwrap();
        assert_eq!(
            scenario,
            json!({
                "setting": { "scene": "FE", "originalScene": "PE" },
                "waves": [{
                    "iceTimes": [601],
                    "waveLength": 900,
                    "actions": [{
                        "op": "FixedFodder",
                        "symbol": "C",
                        "time": 445,
                        "shovelTime": 645,
                        "fodders": ["Normal", "Normal", "Normal", "Normal"],
                        "positions": [
                            {"row": 1, "col": 9}, {"row": 2, "col": 9},
                            {"row": 5, "col": 9}, {"row": 6, "col": 9}
                        ]
                    }]
                }]
            })
        );
        assert_eq!(parsed.params.zombies, Some(vec![zombie::GIGA_GARGANTUAR]));
        assert_eq!(parsed.params.repeat, Some(20000));
    }

    #[test]
    fn set_substitutes_and_evaluates() {
        // x = 776; x = x+24 → 800 (substituted then evaluated); used as a cob time.
        let text = "\
scene:NE
SET x 776
SET x x+24
w 1500
P x 2 9
";
        let parsed = parse(text, false).unwrap();
        let Action::Cob { time, .. } = &parsed.config.waves[0].actions[0] else {
            panic!("expected cob");
        };
        assert_eq!(*time, 800);
    }

    #[test]
    fn wave_range_expands_per_wave() {
        let text = "\
scene:PE
w1~3 601
PP 225 25 9
";
        let parsed = parse(text, false).unwrap();
        assert_eq!(parsed.config.waves.len(), 3);
        for w in &parsed.config.waves {
            assert_eq!(w.wave_length, 601);
            assert_eq!(w.actions.len(), 1);
        }
    }

    #[test]
    fn unknown_zombie_suggests_closest() {
        rust_i18n::set_locale("zh"); // assert against the default-locale wording
        let text = "scene:PE\ntypes:gigaa\nw 601\nPP 225 25 9\n";
        let err = parse(text, false).map(|_| ()).unwrap_err();
        assert!(err.contains("未知僵尸类型"), "got: {err}");
    }

    #[test]
    fn scene_allows_space_after_colon() {
        let text = "scene: PE\nw 601\nPP 225 25 9\n";
        let parsed = parse(text, false).expect("parse ok");
        assert_eq!(parsed.config.setting.original_scene.as_deref(), Some("PE"));
    }

    #[test]
    fn unknown_header_skipped_unless_strict() {
        rust_i18n::set_locale("zh"); // assert against the default-locale wording
        let text = "scene:PE\nfoo: bar\nw 601\nPP 225 25 9\n";
        // Lenient (default): the unknown header line is skipped.
        let parsed = parse(text, false).expect("lenient parse ok");
        assert_eq!(parsed.config.waves.len(), 1);
        // Strict: the unknown header line errors.
        let err = parse(text, true).map(|_| ()).unwrap_err();
        assert!(err.contains("未知符号"), "got: {err}");
    }

    #[test]
    fn bare_unknown_symbol_errors_even_when_lenient() {
        // No colon in the symbol => treated as a malformed action, not a header.
        rust_i18n::set_locale("zh"); // assert against the default-locale wording
        let text = "scene:PE\nZ 1 2 3\nw 601\nPP 225 25 9\n";
        let err = parse(text, false).map(|_| ()).unwrap_err();
        assert!(err.contains("未知符号"), "got: {err}");
    }
}
