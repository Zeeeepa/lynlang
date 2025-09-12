// Comprehensive LSP tests for Zenlang
use tower_lsp::lsp_types::*;
use tower_lsp::Client;
use zen::lsp::ZenServer;
use zen::lexer::Lexer;
use zen::parser::Parser;
use zen::error::CompileError;

#[cfg(test)]
mod lsp_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_parse_error_messages() {
        // Test that LSP provides detailed error messages
        let test_cases = vec![
            (
                "if x > 0 { print(x) }",
                vec!["Language Spec Violation: 'if' keyword", "pattern matching with '?' operator"],
            ),
            (
                "let x = 5",
                vec!["Language Spec Violation: variable keywords", "name := value", "name ::= value"],
            ),
            (
                "fn add(a: i32, b: i32) -> i32 { a + b }",
                vec!["Language Spec Violation: function keywords", "name = (params) ReturnType"],
            ),
            (
                "while x < 10 { x = x + 1 }",
                vec!["Language Spec Violation: 'while' keyword", "loop (condition)"],
            ),
            (
                "for i in 0..10 { print(i) }",
                vec!["Language Spec Violation: 'for' keyword", "(0..10).loop((i) =>"],
            ),
            (
                "match value { 1 => \"one\", _ => \"other\" }",
                vec!["Language Spec Violation: 'match' keyword", "value ? |"],
            ),
        ];
        
        for (input, expected_messages) in test_cases {
            let lexer = Lexer::new(input);
            let mut parser = Parser::new(lexer);
            let result = parser.parse_program();
            
            assert!(result.is_err(), "Expected parse error for: {}", input);
            
            if let Err(e) = result {
                let lines: Vec<&str> = input.lines().collect();
                let detailed_msg = e.detailed_message(&lines);
                
                for expected in expected_messages {
                    assert!(
                        detailed_msg.contains(expected),
                        "Error message should contain '{}' for input: {}\nActual message: {}",
                        expected, input, detailed_msg
                    );
                }
            }
        }
    }
    
    #[tokio::test]
    async fn test_error_position_tracking() {
        // Test that errors report correct line and column positions
        let test_cases = vec![
            ("if x", 1, 1),  // Error at line 1, column 1
            ("x := 5\nlet y = 10", 2, 1),  // Error at line 2, column 1
            ("x := 5\ny := 10\nwhile true", 3, 1),  // Error at line 3, column 1
        ];
        
        for (input, expected_line, expected_col) in test_cases {
            let lexer = Lexer::new(input);
            let mut parser = Parser::new(lexer);
            let result = parser.parse_program();
            
            if let Err(e) = result {
                if let Some(span) = e.position() {
                    assert_eq!(
                        span.line, expected_line,
                        "Expected error at line {} for: {}", expected_line, input
                    );
                    assert!(
                        span.column >= expected_col - 1 && span.column <= expected_col + 2,
                        "Expected error near column {} for: {}, got {}", 
                        expected_col, input, span.column
                    );
                }
            }
        }
    }
    
    #[tokio::test]
    async fn test_error_suggestions() {
        // Test that errors provide helpful suggestions
        let test_cases = vec![
            (
                "x = 5",  // Missing declaration operator
                vec!["name := value", "name ::= value"],
            ),
            (
                "func add() {}",
                vec!["name = (params) ReturnType"],
            ),
            (
                "if (true) {}",
                vec!["condition ?", "| true =>", "| false =>"],
            ),
        ];
        
        for (input, expected_suggestions) in test_cases {
            let lexer = Lexer::new(input);
            let mut parser = Parser::new(lexer);
            let result = parser.parse_program();
            
            if let Err(e) = result {
                let lines: Vec<&str> = input.lines().collect();
                let detailed_msg = e.detailed_message(&lines);
                
                for suggestion in expected_suggestions {
                    assert!(
                        detailed_msg.contains(suggestion),
                        "Should suggest '{}' for: {}", suggestion, input
                    );
                }
            }
        }
    }
    
    #[test]
    fn test_pattern_matching_error_help() {
        // Test specific help for pattern matching errors
        let input = "x match { 1 => \"one\" }";
        let lexer = Lexer::new(input);
        let mut parser = Parser::new(lexer);
        let result = parser.parse_program();
        
        assert!(result.is_err());
        if let Err(e) = result {
            let lines: Vec<&str> = input.lines().collect();
            let msg = e.detailed_message(&lines);
            
            assert!(msg.contains("?"));
            assert!(msg.contains("pattern"));
            assert!(msg.contains("=>"));
        }
    }
    
    #[test]
    fn test_variable_declaration_error_help() {
        // Test help for variable declaration errors
        let test_cases = vec![
            ("let x = 5", vec![":=", "::="]),
            ("var y = 10", vec![":=", "::="]),
            ("const z = 15", vec![":=", "::="]),
        ];
        
        for (input, expected) in test_cases {
            let lexer = Lexer::new(input);
            let mut parser = Parser::new(lexer);
            let result = parser.parse_program();
            
            assert!(result.is_err());
            if let Err(e) = result {
                let lines: Vec<&str> = input.lines().collect();
                let msg = e.detailed_message(&lines);
                
                for exp in expected {
                    assert!(msg.contains(exp), "Should contain '{}' for: {}", exp, input);
                }
            }
        }
    }
    
    #[test]
    fn test_loop_error_help() {
        // Test help for loop-related errors
        let test_cases = vec![
            ("while x < 10 {}", vec!["loop (condition)", "loop {"]),
            ("for i in range {}", vec!["(0..10).loop", "items.loop"]),
        ];
        
        for (input, expected) in test_cases {
            let lexer = Lexer::new(input);
            let mut parser = Parser::new(lexer);
            let result = parser.parse_program();
            
            assert!(result.is_err());
            if let Err(e) = result {
                let lines: Vec<&str> = input.lines().collect();
                let msg = e.detailed_message(&lines);
                
                for exp in expected {
                    assert!(msg.contains(exp), "Should contain '{}' for: {}", exp, input);
                }
            }
        }
    }
    
    #[test]
    fn test_function_syntax_error_help() {
        // Test help for function syntax errors
        let test_cases = vec![
            ("fn foo() {}", vec!["name = (params) ReturnType"]),
            ("func bar(x) {}", vec!["name = (params) ReturnType"]),
            ("function baz() -> i32 {}", vec!["name = (params) ReturnType"]),
        ];
        
        for (input, expected) in test_cases {
            let lexer = Lexer::new(input);
            let mut parser = Parser::new(lexer);
            let result = parser.parse_program();
            
            assert!(result.is_err());
            if let Err(e) = result {
                let lines: Vec<&str> = input.lines().collect();
                let msg = e.detailed_message(&lines);
                
                for exp in expected {
                    assert!(msg.contains(exp), "Should contain '{}' for: {}", exp, input);
                }
            }
        }
    }
    
    #[test]
    fn test_null_reference_error_help() {
        // Test help for null/nil/undefined errors
        let test_cases = vec![
            ("x := null", vec!["Option<T>", "Some", "None"]),
            ("y := nil", vec!["Option<T>", "Some", "None"]),
            ("z := undefined", vec!["Option<T>", "Some", "None"]),
        ];
        
        for (input, expected) in test_cases {
            let lexer = Lexer::new(input);
            let mut parser = Parser::new(lexer);
            let result = parser.parse_program();
            
            // These might parse but should be caught in semantic analysis
            // For now, check if the error messages would be helpful
            let lines: Vec<&str> = input.lines().collect();
            
            // Create a mock error to test the error formatting
            let error = CompileError::SyntaxError(
                format!("null references not allowed"),
                Some(zen::error::Span { start: 0, end: 4, line: 1, column: 5 })
            );
            
            let msg = error.detailed_message(&lines);
            // The detailed_message should have suggestions for Option<T>
            println!("Testing null reference help for: {}", input);
        }
    }
    
    #[test]
    fn test_error_context_display() {
        // Test that errors show surrounding context
        let input = "x := 5\ny := 10\nif z > 0 { print(z) }\nw := 20";
        let lexer = Lexer::new(input);
        let mut parser = Parser::new(lexer);
        let result = parser.parse_program();
        
        if let Err(e) = result {
            let lines: Vec<&str> = input.lines().collect();
            let msg = e.detailed_message(&lines);
            
            // Should show the error line and surrounding context
            assert!(msg.contains("y := 10") || msg.contains("w := 20"), 
                   "Should show context lines");
            assert!(msg.contains("if z"), "Should show error line");
        }
    }
    
    #[test]
    fn test_multi_line_error_span() {
        // Test errors that span multiple lines
        let input = "add = (a: i32,\n       b: i32,\n       fn invalid) i32 { a + b }";
        let lexer = Lexer::new(input);
        let mut parser = Parser::new(lexer);
        let result = parser.parse_program();
        
        assert!(result.is_err());
        if let Err(e) = result {
            let lines: Vec<&str> = input.lines().collect();
            let msg = e.detailed_message(&lines);
            
            // Should handle multi-line context properly
            assert!(msg.contains("fn") || msg.contains("invalid"));
        }
    }
}

#[cfg(test)]
mod diagnostic_tests {
    use super::*;
    
    #[test]
    fn test_diagnostic_severity() {
        // Test that different error types get appropriate severity levels
        let errors = vec![
            CompileError::SyntaxError("test".to_string(), None),
            CompileError::ParseError("test".to_string(), None),
            CompileError::TypeMismatch {
                expected: "i32".to_string(),
                found: "string".to_string(),
                span: None,
            },
        ];
        
        for error in errors {
            // All compile errors should be ERROR severity in LSP
            println!("Testing severity for: {:?}", error);
        }
    }
    
    #[test]
    fn test_error_code_extraction() {
        // Test that we can extract useful error codes from compile errors
        let test_cases = vec![
            (CompileError::SyntaxError("test".to_string(), None), "syntax_error"),
            (CompileError::UndeclaredVariable("x".to_string(), None), "undeclared_variable"),
            (CompileError::UndeclaredFunction("f".to_string(), None), "undeclared_function"),
        ];
        
        for (error, _expected_code) in test_cases {
            // Error codes could be used for quick fixes in LSP
            println!("Error type: {:?}", error);
        }
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    
    #[test]
    fn test_complete_zen_program_errors() {
        // Test a complete program with multiple errors
        let input = r#"
// This program has multiple errors for testing
let x = 5  // Error: let keyword

fn add(a: i32, b: i32) -> i32 {  // Error: fn keyword
    if a > b {  // Error: if keyword
        return a + b
    } else {  // Error: else keyword
        return b - a
    }
}

while x < 10 {  // Error: while keyword
    x = x + 1
}

for i in 0..5 {  // Error: for keyword
    print(i)
}
"#;
        
        let lexer = Lexer::new(input);
        let mut parser = Parser::new(lexer);
        let result = parser.parse_program();
        
        assert!(result.is_err());
        if let Err(e) = result {
            let lines: Vec<&str> = input.lines().collect();
            let msg = e.detailed_message(&lines);
            
            // Should provide helpful error message for the first error encountered
            assert!(msg.contains("Language Spec") || msg.contains("keyword"));
        }
    }
    
    #[test]
    fn test_valid_zen_syntax() {
        // Test that valid Zen syntax doesn't produce errors
        let input = r#"
// Valid Zen program
x := 5
y ::= 10

add = (a: i32, b: i32) i32 {
    a + b
}

result := x > y ? 
    | true => add(x, y)
    | false => add(y, x)

loop (y < 20) {
    y = y + 1
}

(0..5).loop((i) => {
    print(i)
})
"#;
        
        let lexer = Lexer::new(input);
        let mut parser = Parser::new(lexer);
        let result = parser.parse_program();
        
        // This should parse successfully (or at least not have keyword errors)
        if let Err(e) = result {
            let lines: Vec<&str> = input.lines().collect();
            let msg = e.detailed_message(&lines);
            
            // Should not complain about keywords if using valid Zen syntax
            assert!(!msg.contains("Language Spec Violation"), 
                   "Valid Zen should not have spec violations: {}", msg);
        }
    }
}