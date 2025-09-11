// Tests for the new import system without comptime wrapper requirement

#[cfg(test)]
mod tests {
    use zen::lexer::Lexer;
    use zen::parser::Parser;
    use zen::ast::Declaration;

    #[test]
    fn test_direct_std_import() {
        let input = r#"
            core := @std.core
            build := @std.build
        "#;
        
        let lexer = Lexer::new(input);
        let mut parser = Parser::new(lexer);
        let program = parser.parse_program().unwrap();
        
        assert_eq!(program.declarations.len(), 2);
        
        // Check first import
        match &program.declarations[0] {
            Declaration::ModuleImport { alias, module_path } => {
                assert_eq!(alias, "core");
                assert_eq!(module_path, "@std.core");
            }
            _ => panic!("Expected ModuleImport declaration"),
        }
        
        // Check second import
        match &program.declarations[1] {
            Declaration::ModuleImport { alias, module_path } => {
                assert_eq!(alias, "build");
                assert_eq!(module_path, "@std.build");
            }
            _ => panic!("Expected ModuleImport declaration"),
        }
    }

    #[test]
    fn test_build_import_without_comptime() {
        let input = r#"
            build := @std.build
            io := build.import("io")
            math := build.import("math")
        "#;
        
        let lexer = Lexer::new(input);
        let mut parser = Parser::new(lexer);
        let program = parser.parse_program().unwrap();
        
        assert_eq!(program.declarations.len(), 3);
        
        // All should be module imports
        for decl in &program.declarations {
            assert!(matches!(decl, Declaration::ModuleImport { .. }));
        }
    }

    #[test]
    fn test_mixed_imports_and_code() {
        let input = r#"
            // Direct imports
            core := @std.core
            build := @std.build
            io := build.import("io")
            
            // Regular code
            main = () i32 {
                io.print("Hello, World!\n")
                return 0
            }
        "#;
        
        let lexer = Lexer::new(input);
        let mut parser = Parser::new(lexer);
        let program = parser.parse_program().unwrap();
        
        assert_eq!(program.declarations.len(), 4);
        
        // First three should be imports
        assert!(matches!(&program.declarations[0], Declaration::ModuleImport { .. }));
        assert!(matches!(&program.declarations[1], Declaration::ModuleImport { .. }));
        assert!(matches!(&program.declarations[2], Declaration::ModuleImport { .. }));
        
        // Last should be function
        assert!(matches!(&program.declarations[3], Declaration::Function(_)));
    }

    #[test]
    fn test_comptime_still_works_for_metaprogramming() {
        let input = r#"
            // Imports don't need comptime
            build := @std.build
            io := build.import("io")
            
            // But comptime still works for metaprogramming
            comptime {
                MAX_SIZE := 1024
                TABLE_SIZE := 256
            }
            
            main = () void {
                io.print("Comptime still works!\n")
            }
        "#;
        
        let lexer = Lexer::new(input);
        let mut parser = Parser::new(lexer);
        let program = parser.parse_program().unwrap();
        
        // Should have imports, comptime block, and function
        assert_eq!(program.declarations.len(), 4);
        assert!(matches!(&program.declarations[0], Declaration::ModuleImport { .. }));
        assert!(matches!(&program.declarations[1], Declaration::ModuleImport { .. }));
        assert!(matches!(&program.declarations[2], Declaration::ComptimeBlock(_)));
        assert!(matches!(&program.declarations[3], Declaration::Function(_)));
    }

    #[test]
    fn test_no_comptime_wrapper_for_imports() {
        // This should parse correctly WITHOUT comptime wrapper
        let input = r#"
            core := @std.core
            build := @std.build
            io := build.import("io")
            mem := build.import("mem")
            math := build.import("math")
            
            main = () void {
                io.print("All imports work without comptime!\n")
            }
        "#;
        
        let lexer = Lexer::new(input);
        let mut parser = Parser::new(lexer);
        let result = parser.parse_program();
        
        assert!(result.is_ok(), "Parsing should succeed without comptime wrapper");
        
        let program = result.unwrap();
        
        // Count module imports
        let import_count = program.declarations.iter()
            .filter(|d| matches!(d, Declaration::ModuleImport { .. }))
            .count();
        
        assert_eq!(import_count, 5, "Should have 5 module imports");
        
        // Verify no comptime blocks are needed
        let comptime_count = program.declarations.iter()
            .filter(|d| matches!(d, Declaration::ComptimeBlock(_)))
            .count();
        
        assert_eq!(comptime_count, 0, "Should have no comptime blocks for imports");
    }
}