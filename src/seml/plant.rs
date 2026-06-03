//! The handful of `plant_type` ids the SEML card symbols need (values from
//! `crates/pvz-emulator-sys/vendor/lib/object/plant.h`). Stored as the integer
//! the reader expects for the scenario JSON `plantType` field.

pub const CHERRY_BOMB: i32 = 0x2; // A / A_NUM
pub const DOOMSHROOM: i32 = 0xF; // N
pub const SQUASH: i32 = 0x11; // a / W / a_NUM / W_NUM
pub const JALAPENO: i32 = 0x14; // J / J_NUM
pub const GARLIC: i32 = 0x24; // G
