#![allow(dead_code)]

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

pub mod ast;
pub mod behaviors;
pub mod build_system;
pub mod codegen;
pub mod compiler;
pub mod comptime;
pub mod error;
pub mod ffi;
pub mod lexer;
// pub mod lsp;  // Moved to separate binary
pub mod module_system;
pub mod parser;
pub mod stdlib;
pub mod type_system;
pub mod typechecker;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
