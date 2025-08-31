use zen::lexer::Lexer;
use zen::parser::Parser;

fn main() {
    let input = r#"
comptime {
    lexer := @compiler.lexer
}
"#;
    
    let lexer = Lexer::new(input);
    let mut parser = Parser::new(lexer);
    let result = parser.parse_program();
    
    match result {
        Ok(program) => {
            println!("Parse succeeded!");
            println!("Program: {:#?}", program);
        }
        Err(e) => {
            println!("Parse error: {:?}", e);
        }
    }
}