// Test file for spec-compliant lexer
use zen::lexer::{Lexer, Token};

fn main() {
    // Test basic Zen language features according to LANGUAGE_SPEC.zen
    let test_cases = vec![
        // No keywords - all are identifiers
        ("loop", Token::Identifier("loop".to_string())),
        ("break", Token::Identifier("break".to_string())),
        ("continue", Token::Identifier("continue".to_string())),
        ("return", Token::Identifier("return".to_string())),
        ("defer", Token::Identifier("defer".to_string())),
        
        // @ symbols
        ("@std", Token::AtStd),
        ("@this", Token::AtThis),
        ("@meta", Token::AtMeta),
        
        // Assignment operators
        ("=", Token::Operator("=".to_string())),
        (":=", Token::Operator(":=".to_string())),
        ("::", Token::Operator("::".to_string())),
        ("::=", Token::Operator("::=".to_string())),
        
        // Pattern matching
        ("?", Token::Question),
        ("|", Token::Pipe),
        ("_", Token::Underscore),
        
        // Arrows
        ("->", Token::Operator("->".to_string())),
        ("=>", Token::Operator("=>".to_string())),
        
        // Range operators
        ("..", Token::Operator("..".to_string())),
        ("..=", Token::Operator("..=".to_string())),
    ];
    
    for (input, expected) in test_cases {
        let mut lexer = Lexer::new(input);
        let token = lexer.next_token();
        println!("Testing: {} -> {:?} (expected: {:?})", input, token, expected);
        assert_eq!(token, expected, "Failed for input: {}", input);
        println!("✓ Lexer test passed for: {} -> {:?}", input, token);
    }
    
    // Test a more complex example from LANGUAGE_SPEC.zen
    let complex_code = r#"
        // No keywords in Zen!
        x := 10
        y ::= 20
        z : i32 = 30
        
        // Pattern matching
        result ?
            | Ok(val) { return val }
            | Err(e) { return 0 }
        
        // Loop with closure
        (0..10).loop((i) {
            io.println("Count: ${i}")
        })
        
        // @std imports
        { io, math } = @std
    "#;
    
    let mut lexer = Lexer::new(complex_code);
    let mut tokens = Vec::new();
    
    loop {
        let token = lexer.next_token();
        if token == Token::Eof {
            break;
        }
        tokens.push(token);
    }
    
    println!("\n✓ Successfully lexed {} tokens from complex Zen code", tokens.len());
    println!("✓ All lexer tests passed!");
}