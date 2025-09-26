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
fn test_at_symbols() {
    // Test @std and @this tokens
    let mut lexer = Lexer::new("@std");
    assert_eq!(lexer.next_token(), Token::AtStd);

    let mut lexer = Lexer::new("@this");
    assert_eq!(lexer.next_token(), Token::AtThis);

    // Test @meta for metaprogramming
    let mut lexer = Lexer::new("@meta");
    assert_eq!(lexer.next_token(), Token::AtMeta);
}

#[test]
fn test_assignment_operators() {
    // Test assignment operators: = (immutable), ::= (mutable), : (type)
    let tokens = tokenize("x = 10");
    assert_eq!(
        tokens,
        vec![
            Token::Identifier("x".to_string()),
            Token::Operator("=".to_string()),
            Token::Integer("10".to_string())
        ]
    );

    let tokens = tokenize("v ::= 30");
    assert_eq!(
        tokens,
        vec![
            Token::Identifier("v".to_string()),
            Token::Operator("::=".to_string()),
            Token::Integer("30".to_string())
        ]
    );

    let tokens = tokenize("x: i32");
    assert_eq!(
        tokens,
        vec![
            Token::Identifier("x".to_string()),
            Token::Symbol(':'),
            Token::Identifier("i32".to_string())
        ]
    );

    let tokens = tokenize("w :: i32");
    assert_eq!(
        tokens,
        vec![
            Token::Identifier("w".to_string()),
            Token::Operator("::".to_string()),
            Token::Identifier("i32".to_string())
        ]
    );
}

#[test]
fn test_pattern_matching() {
    // Test pattern matching operator ?
    let tokens = tokenize("x ?");
    assert_eq!(
        tokens,
        vec![Token::Identifier("x".to_string()), Token::Question]
    );

    // Test pipe for pattern arms
    let tokens = tokenize("| true");
    assert_eq!(
        tokens,
        vec![Token::Pipe, Token::Identifier("true".to_string())]
    );

    // Test underscore for wildcard
    let tokens = tokenize("| _");
    assert_eq!(tokens, vec![Token::Pipe, Token::Underscore]);
}

#[test]
fn test_operators() {
    // Test arrow operators
    let tokens = tokenize("->");
    assert_eq!(tokens, vec![Token::Operator("->".to_string())]);

    // Test range operators
    let tokens = tokenize("0..10");
    assert_eq!(
        tokens,
        vec![
            Token::Integer("0".to_string()),
            Token::Operator("..".to_string()),
            Token::Integer("10".to_string())
        ]
    );

    let tokens = tokenize("0..=10");
    assert_eq!(
        tokens,
        vec![
            Token::Integer("0".to_string()),
            Token::Operator("..=".to_string()),
            Token::Integer("10".to_string())
        ]
    );

    // Test logical operators
    let tokens = tokenize("&& ||");
    assert_eq!(
        tokens,
        vec![
            Token::Operator("&&".to_string()),
            Token::Operator("||".to_string())
        ]
    );

    // Test comparison operators
    let tokens = tokenize("== != < > <= >=");
    assert_eq!(
        tokens,
        vec![
            Token::Operator("==".to_string()),
            Token::Operator("!=".to_string()),
            Token::Operator("<".to_string()),
            Token::Operator(">".to_string()),
            Token::Operator("<=".to_string()),
            Token::Operator(">=".to_string())
        ]
    );
}

#[test]
fn test_ufc_and_methods() {
    // Test UFC member access
    let tokens = tokenize("shape.area()");
    assert_eq!(
        tokens,
        vec![
            Token::Identifier("shape".to_string()),
            Token::Symbol('.'),
            Token::Identifier("area".to_string()),
            Token::Symbol('('),
            Token::Symbol(')')
        ]
    );

    // Test Option enum syntax
    let tokens = tokenize(".Some(5)");
    assert_eq!(
        tokens,
        vec![
            Token::Symbol('.'),
            Token::Identifier("Some".to_string()),
            Token::Symbol('('),
            Token::Integer("5".to_string()),
            Token::Symbol(')')
        ]
    );
}

#[test]
fn test_string_interpolation() {
    // Test string interpolation
    let input = r#""Hello ${name}!""#;
    let mut lexer = Lexer::new(input);
    let token = lexer.next_token();
    if let Token::StringLiteral(s) = token {
        assert!(s.contains('\x01') && s.contains('\x02'));
    } else {
        panic!("Expected string literal with interpolation");
    }
}

#[test]
fn test_comments() {
    // Test comments are skipped
    let tokens = tokenize("x = 10 // comment\ny = 20");
    assert_eq!(
        tokens,
        vec![
            Token::Identifier("x".to_string()),
            Token::Operator("=".to_string()),
            Token::Integer("10".to_string()),
            Token::Identifier("y".to_string()),
            Token::Operator("=".to_string()),
            Token::Integer("20".to_string())
        ]
    );

    let tokens = tokenize("x = /* block */ 10");
    assert_eq!(
        tokens,
        vec![
            Token::Identifier("x".to_string()),
            Token::Operator("=".to_string()),
            Token::Integer("10".to_string())
        ]
    );
}

#[test]
fn test_numeric_literals() {
    // Test float numbers
    let tokens = tokenize("3.14 0.5");
    assert_eq!(
        tokens,
        vec![
            Token::Float("3.14".to_string()),
            Token::Float("0.5".to_string())
        ]
    );
}
