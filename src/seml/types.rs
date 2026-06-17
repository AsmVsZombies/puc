//! Scenario data structures that serialize to the `Config` JSON the emulator
//! reader expects (`crates/pvz-emulator-sys/vendor/seml/reader/`). Field names
//! and the `op` tag mirror the reader 1:1; absent optionals map to its `-1`
//! defaults, so they are skipped when `None`.

use serde::Serialize;

#[derive(Serialize, Default)]
pub struct Config {
    pub setting: Setting,
    pub waves: Vec<Wave>,
}

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Setting {
    pub scene: String, // mapped scene: "NE" | "FE" | "ME"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_scene: Option<String>, // "DE" | "NE" | "PE" | "FE" | "RE" | "ME"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protect: Option<Vec<ProtectPos>>,
}

#[derive(Serialize, Clone, Copy, PartialEq)]
pub enum ProtectKind {
    Cob,
    Normal,
}

#[derive(Serialize)]
pub struct ProtectPos {
    #[serde(rename = "type")]
    pub kind: ProtectKind,
    pub row: i32,
    pub col: i32,
}

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Wave {
    pub ice_times: Vec<i32>,
    pub wave_length: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_tick: Option<i32>,
    pub actions: Vec<Action>,
}

#[derive(Serialize, Clone, Copy)]
pub struct CobPos {
    pub row: i32,
    pub col: f64,
}

#[derive(Serialize, Clone, Copy)]
pub struct CardPos {
    pub row: i32,
    pub col: i32,
}

#[derive(Serialize, Clone, Copy)]
pub enum Fodder {
    Normal,
    Puff,
    Pot,
}

/// One scheduled action. Internally tagged by `op` to match the reader; each
/// variant renames its fields to camelCase (`cobCol`, `shovelTime`, `plantType`).
#[derive(Serialize, Clone)]
#[serde(tag = "op")]
pub enum Action {
    #[serde(rename_all = "camelCase")]
    Cob {
        symbol: String,
        time: i32,
        positions: Vec<CobPos>,
        #[serde(skip_serializing_if = "Option::is_none")]
        cob_col: Option<i32>,
    },
    #[serde(rename_all = "camelCase")]
    FixedCard {
        symbol: String,
        time: i32,
        #[serde(skip_serializing_if = "Option::is_none")]
        shovel_time: Option<i32>,
        plant_type: i32,
        position: CardPos,
    },
    #[serde(rename_all = "camelCase")]
    SmartCard {
        symbol: String,
        time: i32,
        plant_type: i32,
        positions: Vec<CardPos>,
    },
    #[serde(rename_all = "camelCase")]
    FixedFodder {
        symbol: String,
        time: i32,
        #[serde(skip_serializing_if = "Option::is_none")]
        shovel_time: Option<i32>,
        fodders: Vec<Fodder>,
        positions: Vec<CardPos>,
    },
    #[serde(rename_all = "camelCase")]
    SmartFodder {
        symbol: String,
        time: i32,
        #[serde(skip_serializing_if = "Option::is_none")]
        shovel_time: Option<i32>,
        fodders: Vec<Fodder>,
        positions: Vec<CardPos>,
        choose: i32,
        waves: Vec<i32>,
    },
}

impl Action {
    /// Time the action fires (used by the final per-wave stable sort).
    pub fn time(&self) -> i32 {
        match self {
            Action::Cob { time, .. }
            | Action::FixedCard { time, .. }
            | Action::SmartCard { time, .. }
            | Action::FixedFodder { time, .. }
            | Action::SmartFodder { time, .. } => *time,
        }
    }

    pub fn set_time(&mut self, t: i32) {
        match self {
            Action::Cob { time, .. }
            | Action::FixedCard { time, .. }
            | Action::SmartCard { time, .. }
            | Action::FixedFodder { time, .. }
            | Action::SmartFodder { time, .. } => *time = t,
        }
    }
}

/// Header-derived run parameters. Serialized to the per-calculator params JSON;
/// each calculator reads only the keys it cares about. `show_std` is a display
/// toggle (not a sim param), kept separately.
#[derive(Default)]
pub struct Params {
    pub repeat: Option<i32>,
    pub zombies: Option<Vec<i32>>, // types:
    pub target_x: Option<i32>,     // targetPos:
    pub require: Option<Vec<i32>>, // require:
    pub ban: Option<Vec<i32>>,     // ban:
    pub huge: Option<bool>,        // huge:
    pub activate: Option<bool>,    // activate:
    pub dance: Option<bool>,       // dance:
    pub natural: Option<bool>,     // natural:
    pub cob_delay: Option<bool>,   // cobDelay: -> disableCobDelay = !cob_delay
    pub show_std: bool,            // std:
    pub hit_thres: Option<i32>,    // hitThres: (survive calculator; default 1800)
    pub ncobs: Option<i32>,        // ncobs: (reuse calculator)
    pub r#loop: Option<bool>,      // loop: (reuse calculator)
}
