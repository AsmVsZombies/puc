pub use std::cmp::{min, max};
pub use std::collections::{HashMap, HashSet};
pub use std::str::FromStr;
pub use strum::IntoEnumIterator;

#[derive(strum::EnumString, strum::EnumIter, PartialEq, Eq, Hash, Copy, Clone, Debug)]
#[strum(serialize_all = "PascalCase", ascii_case_insensitive)]
pub enum ZombieType {
    Regular,
    DCFast,
    DCSlow,
    Flag,
    Conehead,
    PoleVaulting,
    Buckethead,
    Newspaper,
    ScreenDoor,
    Football,
    Dancing,
    Snorkel,
    Zomboni,
    DolphinRider,
    #[strum(serialize = "JackInTheBox", serialize = "Jack-in-the-box")]
    JackInTheBox,
    Balloon,
    Digger,
    Pogo,
    Ladder,
    Catapult,
    Gargantuar,
    #[strum(serialize = "Giga", serialize = "GigaGargantuar")]
    GigaGargantuar,
}

pub type Num = num_rational::Rational64;

pub enum MovementType {
    Constant,
    Animation(Vec<Num>),
    Regular(Vec<Num>, Vec<Num>),
    DanceCheat,
    Dancing(Vec<Num>),
    Zomboni,
}

pub struct ZombieData {
    pub spawn: (i64, i64),
    pub spawn_hugewave: (i64, i64),
    pub movement_type: MovementType,
    pub speed: (Num, Num),
    pub freeze_immune: bool,
    pub chill_immune: bool,
    pub def_x: (i32, i32),
    pub def_y: (i32, i32),
    pub atk: (i32, i32),
    pub hp: i32,
    pub summon_weight_normal: u32,
    pub summon_weight_hugewave: u32,
    pub if_generate_in: (bool, bool),
    pub if_generate_in_wave1to5: (bool, bool),
    // Metadata used by the coord/time/extreme/ipp calculators (from 万能表).
    pub pos_col: String,            // column name in natural_fast/slow.csv
    pub dmg_range: (i32, i32),      // leftmost / rightmost zombie x still fully damageable
    pub min_cs_normal: Option<i32>, // earliest valid lookup cs, normal wave
    pub min_cs_flag: Option<i32>,   // earliest valid lookup cs, flag wave
    pub coord_h: i32,               // 僵尸h (vertical render offset used by landing geometry)
    pub flag_offset: i32,           // +x added in flag wave (0 if a dedicated *_flag column exists)
}

pub struct PosDistribution {
    pub dist: [f64; 880],
    pub min: f64,
    pub max: f64,
}
