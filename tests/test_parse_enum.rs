use zen::lexer::Lexer;
use zen::parser::Parser;

fn main() {
    let code = r#"
        test = () void {
            result := .Ok(42)
            return .Err("failed")
        }
    "#;
    
    let lexer = Lexer::new(code);
    let mut parser = Parser::new(lexer);
    match parser.parse_program() {
        Ok(program) => {
            println!("Parse successful!");
            println!("{:#?}", program);
        }
        Err(e) => {
            println!("Parse error: {:?}", e);
        }
    }
}