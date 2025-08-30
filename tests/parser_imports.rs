#[cfg(test)]
mod tests {
    use zen::ast::*;
    use zen::lexer::Lexer;
    use zen::parser::Parser;

    #[test]
    fn test_module_import_without_comptime() {
        let input = r#"
            core := @std.core
            build := @std.build
            io := build.import("io")
            
            main = () void {
                io.print("Hello")
            }
        "#;

        let mut lexer = Lexer::new(input);
        let mut parser = Parser::new(lexer);
        let result = parser.parse_program();

        assert!(result.is_ok(), "Failed to parse: {:?}", result);
        let program = result.unwrap();

        // Check that we have 4 declarations (3 imports + 1 function)
        assert_eq!(program.declarations.len(), 4);

        // Check first import (core)
        match &program.declarations[0] {
            Declaration::ModuleImport { alias, module_path } => {
                assert_eq!(alias, "core");
                assert_eq!(module_path, "@std.core");
            }
            _ => panic!("Expected ModuleImport for core"),
        }

        // Check second import (build)
        match &program.declarations[1] {
            Declaration::ModuleImport { alias, module_path } => {
                assert_eq!(alias, "build");
                assert_eq!(module_path, "@std.build");
            }
            _ => panic!("Expected ModuleImport for build"),
        }

        // Check third import (io)
        match &program.declarations[2] {
            Declaration::ModuleImport { alias, module_path } => {
                assert_eq!(alias, "io");
                assert_eq!(module_path, "std.io");
            }
            _ => panic!("Expected ModuleImport for io"),
        }

        // Check function declaration
        match &program.declarations[3] {
            Declaration::Function(f) => {
                assert_eq!(f.name, "main");
            }
            _ => panic!("Expected Function for main"),
        }
    }

    #[test]
    fn test_std_module_direct_import() {
        let input = r#"
            math := @std.math
            string := @std.string
        "#;

        let mut lexer = Lexer::new(input);
        let mut parser = Parser::new(lexer);
        let result = parser.parse_program();

        assert!(result.is_ok(), "Failed to parse: {:?}", result);
        let program = result.unwrap();

        assert_eq!(program.declarations.len(), 2);

        match &program.declarations[0] {
            Declaration::ModuleImport { alias, module_path } => {
                assert_eq!(alias, "math");
                assert_eq!(module_path, "@std.math");
            }
            _ => panic!("Expected ModuleImport for math"),
        }

        match &program.declarations[1] {
            Declaration::ModuleImport { alias, module_path } => {
                assert_eq!(alias, "string");
                assert_eq!(module_path, "@std.string");
            }
            _ => panic!("Expected ModuleImport for string"),
        }
    }

    #[test]
    fn test_build_import_syntax() {
        let input = r#"
            build := @std.build
            fs := build.import("fs")
            net := build.import("net")
        "#;

        let mut lexer = Lexer::new(input);
        let mut parser = Parser::new(lexer);
        let result = parser.parse_program();

        assert!(result.is_ok(), "Failed to parse: {:?}", result);
        let program = result.unwrap();

        assert_eq!(program.declarations.len(), 3);

        // Check build import
        match &program.declarations[0] {
            Declaration::ModuleImport { alias, module_path } => {
                assert_eq!(alias, "build");
                assert_eq!(module_path, "@std.build");
            }
            _ => panic!("Expected ModuleImport for build"),
        }

        // Check fs import
        match &program.declarations[1] {
            Declaration::ModuleImport { alias, module_path } => {
                assert_eq!(alias, "fs");
                assert_eq!(module_path, "std.fs");
            }
            _ => panic!("Expected ModuleImport for fs"),
        }

        // Check net import
        match &program.declarations[2] {
            Declaration::ModuleImport { alias, module_path } => {
                assert_eq!(alias, "net");
                assert_eq!(module_path, "std.net");
            }
            _ => panic!("Expected ModuleImport for net"),
        }
    }

    #[test]
    fn test_mixed_declarations_with_imports() {
        let input = r#"
            io := @std.io
            
            Point = { x: f64, y: f64 }
            
            math := @std.math
            
            distance = (p1: Point, p2: Point) f64 {
                dx := p2.x - p1.x
                dy := p2.y - p1.y
                return math.sqrt(dx * dx + dy * dy)
            }
        "#;

        let mut lexer = Lexer::new(input);
        let mut parser = Parser::new(lexer);
        let result = parser.parse_program();

        assert!(result.is_ok(), "Failed to parse: {:?}", result);
        let program = result.unwrap();

        assert_eq!(program.declarations.len(), 4);

        // Check first import
        match &program.declarations[0] {
            Declaration::ModuleImport { alias, .. } => {
                assert_eq!(alias, "io");
            }
            _ => panic!("Expected ModuleImport"),
        }

        // Check struct
        match &program.declarations[1] {
            Declaration::Struct(s) => {
                assert_eq!(s.name, "Point");
            }
            _ => panic!("Expected Struct"),
        }

        // Check second import
        match &program.declarations[2] {
            Declaration::ModuleImport { alias, .. } => {
                assert_eq!(alias, "math");
            }
            _ => panic!("Expected ModuleImport"),
        }

        // Check function
        match &program.declarations[3] {
            Declaration::Function(f) => {
                assert_eq!(f.name, "distance");
            }
            _ => panic!("Expected Function"),
        }
    }

    #[test]
    fn test_import_with_member_access() {
        let input = r#"
            core := @std.core
            range := core.range
            range_inclusive := core.range_inclusive
        "#;

        let mut lexer = Lexer::new(input);
        let mut parser = Parser::new(lexer);
        let result = parser.parse_program();

        assert!(result.is_ok(), "Failed to parse: {:?}", result);
        let program = result.unwrap();

        // First should be ModuleImport
        match &program.declarations[0] {
            Declaration::ModuleImport { alias, module_path } => {
                assert_eq!(alias, "core");
                assert_eq!(module_path, "@std.core");
            }
            _ => panic!("Expected ModuleImport for core"),
        }

        // The rest should be parsed as constant declarations, not imports
        // This is expected behavior - they're accessing members of the imported module
        assert!(program.declarations.len() >= 1, "Should have at least the import declaration");
    }
}