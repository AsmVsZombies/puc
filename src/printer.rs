use crate::game;
use game::MAX_INTERCEPTION_DELAY;

pub fn print_warning(msg: &str) {
    errln!("warning: {}", msg);
}

pub fn print_error(error: &str) {
    errln!("error: {}", error);
}

pub fn print_error_with_input(error: &str, input: &str) {
    errln!("error: {} (got: {})", error, input);
}

pub fn print_too_many_arguments_error() {
    errln!("error: too many arguments");
}

pub fn print_bad_format_error() {
    errln!("error: bad input format");
}

fn floor_3(v: f32) -> f32 {
    (v * 1000.0).floor() / 1000.0
}

fn format_ice_cob(times: &game::IceAndCobTimes, min_max_garg_x: (f32, f32)) -> String {
    let ice = times
        .ice_times
        .iter()
        .map(|t| t.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let (lo, hi) = min_max_garg_x;
    format!(
        "ice=[{}] cob={} garg_x=[{:.3},{:.3}]",
        ice,
        times.cob_time,
        floor_3(lo),
        floor_3(hi),
    )
}

pub fn print_wave_status(times: &game::IceAndCobTimes, min_max_garg_x: (f32, f32)) {
    if min_max_garg_x.1 as i32 > 817 {
        print_warning("cannot hit all gargs at this tick");
    }
    outln!("wave {}", format_ice_cob(times, min_max_garg_x));
}

pub fn print_delay_status(times: &game::IceAndCobTimes, min_max_garg_x: (f32, f32)) {
    if min_max_garg_x.1 as i32 > 817 {
        print_warning("cannot hit all gargs at this tick");
    }
    errln!("# delayed {}", format_ice_cob(times, min_max_garg_x));
}

fn format_rows(rows: &[i32]) -> String {
    rows.iter()
        .map(|r| r.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

fn format_cobs(cob_and_garg_rows: &[(game::Cob, Vec<i32>)]) -> String {
    let hit_rows: Vec<String> = cob_and_garg_rows
        .iter()
        .map(|(cob, _)| cob.row().to_string())
        .collect();
    let cob = &cob_and_garg_rows[0].0;
    let col_part = format!("col={}", cob.col());
    let cob_col_part = match cob {
        game::Cob::Roof { cob_col, .. } => format!(" cob_col={}", cob_col),
        _ => String::new(),
    };
    format!("row={} {}{}", hit_rows.join(","), col_part, cob_col_part)
}

fn format_eat_and_intercept(eat: &game::Eat, intercept: &game::Intercept) -> String {
    let intercept_str = match intercept {
        game::Intercept::Empty | game::Intercept::OnlyHighIndexImp | game::Intercept::Fail => {
            "cannot".to_string()
        }
        game::Intercept::Success { min, max } => {
            if *max == MAX_INTERCEPTION_DELAY {
                format!("{}+", min)
            } else {
                format!("{}~{}", min, max)
            }
        }
    };
    let unsafe_part = match game::unsafe_intercept_interval(eat, intercept) {
        None => String::new(),
        Some((min, max)) => {
            if max == MAX_INTERCEPTION_DELAY {
                format!(" unsafe={}+", min)
            } else {
                format!(" unsafe={}~{}", min, max)
            }
        }
    };
    let eat_str = match eat {
        game::Eat::Empty => "none".to_string(),
        game::Eat::Some { eat, iceable: _ } => eat.to_string(),
    };
    let iceable_str = match eat {
        game::Eat::Empty => "none".to_string(),
        game::Eat::Some { eat: _, iceable } => iceable.to_string(),
    };
    format!(
        "intercept={}{} eat={} iceable={}",
        intercept_str, unsafe_part, eat_str, iceable_str
    )
}

pub fn print_delay_result(
    cob_and_garg_rows: &[(game::Cob, Vec<i32>)],
    garg_rows: &[i32],
    eat: &game::Eat,
    intercept: &game::Intercept,
    modified_min_max_garg_x: Option<(f32, f32)>,
) {
    let garg_x_part = match modified_min_max_garg_x {
        Some((lo, hi)) => format!(" garg_x=[{},{}]", lo, hi),
        None => String::new(),
    };
    outln!(
        "delay {} garg_rows=[{}]{} {}",
        format_cobs(cob_and_garg_rows),
        format_rows(garg_rows),
        garg_x_part,
        format_eat_and_intercept(eat, intercept),
    );
}

pub fn print_doom_result(
    doom_row: i32,
    doom_col: i32,
    garg_rows: &[i32],
    eat: &game::Eat,
    intercept: &game::Intercept,
    modified_min_max_garg_x: Option<(f32, f32)>,
) {
    let garg_x_part = match modified_min_max_garg_x {
        Some((lo, hi)) => format!(" garg_x=[{},{}]", lo, hi),
        None => String::new(),
    };
    outln!(
        "doom row={} col={} garg_rows=[{}]{} {}",
        doom_row,
        doom_col,
        format_rows(garg_rows),
        garg_x_part,
        format_eat_and_intercept(eat, intercept),
    );
}

pub fn print_max_result(
    hit_row: i32,
    col_range: (f32, f32),
    cob_col: Option<i32>,
    garg_rows: &[i32],
    best_cols: &[f32],
    eat: &game::Eat,
    intercept: &game::Intercept,
    modified_min_max_garg_x: Option<(f32, f32)>,
) {
    let garg_x_part = match modified_min_max_garg_x {
        Some((lo, hi)) => format!(" garg_x=[{},{}]", lo, hi),
        None => String::new(),
    };
    let cob_col_part = match cob_col {
        Some(c) => format!(" cob_col={}", c),
        None => String::new(),
    };
    let best = best_cols
        .iter()
        .map(|c| c.to_string())
        .collect::<Vec<_>>()
        .join(",");
    outln!(
        "max row={} col_range={}~{}{} garg_rows=[{}]{} best_cols=[{}] {}",
        hit_row,
        col_range.0,
        col_range.1,
        cob_col_part,
        format_rows(garg_rows),
        garg_x_part,
        best,
        format_eat_and_intercept(eat, intercept),
    );
}

pub fn print_max_no_harmless(
    hit_row: i32,
    col_range: (f32, f32),
    cob_col: Option<i32>,
    garg_rows: &[i32],
    modified_min_max_garg_x: Option<(f32, f32)>,
) {
    let garg_x_part = match modified_min_max_garg_x {
        Some((lo, hi)) => format!(" garg_x=[{},{}]", lo, hi),
        None => String::new(),
    };
    let cob_col_part = match cob_col {
        Some(c) => format!(" cob_col={}", c),
        None => String::new(),
    };
    outln!(
        "max row={} col_range={}~{}{} garg_rows=[{}]{} result=cannot_intercept_without_harm",
        hit_row,
        col_range.0,
        col_range.1,
        cob_col_part,
        format_rows(garg_rows),
        garg_x_part,
    );
}

fn fmt_x_col(x: i32) -> String {
    format!("{}(col={})", x, (x as f32) / 80.)
}

pub fn print_hit_cob_dist(scene: &game::Scene, max_garg_x: i32, cob_dist: &game::CobDist) {
    match scene {
        game::Scene::DE | game::Scene::PE => {
            outln!(
                "hit garg_x={} same_below={} three_rows={}",
                max_garg_x,
                fmt_x_col(max_garg_x - cob_dist.hit_same),
                fmt_x_col(max_garg_x - cob_dist.hit_above),
            );
        }
        game::Scene::RE => {
            outln!(
                "hit garg_x={} above={} same={} below={}",
                max_garg_x,
                fmt_x_col(max_garg_x - cob_dist.hit_above),
                fmt_x_col(max_garg_x - cob_dist.hit_same),
                fmt_x_col(max_garg_x - cob_dist.hit_below),
            );
        }
    }
}

pub fn print_nohit_cob_dist(scene: &game::Scene, min_garg_x: i32, cob_dist: &game::CobDist) {
    match scene {
        game::Scene::DE | game::Scene::PE => {
            outln!(
                "nohit garg_x={} same_below={} above={}",
                min_garg_x,
                fmt_x_col(min_garg_x - cob_dist.hit_same - 1),
                fmt_x_col(min_garg_x - cob_dist.hit_above - 1),
            );
        }
        game::Scene::RE => {
            outln!(
                "nohit garg_x={} above={} same={} below={}",
                min_garg_x,
                fmt_x_col(min_garg_x - cob_dist.hit_above - 1),
                fmt_x_col(min_garg_x - cob_dist.hit_same - 1),
                fmt_x_col(min_garg_x - cob_dist.hit_below - 1),
            );
        }
    }
}

pub fn print_imp_x_for_garg(garg_x: f32, imp_x_rnd_0: f32, imp_x_rnd_100: f32) {
    outln!(
        "imp garg_x={} imp_x={:.3}~{:.3}",
        garg_x, imp_x_rnd_0, imp_x_rnd_100
    );
}

pub fn print_garg_x_for_imp(imp_x_label: &str, min_garg_x: f32, max_garg_x: f32) {
    outln!(
        "imp imp_x={} garg_x={:.3}~{:.3}",
        imp_x_label,
        floor_3(min_garg_x),
        floor_3(max_garg_x),
    );
}
