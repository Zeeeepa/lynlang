use zen::lexer::Lexer;
use zen::parser::Parser;

fn main() {
    let codes = vec![
        ".Err(42)",
        "return .Err(42)",
    ];
    
    for code in codes {
        println!("\nTesting: {}", code);
        let lexer = Lexer::new(code);
        let mut parser = Parser::new(lexer);
        
        match parser.parse_expression() {
            Ok(expr) => {
                println!("Expression parsed: {:#?}", expr);
            }
            Err(e) => {
                println!("Expression parse error: {:?}", e);
            }
        }
    }
}