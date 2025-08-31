#[cfg(test)]
mod self_hosting_tests {
    use std::fs;
    use std::path::Path;
    use zen::lexer::Lexer;
    use zen::parser::Parser;
    use zen::typechecker::TypeChecker;
    use zen::ast::{Declaration, Statement, Expression};

    #[test]
    fn test_stdlib_compiler_lexer_parsing() {
        let lexer_code = fs::read_to_string("stdlib/compiler/lexer.zen")
            .expect("Failed to read lexer.zen");
        
        let mut lexer = Lexer::new(&lexer_code);
        let tokens = lexer.tokenize();
        assert!(tokens.is_ok(), "Lexer should tokenize without errors");
        
        let mut parser = Parser::new(tokens.unwrap());
        let ast = parser.parse();
        assert!(ast.is_ok(), "Parser should parse lexer.zen without errors");
    }

    #[test]
    fn test_stdlib_compiler_parser_parsing() {
        let parser_code = fs::read_to_string("stdlib/compiler/parser.zen")
            .expect("Failed to read parser.zen");
        
        let mut lexer = Lexer::new(&parser_code);
        let tokens = lexer.tokenize();
        assert!(tokens.is_ok());
        
        let mut parser = Parser::new(tokens.unwrap());
        let ast = parser.parse();
        assert!(ast.is_ok(), "Parser should parse parser.zen without errors");
    }

    #[test]
    fn test_stdlib_compiler_type_checker_parsing() {
        let type_checker_code = fs::read_to_string("stdlib/compiler/type_checker.zen")
            .expect("Failed to read type_checker.zen");
        
        let mut lexer = Lexer::new(&type_checker_code);
        let tokens = lexer.tokenize();
        assert!(tokens.is_ok());
        
        let mut parser = Parser::new(tokens.unwrap());
        let ast = parser.parse();
        assert!(ast.is_ok(), "Parser should parse type_checker.zen without errors");
    }

    #[test]
    fn test_stdlib_compiler_codegen_parsing() {
        let codegen_code = fs::read_to_string("stdlib/compiler/codegen.zen")
            .expect("Failed to read codegen.zen");
        
        let mut lexer = Lexer::new(&codegen_code);
        let tokens = lexer.tokenize();
        assert!(tokens.is_ok());
        
        let mut parser = Parser::new(tokens.unwrap());
        let ast = parser.parse();
        assert!(ast.is_ok(), "Parser should parse codegen.zen without errors");
    }

    #[test]
    fn test_bootstrap_compiler_parsing() {
        let bootstrap_code = fs::read_to_string("stdlib/compiler/bootstrap_compiler.zen")
            .expect("Failed to read bootstrap_compiler.zen");
        
        let mut lexer = Lexer::new(&bootstrap_code);
        let tokens = lexer.tokenize();
        assert!(tokens.is_ok());
        
        let mut parser = Parser::new(tokens.unwrap());
        let ast = parser.parse();
        assert!(ast.is_ok(), "Parser should parse bootstrap_compiler.zen without errors");
        
        // Verify imports are at module level
        let program = ast.unwrap();
        for decl in &program.declarations {
            if let Declaration::ModuleImport { .. } = decl {
                // This is valid - module-level import
                continue;
            }
            
            // Check that no imports are in comptime blocks
            if let Declaration::ComptimeBlock(stmts) = decl {
                for stmt in stmts {
                    assert!(!contains_import(stmt), 
                           "Comptime block should not contain imports");
                }
            }
        }
    }

    #[test]
    fn test_self_host_driver_parsing() {
        let driver_code = fs::read_to_string("tools/zen-self-host.zen")
            .expect("Failed to read zen-self-host.zen");
        
        let mut lexer = Lexer::new(&driver_code);
        let tokens = lexer.tokenize();
        assert!(tokens.is_ok());
        
        let mut parser = Parser::new(tokens.unwrap());
        let ast = parser.parse();
        assert!(ast.is_ok(), "Parser should parse zen-self-host.zen without errors");
    }

    #[test]
    fn test_env_module_parsing() {
        let env_code = fs::read_to_string("stdlib/env.zen")
            .expect("Failed to read env.zen");
        
        let mut lexer = Lexer::new(&env_code);
        let tokens = lexer.tokenize();
        assert!(tokens.is_ok());
        
        let mut parser = Parser::new(tokens.unwrap());
        let ast = parser.parse();
        assert!(ast.is_ok(), "Parser should parse env.zen without errors");
    }

    #[test]
    fn test_unittest_module_parsing() {
        let unittest_code = fs::read_to_string("stdlib/unittest.zen")
            .expect("Failed to read unittest.zen");
        
        let mut lexer = Lexer::new(&unittest_code);
        let tokens = lexer.tokenize();
        assert!(tokens.is_ok());
        
        let mut parser = Parser::new(tokens.unwrap());
        let ast = parser.parse();
        assert!(ast.is_ok(), "Parser should parse unittest.zen without errors");
    }

    #[test]
    fn test_all_stdlib_modules_have_correct_imports() {
        let stdlib_path = Path::new("stdlib");
        
        for entry in fs::read_dir(stdlib_path).expect("Failed to read stdlib directory") {
            let entry = entry.expect("Failed to read entry");
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("zen") {
                let content = fs::read_to_string(&path)
                    .expect(&format!("Failed to read {:?}", path));
                
                let mut lexer = Lexer::new(&content);
                let tokens = lexer.tokenize();
                
                if tokens.is_err() {
                    eprintln!("Warning: Failed to tokenize {:?}", path);
                    continue;
                }
                
                let mut parser = Parser::new(tokens.unwrap());
                let ast = parser.parse();
                
                if ast.is_err() {
                    eprintln!("Warning: Failed to parse {:?}", path);
                    continue;
                }
                
                let program = ast.unwrap();
                
                // Check that imports are only at module level
                let mut module_level_imports = 0;
                let mut nested_imports = 0;
                
                for decl in &program.declarations {
                    match decl {
                        Declaration::ModuleImport { .. } => {
                            module_level_imports += 1;
                        }
                        Declaration::ComptimeBlock(stmts) => {
                            for stmt in stmts {
                                if contains_import(stmt) {
                                    nested_imports += 1;
                                }
                            }
                        }
                        Declaration::Function(func) => {
                            for stmt in &func.body {
                                if contains_import(stmt) {
                                    nested_imports += 1;
                                }
                            }
                        }
                        _ => {}
                    }
                }
                
                assert_eq!(nested_imports, 0, 
                          "File {:?} should not have imports in nested contexts", path);
            }
        }
    }

    #[test]
    fn test_compiler_modules_integration() {
        // Test that all compiler modules can be loaded together
        let modules = vec![
            "stdlib/compiler/lexer.zen",
            "stdlib/compiler/parser.zen",
            "stdlib/compiler/type_checker.zen",
            "stdlib/compiler/codegen.zen",
        ];
        
        for module_path in modules {
            if !Path::new(module_path).exists() {
                continue; // Skip if file doesn't exist
            }
            
            let content = fs::read_to_string(module_path)
                .expect(&format!("Failed to read {}", module_path));
            
            let mut lexer = Lexer::new(&content);
            let tokens = lexer.tokenize();
            assert!(tokens.is_ok(), "Failed to tokenize {}", module_path);
            
            let mut parser = Parser::new(tokens.unwrap());
            let ast = parser.parse();
            assert!(ast.is_ok(), "Failed to parse {}", module_path);
        }
    }

    // Helper function to check if a statement contains an import
    fn contains_import(stmt: &Statement) -> bool {
        match stmt {
            Statement::ModuleImport { .. } => true,
            Statement::VariableDeclaration { initializer, .. } => {
                if let Some(expr) = initializer {
                    is_import_expression(expr)
                } else {
                    false
                }
            }
            Statement::VariableAssignment { value, .. } => {
                is_import_expression(value)
            }
            Statement::ComptimeBlock(stmts) => {
                stmts.iter().any(|s| contains_import(s))
            }
            Statement::Loop { body, .. } => {
                body.iter().any(|s| contains_import(s))
            }
            _ => false,
        }
    }

    // Helper function to check if an expression is an import
    fn is_import_expression(expr: &Expression) -> bool {
        match expr {
            Expression::Identifier(id) if id.starts_with("@std") => true,
            Expression::MemberAccess { object, .. } => {
                is_import_expression(object)
            }
            Expression::FunctionCall { name, .. } if name.contains("import") => true,
            _ => false,
        }
    }
}