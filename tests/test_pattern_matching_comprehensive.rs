use zen::lexer::Lexer;
use zen::parser::Parser;
use zen::ast::{Expression, Statement, Pattern, BinaryOperator, Declaration};

#[test]
fn test_simple_value_pattern() {
    let code = r#"
        x := 42
        result := x ? | 42 => "found" | _ => "not found"
    "#;
    
    let lexer = Lexer::new(code);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program().unwrap();
    
    // The parser correctly parses these as declarations (variable assignments)
    assert_eq!(program.declarations.len(), 2);
    assert_eq!(program.statements.len(), 0);
    
    // Verify pattern matching is parsed correctly
    if let Some(Declaration::Constant { value: Expression::Conditional { arms, .. }, .. }) = program.declarations.get(1) {
        assert_eq!(arms.len(), 2);
        // First arm should match literal 42
        assert!(matches!(arms[0].pattern, Pattern::Literal(_)));
        // Second arm should be wildcard
        assert!(matches!(arms[1].pattern, Pattern::Wildcard));
    } else {
        panic!("Expected conditional expression in second declaration");
    }
}

#[test]
fn test_range_pattern() {
    let code = r#"
        age := 25
        category := age ? | 0..=12 => "child"
                          | 13..=19 => "teen"
                          | 20..=64 => "adult"
                          | _ => "senior"
    "#;
    
    let lexer = Lexer::new(code);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program().unwrap();
    
    // These are declarations (variable assignments), not statements
    assert_eq!(program.declarations.len(), 2);
    
    // Check the second declaration has pattern matching
    if let Some(Declaration::Constant { value: Expression::Conditional { arms, .. }, .. }) = program.declarations.get(1) {
        assert_eq!(arms.len(), 4); // Four arms: 0..=12, 13..=19, 20..=64, _
        
        // Check patterns (simplified for now - just verify they parsed)
        println!("Parsed {} pattern match arms", arms.len());
        for (i, arm) in arms.iter().enumerate() {
            println!("Arm {}: {:?}", i, arm.pattern);
        }
    } else {
        panic!("Expected conditional expression in second declaration");
    }
}

#[test]
fn test_enum_destructuring_pattern() {
    let code = r#"
        result := value ? | .Ok -> val => process(val)
                          | .Err -> err => handle_error(err)
    "#;
    
    let lexer = Lexer::new(code);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program().unwrap();
    
    assert_eq!(program.declarations.len(), 1);
    
    // Verify pattern matching with enum destructuring
    if let Some(Declaration::Constant { value: Expression::Conditional { arms, .. }, .. }) = program.declarations.get(0) {
        assert_eq!(arms.len(), 2); // Two arms: .Ok and .Err
    } else {
        panic!("Expected conditional expression in first declaration");
    }
}

#[test]
fn test_guard_pattern() {
    let code = r#"
        result := value ? | v -> v > 100 => "large"
                          | v -> v > 50 => "medium"
                          | _ => "small"
    "#;
    
    let lexer = Lexer::new(code);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program().unwrap();
    
    assert_eq!(program.declarations.len(), 1);
    
    // Verify guard patterns
    if let Some(Declaration::Constant { value: Expression::Conditional { arms, .. }, .. }) = program.declarations.get(0) {
        assert_eq!(arms.len(), 3); // Three arms with guards
    } else {
        panic!("Expected conditional expression");
    }
}

#[test]
fn test_multiple_patterns() {
    let code = r#"
        day_type := day ? | 1 | 2 | 3 | 4 | 5 => "weekday"
                          | 6 | 7 => "weekend"
                          | _ => "invalid"
    "#;
    
    let lexer = Lexer::new(code);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program().unwrap();
    
    assert_eq!(program.declarations.len(), 1);
    
    // Verify multiple patterns (or-patterns)
    if let Some(Declaration::Constant { value: Expression::Conditional { arms, .. }, .. }) = program.declarations.get(0) {
        assert_eq!(arms.len(), 3); // Three arms: weekdays, weekend, invalid
    } else {
        panic!("Expected conditional expression");
    }
}

#[test]
#[ignore] // TODO: Parser needs updates for pattern matching syntax per Language Spec v1.1.0
fn test_bool_pattern_short_form() {
    let code = r#"
        is_valid() ? { do_something() }
    "#;
    
    let lexer = Lexer::new(code);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program().unwrap();
    
    assert_eq!(program.statements.len(), 1);
}

#[test]
#[ignore] // TODO: Parser needs updates for pattern matching syntax per Language Spec v1.1.0
fn test_nested_pattern_matching() {
    let code = r#"
        result := outer ? 
            | .Some -> inner => inner ? 
                | 0 => "zero"
                | 1 => "one"
                | _ => "other"
            | .None => "nothing"
    "#;
    
    let lexer = Lexer::new(code);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program().unwrap();
    
    assert_eq!(program.statements.len(), 1);
}

#[test]
#[ignore] // TODO: Parser needs updates for pattern matching syntax per Language Spec v1.1.0
fn test_struct_destructuring_pattern() {
    let code = r#"
        result := point ? | { x -> xval, y -> yval } => format_point(xval, yval)
                          | _ => "invalid"
    "#;
    
    let lexer = Lexer::new(code);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program().unwrap();
    
    assert_eq!(program.statements.len(), 1);
}

#[test]
#[ignore] // TODO: Parser needs updates for pattern matching syntax per Language Spec v1.1.0
fn test_type_pattern() {
    let code = r#"
        result := value ? | i32 -> n => format_int(n)
                          | string -> s => format_string(s)
                          | Point -> p => format_point(p)
                          | _ => "unknown"
    "#;
    
    let lexer = Lexer::new(code);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program().unwrap();
    
    assert_eq!(program.statements.len(), 1);
}

#[test]
#[ignore] // TODO: Parser needs updates for pattern matching syntax per Language Spec v1.1.0
fn test_pattern_in_function() {
    let code = r#"
        factorial = (n: u64) u64 {
            n <= 1 ? 
                | true => return 1
                | false => return n * factorial(n - 1)
        }
    "#;
    
    let lexer = Lexer::new(code);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program().unwrap();
    
    assert_eq!(program.declarations.len(), 1);
}

#[test]
#[ignore] // TODO: Parser needs updates for pattern matching syntax per Language Spec v1.1.0
fn test_pattern_with_blocks() {
    let code = r#"
        x := value ? 
            | 1 => {
                print("one")
                calculate_one()
            }
            | 2 => {
                print("two")
                calculate_two()
            }
            | _ => {
                print("other")
                default_value()
            }
    "#;
    
    let lexer = Lexer::new(code);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program().unwrap();
    
    assert_eq!(program.statements.len(), 1);
}

#[test]
#[ignore] // TODO: Parser needs updates for pattern matching syntax per Language Spec v1.1.0
fn test_no_if_else_keywords() {
    // This should fail if we try to use if/else
    let code = r#"
        if x > 0 {
            print("positive")
        } else {
            print("non-positive")
        }
    "#;
    
    let lexer = Lexer::new(code);
    let mut parser = Parser::new(lexer);
    let result = parser.parse_program();
    
    // Should fail to parse 'if' as it's not a valid keyword
    assert!(result.is_err());
}

#[test]
#[ignore] // TODO: Parser needs updates for pattern matching syntax per Language Spec v1.1.0
fn test_pattern_match_with_continue_break() {
    let code = r#"
        loop {
            input := get_input()
            input ? 
                | "quit" => break
                | "skip" => continue
                | cmd => process(cmd)
        }
    "#;
    
    let lexer = Lexer::new(code);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program().unwrap();
    
    assert_eq!(program.statements.len(), 1);
}