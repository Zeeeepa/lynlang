use zen::lexer::Lexer;
use zen::parser::Parser;
use zen::ast::{Statement, Expression, VariableDeclarationType};

#[test]
fn test_parse_std_namespace() {
    let input = "@std";
    let lexer = Lexer::new(input);
    let mut parser = Parser::new(lexer);
    let expr = parser.parse_expression().unwrap();
    
    assert!(matches!(expr, Expression::Identifier(name) if name == "@std"));
}

#[test]
fn test_parse_std_core() {
    let input = "@std.core";
    let lexer = Lexer::new(input);
    let mut parser = Parser::new(lexer);
    let expr = parser.parse_expression().unwrap();
    
    if let Expression::MemberAccess { object, member } = expr {
        assert!(matches!(*object, Expression::Identifier(name) if name == "@std"));
        assert_eq!(member, "core");
    } else {
        panic!("Expected MemberAccess expression");
    }
}

#[test]
fn test_parse_std_build() {
    let input = "@std.build";
    let lexer = Lexer::new(input);
    let mut parser = Parser::new(lexer);
    let expr = parser.parse_expression().unwrap();
    
    if let Expression::MemberAccess { object, member } = expr {
        assert!(matches!(*object, Expression::Identifier(name) if name == "@std"));
        assert_eq!(member, "build");
    } else {
        panic!("Expected MemberAccess expression");
    }
}

#[test]
fn test_module_level_std_import() {
    let input = "core := @std.core";
    let lexer = Lexer::new(input);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program().unwrap();
    
    assert_eq!(program.declarations.len(), 1);
    
    if let Some(zen::ast::Declaration::ModuleImport { alias, module_path }) = program.declarations.first() {
        assert_eq!(alias, "core");
        assert_eq!(module_path, "@std.core");
    } else {
        panic!("Expected ModuleImport declaration");
    }
}

#[test]
fn test_module_level_build_import() {
    let input = r#"build := @std.build
io := build.import("io")"#;
    
    let lexer = Lexer::new(input);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program().unwrap();
    
    assert_eq!(program.declarations.len(), 2);
    
    // Check first declaration: build := @std.build
    if let Some(zen::ast::Declaration::ModuleImport { alias, module_path }) = program.declarations.first() {
        assert_eq!(alias, "build");
        assert_eq!(module_path, "@std.build");
    } else {
        panic!("Expected ModuleImport declaration for build");
    }
    
    // Check second declaration: io := build.import("io")
    if let Some(zen::ast::Declaration::ModuleImport { alias, module_path }) = program.declarations.get(1) {
        assert_eq!(alias, "io");
        assert_eq!(module_path, "std.io");
    } else {
        panic!("Expected ModuleImport declaration for io");
    }
}

#[test]
fn test_typecheck_std_module() {
    let input = "core := @std.core";
    let lexer = Lexer::new(input);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program().unwrap();
    
    // Type check the program
    let mut type_checker = zen::typechecker::TypeChecker::new();
    let result = type_checker.check_program(&program);
    assert!(result.is_ok());
}

#[test]
fn test_comptime_import_error() {
    // Test that imports inside comptime blocks are rejected by the parser
    let input = "comptime { core := @std.core }";
    let lexer = Lexer::new(input);
    let mut parser = Parser::new(lexer);
    
    // Parser should reject imports in comptime blocks
    let result = parser.parse_program();
    
    // Should error with appropriate message
    assert!(result.is_err());
    if let Err(err) = result {
        let err_msg = format!("{:?}", err);
        assert!(err_msg.contains("Import statements are not allowed inside comptime blocks") || 
                err_msg.contains("imports to module level"));
    }
}