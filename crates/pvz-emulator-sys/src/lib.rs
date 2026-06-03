//! Safe Rust wrapper over the PvZ emulator FFI shim (`puc_shim.{h,cpp}`).
//!
//! The boundary is JSON-in / JSON-out: a scenario `Config` and a per-calculator
//! `RunParams` go in as JSON strings, an aggregated result table comes back as a
//! JSON string. See `doc/cpp-integration-plan.md` for the schemas.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

extern "C" {
    fn puc_run(
        calculator: *const c_char,
        scenario_json: *const c_char,
        params_json: *const c_char,
        out_json: *mut *mut c_char,
    ) -> i32;

    fn puc_free(p: *mut c_char);
}

/// Runs one calculator (`"pos" | "smash" | "explode" | "refresh" | "pogo"`).
///
/// On success returns the result table as a JSON string. On failure returns the
/// error message extracted from the shim's `{"error": ...}` payload.
pub fn run(calculator: &str, scenario_json: &str, params_json: &str) -> Result<String, String> {
    let calc = CString::new(calculator).map_err(|_| "calculator contains NUL".to_string())?;
    let scenario =
        CString::new(scenario_json).map_err(|_| "scenario JSON contains NUL".to_string())?;
    let params = CString::new(params_json).map_err(|_| "params JSON contains NUL".to_string())?;

    let mut out: *mut c_char = std::ptr::null_mut();
    // SAFETY: the three inputs are valid NUL-terminated C strings for the duration
    // of the call; `out` is a valid pointer-to-pointer. The shim either sets `out`
    // to a malloc'd string we free below, or leaves it null (allocation failure).
    let rc = unsafe { puc_run(calc.as_ptr(), scenario.as_ptr(), params.as_ptr(), &mut out) };

    if out.is_null() {
        return Err(format!("puc_run failed (rc={rc}) without producing output"));
    }

    // SAFETY: `out` is a malloc'd NUL-terminated string owned by us until puc_free.
    let json = unsafe { CStr::from_ptr(out) }.to_string_lossy().into_owned();
    unsafe { puc_free(out) };

    if rc == 0 {
        Ok(json)
    } else {
        Err(extract_error(&json))
    }
}

/// Pulls the `"error"` field out of the shim's failure JSON, falling back to the
/// raw payload if it isn't shaped as expected.
fn extract_error(json: &str) -> String {
    // Minimal extraction without pulling in a JSON dependency: the shim emits
    // exactly {"error":"<escaped msg>"}. Fall back to the raw string otherwise.
    if let Some(rest) = json.split_once("\"error\":").map(|(_, r)| r) {
        let rest = rest.trim_start();
        if let Some(inner) = rest.strip_prefix('"') {
            if let Some(end) = inner.find('"') {
                return unescape_json(&inner[..end]);
            }
        }
    }
    json.to_string()
}

fn unescape_json(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // A minimal valid scenario: PE scene, no waves. Exercises the full FFI round
    // trip (parse → run → serialize) without depending on simulation specifics.
    const EMPTY_SCENARIO: &str = r#"{"setting":{"scene":"PE"},"waves":[]}"#;

    #[test]
    fn unknown_calculator_errors() {
        let err = run("nonsense", EMPTY_SCENARIO, "{}").unwrap_err();
        assert!(err.contains("unknown calculator"), "got: {err}");
    }

    #[test]
    fn invalid_scenario_json_errors() {
        let err = run("pos", "{not json", "{}").unwrap_err();
        assert!(err.contains("invalid JSON"), "got: {err}");
    }

    #[test]
    fn schema_invalid_scenario_errors_without_crashing() {
        // `[]` is valid JSON but not a Config object — must throw, not abort.
        let err = run("pos", "[]", "{}").unwrap_err();
        assert!(err.contains("JSON object"), "got: {err}");
    }

    #[test]
    fn pos_runs_on_empty_scenario() {
        // No zombies + no waves => a trivially-quick run that still produces a table.
        let json = run("pos", EMPTY_SCENARIO, r#"{"repeat":1}"#).unwrap();
        assert!(json.contains("\"calculator\":\"pos\""), "got: {json}");
    }
}
