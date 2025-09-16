use std::process::Command;
use zen::*;

#[cfg(test)]
mod tests {
    use super::*;
    use zen::ast::*;
    use zen::lexer::*;
    use zen::parser::core::*;
    use zen::error::*;

    #[test]
    fn test_pointer_type_parsing() {
        let mut lexer = Lexer::new("Ptr<i32>");
        let tokens = lexer.tokenize().expect("Failed to tokenize");
        let mut parser = Parser::new(tokens);
        
        let parsed_type = parser.parse_type().expect("Failed to parse Ptr<i32>");
        match parsed_type {
            AstType::Ptr(inner) => {
                assert!(matches!(**inner, AstType::I32));
            }
            _ => panic!("Expected Ptr type, got: {:?}", parsed_type),
        }
    }
    
    #[test]
    fn test_mut_ptr_type_parsing() {
        let mut lexer = Lexer::new("MutPtr<f64>");
        let tokens = lexer.tokenize().expect("Failed to tokenize");
        let mut parser = Parser::new(tokens);
        
        let parsed_type = parser.parse_type().expect("Failed to parse MutPtr<f64>");
        match parsed_type {
            AstType::MutPtr(inner) => {
                assert!(matches!(**inner, AstType::F64));
            }
            _ => panic!("Expected MutPtr type, got: {:?}", parsed_type),
        }
    }
    
    #[test]
    fn test_raw_ptr_type_parsing() {
        let mut lexer = Lexer::new("RawPtr<bool>");
        let tokens = lexer.tokenize().expect("Failed to tokenize");
        let mut parser = Parser::new(tokens);
        
        let parsed_type = parser.parse_type().expect("Failed to parse RawPtr<bool>");
        match parsed_type {
            AstType::RawPtr(inner) => {
                assert!(matches!(**inner, AstType::Bool));
            }
            _ => panic!("Expected RawPtr type, got: {:?}", parsed_type),
        }
    }
    
    #[test]
    fn test_pointer_dereference_parsing() {
        let mut lexer = Lexer::new("ptr.val");
        let tokens = lexer.tokenize().expect("Failed to tokenize");
        let mut parser = Parser::new(tokens);
        
        let parsed_expr = parser.parse_expression().expect("Failed to parse ptr.val");
        match parsed_expr {
            Expression::PointerDereference(inner) => {
                match *inner {
                    Expression::Identifier(name) => assert_eq!(name, "ptr"),
                    _ => panic!("Expected identifier in dereference"),
                }
            }
            _ => panic!("Expected PointerDereference, got: {:?}", parsed_expr),
        }
    }
    
    #[test]
    fn test_pointer_address_parsing() {
        let mut lexer = Lexer::new("var.addr");
        let tokens = lexer.tokenize().expect("Failed to tokenize");
        let mut parser = Parser::new(tokens);
        
        let parsed_expr = parser.parse_expression().expect("Failed to parse var.addr");
        match parsed_expr {
            Expression::PointerAddress(inner) => {
                match *inner {
                    Expression::Identifier(name) => assert_eq!(name, "var"),
                    _ => panic!("Expected identifier in address operation"),
                }
            }
            _ => panic!("Expected PointerAddress, got: {:?}", parsed_expr),
        }
    }
    
    #[test]
    fn test_create_reference_parsing() {
        let mut lexer = Lexer::new("value.ref()");
        let tokens = lexer.tokenize().expect("Failed to tokenize");
        let mut parser = Parser::new(tokens);
        
        let parsed_expr = parser.parse_expression().expect("Failed to parse value.ref()");
        match parsed_expr {
            Expression::CreateReference(inner) => {
                match *inner {
                    Expression::Identifier(name) => assert_eq!(name, "value"),
                    _ => panic!("Expected identifier in reference creation"),
                }
            }
            _ => panic!("Expected CreateReference, got: {:?}", parsed_expr),
        }
    }
    
    #[test]
    fn test_create_mutable_reference_parsing() {
        let mut lexer = Lexer::new("mut_value.mut_ref()");
        let tokens = lexer.tokenize().expect("Failed to tokenize");
        let mut parser = Parser::new(tokens);
        
        let parsed_expr = parser.parse_expression().expect("Failed to parse mut_value.mut_ref()");
        match parsed_expr {
            Expression::CreateMutableReference(inner) => {
                match *inner {
                    Expression::Identifier(name) => assert_eq!(name, "mut_value"),
                    _ => panic!("Expected identifier in mutable reference creation"),
                }
            }
            _ => panic!("Expected CreateMutableReference, got: {:?}", parsed_expr),
        }
    }
    
    #[test]
    fn test_nested_pointer_types() {
        let mut lexer = Lexer::new("Ptr<MutPtr<i32>>");
        let tokens = lexer.tokenize().expect("Failed to tokenize");
        let mut parser = Parser::new(tokens);
        
        let parsed_type = parser.parse_type().expect("Failed to parse nested pointer");
        match parsed_type {
            AstType::Ptr(outer) => {
                match **outer {
                    AstType::MutPtr(inner) => {
                        assert!(matches!(**inner, AstType::I32));
                    }
                    _ => panic!("Expected MutPtr inside Ptr"),
                }
            }
            _ => panic!("Expected Ptr type, got: {:?}", parsed_type),
        }
    }
    
    #[test] 
    fn test_pointer_with_struct_type() {
        let mut lexer = Lexer::new("Ptr<Point>");
        let tokens = lexer.tokenize().expect("Failed to tokenize");
        let mut parser = Parser::new(tokens);
        
        let parsed_type = parser.parse_type().expect("Failed to parse Ptr<Point>");
        match parsed_type {
            AstType::Ptr(inner) => {
                match **inner {
                    AstType::Generic { name, type_args } => {
                        assert_eq!(name, "Point");
                        assert!(type_args.is_empty());
                    }
                    _ => panic!("Expected Generic type for Point"),
                }
            }
            _ => panic!("Expected Ptr type, got: {:?}", parsed_type),
        }
    }
    
    #[test]
    fn test_complete_pointer_program_parsing() {
        let program = r#"
            fn main() {
                value: i32 = 42
                ptr: Ptr<i32> = value.ref()
                result: i32 = ptr.val
                io.println("Result: ${result}")
            }
        "#;
        
        let mut lexer = Lexer::new(program);
        let tokens = lexer.tokenize().expect("Failed to tokenize program");
        let mut parser = Parser::new(tokens);
        
        let ast = parser.parse_program().expect("Failed to parse pointer program");
        
        // Verify we have a main function
        assert_eq!(ast.declarations.len(), 1);
        match &ast.declarations[0] {
            Declaration::Function(func) => {
                assert_eq!(func.name, "main");
                assert_eq!(func.body.len(), 4); // 4 statements
            }
            _ => panic!("Expected function declaration"),
        }
    }
}