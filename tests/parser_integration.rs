use zen::ast::*;
use zen::error::CompileError;
use zen::lexer::Lexer;
use zen::parser::Parser;

fn parse_code(code: &str) -> Result<Program, CompileError> {
    let lexer = Lexer::new(code);
    let mut parser = Parser::new(lexer);
    parser.parse_program()
}

#[test]
fn test_basic_expressions() {
    let code = "x = 42";
    let result = parse_code(code);
    assert!(
        result.is_ok(),
        "Failed to parse basic assignment: {:?}",
        result.err()
    );

    let program = result.unwrap();
    assert!(
        !program.declarations.is_empty() || !program.statements.is_empty(),
        "Program should contain declarations or statements"
    );
}

#[test]
fn test_function_declarations() {
    let code = r#"
        add = (a: i32, b: i32) i32 {
            return a + b
        }
    "#;
    let result = parse_code(code);
    assert!(
        result.is_ok(),
        "Failed to parse function: {:?}",
        result.err()
    );

    let program = result.unwrap();
    assert!(
        !program.declarations.is_empty(),
        "Should have function declaration"
    );
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

    let program = result.unwrap();
    assert!(
        !program.declarations.is_empty(),
        "Should have struct declaration"
    );
}

#[test]
fn test_option_patterns() {
    let code = r#"
        { Option, io } = @std
        test = () void {
            val = Option.Some(42)
            val ?
                | .Some(x) { io.println("Got: ${x}") }
                | .None { io.println("Empty") }
        }
    "#;
    let result = parse_code(code);
    assert!(
        result.is_ok(),
        "Failed to parse option pattern: {:?}",
        result.err()
    );
}

#[test]
fn test_result_patterns() {
    let code = r#"
        { Result, io } = @std
        test = () void {
            result = Result.Ok(42)
            result ?
                | .Ok(x) { io.println("Success: ${x}") }
                | .Err(e) { io.println("Error: ${e}") }
        }
    "#;
    let result = parse_code(code);
    assert!(
        result.is_ok(),
        "Failed to parse result pattern: {:?}",
        result.err()
    );
}

#[test]
fn test_range_expressions() {
    let code = r#"{ io } = @std
        test = () void {
            range = 0..10
            io.println("Range created")
        }
    "#;
    let result = parse_code(code);
    assert!(
        result.is_ok(),
        "Failed to parse range expression: {:?}",
        result.err()
    );
}

#[test]
fn test_generic_types() {
    let code = r#"
        Color: .Red, .Green, .Blue
    "#;
    let result = parse_code(code);
    assert!(result.is_ok(), "Failed to parse enum: {:?}", result.err());
}

#[test]
fn test_method_calls() {
    let code = r#"{ io } = @std
        test = () void {
            io.println("Hello")
        }
    "#;
    let result = parse_code(code);
    assert!(
        result.is_ok(),
        "Failed to parse method call: {:?}",
        result.err()
    );
}

#[test]
fn test_string_interpolation() {
    let code = r#"{ io } = @std
        test = () void {
            name = "World"
            io.println("Hello ${name}!")
        }
    "#;
    let result = parse_code(code);
    assert!(
        result.is_ok(),
        "Failed to parse string interpolation: {:?}",
        result.err()
    );
}

#[test]
fn test_complex_expressions() {
    let code = r#"
        add = (a: i32, b: i32) i32 {
            result = a + b * 2
            return result
        }
    "#;
    let result = parse_code(code);
    assert!(
        result.is_ok(),
        "Failed to parse complex expression: {:?}",
        result.err()
    );
}
