// Test the enhanced LSP features

#[cfg(test)]
mod tests {
    use zen::lsp::enhanced::*;
    use zen::parser::Parser;
    use zen::lexer::Lexer;
    use tower_lsp::lsp_types::*;

    #[test]
    fn test_build_symbol_table() {
        let code = r#"
// Test function
add = (a: i32, b: i32) i32 {
    a + b
}

// Test struct
Person = struct {
    name: string,
    age: u32
}

// Test enum
Status = enum {
    Ok,
    Error
}
        "#;

        let lexer = Lexer::new(code);
        let mut parser = Parser::new(lexer);
        let program = parser.parse_program().unwrap();
        
        let symbol_table = build_symbol_table(&program, code);
        
        // Check that functions are found
        assert!(symbol_table.find_symbol("add").is_some());
        let add_symbol = symbol_table.find_symbol("add").unwrap();
        assert_eq!(add_symbol.name, "add");
        
        // Check that structs are found
        assert!(symbol_table.find_symbol("Person").is_some());
        
        // Check that enums are found
        assert!(symbol_table.find_symbol("Status").is_some());
    }

    #[test]
    fn test_find_references() {
        let code = r#"
value := 42
result := value + 10
print(value)
        "#;
        
        let position = Position::new(0, 0); // Position of 'value' on first line
        let references = find_references(code, position);
        
        // Should find 3 references to 'value'
        assert!(references.len() >= 2); // At least the usage references
    }

    #[test]
    fn test_enhanced_hover() {
        let code = r#"
add = (a: i32, b: i32) i32 {
    a + b
}
        "#;

        let lexer = Lexer::new(code);
        let mut parser = Parser::new(lexer);
        let program = parser.parse_program().unwrap();
        
        let position = Position::new(1, 0); // Position of 'add'
        let hover = enhanced_hover(&program, code, position);
        
        assert!(hover.is_some());
        if let Some(hover) = hover {
            if let HoverContents::Markup(markup) = hover.contents {
                assert!(markup.value.contains("add"));
                // Should contain function signature info
            }
        }
    }

    #[test]
    fn test_document_symbols() {
        let code = r#"
// Function
greet = () void {
    print("Hello")
}

// Struct
Point = struct {
    x: f32,
    y: f32
}
        "#;

        let symbols = get_document_symbols(code);
        assert!(!symbols.is_empty());
        
        // Check that we have both function and struct symbols
        let has_function = symbols.iter().any(|s| s.kind == SymbolKind::FUNCTION);
        let has_struct = symbols.iter().any(|s| s.kind == SymbolKind::STRUCT);
        
        assert!(has_function);
        assert!(has_struct);
    }
}