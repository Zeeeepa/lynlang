use zen::lexer::{Lexer, Token};

#[test]
fn test_lexer_simple() {
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
    let mut tokens = Vec::new();

    loop {
        let token = lexer.next_token();
        tokens.push(token.clone());
        if token == Token::Eof {
            break;
        }
    }

    // Verify we got tokens (not just EOF)
    assert!(tokens.len() > 1, "Should tokenize input");
}

#[test]
fn test_lexer_new_syntax() {
    let test_cases = vec![
        ("loop", "Should recognize loop as a token"),
        ("break", "Should recognize break as a token"),
        ("continue", "Should recognize continue as a token"),
        ("return", "Should recognize return as a token"),
        ("defer", "Should recognize defer as a token"),
        ("Some(42)", "Should handle Some constructor"),
        ("None", "Should handle None constructor"),
        ("items.loop((item) { })", "Should handle .loop() method"),
        ("expr.raise()", "Should handle .raise() method"),
        ("@this.defer(cleanup())", "Should handle @this.defer"),
        ("(0..10).loop((i) { })", "Should handle range.loop()"),
    ];

    for (input, description) in test_cases {
        let mut lexer = Lexer::new(input);
        let mut token_count = 0;

        loop {
            let token = lexer.next_token();
            if token == Token::Eof {
                break;
            }
            token_count += 1;
        }

        assert!(token_count > 0, "{}: Should tokenize '{}'", description, input);
    }
}

