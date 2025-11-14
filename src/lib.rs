pub mod ast;
pub mod behaviors;
// NOTE: build_system module removed - build system should be self-hosted in build.zen
// The previous Rust implementation contradicted design goals of self-hosting
pub mod codegen;
pub mod compiler;
pub mod comptime;
pub mod error;
pub mod ffi;
pub mod lexer;
pub mod lsp;
pub mod module_system;
pub mod parser;
pub mod stdlib;
pub mod type_system;
pub mod typechecker;
