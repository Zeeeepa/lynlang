use zen::lexer::Lexer;
use zen::parser::Parser;
use zen::ast::{Statement, Expression};

#[test]
fn test_defer_single_statement() {
    let code = r#"
        test_func = () void {
            file := open_file("test.txt")
            defer file.close()
        }
    "#;
    
    let lexer = Lexer::new(code);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program().unwrap();
    
    assert_eq!(program.declarations.len(), 1);
}

#[test]
fn test_defer_block() {
    let code = r#"
        test_func = () void {
            buffer := allocate(1024)
            defer {
                free(buffer)
            }
        }
    "#;
    
    let lexer = Lexer::new(code);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program().unwrap();
    
    assert_eq!(program.declarations.len(), 1);
}

#[test]
fn test_multiple_defer_statements() {
    let code = r#"
        test_func = () void {
            a := allocate(100)
            defer free(a)
            
            b := allocate(200)
            defer free(b)
            
            c := allocate(300)
            defer {
                cleanup(c)
                free(c)
            }
        }
    "#;
    
    let lexer = Lexer::new(code);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program().unwrap();
    
    assert_eq!(program.declarations.len(), 1);
}