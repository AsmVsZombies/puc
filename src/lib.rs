#[macro_use]
extern crate rust_i18n;

#[macro_use]
mod output;

// Output strings are loaded from `locales/*.yml` at compile time. The active
// locale is chosen at runtime via `rust_i18n::set_locale` (see `main.rs`); when
// a key is missing in the selected locale it falls back to `zh`.
i18n!("locales", fallback = "zh");

pub mod calc;
mod constants;
mod game;
#[cfg(feature = "mcp")]
pub mod mcp;
pub mod parser;
mod printer;
pub mod seml;
pub mod tables;
pub mod zmc;

pub use output::capture;
