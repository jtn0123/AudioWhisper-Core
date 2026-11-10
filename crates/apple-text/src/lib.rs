//! Swift's, Foundation's and ICU's text semantics as macOS ships them, for Rust ports of the Mac
//! app's text code that must behave identically: [`swift`] reproduces Swift `Character`s and
//! Foundation's string functions, [`icu`] the ICU regular expressions behind
//! `NSRegularExpression`, and [`tables`] holds the Unicode data, generated from macOS, that they
//! run on.

pub mod icu;
pub mod swift;
pub mod tables;
