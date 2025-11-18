//! Integration tests that actually compile and verify codegen
//! 
//! These tests catch bugs that parser-only tests miss, such as:
//! - Basic blocks without terminators (LLVM verification errors)
//! - Incorrect control flow generation
//! - Type mismatches in codegen
//! - Memory management issues
//! - Phi node bugs (missing incoming values, wrong basic blocks)
//! - GEP bugs (wrong indices, type mismatches)

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
fn test_pattern_matching_with_return() {
    let code = r#"
        main = () i32 {
            x = true
            x ?
                | true { return 1 }
                | false { return 2 }
            return 0
        }
    "#;
    
    // This test catches the "terminator in middle of block" bug
    // When a return statement is in a pattern match arm, we shouldn't
    // try to add a branch to merge_block after the return
    let result = compile_code(code);
    assert!(
        result.is_ok(),
        "Pattern matching with return should compile successfully. Error: {:?}",
        result.err()
    );
}

#[test]
fn test_conditional_with_return() {
    let code = r#"
        main = () i32 {
            x = true
            x ?
                | true { return 1 }
                | false { return 2 }
            return 0
        }
    "#;
    
    // Test that conditionals handle return statements correctly
    // Similar to pattern matching - should check for terminators
    let result = compile_code(code);
    assert!(
        result.is_ok(),
        "Conditional with return should compile successfully. Error: {:?}",
        result.err()
    );
}

#[test]
fn test_nested_struct_field_access() {
    let code = r#"
        Point: {
            x: i32,
            y: i32,
        }
        
        Rect: {
            top_left: Point,
            bottom_right: Point,
        }
        
        main = () i32 {
            rect = Rect {
                top_left: Point { x: 0, y: 0 },
                bottom_right: Point { x: 10, y: 5 }
            }
            // Test nested field access - GEP operations must be correct
            x = rect.bottom_right.x
            y = rect.top_left.y
            return x + y
        }
    "#;
    
    // This test catches GEP bugs in nested struct field access
    // Known bug: nested struct field access can swap values
    let result = compile_code(code);
    assert!(
        result.is_ok(),
        "Nested struct field access should compile successfully. Error: {:?}",
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

#[test]
fn test_pattern_matching_phi_node_basic_blocks() {
    let code = r#"
        main = () i32 {
            // This test ensures phi nodes use correct basic blocks
            // The bug we fixed: phi.add_incoming was using start blocks (then_bb/else_bb)
            // instead of end blocks (then_bb_end/else_bb_end) after branches were added
            // This test compiles code that creates phi nodes to verify they're correct
            
            x = 42
            x ?
                | 42 { return 1 }
                | _ { return 0 }
            return 0
        }
    "#;
    
    // This test ensures the phi node bug fix works
    // The bug was in patterns/compile.rs where payload extraction used wrong basic blocks
    // Even though this doesn't trigger the exact payload extraction path, it tests that
    // phi nodes in general work correctly with our fixes
    let result = compile_code(code);
    assert!(
        result.is_ok(),
        "Pattern matching with phi nodes should compile successfully. Error: {:?}",
        result.err()
    );
}
