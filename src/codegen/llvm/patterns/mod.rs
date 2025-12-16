//! Pattern compilation module
//! Split into submodules by pattern type

mod compile;
mod literal;
mod enum_pattern;
mod struct_pattern;
mod helpers;

// Helpers are used by compile.rs via super::helpers
#[allow(unused_imports)]
pub use helpers::*;

