//! Golden-value tests for the migrated calculators, asserting the CLI reproduces the
//! exact cell values from 万能表_260314.xlsm.

use std::process::Command;

fn run(args: &[&str]) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_puc"))
        .args(args)
        .output()
        .expect("run puc");
    assert!(out.status.success(), "puc {:?} failed: {}", args, String::from_utf8_lossy(&out.stderr));
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
    let o = run(&["extreme", "slow", "755"]);
    assert!(o.contains("coord=760.904"), "{o}");
    assert!(o.contains("two_rows=7.9375"), "{o}");
    assert!(o.contains("back_three=8.025"), "{o}");
    assert!(o.contains("front_three=8.1125"), "{o}");
}

#[test]
fn extreme_fast_matches_sheet() {
    let o = run(&["extreme", "fast", "445"]);
    assert!(o.contains("garg_coord=760.755"), "{o}");
    assert!(o.contains("just_safe_col=7.925"), "{o}");
}

#[test]
fn coord_back_matches_sheet() {
    let o = run(&["coord", "685", "--wave", "normal", "--scene", "pe", "--zombies", "regular,gargantuar,pogo,balloon"]);
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
    let o = run(&["coord", "685", "--wave", "normal", "--scene", "de", "--zombies", "regular,gargantuar"]);
    // front: regular 收上/本/下 = 8.625 / 8.4375 / 8.4875
    assert!(o.contains("8.625~10"), "{o}");
    assert!(o.contains("8.4875~10"), "{o}");
}

#[test]
fn time_back_matches_sheet() {
    let o = run(&["time", "pe", "cob", "2", "9", "--zombies", "regular,gargantuar"]);
    // regular lane1=486~1380 lane2=486~1449; gargantuar lane1=225~1899 lane2=225~1918
    assert!(o.contains("486~1380"), "{o}");
    assert!(o.contains("486~1449"), "{o}");
    assert!(o.contains("225~1899"), "{o}");
    assert!(o.contains("225~1918"), "{o}");
}
