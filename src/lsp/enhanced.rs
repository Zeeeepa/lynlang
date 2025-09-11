// Enhanced LSP features for Zen language server

use std::collections::HashMap;
#[allow(deprecated)]
use tower_lsp::lsp_types::*;
use crate::ast::{Program, Declaration};
use crate::parser::Parser;
use crate::lexer::Lexer;

/// Symbol information for goto definition, references, etc.
#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub range: Range,
    pub detail: Option<String>,
    pub children: Vec<Symbol>,
}

/// Document symbols provider
pub fn get_document_symbols(content: &str) -> Vec<DocumentSymbol> {
    let lexer = Lexer::new(content);
    let mut parser = Parser::new(lexer);
    
    match parser.parse_program() {
        Ok(program) => extract_symbols(&program),
        Err(_) => Vec::new(),
    }
}

fn extract_symbols(program: &Program) -> Vec<DocumentSymbol> {
    let mut symbols = Vec::new();
    
    for (idx, decl) in program.declarations.iter().enumerate() {
        if let Some(symbol) = declaration_to_symbol(decl, idx) {
            symbols.push(symbol);
        }
    }
    
    symbols
}

#[allow(deprecated)]
fn declaration_to_symbol(decl: &Declaration, line: usize) -> Option<DocumentSymbol> {
    match decl {
        Declaration::Function(func) => {
            let mut children = Vec::new();
            
            // Add parameters as children
            for (i, (name, param_type)) in func.args.iter().enumerate() {
                children.push(DocumentSymbol {
                    name: name.clone(),
                    detail: Some(format!("{:?}", param_type)),
                    kind: SymbolKind::VARIABLE,
                    range: Range::new(
                        Position::new(line as u32, 10 + (i as u32 * 10)),
                        Position::new(line as u32, 20 + (i as u32 * 10)),
                    ),
                    selection_range: Range::new(
                        Position::new(line as u32, 10 + (i as u32 * 10)),
                        Position::new(line as u32, 20 + (i as u32 * 10)),
                    ),
                    children: None,
                    tags: None,
                    deprecated: None
                });
            }
            
            Some(DocumentSymbol {
                name: func.name.clone(),
                detail: Some(format!("fn {}(...) -> {:?}", func.name, func.return_type)),
                kind: SymbolKind::FUNCTION,
                range: Range::new(
                    Position::new(line as u32, 0),
                    Position::new(line as u32 + 10, 0),
                ),
                selection_range: Range::new(
                    Position::new(line as u32, 3),
                    Position::new(line as u32, 3 + func.name.len() as u32),
                ),
                children: if children.is_empty() { None } else { Some(children) },
                tags: None,
                deprecated: None
            })
        }
        Declaration::Struct(s) => {
            let mut children = Vec::new();
            
            // Add fields as children
            for (i, field) in s.fields.iter().enumerate() {
                children.push(DocumentSymbol {
                    name: field.name.clone(),
                    detail: Some(format!("{:?}", field.type_)),
                    kind: SymbolKind::FIELD,
                    range: Range::new(
                        Position::new(line as u32 + 1 + i as u32, 4),
                        Position::new(line as u32 + 1 + i as u32, 40),
                    ),
                    selection_range: Range::new(
                        Position::new(line as u32 + 1 + i as u32, 4),
                        Position::new(line as u32 + 1 + i as u32, 4 + field.name.len() as u32),
                    ),
                    children: None,
                    tags: None,
                    deprecated: None
                });
            }
            
            Some(DocumentSymbol {
                name: s.name.clone(),
                detail: Some(format!("struct {}", s.name)),
                kind: SymbolKind::STRUCT,
                range: Range::new(
                    Position::new(line as u32, 0),
                    Position::new(line as u32 + s.fields.len() as u32 + 2, 0),
                ),
                selection_range: Range::new(
                    Position::new(line as u32, 7),
                    Position::new(line as u32, 7 + s.name.len() as u32),
                ),
                children: if children.is_empty() { None } else { Some(children) },
                tags: None,
                deprecated: None
            })
        }
        Declaration::Enum(e) => {
            let mut children = Vec::new();
            
            // Add variants as children
            for (i, variant) in e.variants.iter().enumerate() {
                children.push(DocumentSymbol {
                    name: variant.name.clone(),
                    detail: variant.payload.as_ref().map(|p| format!("{:?}", p)),
                    kind: SymbolKind::ENUM_MEMBER,
                    range: Range::new(
                        Position::new(line as u32 + 1 + i as u32, 4),
                        Position::new(line as u32 + 1 + i as u32, 40),
                    ),
                    selection_range: Range::new(
                        Position::new(line as u32 + 1 + i as u32, 4),
                        Position::new(line as u32 + 1 + i as u32, 4 + variant.name.len() as u32),
                    ),
                    children: None,
                    tags: None,
                    deprecated: None
                });
            }
            
            Some(DocumentSymbol {
                name: e.name.clone(),
                detail: Some(format!("enum {}", e.name)),
                kind: SymbolKind::ENUM,
                range: Range::new(
                    Position::new(line as u32, 0),
                    Position::new(line as u32 + e.variants.len() as u32 + 2, 0),
                ),
                selection_range: Range::new(
                    Position::new(line as u32, 5),
                    Position::new(line as u32, 5 + e.name.len() as u32),
                ),
                children: if children.is_empty() { None } else { Some(children) },
                tags: None,
                deprecated: None
            })
        }
        Declaration::Behavior(b) => {
            let mut children = Vec::new();
            
            // Add methods as children
            for (i, method) in b.methods.iter().enumerate() {
                children.push(DocumentSymbol {
                    name: method.name.clone(),
                    detail: Some(format!("fn {}(...) -> {:?}", method.name, method.return_type)),
                    kind: SymbolKind::METHOD,
                    range: Range::new(
                        Position::new(line as u32 + 1 + i as u32, 4),
                        Position::new(line as u32 + 1 + i as u32, 60),
                    ),
                    selection_range: Range::new(
                        Position::new(line as u32 + 1 + i as u32, 7),
                        Position::new(line as u32 + 1 + i as u32, 7 + method.name.len() as u32),
                    ),
                    children: None,
                    tags: None,
                    deprecated: None
                });
            }
            
            Some(DocumentSymbol {
                name: b.name.clone(),
                detail: Some(format!("behavior {}", b.name)),
                kind: SymbolKind::INTERFACE,
                range: Range::new(
                    Position::new(line as u32, 0),
                    Position::new(line as u32 + b.methods.len() as u32 + 2, 0),
                ),
                selection_range: Range::new(
                    Position::new(line as u32, 9),
                    Position::new(line as u32, 9 + b.name.len() as u32),
                ),
                children: if children.is_empty() { None } else { Some(children) },
                tags: None,
                deprecated: None
            })
        }
        _ => None,
    }
}

/// Extract symbol name at a given position in a line
fn extract_symbol_at_position(line: &str, char_pos: usize) -> String {
    if char_pos >= line.len() {
        return String::new();
    }
    
    let chars: Vec<char> = line.chars().collect();
    
    // Find start of symbol
    let mut start = char_pos;
    while start > 0 && (chars[start - 1].is_alphanumeric() || chars[start - 1] == '_') {
        start -= 1;
    }
    
    // Find end of symbol
    let mut end = char_pos;
    while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
        end += 1;
    }
    
    chars[start..end].iter().collect()
}

/// Find all references to a symbol
pub fn find_references(content: &str, position: Position) -> Vec<Range> {
    let mut references = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    
    // First, extract the symbol name at the position
    if position.line >= lines.len() as u32 {
        return references;
    }
    
    let line = lines[position.line as usize];
    let symbol_name = extract_symbol_at_position(line, position.character as usize);
    
    if symbol_name.is_empty() {
        return references;
    }
    
    // Now find all references to this symbol
    for (line_num, line) in lines.iter().enumerate() {
        let mut start = 0;
        while let Some(pos) = line[start..].find(&symbol_name) {
            let abs_pos = start + pos;
            // Check if it's a whole word match
            let before_ok = abs_pos == 0 || !line.chars().nth(abs_pos - 1).unwrap().is_alphanumeric();
            let after_ok = abs_pos + symbol_name.len() >= line.len() 
                || !line.chars().nth(abs_pos + symbol_name.len()).unwrap().is_alphanumeric();
            
            if before_ok && after_ok {
                references.push(Range::new(
                    Position::new(line_num as u32, abs_pos as u32),
                    Position::new(line_num as u32, (abs_pos + symbol_name.len()) as u32),
                ));
            }
            start = abs_pos + 1;
        }
    }
    
    references
}

/// Rename a symbol at the given position
pub fn rename_symbol(content: &str, position: Position, new_name: &str) -> Option<Vec<TextEdit>> {
    let references = find_references(content, position);
    
    if references.is_empty() {
        return None;
    }
    
    let edits: Vec<TextEdit> = references.into_iter().map(|range| {
        TextEdit {
            range,
            new_text: new_name.to_string(),
        }
    }).collect();
    
    Some(edits)
}

/// Rename symbol support
pub fn prepare_rename(content: &str, position: Position) -> Option<(String, Range)> {
    let lines: Vec<&str> = content.lines().collect();
    
    if position.line as usize >= lines.len() {
        return None;
    }
    
    let line = lines[position.line as usize];
    let char_pos = position.character as usize;
    
    // Find the identifier at the position
    let mut start = char_pos;
    let mut end = char_pos;
    let chars: Vec<char> = line.chars().collect();
    
    if char_pos >= chars.len() {
        return None;
    }
    
    // Find start of identifier
    while start > 0 && (chars[start - 1].is_alphanumeric() || chars[start - 1] == '_') {
        start -= 1;
    }
    
    // Find end of identifier
    while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
        end += 1;
    }
    
    if start >= end {
        return None;
    }
    
    let identifier: String = chars[start..end].iter().collect();
    
    Some((
        identifier,
        Range::new(
            Position::new(position.line, start as u32),
            Position::new(position.line, end as u32),
        ),
    ))
}

/// Generate code actions for quick fixes
pub fn get_code_actions(content: &str, range: Range, context: &CodeActionContext) -> Vec<CodeActionOrCommand> {
    let mut actions = Vec::new();
    
    for diagnostic in &context.diagnostics {
        // Quick fix for import placement errors
        if diagnostic.message.contains("Import statements must be at module level") {
            actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                title: "Move import to module level".to_string(),
                kind: Some(CodeActionKind::QUICKFIX),
                diagnostics: Some(vec![diagnostic.clone()]),
                edit: Some(WorkspaceEdit {
                    changes: Some(HashMap::new()), // Would be filled with actual edits
                    document_changes: None,
                    change_annotations: None,
                }),
                command: None,
                is_preferred: Some(true),
                disabled: None,
                data: None,
            }));
        }
        
        // Quick fix for missing type annotations
        if diagnostic.message.contains("type annotation") {
            actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                title: "Add type annotation".to_string(),
                kind: Some(CodeActionKind::QUICKFIX),
                diagnostics: Some(vec![diagnostic.clone()]),
                edit: None,
                command: None,
                is_preferred: Some(false),
                disabled: None,
                data: None,
            }));
        }
    }
    
    // Add refactoring actions
    let selected_text = get_text_in_range(content, range);
    
    if !selected_text.is_empty() {
        // Extract function
        if selected_text.lines().count() > 1 {
            actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                title: "Extract function".to_string(),
                kind: Some(CodeActionKind::REFACTOR_EXTRACT),
                diagnostics: None,
                edit: None,
                command: None,
                is_preferred: Some(false),
                disabled: None,
                data: None,
            }));
        }
        
        // Extract variable
        if selected_text.lines().count() == 1 && selected_text.contains('=') {
            actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                title: "Extract variable".to_string(),
                kind: Some(CodeActionKind::REFACTOR_EXTRACT),
                diagnostics: None,
                edit: None,
                command: None,
                is_preferred: Some(false),
                disabled: None,
                data: None,
            }));
        }
    }
    
    actions
}

fn get_text_in_range(content: &str, range: Range) -> String {
    let lines: Vec<&str> = content.lines().collect();
    
    if range.start.line == range.end.line {
        // Single line selection
        if (range.start.line as usize) < lines.len() {
            let line = lines[range.start.line as usize];
            let start = range.start.character as usize;
            let end = range.end.character as usize;
            if start < line.len() && end <= line.len() {
                return line[start..end].to_string();
            }
        }
    } else {
        // Multi-line selection
        let mut result = String::new();
        for line_num in range.start.line..=range.end.line {
            if (line_num as usize) < lines.len() {
                let line = lines[line_num as usize];
                if line_num == range.start.line {
                    result.push_str(&line[range.start.character as usize..]);
                } else if line_num == range.end.line {
                    result.push_str(&line[..range.end.character as usize]);
                } else {
                    result.push_str(line);
                }
                if line_num < range.end.line {
                    result.push('\n');
                }
            }
        }
        return result;
    }
    
    String::new()
}

/// Semantic tokens for syntax highlighting
pub fn get_semantic_tokens(content: &str) -> SemanticTokens {
    let lexer = Lexer::new(content);
    let _parser = Parser::new(lexer);
    
    let tokens = Vec::new();
    
    // This would be filled with actual semantic token data
    // based on parsing results
    
    SemanticTokens {
        result_id: None,
        data: tokens,
    }
}