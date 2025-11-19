use zen::lexer::{Lexer, Token};

fn tokenize(input: &str) -> Vec<Token> {
    let mut lexer = Lexer::new(input);
    std::iter::from_fn(|| {
        let token = lexer.next_token();
        if token == Token::Eof {
            None
        } else {
            Some(token)
        }
    })
    .collect()
}

#[test]
fn test_lexer_simple() {
    let input = r#"loop"#;
    let tokens = tokenize(input);
    
    // Verify we got the loop token
    assert_eq!(tokens.len(), 1, "Should tokenize 'loop' as one token");
    assert_eq!(tokens[0], Token::Identifier("loop".to_string()), 
        "loop should be recognized as identifier");
}

#[test]
fn test_lexer_new_syntax() {
    // Test 'loop' keyword
    let tokens = tokenize("loop");
    assert!(!tokens.is_empty(), "Should tokenize 'loop'");
    
    // Test 'break' keyword
    let tokens = tokenize("break");
    assert!(!tokens.is_empty(), "Should tokenize 'break'");
    
    // Test identifier
    let tokens = tokenize("Some");
    assert_eq!(tokens.len(), 1, "Should tokenize 'Some' as single token");
    
    // Test method call
    let tokens = tokenize("items.loop");
    assert_eq!(tokens.len(), 3, "Should tokenize 'items.loop' as 3 tokens (id, dot, id)");
    assert_eq!(tokens[0], Token::Identifier("items".to_string()));
    assert_eq!(tokens[1], Token::Symbol('.'));
    assert_eq!(tokens[2], Token::Identifier("loop".to_string()));
}

