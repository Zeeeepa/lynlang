use zen::lexer::Lexer;
use zen::parser::Parser;
use zen::lexer::Token;

fn main() {
    let code = r#"// Test file for position tracking
x := 42;
y ::= "hello";
z: i32 = 100;

main = () void {
    print(x);
}"#;

    println!("Testing LSP position tracking...\n");
    println!("Input code:");
    println!("{}", code);
    println!("\n---\n");

    let mut lexer = Lexer::new(code);
    
    // Track all tokens with their positions
    println!("Token positions:");
    loop {
        let token_with_span = lexer.next_token_with_span();
        if token_with_span.token == Token::Eof {
            break;
        }
        println!("Token: {:?}", token_with_span.token);
        println!("  Line: {} (1-indexed), Column: {} (0-indexed)", 
                 token_with_span.span.line, 
                 token_with_span.span.column);
        println!("  Start: {}, End: {}", 
                 token_with_span.span.start, 
                 token_with_span.span.end);
    }

    println!("\n---\n");
    println!("Parsing for error positions...");
    
    // Test parser error positions
    let error_code = r#"// Error test
if (true) {  // This should error - 'if' not allowed
    print("hello");
}"#;

    let lexer2 = Lexer::new(error_code);
    let mut parser = Parser::new(lexer2);
    
    match parser.parse_program() {
        Ok(_) => println!("Unexpectedly parsed successfully"),
        Err(e) => {
            println!("Parse error: {}", e);
            if let Some(span) = e.position() {
                println!("Error position:");
                println!("  Line: {} (1-indexed)", span.line);
                println!("  Column: {} (0-indexed)", span.column);
                println!("  Character range: {}-{}", span.start, span.end);
                
                // Show the error location in context
                let lines: Vec<&str> = error_code.lines().collect();
                if span.line > 0 && span.line <= lines.len() {
                    let error_line = lines[span.line - 1];
                    println!("  Context: {}", error_line);
                    println!("           {}^", " ".repeat(span.column));
                }
            }
        }
    }
}
