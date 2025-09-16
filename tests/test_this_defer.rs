use zen::lexer::Lexer;
use zen::parser::Parser;
use zen::ast::{Statement, Expression};

#[test]
fn test_this_defer_single_statement() {
    let code = r#"
        test_func = () void {
            resource = allocate()
            @this.defer(resource.cleanup())
        }
    "#;
    
    let lexer = Lexer::new(code);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program().unwrap();
    
    assert_eq!(program.declarations.len(), 1);
}

#[test]
fn test_this_defer_multiple_statements() {
    let code = r#"
        test_func = () void {
            a = allocate(100)
            @this.defer(a.cleanup())
            
            b = allocate(200)
            @this.defer(b.cleanup())
            
            c = allocate(300)
            @this.defer(c.cleanup())
        }
    "#;
    
    let lexer = Lexer::new(code);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program().unwrap();
    
    assert_eq!(program.declarations.len(), 1);
}

#[test]
fn test_this_defer_with_function_call() {
    let code = r#"
        test_func = () void {
            file = open_file("test.txt")
            @this.defer(file.close())
            
            buffer = allocate(1024)
            @this.defer(free(buffer))
        }
    "#;
    
    let lexer = Lexer::new(code);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program().unwrap();
    
    assert_eq!(program.declarations.len(), 1);
}

#[test]
fn test_this_defer_syntax_error() {
    // Missing parentheses
    let code = r#"
        test_func = () void {
            @this.defer resource.cleanup()
        }
    "#;
    
    let lexer = Lexer::new(code);
    let mut parser = Parser::new(lexer);
    let result = parser.parse_program();
    
    assert!(result.is_err());
}

#[test]
fn test_this_defer_wrong_method() {
    // Wrong method name after @this.
    let code = r#"
        test_func = () void {
            @this.cleanup(resource)
        }
    "#;
    
    let lexer = Lexer::new(code);
    let mut parser = Parser::new(lexer);
    let result = parser.parse_program();
    
    assert!(result.is_err());
}

#[test]
fn test_this_defer_ast_structure() {
    let code = r#"
        test_func = () void {
            @this.defer(cleanup())
        }
    "#;
    
    let lexer = Lexer::new(code);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program().unwrap();
    
    // Extract the function and check its body
    if let Some(zen::ast::Declaration::Function(func)) = program.declarations.first() {
        assert_eq!(func.body.len(), 1);
        
        // Check that the statement is a ThisDefer
        if let Statement::ThisDefer(expr) = &func.body[0] {
            // Check that the expression is a function call
            match expr {
                Expression::FunctionCall { name, args } => {
                    assert_eq!(name, "cleanup");
                    assert_eq!(args.len(), 0);
                }
                _ => panic!("Expected function call expression in defer"),
            }
        } else {
            panic!("Expected ThisDefer statement");
        }
    } else {
        panic!("Expected function declaration");
    }
}

#[test]
fn test_this_defer_with_complex_expression() {
    let code = r#"
        test_func = () void {
            @this.defer(resource.method(arg1, arg2))
        }
    "#;
    
    let lexer = Lexer::new(code);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program().unwrap();
    
    assert_eq!(program.declarations.len(), 1);
    
    // Check the AST structure
    if let Some(zen::ast::Declaration::Function(func)) = program.declarations.first() {
        assert_eq!(func.body.len(), 1);
        
        if let Statement::ThisDefer(_expr) = &func.body[0] {
            // The expression should be parsed correctly
            // This confirms the parser can handle complex expressions in defer
        } else {
            panic!("Expected ThisDefer statement");
        }
    }
}