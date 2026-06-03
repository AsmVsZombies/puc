#pragma once

// C ABI for the PvZ emulator FFI boundary. The Rust `pvz-emulator-sys` crate
// links against exactly these two symbols; everything else (the Config schema,
// per-calculator RunParams, threading, RNG) lives behind them in C++.

#ifdef __cplusplus
extern "C" {
#endif

// Runs one calculator end-to-end.
//
//   calculator    : "pos" | "smash" | "explode" | "refresh" | "pogo"
//   scenario_json : Config schema (setting + waves + actions) — read_config input
//   params_json   : RunParams for that calculator (repeat, zombies, flags, ...)
//   out_json      : receives a malloc'd, NUL-terminated UTF-8 JSON string. On
//                   success it is the result table; on failure it is
//                   {"error":"..."}. Always set (never left dangling) when the
//                   call returns. The caller owns it and must puc_free it.
//
// Returns 0 on success, non-zero on failure (out_json still holds error JSON).
int puc_run(const char* calculator, const char* scenario_json, const char* params_json,
    char** out_json);

// Frees a string previously returned via out_json. Safe to call on NULL.
void puc_free(char* p);

#ifdef __cplusplus
}
#endif
