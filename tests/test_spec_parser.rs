use zen::lexer::Lexer;
use zen::parser::Parser;

fn main() {
    println!("Testing Parser with LANGUAGE_SPEC.zen examples");

    // Test simple variable declarations from spec
    let test_cases = vec![
        ("x: i32", "Forward declaration"),
        ("x = 10", "Immutable assignment"),
        ("y = 10", "Immutable assignment without type"),
        ("z : i32 = 20", "Immutable assignment with type"),
        ("w :: i32", "Mutable forward declaration"),
        ("w = 20", "Assignment to forward declared"),
        ("v ::= 30", "Mutable assignment"),
        ("u :: i32 = 40", "Mutable assignment with type"),
    ];

    for (code, description) in test_cases {
        println!("\nTesting: {} - {}", code, description);
        let lexer = Lexer::new(code);
        let mut parser = Parser::new(lexer);

        match parser.parse_statement() {
            Ok(stmt) => println!("  ✓ Parsed: {:?}", stmt),
            Err(e) => println!("  ✗ Error: {:?}", e),
        }
    }

    // Test imports from spec
    let import_tests = vec![
        "{ io, maths } = @std",
        "{ String, StringBuilder } = @std",
        "{ GPA, AsyncPool, Allocator} = @std",
        "sdl2 = @std.import(\"sdl2\")",
    ];

    println!("\n\nTesting imports:");
    for code in import_tests {
        println!("\nTesting: {}", code);
        let lexer = Lexer::new(code);
        let mut parser = Parser::new(lexer);

        match parser.parse_statement() {
            Ok(stmt) => println!("  ✓ Parsed successfully"),
            Err(e) => println!("  ✗ Error: {:?}", e),
        }
    }

    // Test struct definitions
    let struct_test = r#"
Point: {
    x:: f64,
    y:: f64 = 0
}
"#;

    println!("\n\nTesting struct definition:");
    println!("{}", struct_test);
    let lexer = Lexer::new(struct_test);
    let mut parser = Parser::new(lexer);

    match parser.parse_statement() {
        Ok(stmt) => println!("  ✓ Parsed struct successfully"),
        Err(e) => println!("  ✗ Error: {:?}", e),
    }

    // Test enum type definition
    let enum_test = "Shape: Circle | Rectangle";

    println!("\n\nTesting enum definition:");
    println!("{}", enum_test);
    let lexer = Lexer::new(enum_test);
    let mut parser = Parser::new(lexer);

    match parser.parse_statement() {
        Ok(stmt) => println!("  ✓ Parsed enum successfully"),
        Err(e) => println!("  ✗ Error: {:?}", e),
    }

    // Test pattern matching
    let pattern_test = r#"
is_ready ?
    | true { process_data() }
    | false { io.println("Waiting...") }
"#;

    println!("\n\nTesting pattern matching:");
    println!("{}", pattern_test);
    let lexer = Lexer::new(pattern_test);
    let mut parser = Parser::new(lexer);

    match parser.parse_expression() {
        Ok(expr) => println!("  ✓ Parsed pattern match successfully"),
        Err(e) => println!("  ✗ Error: {:?}", e),
    }

    // Test function definition
    let func_test = r#"
area = (self) f64 {
    return math.pi * self.radius * self.radius
}
"#;

    println!("\n\nTesting function definition:");
    println!("{}", func_test);
    let lexer = Lexer::new(func_test);
    let mut parser = Parser::new(lexer);

    match parser.parse_statement() {
        Ok(stmt) => println!("  ✓ Parsed function successfully"),
        Err(e) => println!("  ✗ Error: {:?}", e),
    }

    // Test range iteration
    let range_test = "(0..10).loop((i) { io.println(i) })";

    println!("\n\nTesting range iteration:");
    println!("{}", range_test);
    let lexer = Lexer::new(range_test);
    let mut parser = Parser::new(lexer);

    match parser.parse_expression() {
        Ok(expr) => println!("  ✓ Parsed range iteration successfully"),
        Err(e) => println!("  ✗ Error: {:?}", e),
    }

    // Test Option types
    let option_test = "maybe_radius: Option<f64> = Some(5.5)";

    println!("\n\nTesting Option type:");
    println!("{}", option_test);
    let lexer = Lexer::new(option_test);
    let mut parser = Parser::new(lexer);

    match parser.parse_statement() {
        Ok(stmt) => println!("  ✓ Parsed Option type successfully"),
        Err(e) => println!("  ✗ Error: {:?}", e),
    }

    println!("\n\nDone testing parser!");
}
