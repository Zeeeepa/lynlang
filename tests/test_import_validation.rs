use zen::lexer::Lexer;
use zen::parser::Parser;
use zen::typechecker::TypeChecker;
use zen::ast::{Declaration, Expression};

#[test]
fn test_module_level_std_imports() {
    // Test that module-level imports work correctly
    let cases = vec![
        "core := @std.core",
        "build := @std.build",
        "io := @std.io",
        "math := @std.math",
    ];
    
    for input in cases {
        let lexer = Lexer::new(input);
        let mut parser = Parser::new(lexer);
        let program = parser.parse_program().unwrap();
        
        // Should parse as ModuleImport
        assert_eq!(program.declarations.len(), 1);
        assert!(matches!(
            program.declarations.first(),
            Some(Declaration::ModuleImport { .. })
        ));
        
        // Should pass type checking
        let mut type_checker = TypeChecker::new();
        assert!(type_checker.check_program(&program).is_ok());
    }
}

#[test]
fn test_build_import_pattern() {
    let input = r#"
build := @std.build
io := build.import("io")
fs := build.import("fs")
"#;
    
    let lexer = Lexer::new(input);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program().unwrap();
    
    // Should have 3 module imports
    assert_eq!(program.declarations.len(), 3);
    
    // All should be ModuleImport declarations
    for decl in &program.declarations {
        assert!(matches!(decl, Declaration::ModuleImport { .. }));
    }
    
    // Check specific imports
    if let Declaration::ModuleImport { alias, module_path } = &program.declarations[0] {
        assert_eq!(alias, "build");
        assert_eq!(module_path, "@std.build");
    }
    
    if let Declaration::ModuleImport { alias, module_path } = &program.declarations[1] {
        assert_eq!(alias, "io");
        assert_eq!(module_path, "std.io");
    }
    
    // Should pass type checking
    let mut type_checker = TypeChecker::new();
    assert!(type_checker.check_program(&program).is_ok());
}

#[test]
fn test_comptime_import_acceptance() {
    // Test that imports inside comptime blocks are now accepted
    let valid_cases = vec![
        "comptime { core := @std.core }",
        "comptime { build := @std.build }",
        r#"comptime { 
            io := @std.io
            math := @std.math
        }"#,
        r#"comptime {
            build := @std.build
        }"#,
    ];
    
    for input in valid_cases {
        let lexer = Lexer::new(input);
        let mut parser = Parser::new(lexer);
        
        // Parser should now accept imports in comptime blocks
        let result = parser.parse_program();
        assert!(result.is_ok(), "Parser should accept imports in comptime blocks: {}", input);
    }
}

#[test]
fn test_nested_comptime_import_acceptance() {
    // Test that nested comptime blocks with imports are now accepted
    let input = r#"
comptime {
    x := 42
    comptime {
        core := @std.core
    }
}
"#;
    
    let lexer = Lexer::new(input);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program().unwrap();
    
    // Type checker should now accept nested imports
    let mut type_checker = TypeChecker::new();
    let result = type_checker.check_program(&program);
    
    // Should pass now that imports are allowed in comptime
    assert!(result.is_ok(), "Nested comptime imports should be accepted");
}

#[test]
fn test_valid_comptime_without_imports() {
    // Test that comptime blocks without imports are still valid
    let valid_cases = vec![
        "comptime { x := 42 }",
        "comptime { result := 10 * 20 }",
        r#"comptime {
            a := 1
            b := 2
            c := a + b
        }"#,
    ];
    
    for input in valid_cases {
        let lexer = Lexer::new(input);
        let mut parser = Parser::new(lexer);
        let program = parser.parse_program().unwrap();
        
        // Should parse as ComptimeBlock
        assert_eq!(program.declarations.len(), 1);
        assert!(matches!(
            program.declarations.first(),
            Some(Declaration::ComptimeBlock(_))
        ));
        
        // Should pass type checking (no imports)
        let mut type_checker = TypeChecker::new();
        assert!(
            type_checker.check_program(&program).is_ok(),
            "Valid comptime block rejected: {}",
            input
        );
    }
}

#[test]
fn test_mixed_declarations() {
    // Test files with both imports and other declarations
    let input = r#"
// Module imports at top level
core := @std.core
build := @std.build
io := build.import("io")

// Function declaration
main = () i32 {
    io.print("Hello, World!\n")
    return 0
}

// Valid comptime block (no imports)
comptime {
    VERSION := 1
}
"#;
    
    let lexer = Lexer::new(input);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program().unwrap();
    
    // Should have multiple declarations
    assert!(program.declarations.len() >= 3);
    
    // First declarations should be imports
    assert!(matches!(
        &program.declarations[0],
        Declaration::ModuleImport { .. }
    ));
    
    // Should pass type checking
    let mut type_checker = TypeChecker::new();
    assert!(type_checker.check_program(&program).is_ok());
}

#[test]
fn test_import_expression_detection() {
    // Test the helper function that detects import expressions
    let checker = TypeChecker::new();
    
    // These should be detected as import expressions
    let import_exprs = vec![
        Expression::Identifier("@std".to_string()),
        Expression::Identifier("@std.core".to_string()),
        Expression::MemberAccess {
            object: Box::new(Expression::Identifier("@std".to_string())),
            member: "core".to_string(),
        },
        Expression::MemberAccess {
            object: Box::new(Expression::Identifier("build".to_string())),
            member: "import".to_string(),
        },
        Expression::FunctionCall {
            name: "build.import".to_string(),
            args: vec![Expression::String("io".to_string())],
        },
    ];
    
    // Validate using the contains_import_expression helper
    use zen::typechecker::validation::contains_import_expression;
    for expr in import_exprs {
        assert!(
            contains_import_expression(&expr),
            "Failed to detect import expression: {:?}",
            expr
        );
    }
    
    // These should NOT be detected as import expressions
    let non_import_exprs = vec![
        Expression::Integer32(42),
        Expression::String("hello".to_string()),
        Expression::Identifier("foo".to_string()),
        Expression::MemberAccess {
            object: Box::new(Expression::Identifier("obj".to_string())),
            member: "field".to_string(),
        },
    ];
    
    for expr in non_import_exprs {
        assert!(
            !contains_import_expression(&expr),
            "Incorrectly detected as import expression: {:?}",
            expr
        );
    }
}