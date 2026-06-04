use crate::constants;
use crate::game;
use crate::printer;
use dyn_fmt::AsStrFormatExt;
use evalexpr::{eval_with_context_mut, HashMapContext, Value};

#[cfg(feature = "en")]
use crate::lang::en::*;

#[cfg(feature = "zh")]
use crate::lang::zh::*;

const DEFAULT_SCENE: game::Scene = game::Scene::PE;
const DEFAULT_COB_TIME: i32 = 318;
const DEFAULT_ROOF_COB_ROW: i32 = 3;

enum EvalError {
    Eval(String),
    Type(String),
}

fn eval_as_i32(expr: &str, ctx: &mut HashMapContext) -> Result<i32, EvalError> {
    match eval_with_context_mut(expr, ctx) {
        Ok(Value::Int(v)) => i32::try_from(v).map_err(|_| EvalError::Type(v.to_string())),
        Ok(Value::Float(f)) => {
            let r = f.round();
            if (f - r).abs() < 1e-6 {
                i32::try_from(r as i64).map_err(|_| EvalError::Type(f.to_string()))
            } else {
                Err(EvalError::Type(f.to_string()))
            }
        }
        Ok(v) => Err(EvalError::Type(v.to_string())),
        Err(e) => Err(EvalError::Eval(e.to_string())),
    }
}

fn eval_as_f32(expr: &str, ctx: &mut HashMapContext) -> Result<f32, EvalError> {
    match eval_with_context_mut(expr, ctx) {
        Ok(Value::Int(v)) => Ok(v as f32),
        Ok(Value::Float(f)) => Ok(f as f32),
        Ok(v) => Err(EvalError::Type(v.to_string())),
        Err(e) => Err(EvalError::Eval(e.to_string())),
    }
}

fn validate_garg_x_range(min_max_garg_x: &mut (f32, f32)) -> Result<game::GargXRange, ()> {
    match game::GargXRange::of_min_max_garg_pos(*min_max_garg_x) {
        game::GargXRange::Cancelled => {
            printer::print_warning(GARG_X_RANGE_CANCELLED);
            Err(())
        }
        game::GargXRange::Modified { min, max } => {
            printer::print_warning(GARG_X_RANGE_MODIFIED.format(&[min, max]).as_str());
            *min_max_garg_x = (min, max);
            Ok(game::GargXRange::Modified { min, max })
        }
        game::GargXRange::Ok { min, max } => Ok(game::GargXRange::Modified { min, max }),
    }
}

struct ParsedGargPos {
    garg_rows: Vec<i32>,
    min_max_garg_x: Option<(f32, f32)>,
    ice_flag: Option<i32>,
}

pub struct Parser {
    scene: game::Scene,
    ice_and_cob_times: game::IceAndCobTimes,
    min_max_garg_x: (f32, f32),
    eval_context: HashMapContext,
}

pub enum ParseResult {
    Unmatched,
    Ok,
    Err,
}

impl Default for Parser {
    fn default() -> Self {
        let scene = DEFAULT_SCENE;
        let ice_and_cob_times =
            game::IceAndCobTimes::of_ice_times_and_cob_time(&[], DEFAULT_COB_TIME).unwrap();
        let min_max_garg_x = game::min_max_garg_x(&ice_and_cob_times).unwrap();
        Parser {
            scene,
            ice_and_cob_times,
            min_max_garg_x,
            eval_context: HashMapContext::new(),
        }
    }
}

impl Parser {
    pub fn parse_scene(&mut self, input: &str) -> ParseResult {
        match input {
            "de" | "ne" => {
                self.scene = game::Scene::DE;
                ParseResult::Ok
            }
            "pe" | "fe" => {
                self.scene = game::Scene::PE;
                ParseResult::Ok
            }
            "re" | "me" => {
                self.scene = game::Scene::RE;
                ParseResult::Ok
            }
            _ => ParseResult::Unmatched,
        }
    }

    pub fn parse_wave(&mut self, input: &str) -> ParseResult {
        match input.split_whitespace().collect::<Vec<&str>>().as_slice() {
            ["wave", extra_args @ ..] => match extra_args {
                [] => {
                    printer::print_wave_status(&self.ice_and_cob_times, self.min_max_garg_x);
                    ParseResult::Ok
                }
                [ice_times @ .., cob_time] => {
                    let (Ok(ice_times), Ok(cob_time)) = (
                        Parser::parse_ice_times(ice_times, &mut self.eval_context),
                        Parser::parse_cob_time(cob_time, &mut self.eval_context),
                    ) else {
                        return ParseResult::Err;
                    };
                    match game::IceAndCobTimes::of_ice_times_and_cob_time(&ice_times, cob_time) {
                        Err(err) => {
                            printer::print_error(err.as_str());
                            ParseResult::Err
                        }
                        Ok(ice_and_cob_times) => match game::min_max_garg_x(&ice_and_cob_times) {
                            Err(err) => {
                                printer::print_error(err.as_str());
                                ParseResult::Err
                            }
                            Ok((min_x, max_x)) => {
                                self.ice_and_cob_times = ice_and_cob_times;
                                self.min_max_garg_x = (min_x, max_x);
                                ParseResult::Ok
                            }
                        },
                    }
                }
            },
            _ => ParseResult::Unmatched,
        }
    }

    pub fn parse_delay(&mut self, input: &str) -> ParseResult {
        match input.split_whitespace().collect::<Vec<&str>>().as_slice() {
            [command, extra_args @ ..] => {
                let delay_mode = match *command {
                    "delay1" => Some(game::DelayMode::Delay1),
                    "delay2" => Some(game::DelayMode::Delay2),
                    "delay3" => Some(game::DelayMode::Delay3),
                    "delay" => None,
                    _ => {
                        return ParseResult::Unmatched;
                    }
                };
                let (cob_and_garg_rows, mut min_max_garg_x, ice_flag, garg_rows_explicit): (
                    Vec<(game::Cob, Vec<i32>)>,
                    _,
                    _,
                    Option<Vec<i32>>,
                ) =
                    if !self.scene.is_roof() {
                        match extra_args {
                            [">", ..] if *command == "delay" => {
                                printer::print_error(NEED_HIT_ROW_HIT_COL);
                                return ParseResult::Err;
                            }
                            [_, ">", ..] if *command == "delay" => {
                                printer::print_error(NEED_HIT_COL);
                                return ParseResult::Err;
                            }
                            [hit_row, hit_col, ">", garg_pos_args @ ..] if *command == "delay" => {
                                let (Ok(hit_row), Ok(hit_col)) = (
                                    Parser::parse_hit_row(
                                        hit_row,
                                        &self.scene.all_rows(),
                                        &mut self.eval_context,
                                    ),
                                    Parser::parse_hit_col(hit_col, &mut self.eval_context),
                                ) else {
                                    return ParseResult::Err;
                                };
                                let Ok(ParsedGargPos {
                                    garg_rows,
                                    min_max_garg_x,
                                    ice_flag,
                                }) = Parser::parse_garg_pos(
                                    garg_pos_args,
                                    &self.scene.garg_rows_for_cob(hit_row),
                                    &mut self.eval_context,
                                )
                                else {
                                    return ParseResult::Err;
                                };
                                let cob = game::Cob::Ground {
                                    row: hit_row,
                                    col: hit_col,
                                };
                                (
                                    vec![(cob.clone(), garg_rows.clone())],
                                    min_max_garg_x.unwrap_or(self.min_max_garg_x),
                                    ice_flag.unwrap_or(self.ice_and_cob_times.is_iced()),
                                    Some(garg_rows),
                                )
                            }
                            [] => {
                                printer::print_error(NEED_HIT_COL);
                                return ParseResult::Err;
                            }
                            [hit_col] => {
                                let Ok(hit_col) =
                                    Parser::parse_hit_col(hit_col, &mut self.eval_context)
                                else {
                                    return ParseResult::Err;
                                };
                                (
                                    self.scene
                                        .hit_row_and_garg_rows_of_delay_mode(&delay_mode.unwrap_or(
                                            self.scene.default_delay_mode(hit_col, None),
                                        ))
                                        .iter()
                                        .map(|(hit_row, garg_rows)| {
                                            (
                                                game::Cob::Ground {
                                                    row: *hit_row,
                                                    col: hit_col,
                                                },
                                                garg_rows.clone(),
                                            )
                                        })
                                        .collect(),
                                    self.min_max_garg_x,
                                    self.ice_and_cob_times.is_iced(),
                                    None,
                                )
                            }
                            _ => {
                                printer::print_too_many_arguments_error();
                                return ParseResult::Err;
                            }
                        }
                    } else {
                        match extra_args {
                            [">", ..] if *command == "delay" => {
                                printer::print_error(NEED_HIT_ROW_HIT_COL_COB_COL);
                                return ParseResult::Err;
                            }
                            [_, ">", ..] if *command == "delay" => {
                                printer::print_error(NEED_HIT_COL_COB_COL);
                                return ParseResult::Err;
                            }
                            [_, _, ">", ..] if *command == "delay" => {
                                printer::print_error(NEED_COB_COL);
                                return ParseResult::Err;
                            }
                            [hit_row, hit_col, cob_col, ">", garg_pos_args @ ..]
                                if *command == "delay" =>
                            {
                                let (Ok(hit_row), Ok(hit_col), Ok(cob_col)) = (
                                    Parser::parse_hit_row(
                                        hit_row,
                                        &self.scene.all_rows(),
                                        &mut self.eval_context,
                                    ),
                                    Parser::parse_hit_col(hit_col, &mut self.eval_context),
                                    Parser::parse_cob_col(cob_col, &mut self.eval_context),
                                ) else {
                                    return ParseResult::Err;
                                };
                                let Ok(ParsedGargPos {
                                    garg_rows,
                                    min_max_garg_x,
                                    ice_flag,
                                }) = Parser::parse_garg_pos(
                                    garg_pos_args,
                                    &self.scene.garg_rows_for_cob(hit_row),
                                    &mut self.eval_context,
                                )
                                else {
                                    return ParseResult::Err;
                                };
                                let cob = game::Cob::Roof {
                                    row: hit_row,
                                    col: hit_col,
                                    cob_col,
                                    cob_row: DEFAULT_ROOF_COB_ROW,
                                };
                                (
                                    vec![(cob.clone(), garg_rows.clone())],
                                    min_max_garg_x.unwrap_or(self.min_max_garg_x),
                                    ice_flag.unwrap_or(self.ice_and_cob_times.is_iced()),
                                    Some(garg_rows),
                                )
                            }
                            [] => {
                                printer::print_error(NEED_HIT_COL_COB_COL);
                                return ParseResult::Err;
                            }
                            [_] => {
                                printer::print_error(NEED_COB_COL);
                                return ParseResult::Err;
                            }
                            [hit_col, cob_col] => {
                                let (Ok(hit_col), Ok(cob_col)) = (
                                    Parser::parse_hit_col(hit_col, &mut self.eval_context),
                                    Parser::parse_cob_col(cob_col, &mut self.eval_context),
                                ) else {
                                    return ParseResult::Err;
                                };
                                (
                                    self.scene
                                        .hit_row_and_garg_rows_of_delay_mode(&delay_mode.unwrap_or(
                                            self.scene.default_delay_mode(hit_col, Some(cob_col)),
                                        ))
                                        .iter()
                                        .map(|(hit_row, garg_rows)| {
                                            (
                                                game::Cob::Roof {
                                                    row: *hit_row,
                                                    col: hit_col,
                                                    cob_col,
                                                    cob_row: DEFAULT_ROOF_COB_ROW,
                                                },
                                                garg_rows.clone(),
                                            )
                                        })
                                        .collect(),
                                    self.min_max_garg_x,
                                    self.ice_and_cob_times.is_iced(),
                                    None,
                                )
                            }
                            _ => {
                                printer::print_too_many_arguments_error();
                                return ParseResult::Err;
                            }
                        }
                    };
                let Ok(garg_x_range) = validate_garg_x_range(&mut min_max_garg_x) else {
                    return ParseResult::Err;
                };
                let explode_and_garg_rows: Vec<(game::Explode, &Vec<i32>)> = cob_and_garg_rows
                    .iter()
                    .map(|(cob, garg_rows)| (game::Explode::of_cob(cob, &self.scene), garg_rows))
                    .collect();
                let (eat, intercept) =
                    game::judge(&garg_x_range, &explode_and_garg_rows, ice_flag, &self.scene);
                let display_garg_rows = match garg_rows_explicit {
                    Some(rows) => rows,
                    None => {
                        let mut seen = Vec::new();
                        for (_, rows) in &cob_and_garg_rows {
                            for r in rows {
                                if !seen.contains(r) {
                                    seen.push(*r);
                                }
                            }
                        }
                        seen
                    }
                };
                printer::print_delay_result(
                    &cob_and_garg_rows,
                    &display_garg_rows,
                    &eat,
                    &intercept,
                    if min_max_garg_x != self.min_max_garg_x {
                        Some(min_max_garg_x)
                    } else {
                        None
                    },
                );
                ParseResult::Ok
            }
            _ => ParseResult::Unmatched,
        }
    }

    pub fn parse_doom(&mut self, input: &str) -> ParseResult {
        match input.split_whitespace().collect::<Vec<&str>>().as_slice() {
            ["doom", extra_args @ ..] => match extra_args {
                [] => {
                    printer::print_error(NEED_DOOM_ROW_DOOM_COL);
                    ParseResult::Err
                }
                [_] => {
                    printer::print_error(NEED_DOOM_ROW);
                    ParseResult::Err
                }
                [doom_row, doom_col, garg_pos_args @ ..] => {
                    let (Ok(doom_row), Ok(doom_col)) = (
                        Parser::parse_doom_row(
                            doom_row,
                            &self.scene.all_rows(),
                            &mut self.eval_context,
                        ),
                        Parser::parse_doom_col(doom_col, &mut self.eval_context),
                    ) else {
                        return ParseResult::Err;
                    };
                    let explode = game::Explode::of_doom(
                        &game::Doom {
                            row: doom_row,
                            col: doom_col,
                        },
                        &self.scene,
                    );
                    let (garg_rows, mut min_max_garg_x, ice_flag) = match garg_pos_args {
                        [] => (
                            self.scene.garg_rows_for_doom(doom_row),
                            self.min_max_garg_x,
                            self.ice_and_cob_times.is_iced(),
                        ),
                        [">", garg_pos_args @ ..] => {
                            let Ok(ParsedGargPos {
                                garg_rows,
                                min_max_garg_x,
                                ice_flag,
                            }) = Parser::parse_garg_pos(
                                garg_pos_args,
                                &self.scene.garg_rows_for_doom(doom_row),
                                &mut self.eval_context,
                            )
                            else {
                                return ParseResult::Err;
                            };
                            (
                                garg_rows,
                                min_max_garg_x.unwrap_or(self.min_max_garg_x),
                                ice_flag.unwrap_or(self.ice_and_cob_times.is_iced()),
                            )
                        }
                        _ => {
                            printer::print_too_many_arguments_error();
                            return ParseResult::Err;
                        }
                    };
                    let Ok(garg_x_range) = validate_garg_x_range(&mut min_max_garg_x) else {
                        return ParseResult::Err;
                    };
                    let (mut eat, mut intercept) = game::judge(
                        &garg_x_range,
                        &[(explode.clone(), &garg_rows)],
                        ice_flag,
                        &self.scene,
                    );
                    eat.shift_to_plant_intercept();
                    intercept.shift_to_plant_intercept();
                    printer::print_doom_result(
                        doom_row,
                        doom_col,
                        &garg_rows,
                        &eat,
                        &intercept,
                        if min_max_garg_x != self.min_max_garg_x {
                            Some(min_max_garg_x)
                        } else {
                            None
                        },
                    );
                    ParseResult::Ok
                }
            },
            _ => ParseResult::Unmatched,
        }
    }

    pub fn parse_hit_or_nohit(&mut self, input: &str) -> ParseResult {
        match input.split_whitespace().collect::<Vec<&str>>().as_slice() {
            ["hit", extra_args @ ..] | ["nohit", extra_args @ ..] => {
                let (min_max_garg_x, cob_dist) = if !self.scene.is_roof() {
                    match extra_args {
                        [] => (self.min_max_garg_x, self.scene.cob_dist(None)),
                        [delay_time] => {
                            let Ok(delay_time) =
                                Parser::parse_delay_time(delay_time, &mut self.eval_context)
                            else {
                                return ParseResult::Err;
                            };
                            match game::IceAndCobTimes::of_ice_times_and_cob_time(
                                &self.ice_and_cob_times.ice_times,
                                self.ice_and_cob_times.cob_time + delay_time,
                            ) {
                                Err(err) => {
                                    printer::print_error(err.as_str());
                                    return ParseResult::Err;
                                }
                                Ok(ice_and_cob_times) => {
                                    match game::min_max_garg_x(&ice_and_cob_times) {
                                        Err(err) => {
                                            printer::print_error(err.as_str());
                                            return ParseResult::Err;
                                        }
                                        Ok((min_garg_x, max_garg_x)) => {
                                            printer::print_delay_status(
                                                &ice_and_cob_times,
                                                (min_garg_x, max_garg_x),
                                            );
                                            ((min_garg_x, max_garg_x), self.scene.cob_dist(None))
                                        }
                                    }
                                }
                            }
                        }
                        _ => {
                            printer::print_too_many_arguments_error();
                            return ParseResult::Err;
                        }
                    }
                } else {
                    match extra_args {
                        [] => {
                            printer::print_error(NEED_COB_COL);
                            return ParseResult::Err;
                        }
                        [cob_col] => {
                            let Ok(cob_col) =
                                Parser::parse_cob_col(cob_col, &mut self.eval_context)
                            else {
                                return ParseResult::Err;
                            };
                            (self.min_max_garg_x, self.scene.cob_dist(Some(cob_col)))
                        }
                        [cob_col, delay_time] => {
                            let (Ok(delay_time), Ok(cob_col)) = (
                                Parser::parse_delay_time(delay_time, &mut self.eval_context),
                                Parser::parse_cob_col(cob_col, &mut self.eval_context),
                            ) else {
                                return ParseResult::Err;
                            };
                            match game::IceAndCobTimes::of_ice_times_and_cob_time(
                                &self.ice_and_cob_times.ice_times,
                                self.ice_and_cob_times.cob_time + delay_time,
                            ) {
                                Err(err) => {
                                    printer::print_error(err.as_str());
                                    return ParseResult::Err;
                                }
                                Ok(ice_and_cob_times) => {
                                    match game::min_max_garg_x(&ice_and_cob_times) {
                                        Err(err) => {
                                            printer::print_error(err.as_str());
                                            return ParseResult::Err;
                                        }
                                        Ok((min_garg_x, max_garg_x)) => {
                                            printer::print_delay_status(
                                                &ice_and_cob_times,
                                                (min_garg_x, max_garg_x),
                                            );
                                            (
                                                (min_garg_x, max_garg_x),
                                                self.scene.cob_dist(Some(cob_col)),
                                            )
                                        }
                                    }
                                }
                            }
                        }
                        _ => {
                            printer::print_too_many_arguments_error();
                            return ParseResult::Err;
                        }
                    }
                };
                if input.starts_with("hit") {
                    printer::print_hit_cob_dist(&self.scene, min_max_garg_x.1 as i32, cob_dist)
                } else if input.starts_with("nohit") {
                    printer::print_nohit_cob_dist(&self.scene, min_max_garg_x.0 as i32, cob_dist);
                };
                ParseResult::Ok
            }
            _ => ParseResult::Unmatched,
        }
    }

    pub fn parse_find_max_delay(&mut self, input: &str) -> ParseResult {
        match input.split_whitespace().collect::<Vec<&str>>().as_slice() {
            ["max", extra_args @ ..] => {
                let (cob_list, garg_rows, mut min_max_garg_x, ice_flag, hit_row, cob_col_opt) =
                    if !self.scene.is_roof() {
                        match extra_args {
                            [] | [">", ..] => {
                                printer::print_error(NEED_HIT_ROW_HIT_COL_RANGE);
                                return ParseResult::Err;
                            }
                            [_] | [_, ">", ..] => {
                                printer::print_error(NEED_HIT_COL_RANGE);
                                return ParseResult::Err;
                            }
                            [hit_row, min_max_hit_col, ">", garg_pos_args @ ..] => {
                                let (Ok(hit_row), Ok((min_hit_col, max_hit_col))) = (
                                    Parser::parse_hit_row(
                                        hit_row,
                                        &self.scene.all_rows(),
                                        &mut self.eval_context,
                                    ),
                                    Parser::parse_min_max_hit_col(
                                        min_max_hit_col,
                                        &mut self.eval_context,
                                    ),
                                ) else {
                                    return ParseResult::Err;
                                };
                                let Ok(ParsedGargPos {
                                    garg_rows,
                                    min_max_garg_x,
                                    ice_flag,
                                }) = Parser::parse_garg_pos(
                                    garg_pos_args,
                                    &self.scene.garg_rows_for_cob(hit_row),
                                    &mut self.eval_context,
                                )
                                else {
                                    return ParseResult::Err;
                                };
                                (
                                    ((min_hit_col * 80.).round() as i32
                                        ..=(max_hit_col * 80.).round() as i32)
                                        .map(|v| game::Cob::Ground {
                                            row: hit_row,
                                            col: v as f32 / 80.,
                                        })
                                        .collect::<Vec<game::Cob>>(),
                                    garg_rows,
                                    min_max_garg_x.unwrap_or(self.min_max_garg_x),
                                    ice_flag.unwrap_or(self.ice_and_cob_times.is_iced()),
                                    hit_row,
                                    None,
                                )
                            }
                            _ => {
                                printer::print_bad_format_error();
                                return ParseResult::Err;
                            }
                        }
                    } else {
                        match extra_args {
                            [] | [">", ..] => {
                                printer::print_error(NEED_HIT_ROW_HIT_COL_RANGE_COB_COL);
                                return ParseResult::Err;
                            }
                            [_] | [_, ">", ..] => {
                                printer::print_error(NEED_HIT_COL_RANGE_COB_COL);
                                return ParseResult::Err;
                            }
                            [_, _] | [_, _, ">", ..] => {
                                printer::print_error(NEED_COB_COL);
                                return ParseResult::Err;
                            }
                            [hit_row, min_max_hit_col, cob_col, ">", garg_pos_args @ ..] => {
                                let (Ok(hit_row), Ok((min_hit_col, max_hit_col)), Ok(cob_col)) = (
                                    Parser::parse_hit_row(
                                        hit_row,
                                        &self.scene.all_rows(),
                                        &mut self.eval_context,
                                    ),
                                    Parser::parse_min_max_hit_col(
                                        min_max_hit_col,
                                        &mut self.eval_context,
                                    ),
                                    Parser::parse_cob_col(cob_col, &mut self.eval_context),
                                ) else {
                                    return ParseResult::Err;
                                };
                                let Ok(ParsedGargPos {
                                    garg_rows,
                                    min_max_garg_x,
                                    ice_flag,
                                }) = Parser::parse_garg_pos(
                                    garg_pos_args,
                                    &self.scene.garg_rows_for_cob(hit_row),
                                    &mut self.eval_context,
                                )
                                else {
                                    return ParseResult::Err;
                                };
                                (
                                    ((min_hit_col * 80.).round() as i32
                                        ..=(max_hit_col * 80.).round() as i32)
                                        .map(|v| game::Cob::Roof {
                                            row: hit_row,
                                            col: v as f32 / 80.,
                                            cob_col,
                                            cob_row: DEFAULT_ROOF_COB_ROW,
                                        })
                                        .collect::<Vec<game::Cob>>(),
                                    garg_rows,
                                    min_max_garg_x.unwrap_or(self.min_max_garg_x),
                                    ice_flag.unwrap_or(self.ice_and_cob_times.is_iced()),
                                    hit_row,
                                    Some(cob_col),
                                )
                            }
                            _ => {
                                printer::print_bad_format_error();
                                return ParseResult::Err;
                            }
                        }
                    };
                if cob_list.is_empty() {
                    return ParseResult::Err;
                }
                let Ok(garg_x_range) = validate_garg_x_range(&mut min_max_garg_x) else {
                    return ParseResult::Err;
                };
                let col_range = (cob_list[0].col(), cob_list.last().unwrap().col());
                let mut max_delay: Option<i32> = None;
                let mut cob_cols: Vec<f32> = vec![];
                let mut eat = game::Eat::Empty;
                let mut intercept = game::Intercept::Empty;
                for cob in &cob_list {
                    let (new_eat, new_intercept) = game::judge(
                        &garg_x_range,
                        &[(game::Explode::of_cob(cob, &self.scene), &garg_rows)],
                        ice_flag,
                        &self.scene,
                    );
                    match (
                        max_delay,
                        game::safe_intercept_interval(&new_eat, &new_intercept),
                    ) {
                        (_, None) => {}
                        (None, Some((_, new_max))) => {
                            max_delay = Some(new_max);
                            cob_cols = vec![cob.col()];
                            eat = new_eat;
                            intercept = new_intercept;
                        }
                        (Some(prev_max), Some((_, new_max))) => match new_max.cmp(&prev_max) {
                            std::cmp::Ordering::Greater => {
                                max_delay = Some(new_max);
                                cob_cols = vec![cob.col()];
                                eat = new_eat;
                                intercept = new_intercept;
                            }
                            _ => {
                                cob_cols.push(cob.col());
                            }
                        },
                    };
                }
                let modified = if min_max_garg_x != self.min_max_garg_x {
                    Some(min_max_garg_x)
                } else {
                    None
                };
                match cob_cols.as_mut_slice() {
                    [] => {
                        printer::print_max_no_harmless(
                            hit_row,
                            col_range,
                            cob_col_opt,
                            &garg_rows,
                            modified,
                        );
                    }
                    cob_cols => {
                        cob_cols.sort_by(|a, b| a.partial_cmp(b).unwrap());
                        printer::print_max_result(
                            hit_row,
                            col_range,
                            cob_col_opt,
                            &garg_rows,
                            cob_cols,
                            &eat,
                            &intercept,
                            modified,
                        );
                    }
                };
                ParseResult::Ok
            }
            _ => ParseResult::Unmatched,
        }
    }

    pub fn parse_imp(&mut self, input: &str) -> ParseResult {
        match input.split_whitespace().collect::<Vec<&str>>().as_slice() {
            ["imp", extra_args @ ..] => match extra_args {
                [] => {
                    printer::print_error(NEED_GARG_X_OR_IMP_X);
                    ParseResult::Err
                }
                ["garg"] => {
                    printer::print_error(NEED_IMP_X_RANGE);
                    ParseResult::Err
                }
                ["garg", imp_x] => {
                    let is_roof = self.scene.is_roof();
                    let (min_valid_imp_x, max_valid_imp_x) = constants::imp_x_bounds(is_roof);
                    match imp_x
                        .replace('，', ",")
                        .split(',')
                        .collect::<Vec<&str>>()
                        .as_slice()
                    {
                        [single_imp_x] => {
                            let imp_x = match eval_as_i32(single_imp_x, &mut self.eval_context) {
                                Ok(v) => v,
                                Err(EvalError::Eval(e)) => {
                                    printer::print_error(&e);
                                    return ParseResult::Err;
                                }
                                Err(EvalError::Type(v)) => {
                                    printer::print_error_with_input(IMP_X_SHOULD_BE_INTEGER, &v);
                                    return ParseResult::Err;
                                }
                            };
                            let Some((min_garg_x, max_garg_x)) =
                                constants::min_max_garg_pos_of_imp_x_by_scene(imp_x, is_roof)
                            else {
                                printer::print_error_with_input(
                                    &IMP_X_SHOULD_BE_IN_RANGE
                                        .format(&[min_valid_imp_x, max_valid_imp_x]),
                                    imp_x.to_string().as_str(),
                                );
                                return ParseResult::Err;
                            };
                            printer::print_garg_x_for_imp(
                                &imp_x.to_string(),
                                min_garg_x,
                                max_garg_x,
                            );
                            ParseResult::Ok
                        }
                        [min_imp_x, max_imp_x] => {
                            let min_imp_x = match eval_as_i32(min_imp_x, &mut self.eval_context) {
                                Ok(v) => v,
                                Err(EvalError::Eval(e)) => {
                                    printer::print_error(&e);
                                    return ParseResult::Err;
                                }
                                Err(EvalError::Type(v)) => {
                                    printer::print_error_with_input(IMP_X_SHOULD_BE_INTEGER, &v);
                                    return ParseResult::Err;
                                }
                            };
                            let max_imp_x = match eval_as_i32(max_imp_x, &mut self.eval_context) {
                                Ok(v) => v,
                                Err(EvalError::Eval(e)) => {
                                    printer::print_error(&e);
                                    return ParseResult::Err;
                                }
                                Err(EvalError::Type(v)) => {
                                    printer::print_error_with_input(IMP_X_SHOULD_BE_INTEGER, &v);
                                    return ParseResult::Err;
                                }
                            };
                            let (min_imp_x, max_imp_x) = if min_imp_x <= max_imp_x {
                                (min_imp_x, max_imp_x)
                            } else {
                                (max_imp_x, min_imp_x)
                            };
                            let clamped_min_imp_x = min_imp_x.max(min_valid_imp_x);
                            let clamped_max_imp_x = max_imp_x.min(max_valid_imp_x);
                            if clamped_min_imp_x > clamped_max_imp_x {
                                printer::print_error_with_input(
                                    &IMP_X_SHOULD_BE_IN_RANGE
                                        .format(&[min_valid_imp_x, max_valid_imp_x]),
                                    imp_x,
                                );
                                return ParseResult::Err;
                            }
                            let Some((range_min, range_max)) =
                                constants::union_min_max_garg_pos_of_imp_x_range(
                                    clamped_min_imp_x,
                                    clamped_max_imp_x,
                                    is_roof,
                                )
                            else {
                                printer::print_error_with_input(
                                    &IMP_X_SHOULD_BE_IN_RANGE
                                        .format(&[min_valid_imp_x, max_valid_imp_x]),
                                    imp_x,
                                );
                                return ParseResult::Err;
                            };
                            printer::print_garg_x_for_imp(
                                &format!("{}~{}", min_imp_x, max_imp_x),
                                range_min,
                                range_max,
                            );
                            ParseResult::Ok
                        }
                        _ => {
                            printer::print_too_many_arguments_error();
                            ParseResult::Err
                        }
                    }
                }
                [garg_x] => {
                    let garg_x_value = match eval_as_f32(garg_x, &mut self.eval_context) {
                        Ok(v) => v,
                        Err(EvalError::Eval(e)) => {
                            printer::print_error(&e);
                            return ParseResult::Err;
                        }
                        Err(EvalError::Type(v)) => {
                            printer::print_error_with_input(GARG_X_SHOULD_BE_NUMBER, &v);
                            return ParseResult::Err;
                        }
                    };
                    if garg_x_value <= 400.0 {
                        printer::print_error_with_input(
                            &MIN_GARG_X_SHOULD_BE_LARGER_THAN_LOWER_BOUND.format(&[400]),
                            garg_x,
                        );
                        return ParseResult::Err;
                    }
                    let imp_x_rnd_0 = game::get_imp_x(garg_x_value, 0.0, &self.scene);
                    let imp_x_rnd_100 = game::get_imp_x(garg_x_value, 100.0, &self.scene);
                    printer::print_imp_x_for_garg(garg_x_value, imp_x_rnd_0, imp_x_rnd_100);
                    ParseResult::Ok
                }
                _ => {
                    printer::print_too_many_arguments_error();
                    ParseResult::Err
                }
            },
            _ => ParseResult::Unmatched,
        }
    }

    fn parse_ice_times(ice_times: &[&str], ctx: &mut HashMapContext) -> Result<Vec<i32>, ()> {
        match ice_times
            .iter()
            .map(|&s| eval_as_i32(s, ctx))
            .collect::<Result<Vec<i32>, _>>()
        {
            Err(EvalError::Eval(e)) => {
                printer::print_error(&e);
                Err(())
            }
            Err(EvalError::Type(v)) => {
                printer::print_error_with_input(ICE_TIMES_SHOULD_BE_INTEGER, &v);
                Err(())
            }
            Ok(ice_times) => Ok(ice_times),
        }
    }

    fn parse_cob_time(cob_time: &&str, ctx: &mut HashMapContext) -> Result<i32, ()> {
        match eval_as_i32(cob_time, ctx) {
            Err(EvalError::Eval(e)) => {
                printer::print_error(&e);
                Err(())
            }
            Err(EvalError::Type(v)) => {
                printer::print_error_with_input(COB_TIME_SHOULD_BE_INTEGER, &v);
                Err(())
            }
            Ok(cob_time) if cob_time < 0 => {
                printer::print_error_with_input(
                    COB_TIME_SHOULD_BE_NON_NEGATIVE,
                    cob_time.to_string().as_str(),
                );
                Err(())
            }
            Ok(cob_time) => Ok(cob_time),
        }
    }

    fn parse_delay_time(delay_time: &&str, ctx: &mut HashMapContext) -> Result<i32, ()> {
        match eval_as_i32(delay_time, ctx) {
            Err(EvalError::Eval(e)) => {
                printer::print_error(&e);
                Err(())
            }
            Err(EvalError::Type(v)) => {
                printer::print_error_with_input(DELAY_TIME_SHOULD_BE_INTEGER, &v);
                Err(())
            }
            Ok(delay) => Ok(delay),
        }
    }

    fn parse_hit_row(
        hit_row: &&str,
        valid_hit_rows: &[i32],
        ctx: &mut HashMapContext,
    ) -> Result<i32, ()> {
        match eval_as_i32(hit_row, ctx) {
            Err(EvalError::Eval(e)) => {
                printer::print_error(&e);
                Err(())
            }
            Err(EvalError::Type(v)) => {
                printer::print_error_with_input(HIT_ROW_SHOULD_BE_INTEGER, &v);
                Err(())
            }
            Ok(hit_row) if !(valid_hit_rows.contains(&hit_row)) => {
                printer::print_error_with_input(
                    &HIT_ROW_OUT_OF_RANGE.format(&[format!("{:?}", valid_hit_rows)]),
                    hit_row.to_string().as_str(),
                );
                Err(())
            }
            Ok(hit_row) => Ok(hit_row),
        }
    }

    fn parse_hit_col(hit_col: &&str, ctx: &mut HashMapContext) -> Result<f32, ()> {
        match eval_as_f32(hit_col, ctx) {
            Err(EvalError::Eval(e)) => {
                printer::print_error(&e);
                Err(())
            }
            Err(EvalError::Type(v)) => {
                printer::print_error_with_input(HIT_COL_SHOULD_BE_NUMBER, &v);
                Err(())
            }
            Ok(hit_col) if !((0. ..10.).contains(&hit_col)) => {
                printer::print_error_with_input(
                    HIT_COL_SHOULD_BE_IN_RANGE,
                    hit_col.to_string().as_str(),
                );
                Err(())
            }
            Ok(hit_col) => match game::hit_col_matching_int_pixel(hit_col) {
                None => Ok(hit_col),
                Some(corrected_hit_col) => {
                    printer::print_warning(
                        &HIT_COL_TIMES_EIGHTY_NOT_INTEGER.format(&[hit_col, corrected_hit_col]),
                    );
                    Ok(corrected_hit_col)
                }
            },
        }
    }

    fn parse_min_max_hit_col(
        min_max_hit_col: &&str,
        ctx: &mut HashMapContext,
    ) -> Result<(f32, f32), ()> {
        match min_max_hit_col
            .replace('，', ",")
            .split(',')
            .collect::<Vec<&str>>()
            .as_slice()
        {
            [] => {
                printer::print_error(NEED_MIN_MAX_HIT_COL);
                Err(())
            }
            [_] => {
                printer::print_error(NEED_MAX_HIT_COL);
                Err(())
            }
            [min_hit_col, max_hit_col] => {
                match (
                    Parser::parse_hit_col(min_hit_col, ctx),
                    Parser::parse_hit_col(max_hit_col, ctx),
                ) {
                    (Err(_), _) | (_, Err(_)) => Err(()),
                    (Ok(min_hit_col), Ok(max_hit_col)) => {
                        let min_hit_pixel = (min_hit_col * 80.).round() as i32;
                        let max_hit_pixel = (max_hit_col * 80.).round() as i32;
                        if min_hit_pixel > max_hit_pixel {
                            printer::print_error_with_input(
                                MIN_COL_SHOULD_BE_SMALLER_THAN_MAX_COL,
                                format!("{}, {}", min_hit_col, max_hit_col).as_str(),
                            );
                            return Err(());
                        }
                        Ok((min_hit_col, max_hit_col))
                    }
                }
            }
            _ => {
                printer::print_too_many_arguments_error();
                Err(())
            }
        }
    }

    fn parse_cob_col(cob_col: &&str, ctx: &mut HashMapContext) -> Result<i32, ()> {
        match eval_as_i32(cob_col, ctx) {
            Err(EvalError::Eval(e)) => {
                printer::print_error(&e);
                Err(())
            }
            Err(EvalError::Type(v)) => {
                printer::print_error_with_input(COB_COL_SHOULD_BE_INTEGER, &v);
                Err(())
            }
            Ok(cob_col) if !((1..=8).contains(&cob_col)) => {
                printer::print_error_with_input(
                    COB_COL_SHOULD_BE_IN_RANGE,
                    cob_col.to_string().as_str(),
                );
                Err(())
            }
            Ok(cob_col) => Ok(cob_col),
        }
    }

    fn parse_doom_row(
        doom_row: &&str,
        valid_doom_rows: &[i32],
        ctx: &mut HashMapContext,
    ) -> Result<i32, ()> {
        match eval_as_i32(doom_row, ctx) {
            Err(EvalError::Eval(e)) => {
                printer::print_error(&e);
                Err(())
            }
            Err(EvalError::Type(v)) => {
                printer::print_error_with_input(DOOM_ROW_SHOULD_BE_INTEGER, &v);
                Err(())
            }
            Ok(doom_row) if !(valid_doom_rows.contains(&doom_row)) => {
                printer::print_error_with_input(
                    &DOOM_ROW_OUT_OF_RANGE.format(&[format!("{:?}", valid_doom_rows)]),
                    doom_row.to_string().as_str(),
                );
                Err(())
            }
            Ok(doom_row) => Ok(doom_row),
        }
    }

    fn parse_doom_col(doom_col: &&str, ctx: &mut HashMapContext) -> Result<i32, ()> {
        match eval_as_i32(doom_col, ctx) {
            Err(EvalError::Eval(e)) => {
                printer::print_error(&e);
                Err(())
            }
            Err(EvalError::Type(v)) => {
                printer::print_error_with_input(DOOM_COL_SHOULD_BE_INTEGER, &v);
                Err(())
            }
            Ok(doom_col) if !((1..=9).contains(&doom_col)) => {
                printer::print_error_with_input(
                    DOOM_COL_SHOULD_BE_IN_RANGE,
                    doom_col.to_string().as_str(),
                );
                Err(())
            }
            Ok(doom_col) => Ok(doom_col),
        }
    }

    fn parse_garg_pos(
        garg_pos_args: &[&str],
        valid_garg_rows: &[i32],
        ctx: &mut HashMapContext,
    ) -> Result<ParsedGargPos, ()> {
        let (garg_rows, min_max_garg_x, ice_flag) = match garg_pos_args {
            [] => {
                printer::print_error(NEED_GARG_ROWS_X_RANGE_ICE_FLAG);
                (Err(()), Err(()), Err(()))
            }
            [garg_rows] => (
                Parser::parse_garg_rows(garg_rows, valid_garg_rows, ctx),
                Ok(None),
                Ok(None),
            ),
            [garg_rows, min_max_garg_x] => (
                Parser::parse_garg_rows(garg_rows, valid_garg_rows, ctx),
                Parser::parse_min_max_garg_x(min_max_garg_x, ctx).map(Some),
                Ok(None),
            ),
            [garg_rows, min_max_garg_x, ice_flag] => (
                Parser::parse_garg_rows(garg_rows, valid_garg_rows, ctx),
                Parser::parse_min_max_garg_x(min_max_garg_x, ctx).map(Some),
                Parser::parse_ice_flag(ice_flag).map(Some),
            ),
            _ => {
                printer::print_too_many_arguments_error();
                (Err(()), Err(()), Err(()))
            }
        };
        match (garg_rows, min_max_garg_x, ice_flag) {
            (Err(()), _, _) | (_, Err(()), _) | (_, _, Err(())) => Err(()),
            (Ok(garg_rows), Ok(min_max_garg_x), Ok(ice_flag)) => Ok(ParsedGargPos {
                garg_rows,
                min_max_garg_x,
                ice_flag,
            }),
        }
    }

    fn parse_garg_rows(
        garg_rows: &&str,
        valid_garg_rows: &[i32],
        ctx: &mut HashMapContext,
    ) -> Result<Vec<i32>, ()> {
        match garg_rows
            .replace('，', ",")
            .split(',')
            .map(|s| eval_as_i32(s, ctx))
            .collect::<Result<Vec<i32>, _>>()
        {
            Err(EvalError::Eval(e)) => {
                printer::print_error(&e);
                Err(())
            }
            Err(EvalError::Type(v)) => {
                printer::print_error_with_input(GARG_ROWS_SHOULD_BE_INTEGER, &v);
                Err(())
            }
            Ok(garg_rows) => {
                let filtered_garg_rows = garg_rows
                    .iter()
                    .filter(|&v| valid_garg_rows.contains(v))
                    .cloned()
                    .collect::<Vec<i32>>();
                if filtered_garg_rows.is_empty() {
                    printer::print_error_with_input(
                        &GARG_ROWS_ALL_OUT_OF_RANGE.format(&[format!("{:?}", valid_garg_rows)]),
                        format!("{:?}", garg_rows).as_str(),
                    );
                    Err(())
                } else {
                    Ok(filtered_garg_rows)
                }
            }
        }
    }

    fn parse_min_max_garg_x(
        min_max_garg_x: &&str,
        ctx: &mut HashMapContext,
    ) -> Result<(f32, f32), ()> {
        match min_max_garg_x
            .replace('，', ",")
            .split(',')
            .collect::<Vec<&str>>()
            .as_slice()
        {
            [] => {
                printer::print_error(NEED_MIN_MAX_GARG_X);
                Err(())
            }
            [_] => {
                printer::print_error(NEED_MAX_GARG_X);
                Err(())
            }
            [min_garg_x, max_garg_x] => {
                match (eval_as_f32(min_garg_x, ctx), eval_as_f32(max_garg_x, ctx)) {
                    (Err(EvalError::Eval(e)), _) | (_, Err(EvalError::Eval(e))) => {
                        printer::print_error(&e);
                        Err(())
                    }
                    (Err(EvalError::Type(v)), _) => {
                        printer::print_error_with_input(MIN_GARG_X_SHOULD_BE_NUMBER, &v);
                        Err(())
                    }
                    (_, Err(EvalError::Type(v))) => {
                        printer::print_error_with_input(MAX_GARG_X_SHOULD_BE_NUMBER, &v);
                        Err(())
                    }
                    (Ok(min_garg_x), Ok(max_garg_x)) if min_garg_x > max_garg_x => {
                        printer::print_error_with_input(
                            MIN_GARG_X_SHOULD_BE_SMALLER_THAN_MAX_GARG_X,
                            format!("{}, {}", min_garg_x, max_garg_x).as_str(),
                        );
                        Err(())
                    }
                    (Ok(min_garg_x), _) if min_garg_x <= game::MIN_GARG_X => {
                        printer::print_error_with_input(
                            &MIN_GARG_X_SHOULD_BE_LARGER_THAN_LOWER_BOUND
                                .format(&[game::MIN_GARG_X]),
                            format!("{}", min_garg_x).as_str(),
                        );
                        Err(())
                    }
                    (_, Ok(max_garg_x)) if max_garg_x > game::MAX_GARG_X => {
                        printer::print_error_with_input(
                            &MAX_GARG_X_SHOULD_BE_SMALLER_THAN_UPPER_BOUND
                                .format(&[game::MAX_GARG_X]),
                            format!("{}", max_garg_x).as_str(),
                        );
                        Err(())
                    }
                    (Ok(min_garg_x), Ok(max_garg_x)) => Ok((min_garg_x, max_garg_x)),
                }
            }
            _ => {
                printer::print_too_many_arguments_error();
                Err(())
            }
        }
    }

    fn parse_ice_flag(ice_mode: &&str) -> Result<i32, ()> {
        if *ice_mode == "u" {
            Ok(0)
        } else if *ice_mode == "i" {
            Ok(100000)
        } else {
            printer::print_error_with_input(ICE_FLAG_SHOULD_BE_U_OR_I, ice_mode);
            Err(())
        }
    }
}

/// Run a semicolon-separated interception command string against a fresh
/// [`Parser`]. Output is written through the `out!`/`errln!` sink (stdout/stderr
/// for the CLI, or the in-memory capture buffer for the MCP server). Returns
/// `Err(())` on the first parse error or unknown command (already reported via
/// the diagnostic sink).
pub fn run_intercept(command: &str) -> Result<(), ()> {
    let mut parser = Parser::default();
    for segment in command.split(';') {
        let line = segment.trim().to_lowercase();
        if line.is_empty() {
            continue;
        }
        match dispatch(&mut parser, &line) {
            ParseResult::Ok => continue,
            ParseResult::Err => return Err(()),
            ParseResult::Unmatched => {
                errln!("error: unknown command (got: {})", line);
                return Err(());
            }
        }
    }
    Ok(())
}

fn dispatch(parser: &mut Parser, input: &str) -> ParseResult {
    let dispatchers: [fn(&mut Parser, &str) -> ParseResult; 7] = [
        Parser::parse_scene,
        Parser::parse_wave,
        Parser::parse_delay,
        Parser::parse_doom,
        Parser::parse_hit_or_nohit,
        Parser::parse_find_max_delay,
        Parser::parse_imp,
    ];
    for d in dispatchers {
        match d(parser, input) {
            ParseResult::Unmatched => continue,
            other => return other,
        }
    }
    ParseResult::Unmatched
}
