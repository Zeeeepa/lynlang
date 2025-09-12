use zen::lexer::Lexer;
use zen::parser::Parser;
use zen::ast::{Expression, Pattern, ConditionalArm};

#[test]
fn test_bool_pattern_short_form() {
    // Test: expr ? { block }
    let input = "is_valid() ? { print(\"Valid!\") }";
    let lexer = Lexer::new(input);
    let mut parser = Parser::new(lexer);
    
    let expr = parser.parse_expression().unwrap();
    
    // Verify it's parsed as a conditional
    match expr {
        Expression::Conditional { scrutinee, arms } => {
            // Check scrutinee is the function call
            match scrutinee.as_ref() {
                Expression::FunctionCall { name, .. } => {
                    assert_eq!(name, "is_valid");
                }
                _ => panic!("Expected function call as scrutinee"),
            }
            
            // Should have 2 arms: true case and wildcard
            assert_eq!(arms.len(), 2);
            
            // First arm should match true
            match &arms[0].pattern {
                Pattern::Literal(Expression::Boolean(true)) => {},
                _ => panic!("Expected true pattern"),
            }
            
            // Second arm should be wildcard
            match &arms[1].pattern {
                Pattern::Wildcard => {},
                _ => panic!("Expected wildcard pattern"),
            }
        }
        _ => panic!("Expected conditional expression"),
    }
}

#[test]
fn test_bool_pattern_with_statements() {
    // Test: expr ? { multiple; statements; }
    let input = "condition ? { print(10) }";
    let lexer = Lexer::new(input);
    let mut parser = Parser::new(lexer);
    
    let expr = parser.parse_expression().unwrap();
    
    // Verify it's parsed as a conditional
    match expr {
        Expression::Conditional { scrutinee, arms } => {
            // Check scrutinee is an identifier
            match scrutinee.as_ref() {
                Expression::Identifier(name) => {
                    assert_eq!(name, "condition");
                }
                _ => panic!("Expected identifier as scrutinee"),
            }
            
            // Should have 2 arms
            assert_eq!(arms.len(), 2);
        }
        _ => panic!("Expected conditional expression"),
    }
}

#[test]
fn test_bool_pattern_alternative_syntaxes() {
    // Test the variations from the spec
    let variations = vec![
        "func() ? { do_something() }",
        "x > 5 ? { print(\"Greater\") }",
        "is_ready ? { process() }",
    ];
    
    for input in variations {
        let lexer = Lexer::new(input);
        let mut parser = Parser::new(lexer);
        
        let expr = parser.parse_expression();
        assert!(expr.is_ok(), "Failed to parse: {}", input);
        
        // Verify it's a conditional
        match expr.unwrap() {
            Expression::Conditional { .. } => {},
            _ => panic!("Expected conditional for: {}", input),
        }
    }
}

#[test]
fn test_bool_pattern_vs_standard_pattern() {
    // Test that both syntaxes work
    let bool_form = "is_valid() ? { print(\"Valid!\") }";
    let standard_form = "is_valid() ? | true => print(\"Valid!\") | false => {}";
    
    // Both should parse successfully
    let lexer1 = Lexer::new(bool_form);
    let mut parser1 = Parser::new(lexer1);
    assert!(parser1.parse_expression().is_ok());
    
    let lexer2 = Lexer::new(standard_form);
    let mut parser2 = Parser::new(lexer2);
    assert!(parser2.parse_expression().is_ok());
}