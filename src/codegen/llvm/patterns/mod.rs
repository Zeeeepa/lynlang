//! Pattern compilation module
//! Split into submodules by pattern type

mod compile;
mod literal;
mod enum_pattern;
mod struct_pattern;
mod helpers;

// Re-export helpers for use in compile.rs
pub use helpers::*;

