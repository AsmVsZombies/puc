//! Integration tests for `puc seml … --csv`: the export lands as a UTF-8-BOM
//! file (named per the `open_csv` convention for directory targets), the
//! aligned-text table is still printed, and absent `--csv` produces no file.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// A unique scratch directory under the system temp dir, removed on drop.
struct TmpDir(PathBuf);

impl TmpDir {
    fn new(tag: &str) -> Self {
        static SEQ: AtomicU32 = AtomicU32::new(0);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let seq = SEQ.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("puc_csv_{tag}_{nanos}_{seq}"));
        std::fs::create_dir_all(&dir).unwrap();
        TmpDir(dir)
    }
    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TmpDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

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

fn fixture(name: &str) -> String {
    format!("{}/tests/fixtures/{}", env!("CARGO_MANIFEST_DIR"), name)
}

/// Reads a CSV export, asserting the UTF-8 BOM and returning the body after it.
fn read_csv_body(path: &Path) -> String {
    let bytes = std::fs::read(path).unwrap();
    assert_eq!(&bytes[..3], b"\xEF\xBB\xBF", "missing UTF-8 BOM: {path:?}");
    String::from_utf8(bytes[3..].to_vec()).unwrap()
}

#[test]
fn directory_target_uses_naming_convention() {
    let dir = TmpDir::new("pos");
    let stdout = run(&[
        "seml",
        "pos",
        &fixture("pos.seml"),
        "--csv",
        dir.path().to_str().unwrap(),
    ]);

    // The aligned-text table is still printed (keep-both).
    assert!(stdout.contains("seml pos"), "text table missing: {stdout}");
    assert!(stdout.contains("CSV written to"), "no CSV notice: {stdout}");

    // Exactly one file, named "<stem> (<timestamp>) .csv".
    let entries: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    assert_eq!(entries.len(), 1, "expected one CSV, got {entries:?}");
    let name = &entries[0];
    assert!(name.starts_with("pos ("), "bad name: {name}");
    assert!(name.ends_with(") .csv"), "bad name: {name}");

    let body = read_csv_body(&dir.path().join(name));
    assert!(body.starts_with(",900\n"), "row1 mismatch: {body:?}");
    assert!(body.contains("僵尸类别,红\n"), "header row missing");
    assert!(body.contains("\n累积概率,\n"), "cumulative marker missing");
}

#[test]
fn file_target_written_verbatim() {
    let dir = TmpDir::new("pogo");
    let target = dir.path().join("out.csv");
    run(&[
        "seml",
        "pogo",
        &fixture("pogo.seml"),
        "--csv",
        target.to_str().unwrap(),
    ]);

    assert!(target.exists(), "literal-file target not written");
    let body = read_csv_body(&target);
    // Header matches the reference driver exactly (including the trailing comma).
    assert!(
        body.starts_with("时刻,收上行跳跳左,右,收本行跳跳左,右,收下行跳跳左,右,\n"),
        "pogo header mismatch: {:?}",
        body.lines().next()
    );
    // Each data row has 7 comma-terminated cells (tick + 3 ranges × min/max).
    let data = body.lines().nth(1).unwrap();
    assert_eq!(data.matches(',').count(), 7, "row width: {data}");
}

#[test]
fn survive_table_and_csv() {
    let dir = TmpDir::new("survive");
    let stdout = run(&[
        "seml",
        "survive",
        &fixture("survive.seml"),
        "--csv",
        dir.path().to_str().unwrap(),
    ]);

    // Header echoes the (defaulted) hit threshold; the auto-derived type set
    // includes 白/红 (gargs) but never 鸭 (ducky tube, always excluded).
    assert!(stdout.contains("seml survive"), "header missing: {stdout}");
    assert!(stdout.contains("hitThres=1800"), "no hitThres: {stdout}");
    assert!(stdout.contains("白") && stdout.contains("红"), "garg rows missing");
    assert!(!stdout.contains("鸭"), "ducky tube should be excluded: {stdout}");
    assert!(!stdout.contains("偷"), "bungee should be excluded: {stdout}");

    // Every printed 受击率 cell is a percentage in [0, 100].
    for line in stdout.lines().filter(|l| l.trim_start().starts_with("1 ")) {
        let pct = line
            .split_whitespace()
            .find_map(|t| t.strip_suffix('%'))
            .and_then(|n| n.parse::<f64>().ok())
            .unwrap_or_else(|| panic!("no rate in row: {line}"));
        assert!((0.0..=100.0).contains(&pct), "rate out of range: {line}");
    }

    let entries: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    assert_eq!(entries.len(), 1, "expected one CSV, got {entries:?}");
    let body = read_csv_body(&dir.path().join(&entries[0]));
    assert!(body.contains("受击率,"), "csv 受击率 row missing: {body}");
    assert!(body.contains("未受击均血,"), "csv 未受击均血 row missing: {body}");
}

#[test]
fn absent_csv_writes_no_file() {
    let dir = TmpDir::new("none");
    let stdout = run(&["seml", "smash", &fixture("smash.seml")]);
    assert!(!stdout.contains("CSV written to"), "unexpected CSV notice");
    // The flag default is off — nothing should have been created anywhere we own.
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}
