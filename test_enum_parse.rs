use zen::lexer::Lexer;
use zen::parser::core::Parser;

fn main() {
    let input = "Shape: .Circle | .Rectangle";
    let mut lexer = Lexer::new(input);
    let mut parser = Parser::new(&mut lexer);
    
    // Parse the tokens
    parser.next_token();
    parser.next_token();
    
    println!("Current token: {:?}", parser.current_token);
    println!("Peek token: {:?}", parser.peek_token);
    
    // Try parsing enum
    match parser.parse_enum() {
        Ok(enum_def) => println!("Parsed enum: {:?}", enum_def),
        Err(e) => println!("Error: {:?}", e),
    }
}
