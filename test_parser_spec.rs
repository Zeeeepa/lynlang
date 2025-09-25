// Test file for spec-compliant parser
use zen::ast::{Declaration, Expression, Program, Statement};
use zen::lexer::Lexer;
use zen::parser::Parser;

fn main() {
    // Test parsing spec-compliant code examples

    // Test 1: Assignment operators (no keywords!)
    let code = r#"
        x = 10      // Immutable
        y := 20     // Immutable with explicit operator
        z ::= 30    // Mutable
        w :: i32 = 40  // Mutable with type
    "#;

    println!("Testing assignment operators...");
    let lexer = Lexer::new(code);
    let mut parser = Parser::new(lexer);
    match parser.parse_program() {
        Ok(_) => println!("✓ Assignment operators parsed successfully"),
        Err(e) => println!("✗ Failed to parse assignments: {:?}", e),
    }

    // Test 2: @std imports
    let code = r#"
        { io, math } = @std
        Build = @std
        sdl2 = @std.import("sdl2")
    "#;

    println!("\nTesting @std imports...");
    let lexer = Lexer::new(code);
    let mut parser = Parser::new(lexer);
    match parser.parse_program() {
        Ok(_) => println!("✓ @std imports parsed successfully"),
        Err(e) => println!("✗ Failed to parse imports: {:?}", e),
    }

    // Test 3: Pattern matching with ? (no match keyword!)
    let code = r#"
        result ?
            | Ok(val) { io.println(val) }
            | Err(e) { io.println(e) }
    "#;

    println!("\nTesting pattern matching...");
    let lexer = Lexer::new(code);
    let mut parser = Parser::new(lexer);
    match parser.parse_program() {
        Ok(_) => println!("✓ Pattern matching parsed successfully"),
        Err(e) => println!("✗ Failed to parse pattern matching: {:?}", e),
    }

    // Test 4: loop() syntax (no loop keyword, it's a function!)
    let code = r#"
        loop(() {
            counter = counter + 1
        })
    "#;

    println!("\nTesting loop() function syntax...");
    let lexer = Lexer::new(code);
    let mut parser = Parser::new(lexer);
    match parser.parse_program() {
        Ok(_) => println!("✓ loop() function parsed successfully"),
        Err(e) => println!("✗ Failed to parse loop(): {:?}", e),
    }

    // Test 5: Collection loop with .loop() method
    let code = r#"
        (0..10).loop((i) {
            io.println(i)
        })
    "#;

    println!("\nTesting collection .loop() method...");
    let lexer = Lexer::new(code);
    let mut parser = Parser::new(lexer);
    match parser.parse_program() {
        Ok(_) => println!("✓ Collection .loop() parsed successfully"),
        Err(e) => println!("✗ Failed to parse .loop(): {:?}", e),
    }

    // Test 6: Structs and enums
    let code = r#"
        Point: {
            x: f64,
            y: f64 = 0
        }
        
        Shape: Circle | Rectangle
        
        Option<T>: Some(T) | None
    "#;

    println!("\nTesting structs and enums...");
    let lexer = Lexer::new(code);
    let mut parser = Parser::new(lexer);
    match parser.parse_program() {
        Ok(_) => println!("✓ Structs and enums parsed successfully"),
        Err(e) => println!("✗ Failed to parse structs/enums: {:?}", e),
    }

    // Test 7: @this.defer()
    let code = r#"
        @this.defer(cleanup())
    "#;

    println!("\nTesting @this.defer()...");
    let lexer = Lexer::new(code);
    let mut parser = Parser::new(lexer);
    match parser.parse_program() {
        Ok(_) => println!("✓ @this.defer() parsed successfully"),
        Err(e) => println!("✗ Failed to parse @this.defer(): {:?}", e),
    }

    // Test 8: No keywords - these are all identifiers!
    let code = r#"
        loop = 5
        break = 10
        continue = true
        return = "hello"
        defer = 42
    "#;

    println!("\nTesting that keywords are identifiers...");
    let lexer = Lexer::new(code);
    let mut parser = Parser::new(lexer);
    match parser.parse_program() {
        Ok(_) => println!("✓ Keywords as identifiers parsed successfully"),
        Err(e) => println!("✗ Failed to parse keywords as identifiers: {:?}", e),
    }

    println!("\n✅ Parser spec compliance tests completed!");
}
