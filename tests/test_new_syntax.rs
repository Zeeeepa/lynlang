use zen::lexer::{Lexer, Token};

fn main() {
    println!("Testing Zen lexer with new loop/option syntax...\n");
    
    let test_cases = vec![
        ("loop", "Should recognize loop as a token"),
        ("break", "Should recognize break as a token"),
        ("continue", "Should recognize continue as a token"),
        ("return", "Should recognize return as a token"),
        ("defer", "Should recognize defer as a token"),
        ("Some(42)", "Should handle Some constructor"),
        ("None", "Should handle None constructor"),
        ("items.loop((item) { })", "Should handle .loop() method"),
        ("expr.raise()", "Should handle .raise() method"),
        ("@this.defer(cleanup())", "Should handle @this.defer"),
        ("(0..10).loop((i) { })", "Should handle range.loop()"),
    ];
    
    for (input, description) in test_cases {
        println!("Test: {}", description);
        println!("Input: {}", input);
        let mut lexer = Lexer::new(input);
        
        print!("Tokens: ");
        loop {
            let token = lexer.next_token();
            if token == Token::Eof {
                println!();
                break;
            }
            print!("{:?} ", token);
        }
        println!();
    }
}