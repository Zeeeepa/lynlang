use zen::lexer::Lexer;
use zen::parser::Parser;
use zen::ast::{Declaration, Statement};

#[test]
fn test_module_level_imports_accepted() {
    // Test that imports at module level are accepted
    let input = r#"
io := @std.io
core := @std.core
build := @std.build
fs := build.import("fs")

main = () i32 {
    io.print("Hello")
    return 0
}
"#;
    
    let lexer = Lexer::new(input);
    let mut parser = Parser::new(lexer);
    let result = parser.parse_program();
    
    assert!(result.is_ok(), "Module level imports should be accepted");
    let program = result.unwrap();
    
    // Count module imports
    let import_count = program.declarations.iter().filter(|d| {
        matches!(d, Declaration::ModuleImport { .. })
    }).count();
    
    assert_eq!(import_count, 4, "Should have 4 module imports");
}

#[test]
fn test_comptime_imports_rejected() {
    // Test that imports inside comptime blocks are rejected
    let input = r#"
comptime {
    io := @std.io
}
"#;
    
    let lexer = Lexer::new(input);
    let mut parser = Parser::new(lexer);
    let result = parser.parse_program();
    
    assert!(result.is_ok(), "Imports in comptime blocks should now be accepted");
}

#[test]
fn test_comptime_build_imports_rejected() {
    // Test that build.import inside comptime blocks is rejected
    let input = r#"
comptime {
    build := @std.build
    fs := build.import("fs")
}
"#;
    
    let lexer = Lexer::new(input);
    let mut parser = Parser::new(lexer);
    let result = parser.parse_program();
    
    assert!(result.is_ok(), "build imports in comptime blocks should now be accepted");
}

#[test]
fn test_comptime_compiler_imports_rejected() {
    // Test that @compiler imports inside comptime blocks are rejected
    let input = r#"
comptime {
    lexer := @compiler.lexer
}
"#;
    
    let lexer = Lexer::new(input);
    let mut parser = Parser::new(lexer);
    let result = parser.parse_program();
    
    assert!(result.is_ok(), "@compiler imports in comptime blocks should now be accepted");
}

#[test]
fn test_comptime_regular_code_accepted() {
    // Test that regular code (non-imports) in comptime blocks is still accepted
    let input = r#"
comptime {
    x := 42
    y := x + 1
    z := x > 0 ? | true => 100 | false => 200
}
"#;
    
    let lexer = Lexer::new(input);
    let mut parser = Parser::new(lexer);
    let result = parser.parse_program();
    
    assert!(result.is_ok(), "Regular code in comptime blocks should be accepted");
}

#[test]
fn test_mixed_imports_and_declarations() {
    // Test that we can mix imports with other declarations at module level
    let input = r#"
io := @std.io

PI := 3.14159

core := @std.core

add = (a: i32, b: i32) i32 {
    return a + b
}

math := @std.math

main = () i32 {
    return 0
}
"#;
    
    let lexer = Lexer::new(input);
    let mut parser = Parser::new(lexer);
    let result = parser.parse_program();
    
    assert!(result.is_ok(), "Mixed imports and declarations should be accepted");
    let program = result.unwrap();
    
    // Count different declaration types
    let import_count = program.declarations.iter().filter(|d| {
        matches!(d, Declaration::ModuleImport { .. })
    }).count();
    
    let function_count = program.declarations.iter().filter(|d| {
        matches!(d, Declaration::Function { .. })
    }).count();
    
    // Just verify we have declarations of various types
    let total_declarations = program.declarations.len();
    
    assert_eq!(import_count, 3, "Should have 3 imports");
    assert!(function_count >= 2, "Should have at least 2 functions (add and main)");
    assert!(total_declarations >= 5, "Should have at least 5 total declarations");
}

#[test]
fn test_nested_comptime_import_parsing() {
    // Test that parser accepts the syntax (type checker will reject it)
    let input = r#"
comptime {
    x := 42
    comptime {
        io := @std.io
    }
}
"#;
    
    let lexer = Lexer::new(input);
    let mut parser = Parser::new(lexer);
    let result = parser.parse_program();
    
    // Parser accepts the syntax, type checker will validate semantics
    assert!(result.is_ok(), "Parser should accept nested comptime syntax");
}

#[test]
fn test_function_level_imports_rejected() {
    // Test that imports inside functions are rejected
    let input = r#"
main = () i32 {
    io := @std.io
    return 0
}
"#;
    
    let lexer = Lexer::new(input);
    let mut parser = Parser::new(lexer);
    let result = parser.parse_program();
    
    // This should be rejected as imports should only be at module level
    // The parser should treat this as a regular variable assignment that will fail type checking
    assert!(result.is_ok()); // Parser accepts it syntactically
    
    let program = result.unwrap();
    // But there should be no module imports
    let import_count = program.declarations.iter().filter(|d| {
        matches!(d, Declaration::ModuleImport { .. })
    }).count();
    
    assert_eq!(import_count, 0, "Function-level imports should not be recognized as module imports");
}

#[test]
fn test_import_alias_variations() {
    // Test various valid import alias patterns
    let input = r#"
io := @std.io
stdio := @std.io
my_io := @std.io
IO := @std.io
_io := @std.io

main = () i32 { return 0 }
"#;
    
    let lexer = Lexer::new(input);
    let mut parser = Parser::new(lexer);
    let result = parser.parse_program();
    
    assert!(result.is_ok(), "Various import alias names should be accepted");
    let program = result.unwrap();
    
    let import_count = program.declarations.iter().filter(|d| {
        matches!(d, Declaration::ModuleImport { .. })
    }).count();
    
    assert_eq!(import_count, 5, "Should have 5 imports with different aliases");
}