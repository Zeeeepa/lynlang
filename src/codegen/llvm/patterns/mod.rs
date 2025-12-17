//! Pattern compilation module
//! Split into submodules by pattern type

mod compile;
mod enum_pattern;
mod helpers;
mod literal;
mod struct_pattern;

// Helpers are used by compile.rs via super::helpers
#[allow(unused_imports)]
pub use helpers::*;
