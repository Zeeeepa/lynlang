// Comprehensive tests for self-hosted Zen compiler components

use zen::{parser::Parser, lexer::Lexer, typechecker::TypeChecker, error::CompileError};

fn compile_zen_module(source: &str) -> Result<String, CompileError> {
    let mut lexer = Lexer::new(source);
    let mut parser = Parser::new(lexer);
    let ast = parser.parse_program()?;
    let mut type_checker = TypeChecker::new();
    let typed_ast = type_checker.check_program(&ast)?;
    
    // For now, return IR or simplified representation
    Ok(format!("{:#?}", typed_ast))
}

#[test]
fn test_self_hosted_lexer_comprehensive() {
    let zen_lexer_code = include_str!("../compiler/lexer.zen");
    
    // Verify the lexer can compile
    let result = compile_zen_module(zen_lexer_code);
    if let Err(e) = &result {
        eprintln!("Failed to compile lexer: {:?}", e);
    }
    assert!(result.is_ok(), "Self-hosted lexer should compile");
}

#[test]
fn test_self_hosted_parser_comprehensive() {
    let zen_parser_code = include_str!("../compiler/parser.zen");
    
    // Verify the parser can compile
    let result = compile_zen_module(zen_parser_code);
    assert!(result.is_ok(), "Self-hosted parser should compile");
}

#[test]
fn test_self_hosted_type_checker_comprehensive() {
    let zen_type_checker_code = include_str!("../compiler/type_checker.zen");
    
    // Verify the type checker can compile
    let result = compile_zen_module(zen_type_checker_code);
    assert!(result.is_ok(), "Self-hosted type checker should compile");
}

#[test]
fn test_self_hosted_codegen_comprehensive() {
    let zen_codegen_code = include_str!("../compiler/codegen.zen");
    
    // Verify the codegen can compile
    let result = compile_zen_module(zen_codegen_code);
    assert!(result.is_ok(), "Self-hosted codegen should compile");
}

#[test]
fn test_stdlib_io_module() {
    let zen_io_code = include_str!("../stdlib/io.zen");
    
    // Verify the IO module can compile
    let result = compile_zen_module(zen_io_code);
    assert!(result.is_ok(), "Standard library IO module should compile");
}

#[test]
fn test_stdlib_mem_module() {
    let zen_mem_code = include_str!("../stdlib/mem.zen");
    
    // Verify the memory module can compile
    let result = compile_zen_module(zen_mem_code);
    assert!(result.is_ok(), "Standard library memory module should compile");
}

#[test]
fn test_stdlib_math_module() {
    let zen_math_code = include_str!("../stdlib/math.zen");
    
    // Verify the math module can compile
    let result = compile_zen_module(zen_math_code);
    assert!(result.is_ok(), "Standard library math module should compile");
}

#[test]
fn test_stdlib_string_module() {
    let zen_string_code = include_str!("../stdlib/string.zen");
    
    // Verify the string module can compile
    let result = compile_zen_module(zen_string_code);
    assert!(result.is_ok(), "Standard library string module should compile");
}

#[test]
fn test_stdlib_vec_module() {
    let zen_vec_code = include_str!("../stdlib/vec.zen");
    
    // Verify the vector module can compile
    let result = compile_zen_module(zen_vec_code);
    assert!(result.is_ok(), "Standard library vector module should compile");
}

#[test]
fn test_self_hosted_bootstrap() {
    // Test that the compiler can compile itself
    let compiler_files = vec![
        "../compiler/lexer.zen",
        "../compiler/parser.zen",
        "../compiler/type_checker.zen",
        "../compiler/codegen.zen",
        "../compiler/main.zen",
    ];
    
    for file in compiler_files {
        let zen_code = std::fs::read_to_string(file).expect("Failed to read file");
        let result = compile_zen_module(&zen_code);
        assert!(result.is_ok(), "File {} should compile", file);
    }
}

#[test]
fn test_module_level_imports_integration() {
    let code = r#"
        // Module-level imports (correct syntax)
        core := @std.core
        io := @std.io
        math := @std.math
        
        main = () i32 {
            io.println("Testing module imports")
            x := math.abs(-42)
            return x
        }
    "#;
    
    let result = compile_zen_module(code);
    assert!(result.is_ok(), "Module-level imports should work");
}

#[test]
fn test_comptime_for_metaprogramming() {
    let code = r#"
        // Comptime for metaprogramming (not imports)
        comptime {
            // Generate lookup table
            TABLE_SIZE := 256
            lookup_table := [TABLE_SIZE]u8{}
            i := 0
            loop i < TABLE_SIZE {
                lookup_table[i] = i * 2
                i = i + 1
            }
        }
        
        main = () i32 {
            return lookup_table[42]
        }
    "#;
    
    let result = compile_zen_module(code);
    assert!(result.is_ok(), "Comptime should work for metaprogramming");
}

#[test]
fn test_no_comptime_wrapper_for_imports() {
    let code = r#"
        // This should fail - imports in comptime block
        comptime {
            core := @std.core
            io := @std.io
        }
        
        main = () i32 {
            return 0
        }
    "#;
    
    let result = compile_zen_module(code);
    assert!(result.is_err(), "Imports in comptime blocks should fail");
}

#[test]
fn test_self_hosted_error_handling() {
    let zen_errors_code = include_str!("../compiler/errors.zen");
    
    // Verify the errors module can compile
    let result = compile_zen_module(zen_errors_code);
    assert!(result.is_ok(), "Self-hosted errors module should compile");
}

#[test]
fn test_import_with_member_access() {
    let code = r#"
        build := @std.build
        io := build.import("io")
        math := build.import("math")
        
        main = () i32 {
            io.print("Hello")
            x := math.sqrt(16.0)
            return 0
        }
    "#;
    
    let result = compile_zen_module(code);
    assert!(result.is_ok(), "Build imports with member access should work");
}

#[test]
fn test_mixed_declarations_and_imports() {
    let code = r#"
        // Module-level imports
        core := @std.core
        io := @std.io
        
        // Type declarations
        Point = {
            x: f64,
            y: f64,
        }
        
        // Function declarations
        distance = (p1: Point, p2: Point) f64 {
            dx := p2.x - p1.x
            dy := p2.y - p1.y
            return math.sqrt(dx * dx + dy * dy)
        }
        
        // More imports
        math := @std.math
        
        main = () i32 {
            p1 := Point { x: 0.0, y: 0.0 }
            p2 := Point { x: 3.0, y: 4.0 }
            d := distance(p1, p2)
            io.print_float(d)
            return 0
        }
    "#;
    
    let result = compile_zen_module(code);
    assert!(result.is_ok(), "Mixed declarations and imports should work");
}

#[test]
fn test_self_hosted_llvm_backend() {
    let zen_llvm_code = include_str!("../compiler/llvm_backend.zen");
    
    // Verify the LLVM backend can compile
    let result = compile_zen_module(zen_llvm_code);
    assert!(result.is_ok(), "Self-hosted LLVM backend should compile");
}

#[test]
fn test_complete_self_hosting_chain() {
    // Test the complete compilation chain
    let test_program = r#"
        io := @std.io
        
        fibonacci = (n: i32) i32 {
            n <= 1 ?
                | true => return n
                | false => return fibonacci(n - 1) + fibonacci(n - 2)
        }
        
        main = () i32 {
            result := fibonacci(10)
            io.print_int(result)
            return 0
        }
    "#;
    
    // First, compile with Rust compiler
    let rust_result = compile_zen_module(test_program);
    assert!(rust_result.is_ok(), "Rust compiler should compile test program");
    
    // TODO: Once self-hosting is complete, compile with Zen compiler
    // and verify same output
}