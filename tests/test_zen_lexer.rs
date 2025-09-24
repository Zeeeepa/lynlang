use zen::lexer::{Lexer, Token};

fn test_lexer(code: &str, expected: Vec<Token>) {
    let mut lexer = Lexer::new(code);
    let mut tokens = Vec::new();
    
    loop {
        let token = lexer.next_token();
        if token == Token::Eof {
            tokens.push(token);
            break;
        }
        tokens.push(token);
    }
    
    if tokens != expected {
        println!("FAIL: {}", code);
        println!("  Expected: {:?}", expected);
        println!("  Got:      {:?}", tokens);
    } else {
        println!("PASS: {}", code);
    }
}

fn main() {
    println!("Testing Zen Lexer based on LANGUAGE_SPEC.zen");
    println!("{}", "=".repeat(50));
    
    // Test @std and @this special tokens
    test_lexer("@std", vec![Token::AtStd, Token::Eof]);
    test_lexer("@this", vec![Token::AtThis, Token::Eof]);
    test_lexer("@other", vec![Token::Identifier("@other".to_string()), Token::Eof]);
    
    // Test assignment operators from spec
    test_lexer("=", vec![Token::Operator("=".to_string()), Token::Eof]);
    test_lexer(":=", vec![Token::Operator(":=".to_string()), Token::Eof]); 
    test_lexer("::", vec![Token::Operator("::".to_string()), Token::Eof]);
    test_lexer("::=", vec![Token::Operator("::=".to_string()), Token::Eof]);
    test_lexer(":", vec![Token::Symbol(':'), Token::Eof]);
    
    // Test pattern matching operators
    test_lexer("?", vec![Token::Question, Token::Eof]);
    test_lexer("|", vec![Token::Pipe, Token::Eof]);
    test_lexer("_", vec![Token::Underscore, Token::Eof]);
    
    // Test arrow operators
    test_lexer("->", vec![Token::Operator("->".to_string()), Token::Eof]);
    test_lexer("=>", vec![Token::Operator("=>".to_string()), Token::Eof]);
    
    // Test range operators
    test_lexer("..", vec![Token::Operator("..".to_string()), Token::Eof]);
    test_lexer("..=", vec![Token::Operator("..=".to_string()), Token::Eof]);
    test_lexer("...", vec![Token::Operator("...".to_string()), Token::Eof]);
    
    // Test complex code from spec
    test_lexer("x: i32", vec![
        Token::Identifier("x".to_string()),
        Token::Symbol(':'),
        Token::Identifier("i32".to_string()),
        Token::Eof
    ]);
    
    test_lexer("w :: i32", vec![
        Token::Identifier("w".to_string()),
        Token::Operator("::".to_string()),
        Token::Identifier("i32".to_string()),
        Token::Eof
    ]);
    
    test_lexer("v ::= 30", vec![
        Token::Identifier("v".to_string()),
        Token::Operator("::=".to_string()),
        Token::Integer("30".to_string()),
        Token::Eof
    ]);
    
    // Test imports from spec
    test_lexer("{ io, math } = @std", vec![
        Token::Symbol('{'),
        Token::Identifier("io".to_string()),
        Token::Symbol(','),
        Token::Identifier("math".to_string()),
        Token::Symbol('}'),
        Token::Operator("=".to_string()),
        Token::AtStd,
        Token::Eof
    ]);
    
    // Test string interpolation
    test_lexer("\"Hello ${name}\"", vec![
        Token::StringLiteral("Hello \x01name\x02".to_string()),
        Token::Eof
    ]);
    
    // Test Option type syntax
    test_lexer("Option<T>: Some(T) | None", vec![
        Token::Identifier("Option".to_string()),
        Token::Operator("<".to_string()),
        Token::Identifier("T".to_string()),
        Token::Operator(">".to_string()),
        Token::Symbol(':'),
        Token::Identifier("Some".to_string()),
        Token::Symbol('('),
        Token::Identifier("T".to_string()),
        Token::Symbol(')'),
        Token::Pipe,
        Token::Identifier("None".to_string()),
        Token::Eof
    ]);
    
    println!("\nAll tests complete!");
}