use zen::parser::Parser;
use zen::lexer::Lexer;
use zen::ast::*;
use zen::error::CompileError;

fn parse_code(code: &str) -> Result<Program, CompileError> {
    let lexer = Lexer::new(code);
    let mut parser = Parser::new(lexer);
    parser.parse_program()
}

#[test]
fn test_basic_expressions() {
    let code = "x = 42";
    let result = parse_code(code);
    assert!(result.is_ok(), "Failed to parse basic assignment: {:?}", result.err());
}

#[test]
fn test_function_declarations() {
    let code = r#"
        add = (a: i32, b: i32) i32 {
            return a + b
        }
    "#;
    let result = parse_code(code);
    assert!(result.is_ok(), "Failed to parse function: {:?}", result.err());
}

#[test]
fn test_struct_declarations() {
    let code = r#"
        Point = struct {
            x: i32,
            y: i32
        }
    "#;
    let result = parse_code(code);
    assert!(result.is_ok(), "Failed to parse struct: {:?}", result.err());
}

#[test]
fn test_option_patterns() {
    let code = r#"
        val = Some(42)
        val ?
            | Some(x) { println("Got: ${x}") }
            | None { println("Empty") }
    "#;
    let result = parse_code(code);
    assert!(result.is_ok(), "Failed to parse option pattern: {:?}", result.err());
}

#[test]
fn test_result_patterns() {
    let code = r#"
        result = Ok(42)
        result ?
            | Ok(x) { println("Success: ${x}") }
            | Err(e) { println("Error: ${e}") }
    "#;
    let result = parse_code(code);
    assert!(result.is_ok(), "Failed to parse result pattern: {:?}", result.err());
}

#[test]
fn test_range_expressions() {
    let code = "for i in 0..10 { println(\"${i}\") }";
    let result = parse_code(code);
    assert!(result.is_ok(), "Failed to parse range expression: {:?}", result.err());
}

#[test]
fn test_generic_types() {
    let code = r#"
        Option = enum<T> {
            Some(T),
            None
        }
    "#;
    let result = parse_code(code);
    assert!(result.is_ok(), "Failed to parse generic enum: {:?}", result.err());
}

#[test]
fn test_method_calls() {
    let code = "shape.area()";
    let result = parse_code(code);
    assert!(result.is_ok(), "Failed to parse method call: {:?}", result.err());
}

#[test]
fn test_string_interpolation() {
    let code = r#"println("Hello ${name}!")"#;
    let result = parse_code(code);
    assert!(result.is_ok(), "Failed to parse string interpolation: {:?}", result.err());
}

#[test]
fn test_complex_expressions() {
    let code = r#"
        result = if x > 0 {
            Some(x * 2)
        } else {
            None
        }
    "#;
    let result = parse_code(code);
    assert!(result.is_ok(), "Failed to parse complex expression: {:?}", result.err());
}
