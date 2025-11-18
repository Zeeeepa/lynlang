//! Integration tests that actually compile and verify codegen
//! 
//! These tests catch bugs that parser-only tests miss, such as:
//! - Basic blocks without terminators (LLVM verification errors)
//! - Incorrect control flow generation
//! - Type mismatches in codegen
//! - Memory management issues

use zen::compiler::Compiler;
use zen::error::CompileError;
use zen::lexer::Lexer;
use zen::parser::Parser;
use inkwell::context::Context;

/// Compile Zen code and verify it generates valid LLVM IR
fn compile_code(code: &str) -> Result<(), CompileError> {
    let context = Context::create();
    let compiler = Compiler::new(&context);
    
    // Parse the source
    let lexer = Lexer::new(code);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program()?;
    
    // Compile to LLVM IR (this will catch codegen bugs like missing terminators)
    compiler.get_module(&program)?;
    
    Ok(())
}

#[test]
fn test_pattern_matching_compiles() {
    let code = r#"
        main = () i32 {
            age = 20
            age > 18 ?
                | true {
                    // true branch
                }
                | false {
                    // false branch
                }
            return 0
        }
    "#;
    
    // This test would have caught the "basic block without terminator" bug
    // The bug was: arm_blocks were created but never properly terminated,
    // causing LLVM verification errors
    let result = compile_code(code);
    assert!(
        result.is_ok(),
        "Pattern matching should compile successfully. Error: {:?}",
        result.err()
    );
}

#[test]
fn test_multiple_pattern_arms_compiles() {
    let code = r#"
        main = () i32 {
            x = 5
            x ?
                | 1 { }
                | 2 { }
                | 3 { }
                | _ { }
            return 0
        }
    "#;
    
    let result = compile_code(code);
    assert!(
        result.is_ok(),
        "Multiple pattern arms should compile successfully. Error: {:?}",
        result.err()
    );
}

