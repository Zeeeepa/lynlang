use zen::lexer::Lexer;
use zen::parser::Parser;

fn main() {
    let test_cases = vec![
        ("if x", "Single line 'if'"),
        ("x := 5\nlet y = 10", "Second line 'let'"),
        ("x := 5\ny := 10\nwhile true", "Third line 'while'"),
    ];
    
    for (input, desc) in test_cases {
        println!("\n=== Test: {} ===", desc);
        println!("Input:\n{}", input);
        println!("---");
        
        let lexer = Lexer::new(input);
        let mut parser = Parser::new(lexer);
        let result = parser.parse_program();
        
        if let Err(e) = result {
            if let Some(span) = e.position() {
                println!("Error position:");
                println!("  Line: {} (1-indexed in span)", span.line);
                println!("  Column: {} (0-indexed in span)", span.column);
                
                // What LSP would see
                let lsp_line = if span.line > 0 { span.line - 1 } else { 0 };
                let lsp_col = span.column;
                println!("LSP position (0-indexed):");
                println!("  Line: {}", lsp_line);
                println!("  Column: {}", lsp_col);
            }
            println!("Error: {}", e);
        }
    }
}