use zen::lexer::Lexer;
use zen::parser::Parser;
use zen::ast::Expression;

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
#[ignore] // TODO: Parser doesn't fully support comptime blocks yet
fn test_comptime_import_error() {
    // Test that imports inside comptime blocks are rejected by the type checker
    let input = "comptime { core := @std.core }";
    let lexer = Lexer::new(input);
    let mut parser = Parser::new(lexer);
    
    // Parser should accept the syntax (no longer rejects at parse time)
    let result = parser.parse_program();
    assert!(result.is_ok());
    
    // Type checker should reject imports in comptime blocks
    let program = result.unwrap();
    let mut type_checker = zen::typechecker::TypeChecker::new();
    let tc_result = type_checker.check_program(&program);
    
    assert!(tc_result.is_err());
    if let Err(err) = tc_result {
        let err_msg = format!("{:?}", err);
        assert!(err_msg.contains("Import") || 
                err_msg.contains("import") ||
                err_msg.contains("comptime"));
    }
}