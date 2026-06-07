//! Golden-value tests for the migrated calculators, asserting the CLI reproduces the
//! exact cell values from 万能表_260314.xlsm.

use std::process::Command;

fn run(args: &[&str]) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_puc"))
        .args(args)
        .output()
        .expect("run puc");
    assert!(
        out.status.success(),
        "puc {:?} failed: {}",
        args,
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn ipp_matches_sheet() {
    let o = run(&["ipp", "433", "--wave-len", "601", "--ice", "0"]);
    assert!(o.contains("garg_x=719.94"), "{o}");
    assert!(o.contains("cob_col=7.4125"), "{o}");
    // 后院收二 7.4125~8.55, 收三 7.45~8.4625
    assert!(o.contains("7.4125~8.55"), "{o}");
    assert!(o.contains("7.45~8.4625"), "{o}");
    // 前院收二 7.425~8.5125, 收三 7.5125~8.375
    assert!(o.contains("7.425~8.5125"), "{o}");
    assert!(o.contains("7.5125~8.375"), "{o}");
}

#[test]
fn extreme_slow_matches_sheet() {
    let o = run(&["extreme", "--slow", "755"]);
    assert!(o.contains("coord=760.904"), "{o}");
    assert!(o.contains("two_rows=7.9375"), "{o}");
    assert!(o.contains("back_three=8.025"), "{o}");
    assert!(o.contains("front_three=8.1125"), "{o}");
}

#[test]
fn extreme_fast_matches_sheet() {
    let o = run(&["extreme", "445"]);
    assert!(o.contains("coord=760.755"), "{o}");
    assert!(o.contains("just_safe=7.925"), "{o}");
}

#[test]
fn coord_back_matches_sheet() {
    let o = run(&[
        "coord",
        "685",
        "--wave",
        "normal",
        "--scene",
        "pe",
        "--zombies",
        "regular,gargantuar,pogo,balloon",
    ]);
    // regular: x 656~747, 收上/本/下 left bounds 8.5375 / 8.4375 / 8.45
    assert!(o.contains("656~747"), "{o}");
    assert!(o.contains("8.5375~10"), "{o}");
    assert!(o.contains("8.4375~10"), "{o}");
    assert!(o.contains("8.45~10"), "{o}");
    // gargantuar 718~775, 8.2125 / 8.125
    assert!(o.contains("718~775"), "{o}");
    assert!(o.contains("8.2125~10"), "{o}");
    // pogo 收上 6.3375, 收本 5.5
    assert!(o.contains("6.3375~7.525"), "{o}");
    // balloon 收上 7.625
    assert!(o.contains("7.625~8.8125"), "{o}");
}

#[test]
fn coord_front_matches_sheet() {
    let o = run(&[
        "coord",
        "685",
        "--wave",
        "normal",
        "--scene",
        "de",
        "--zombies",
        "regular,gargantuar",
    ]);
    // front: regular 收上/本/下 = 8.625 / 8.4375 / 8.4875
    assert!(o.contains("8.625~10"), "{o}");
    assert!(o.contains("8.4875~10"), "{o}");
}

#[test]
fn time_back_matches_sheet() {
    let o = run(&[
        "time",
        "pe",
        "cob",
        "2",
        "9",
        "--zombies",
        "regular,gargantuar",
    ]);
    // regular lane1=486~1380 lane2=486~1449; gargantuar lane1=225~1899 lane2=225~1918
    assert!(o.contains("486~1380"), "{o}");
    assert!(o.contains("486~1449"), "{o}");
    assert!(o.contains("225~1899"), "{o}");
    assert!(o.contains("225~1918"), "{o}");
}

#[test]
fn seml_reuse_loop() {
    let o = run(&["seml", "reuse", "tests/fixtures/reuse_loop.seml"]);
    assert!(o.contains("seml reuse ncobs=15 loop=true"), "{o}");
    // First shot reuses exactly at the 3475cs recharge boundary.
    assert!(o.contains("w1 395 -> w3 968: 3475cs"), "{o}");
    // The 2nd cob of the PP at 1251 reuses two cycles later.
    assert!(o.contains("w1 1251 -> w4 623: 3725cs"), "{o}");
    // Loop mode is cyclic: no trailing `next:` line.
    assert!(!o.contains("next:"), "{o}");
}

#[test]
fn seml_reuse_noloop() {
    let o = run(&["seml", "reuse", "tests/fixtures/reuse_noloop.seml"]);
    assert!(o.contains("seml reuse ncobs=4 loop=false"), "{o}");
    // Both cobs of w1's PP reuse at w4's PP, right at the recharge boundary.
    assert_eq!(
        o.matches("w1 1525 -> w4 341: 3475cs").count(),
        2,
        "{o}"
    );
    // Remaining recharge per cannon at sim end (sorted ascending, unclamped).
    assert!(o.contains("next: 1525 1525 3215 3215"), "{o}");
}

#[test]
fn seml_reuse_compact_all_ok() {
    // No reuse falls short of recharge -> compact drops the per-shot lines but keeps
    // the header, "all ok", and the `next:` detail.
    let o = run(&["seml", "--compact", "reuse", "tests/fixtures/reuse_noloop.seml"]);
    assert!(o.contains("seml reuse ncobs=4 loop=false"), "{o}");
    assert!(o.contains("all ok"), "{o}");
    assert!(!o.contains("->"), "{o}");
    assert!(o.contains("next: 1525 1525 3215 3215"), "{o}");
}

#[test]
fn seml_reuse_compact_shows_failures() {
    // ncobs=2 with shots 1000cs apart can't recharge (need 3475) -> only the flagged
    // failing lines are shown, and no "all ok".
    let dir = std::env::temp_dir().join("puc_reuse_tight");
    std::fs::create_dir_all(&dir).unwrap();
    let f = dir.join("tight.seml");
    std::fs::write(&f, "ncobs:2\nloop:true\n\nw 0 1000\nP 100 1 9\nP 200 1 9\n").unwrap();
    let o = run(&["seml", "--compact", "reuse", f.to_str().unwrap()]);
    assert!(o.contains("w1 100 -> w2 100: 1000cs (!)"), "{o}");
    assert!(!o.contains("all ok"), "{o}");
}

#[test]
fn seml_reuse_orders_by_absolute_time() {
    // A delayed cob in w1 (time 1600 > wave_length 1000, abs 1600) fires *after*
    // w2's cob (time 100, abs 1100). FIFO must follow absolute time, so the source
    // is w2 100 and its reuse target is the later w1 1600 shot.
    let dir = std::env::temp_dir().join("puc_reuse_spill");
    std::fs::create_dir_all(&dir).unwrap();
    let f = dir.join("spill.seml");
    std::fs::write(&f, "ncobs:1\n\nw 0 1000\nP 1600 1 9\n\nw 1000\nP 100 1 9\n").unwrap();
    let o = run(&["seml", "reuse", f.to_str().unwrap()]);
    assert!(o.contains("w2 100 -> w1 1600: 500cs (!)"), "{o}");
}
