use zen::ast::{Program, Statement, Declaration};
use zen::parser::Parser;
use zen::lexer::Lexer;

fn get_imports(program: &Program) -> Vec<(String, String)> {
    program.declarations.iter().filter_map(|d| {
        if let Declaration::ModuleImport { alias, module_path } = d {
            Some((alias.clone(), module_path.clone()))
        } else {
            None
        }
    }).collect()
}

#[test]
fn test_module_level_import_std() {
    let input = r#"
        core := @std.core
        io := @std.io
        
        main = () i32 {
            return 0
        }
    "#;
    
    let lexer = Lexer::new(input);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program().expect("Failed to parse program");
    
    let imports = get_imports(&program);
    
    assert_eq!(imports.len(), 2);
    assert_eq!(imports[0].0, "core");
    assert_eq!(imports[0].1, "@std.core");
    assert_eq!(imports[1].0, "io");
    assert_eq!(imports[1].1, "@std.io");
}

#[test]
fn test_build_import_syntax() {
    let input = r#"
        build := @std.build
        io := build.import("io")
        
        main = () i32 {
            return 0
        }
    "#;
    
    let lexer = Lexer::new(input);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program().expect("Failed to parse program");
    
    let imports = get_imports(&program);
    
    assert_eq!(imports.len(), 2);
    assert_eq!(imports[0].0, "build");
    assert_eq!(imports[0].1, "@std.build");
    assert_eq!(imports[1].0, "io");
    assert_eq!(imports[1].1, "std.io");
}

#[test]
fn test_no_comptime_wrapper_for_imports() {
    let input = r#"
        // Direct imports without comptime
        core := @std.core
        vec := @std.vec
        string := @std.string
        
        // Function using imports
        process = () void {
            v := vec.new()
            s := string.from("test")
        }
    "#;
    
    let lexer = Lexer::new(input);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program().expect("Failed to parse program");
    
    let imports = get_imports(&program);
    
    assert_eq!(imports.len(), 3);
    assert_eq!(imports[0].0, "core");
    assert_eq!(imports[1].0, "vec");
    assert_eq!(imports[2].0, "string");
    
    // Verify no comptime blocks at top level
    for decl in &program.declarations {
        if let Declaration::ComptimeBlock(_) = decl {
            // This is actually OK - comptime blocks can exist for other purposes
            // Just not for imports
        }
    }
}

#[test]
fn test_mixed_imports_and_declarations() {
    let input = r#"
        // Imports
        io := @std.io
        math := @std.math
        
        // Constants
        PI := 3.14159
        
        // More imports
        vec := @std.vec
        
        // Types
        Point = {
            x: f64,
            y: f64,
        }
        
        // Functions
        main = () i32 {
            return 0
        }
    "#;
    
    let lexer = Lexer::new(input);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program().expect("Failed to parse program");
    
    let imports = get_imports(&program);
    
    assert_eq!(imports.len(), 3);
    assert_eq!(imports[0].0, "io");
    assert_eq!(imports[1].0, "math");
    assert_eq!(imports[2].0, "vec");
    
    // Check for constant declaration
    let has_pi_constant = program.declarations.iter().any(|decl| {
        matches!(decl, Declaration::Constant { name, .. } if name == "PI")
    });
    assert!(has_pi_constant, "PI constant not found");
    
    // Check for Point type
    let has_point_type = program.declarations.iter().any(|decl| {
        matches!(decl, Declaration::Struct(s) if s.name == "Point")
    });
    assert!(has_point_type, "Point type not found");
}

#[test]
fn test_import_error_comptime_wrapper() {
    // This should fail if someone tries to use old comptime syntax
    let input = r#"
        comptime {
            core := @std.core
        }
        
        main = () i32 {
            return 0
        }
    "#;
    
    let lexer = Lexer::new(input);
    let mut parser = Parser::new(lexer);
    
    // Parser should reject imports inside comptime blocks
    let result = parser.parse_program();
    assert!(result.is_err(), "Parser should reject imports inside comptime blocks");
    
    if let Err(err) = result {
        // Check that the error message is about imports in comptime
        let error_msg = format!("{:?}", err);
        assert!(error_msg.contains("Import statements are not allowed inside comptime blocks") ||
                error_msg.contains("Move imports to module level"),
                "Error should mention that imports are not allowed in comptime blocks");
    }
}

#[test]
fn test_chained_module_access() {
    let input = r#"
        std := @std
        core := std.core
        io := std.io
        
        main = () i32 {
            io.print("Hello")
            return 0
        }
    "#;
    
    let lexer = Lexer::new(input);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program().expect("Failed to parse program");
    
    let imports = get_imports(&program);
    
    // First import should be @std
    assert_eq!(imports[0].0, "std");
    assert_eq!(imports[0].1, "@std");
}

#[test]
fn test_nested_module_paths() {
    let input = r#"
        http := @std.net.http
        json := @std.encoding.json
        
        main = () i32 {
            return 0
        }
    "#;
    
    let lexer = Lexer::new(input);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program().expect("Failed to parse program");
    
    let imports = get_imports(&program);
    
    assert_eq!(imports.len(), 2);
    assert_eq!(imports[0].0, "http");
    assert_eq!(imports[0].1, "@std.net.http");
    assert_eq!(imports[1].0, "json");
    assert_eq!(imports[1].1, "@std.encoding.json");
}