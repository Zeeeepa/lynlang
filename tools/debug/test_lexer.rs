use zen::lexer::Lexer;

fn main() {
    let code = "return Err(42)";
    
    let mut lexer = Lexer::new(code);
    
    println!("Tokenizing: {}", code);
    loop {
        let token = lexer.next_token();
        println!("{:?}", token);
        if token == zen::lexer::Token::Eof {
            break;
        }
    }
}