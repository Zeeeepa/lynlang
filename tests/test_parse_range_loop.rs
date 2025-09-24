extern crate zen;
use zen::lexer::{Lexer, Token};
use zen::parser::Parser;

fn main() {
    let code = r#"
main = () void {
    (0..3).loop((i) {
        io.println("Test: ${i}")
    })
}
"#;

    println!("Testing range.loop() parsing");
    println!("Code:\n{}", code);

    let mut lexer = Lexer::new(code);

    // Tokenize and print tokens
    println!("\nTokens:");
    loop {
        let token = lexer.next_token();
        println!("  {:?}", token);
        if token == Token::Eof {
            break;
        }
    }

    // Now parse
    println!("\nParsing:");
    let lexer = Lexer::new(code);
    let mut parser = Parser::new(lexer);

    match parser.parse_program() {
        Ok(program) => {
            println!("✅ Parsed successfully!");
            println!("Program: {:#?}", program);
        }
        Err(e) => {
            println!("❌ Parse error: {:?}", e);
        }
    }
}
