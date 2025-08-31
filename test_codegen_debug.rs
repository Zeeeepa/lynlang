use zen::{parser::Parser, lexer::Lexer, typechecker::TypeChecker, error::CompileError};

fn main() {
    let source = include_str!("compiler/codegen.zen");
    let mut lexer = Lexer::new(source);
    let mut parser = Parser::new(lexer);
    
    match parser.parse_program() {
        Ok(ast) => {
            println!("Parser succeeded!");
            let mut type_checker = TypeChecker::new();
            match type_checker.check_program(&ast) {
                Ok(_) => println!("Type checker succeeded!"),
                Err(e) => println!("Type checker error: {:?}", e),
            }
        }
        Err(e) => println!("Parser error: {:?}", e),
    }
}