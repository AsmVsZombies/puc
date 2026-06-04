#[macro_use]
mod output;

pub mod calc;
mod constants;
mod game;
pub mod lang;
#[cfg(feature = "mcp")]
pub mod mcp;
pub mod parser;
mod printer;
pub mod seml;
pub mod tables;
pub mod zmc;

pub use output::capture;
