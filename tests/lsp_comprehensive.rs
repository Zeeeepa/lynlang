// Comprehensive LSP tests for enhanced error reporting

use zen::error::{CompileError, Span};

#[test]
fn test_detailed_error_message_with_context() {
    let source = "fn main() {\n    let x = 42\n    y = x + 1\n}";
    let lines: Vec<&str> = source.lines().collect();
    
    let error = CompileError::UndeclaredVariable(
        "y".to_string(),
        Some(Span {
            start: 29,
            end: 30,
            line: 3,
            column: 4,
        }),
    );
    
    let message = error.detailed_message(&lines);
    
    // Check for location context
    assert!(message.contains("📍 Error Location:"));
    assert!(message.contains("y = x + 1"));
    assert!(message.contains("^"));
    
    // Check for suggestions
    assert!(message.contains("Did you mean to declare 'y'?"));
    assert!(message.contains(":="));
    assert!(message.contains("::="));
}

#[test]
fn test_syntax_error_with_if_else_suggestion() {
    let source = "if (condition) {\n    do_something()\n}";
    let lines: Vec<&str> = source.lines().collect();
    
    let error = CompileError::SyntaxError(
        "unexpected 'if' keyword".to_string(),
        Some(Span {
            start: 0,
            end: 2,
            line: 1,
            column: 0,
        }),
    );
    
    let message = error.detailed_message(&lines);
    
    // Check for Zen-specific suggestions
    assert!(message.contains("💡 Suggestions:"));
    assert!(message.contains("Zen uses the '?' operator"));
    assert!(message.contains("condition ? { do_something() }"));
}

#[test]
fn test_pattern_matching_error_help() {
    let source = "match value {\n    1 => println(\"one\"),\n    _ => println(\"other\")\n}";
    let lines: Vec<&str> = source.lines().collect();
    
    let error = CompileError::InvalidPattern(
        "invalid pattern syntax".to_string(),
        Some(Span {
            start: 0,
            end: 5,
            line: 1,
            column: 0,
        }),
    );
    
    let message = error.detailed_message(&lines);
    
    assert!(message.contains("Zen pattern matching syntax:"));
    assert!(message.contains("value ? | pattern1 => result1"));
    assert!(message.contains("Destructuring: .Ok -> val"));
}

#[test]
fn test_import_error_in_comptime() {
    let source = "comptime {\n    io := @std.build.import(\"io\")\n}";
    let lines: Vec<&str> = source.lines().collect();
    
    let error = CompileError::ImportError(
        "import in comptime block".to_string(),
        Some(Span {
            start: 15,
            end: 45,
            line: 2,
            column: 10,
        }),
    );
    
    let message = error.detailed_message(&lines);
    
    assert!(message.contains("Imports are not allowed inside comptime blocks"));
    assert!(message.contains("Move import statements to module level"));
}

#[test]
fn test_ffi_error_with_builder_example() {
    let source = "lib := FFI.lib(\"mylib\").build()";
    let lines: Vec<&str> = source.lines().collect();
    
    let error = CompileError::FFIError(
        "missing library path".to_string(),
        Some(Span {
            start: 7,
            end: 32,
            line: 1,
            column: 7,
        }),
    );
    
    let message = error.detailed_message(&lines);
    
    assert!(message.contains("FFI usage in Zen:"));
    assert!(message.contains(".path(\"/path/to/library\")"));
    assert!(message.contains(".function(\"func_name\", signature)"));
}

#[test]
fn test_type_mismatch_with_conversion_hint() {
    let source = "let x: i32 = 3.14";
    let lines: Vec<&str> = source.lines().collect();
    
    let error = CompileError::TypeMismatch {
        expected: "i32".to_string(),
        found: "f64".to_string(),
        span: Some(Span {
            start: 13,
            end: 17,
            line: 1,
            column: 13,
        }),
    };
    
    let message = error.detailed_message(&lines);
    
    assert!(message.contains("Type conversion needed:"));
    assert!(message.contains("Expected: i32"));
    assert!(message.contains("Found: f64"));
    assert!(message.contains("Try: value as i32"));
}

#[test]
fn test_unexpected_token_with_hints() {
    let source = "fn main() \n    println(\"hello\")";
    let lines: Vec<&str> = source.lines().collect();
    
    let error = CompileError::UnexpectedToken {
        expected: vec!["{".to_string()],
        found: "println".to_string(),
        span: Some(Span {
            start: 14,
            end: 21,
            line: 2,
            column: 4,
        }),
    };
    
    let message = error.detailed_message(&lines);
    
    assert!(message.contains("Expected tokens:"));
    assert!(message.contains("- {"));
    assert!(message.contains("Found: 'println'"));
    assert!(message.contains("Check for unclosed braces"));
}

#[test]
fn test_loop_condition_error() {
    let source = "loop \"invalid\" {\n    break\n}";
    let lines: Vec<&str> = source.lines().collect();
    
    let error = CompileError::InvalidLoopCondition(
        "condition must be boolean".to_string(),
        Some(Span {
            start: 5,
            end: 14,
            line: 1,
            column: 5,
        }),
    );
    
    let message = error.detailed_message(&lines);
    
    assert!(message.contains("Valid loop forms:"));
    assert!(message.contains("loop { ... }           // infinite loop"));
    assert!(message.contains("loop (condition) { ... } // conditional loop"));
    assert!(message.contains("Loop condition must evaluate to bool"));
}

#[test]
fn test_missing_return_statement() {
    let source = "fn calculate() i32 {\n    let x = 10\n    let y = 20\n}";
    let lines: Vec<&str> = source.lines().collect();
    
    let error = CompileError::MissingReturnStatement(
        "calculate".to_string(),
        Some(Span {
            start: 0,
            end: 19,
            line: 1,
            column: 0,
        }),
    );
    
    let message = error.detailed_message(&lines);
    
    assert!(message.contains("Function 'calculate' must return a value"));
    assert!(message.contains("Add explicit return: return value"));
    assert!(message.contains("Use implicit return (no semicolon on last expression)"));
}

#[test]
fn test_comptime_error_with_requirements() {
    let error = CompileError::ComptimeError(
        "side effects detected in comptime block".to_string(),
    );
    
    let lines: Vec<&str> = vec![];
    let message = error.detailed_message(&lines);
    
    assert!(message.contains("💡 Comptime Requirements:"));
    assert!(message.contains("Code must be deterministic"));
    assert!(message.contains("No side effects allowed"));
    assert!(message.contains("Only pure computations"));
}

#[test]
fn test_duplicate_declaration_error() {
    let source = "let x = 10\nlet y = 20\nlet x = 30";
    let lines: Vec<&str> = source.lines().collect();
    
    let error = CompileError::DuplicateDeclaration {
        name: "x".to_string(),
        first_location: Some(Span {
            start: 4,
            end: 5,
            line: 1,
            column: 4,
        }),
        duplicate_location: Some(Span {
            start: 26,
            end: 27,
            line: 3,
            column: 4,
        }),
    };
    
    let message = error.detailed_message(&lines);
    
    assert!(message.contains("'x' has already been declared"));
    assert!(message.contains("Use a different name"));
    assert!(message.contains("Remove the duplicate declaration"));
}

#[test]
fn test_multiline_error_span() {
    let source = "fn test() {\n    let x = (\n        1 + 2 +\n        3 + 4\n    \n}";
    let lines: Vec<&str> = source.lines().collect();
    
    let error = CompileError::SyntaxError(
        "unexpected end of expression".to_string(),
        Some(Span {
            start: 40,
            end: 55,
            line: 3,
            column: 8,
        }),
    );
    
    let message = error.detailed_message(&lines);
    
    // Should show context with surrounding lines
    assert!(message.contains("📍 Error Location:"));
    assert!(message.contains("1 + 2 +"));
    assert!(message.contains("3 + 4"));
}

#[test]
fn test_function_syntax_help() {
    let source = "function greet() {\n    print(\"hello\")\n}";
    let lines: Vec<&str> = source.lines().collect();
    
    let error = CompileError::SyntaxError(
        "invalid function declaration".to_string(),
        Some(Span {
            start: 0,
            end: 8,
            line: 1,
            column: 0,
        }),
    );
    
    let message = error.detailed_message(&lines);
    
    assert!(message.contains("Function syntax in Zen"));
    assert!(message.contains("name = (params) ReturnType { body }"));
    assert!(message.contains("add = (a: i32, b: i32) i32"));
    assert!(message.contains("No parameters: greet = () void"));
}

#[test]
fn test_invalid_syntax_with_custom_suggestion() {
    let source = "var x = 10";
    let lines: Vec<&str> = source.lines().collect();
    
    let error = CompileError::InvalidSyntax {
        message: "invalid variable declaration".to_string(),
        suggestion: "Use ':=' for immutable or '::=' for mutable variables".to_string(),
        span: Some(Span {
            start: 0,
            end: 3,
            line: 1,
            column: 0,
        }),
    };
    
    let message = error.detailed_message(&lines);
    
    assert!(message.contains("💡 Suggestion:"));
    assert!(message.contains("Use ':=' for immutable or '::=' for mutable"));
}

#[test]
fn test_missing_type_annotation() {
    let source = "let x =";
    let lines: Vec<&str> = source.lines().collect();
    
    let error = CompileError::MissingTypeAnnotation(
        "x".to_string(),
        Some(Span {
            start: 4,
            end: 5,
            line: 1,
            column: 4,
        }),
    );
    
    let message = error.detailed_message(&lines);
    
    assert!(message.contains("Variable 'x' needs a type annotation"));
    assert!(message.contains("Use type inference: name := value"));
    assert!(message.contains("Explicit type: name: Type = value"));
}