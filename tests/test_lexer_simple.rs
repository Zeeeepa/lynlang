use zen::lexer::{Lexer, Token};

fn main() {
    let input = r#"
        // Test new tokens
        loop(() { break })
        items.loop((item) { })
        Some(42)
        None
        expr.raise()
        @this.defer(cleanup())
        return value
        continue
    "#;
    
    let mut lexer = Lexer::new(input);
    
    loop {
        let token = lexer.next_token();
        println!("{:?}", token);
        if token == Token::Eof {
            break;
        }
    }
}