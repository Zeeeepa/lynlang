use zen::lexer::Lexer;
use zen::parser::Parser;

#[test]
fn test_parse_range_loop() {
    let code = r#"
main = () void {
    (0..3).loop((i) {
        io.println("Test: ${i}")
    })
}
"#;

    let lexer = Lexer::new(code);
    let mut parser = Parser::new(lexer);

    match parser.parse_program() {
        Ok(program) => {
            // Verify we got a program with declarations
            assert!(!program.declarations.is_empty() || !program.statements.is_empty(), 
                "Should parse program successfully");
        }
        Err(e) => {
            panic!("Parse error: {:?}", e);
        }
    }
}

