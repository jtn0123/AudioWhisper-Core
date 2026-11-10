//! Swift's, Foundation's and ICU's text semantics as macOS ships them, for Rust ports of the Mac
//! app's text code that must behave identically: [`swift`] reproduces Swift `Character`s and
//! Foundation's string functions, and [`tables`] holds the Unicode data, generated from macOS, that
//! both they and ICU-compatible pattern matching run on.

pub mod swift;
pub mod tables;
