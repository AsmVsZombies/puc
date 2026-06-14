//! Zombie type tables ported from `../seml/src/zombie_types.ts`, plus the
//! id→display-name map from `vendor/seml/refresh/name.h`. Zombie types are the
//! integer enum values the reader/emulator use.

// Enum values (see vendor/lib/object/zombie.h).
pub const ZOMBIE: i32 = 0x0;
pub const FLAG: i32 = 0x1;
pub const CONEHEAD: i32 = 0x2;
pub const POLE_VAULTING: i32 = 0x3;
pub const BUCKETHEAD: i32 = 0x4;
pub const NEWSPAPER: i32 = 0x5;
pub const SCREENDOOR: i32 = 0x6;
pub const FOOTBALL: i32 = 0x7;
pub const DANCING: i32 = 0x8;
pub const BACKUP_DANCER: i32 = 0x9;
pub const DUCKY_TUBE: i32 = 0xa;
pub const SNORKEL: i32 = 0xb;
pub const ZOMBONI: i32 = 0xc;
pub const DOLPHIN_RIDER: i32 = 0xe;
pub const JACK_IN_THE_BOX: i32 = 0xf;
pub const BALLOON: i32 = 0x10;
pub const DIGGER: i32 = 0x11;
pub const POGO: i32 = 0x12;
pub const YETI: i32 = 0x13;
pub const BUNGEE: i32 = 0x14;
pub const LADDER: i32 = 0x15;
pub const CATAPULT: i32 = 0x16;
pub const GARGANTUAR: i32 = 0x17;
pub const IMP: i32 = 0x18;
pub const GIGA_GARGANTUAR: i32 = 0x20;

// CN single-char abbreviation -> enum.
//
// Diverges from upstream `zombie_types.ts`, which aliased 报/news to BUCKETHEAD;
// here they map to NEWSPAPER (the glyph name.h assigns to that type), so the
// abbreviation matches the display name and round-trips.
pub const CN_ABBR: &[(&str, i32)] = &[
    ("普", ZOMBIE),
    ("旗", FLAG),
    ("障", CONEHEAD),
    ("杆", POLE_VAULTING),
    ("桶", BUCKETHEAD),
    ("报", NEWSPAPER),
    ("门", SCREENDOOR),
    ("橄", FOOTBALL),
    ("舞", DANCING),
    ("伴", BACKUP_DANCER),
    ("鸭", DUCKY_TUBE),
    ("潜", SNORKEL),
    ("车", ZOMBONI),
    ("豚", DOLPHIN_RIDER),
    ("丑", JACK_IN_THE_BOX),
    ("气", BALLOON),
    ("矿", DIGGER),
    ("跳", POGO),
    ("雪", YETI),
    ("偷", BUNGEE),
    ("梯", LADDER),
    ("篮", CATAPULT),
    ("白", GARGANTUAR),
    ("鬼", IMP),
    ("红", GIGA_GARGANTUAR),
];

// EN four-char abbreviation -> enum.
pub const EN_ABBR: &[(&str, i32)] = &[
    ("regu", ZOMBIE),
    ("flag", FLAG),
    ("cone", CONEHEAD),
    ("pole", POLE_VAULTING),
    ("buck", BUCKETHEAD),
    ("news", NEWSPAPER),
    ("scre", SCREENDOOR),
    ("foot", FOOTBALL),
    ("danc", DANCING),
    ("back", BACKUP_DANCER),
    ("duck", DUCKY_TUBE),
    ("snor", SNORKEL),
    ("zomb", ZOMBONI),
    ("dolp", DOLPHIN_RIDER),
    ("jack", JACK_IN_THE_BOX),
    ("ball", BALLOON),
    ("digg", DIGGER),
    ("pogo", POGO),
    ("yeti", YETI),
    ("bung", BUNGEE),
    ("ladd", LADDER),
    ("cata", CATAPULT),
    ("garg", GARGANTUAR),
    ("imp", IMP),
    ("giga", GIGA_GARGANTUAR),
];

pub const ACCEPTABLE: &[i32] = &[
    POLE_VAULTING,
    BUCKETHEAD,
    NEWSPAPER,
    SCREENDOOR,
    FOOTBALL,
    DANCING,
    SNORKEL,
    ZOMBONI,
    DOLPHIN_RIDER,
    JACK_IN_THE_BOX,
    BALLOON,
    DIGGER,
    POGO,
    BUNGEE,
    LADDER,
    CATAPULT,
    GARGANTUAR,
    GIGA_GARGANTUAR,
];

/// Banned zombie types per *original* scene code.
pub fn banned(scene: &str) -> &'static [i32] {
    match scene {
        "DE" => &[SNORKEL, DOLPHIN_RIDER],
        "NE" => &[SNORKEL, ZOMBONI, DOLPHIN_RIDER],
        "RE" | "ME" => &[DANCING, SNORKEL, DOLPHIN_RIDER, DIGGER],
        _ => &[], // PE, FE
    }
}

pub fn cn_lookup(abbr: &str) -> Option<i32> {
    CN_ABBR.iter().find(|(k, _)| *k == abbr).map(|(_, v)| *v)
}

pub fn en_lookup(abbr: &str) -> Option<i32> {
    EN_ABBR.iter().find(|(k, _)| *k == abbr).map(|(_, v)| *v)
}

pub fn en_keys() -> Vec<&'static str> {
    EN_ABBR.iter().map(|(k, _)| *k).collect()
}

// id -> display name (CN single char), from refresh/name.h. Falls back to the
// numeric id for types name.h omits (flag/backup/duck/yeti/imp).
pub fn name(id: i32) -> String {
    let n = match id {
        ZOMBIE => "普",
        CONEHEAD => "障",
        POLE_VAULTING => "杆",
        BUCKETHEAD => "桶",
        NEWSPAPER => "报",
        SCREENDOOR => "门",
        FOOTBALL => "橄",
        DANCING => "舞",
        SNORKEL => "潜",
        ZOMBONI => "车",
        DOLPHIN_RIDER => "豚",
        JACK_IN_THE_BOX => "丑",
        BALLOON => "气",
        DIGGER => "矿",
        POGO => "跳",
        BUNGEE => "偷",
        LADDER => "梯",
        CATAPULT => "篮",
        GARGANTUAR => "白",
        GIGA_GARGANTUAR => "红",
        other => return other.to_string(),
    };
    n.to_string()
}

/// Locale-aware display name for the clean-table output (`format.rs`). The `zh`
/// locale reproduces the vendor single-char glyphs of [`name`]; `en` uses the
/// 4-char abbreviations. Types without a glyph fall back to the numeric id.
///
/// CSV export does NOT use this — it always calls [`name`] for byte-fidelity
/// with the vendor C++ output.
pub fn name_i18n(id: i32) -> String {
    let key = match id {
        ZOMBIE => "zname_zombie",
        CONEHEAD => "zname_conehead",
        POLE_VAULTING => "zname_pole_vaulting",
        BUCKETHEAD => "zname_buckethead",
        NEWSPAPER => "zname_newspaper",
        SCREENDOOR => "zname_screendoor",
        FOOTBALL => "zname_football",
        DANCING => "zname_dancing",
        SNORKEL => "zname_snorkel",
        ZOMBONI => "zname_zomboni",
        DOLPHIN_RIDER => "zname_dolphin_rider",
        JACK_IN_THE_BOX => "zname_jack_in_the_box",
        BALLOON => "zname_balloon",
        DIGGER => "zname_digger",
        POGO => "zname_pogo",
        BUNGEE => "zname_bungee",
        LADDER => "zname_ladder",
        CATAPULT => "zname_catapult",
        GARGANTUAR => "zname_gargantuar",
        GIGA_GARGANTUAR => "zname_giga_gargantuar",
        other => return other.to_string(),
    };
    t!(key).to_string()
}
