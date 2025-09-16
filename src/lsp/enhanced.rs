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
    pub definition_range: Range,
    pub references: Vec<Range>,
}

/// Symbol table for tracking all definitions and references
#[derive(Debug, Default)]
pub struct SymbolTable {
    pub functions: HashMap<String, Symbol>,
    pub structs: HashMap<String, Symbol>,
    pub enums: HashMap<String, Symbol>,
    pub behaviors: HashMap<String, Symbol>,
    pub variables: HashMap<String, Symbol>,
    pub imports: HashMap<String, Symbol>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn add_function(&mut self, name: String, symbol: Symbol) {
        self.functions.insert(name, symbol);
    }
    
    pub fn add_struct(&mut self, name: String, symbol: Symbol) {
        self.structs.insert(name, symbol);
    }
    
    pub fn add_enum(&mut self, name: String, symbol: Symbol) {
        self.enums.insert(name, symbol);
    }
    
    pub fn add_variable(&mut self, name: String, symbol: Symbol) {
        self.variables.insert(name, symbol);
    }
    
    pub fn find_symbol(&self, name: &str) -> Option<&Symbol> {
        self.functions.get(name)
            .or_else(|| self.structs.get(name))
            .or_else(|| self.enums.get(name))
            .or_else(|| self.behaviors.get(name))
            .or_else(|| self.variables.get(name))
            .or_else(|| self.imports.get(name))
    }
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
pub fn extract_symbol_at_position(line: &str, char_pos: usize) -> String {
    let chars: Vec<char> = line.chars().collect();
    
    if char_pos > chars.len() {
        return String::new();
    }
    
    // Adjust position if we're at the end of a word
    let mut pos = char_pos;
    if pos > 0 && pos == chars.len() {
        pos -= 1;
    }
    
    // If we're not on an identifier character, return empty
    if pos < chars.len() && !chars[pos].is_alphanumeric() && chars[pos] != '_' && chars[pos] != '@' {
        // Check if we're just after an identifier
        if pos > 0 && (chars[pos - 1].is_alphanumeric() || chars[pos - 1] == '_' || chars[pos - 1] == '@') {
            pos -= 1;
        } else {
            return String::new();
        }
    }
    
    // Find start of symbol
    let mut start = pos;
    while start > 0 && (chars[start - 1].is_alphanumeric() || chars[start - 1] == '_' || chars[start - 1] == '@') {
        start -= 1;
    }
    
    // Find end of symbol
    let mut end = pos;
    while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_' || (end == start && chars[end] == '@')) {
        end += 1;
    }
    
    // Handle @std modules specially
    if start < chars.len() && chars[start] == '@' {
        // Continue until we hit a space or non-module character
        while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_' || chars[end] == '.') {
            end += 1;
        }
    }
    
    if start >= end {
        return String::new();
    }
    
    chars[start..end].iter().collect()
}

/// Find all references to a symbol with context awareness
pub fn find_references(content: &str, position: Position) -> Vec<Range> {
    let lines: Vec<&str> = content.lines().collect();
    
    // First, extract the symbol name at the position
    if position.line >= lines.len() as u32 {
        return Vec::new();
    }
    
    let line = lines[position.line as usize];
    let symbol_name = extract_symbol_at_position(line, position.character as usize);
    
    if symbol_name.is_empty() {
        return Vec::new();
    }
    
    find_all_references(&symbol_name, content)
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
        
        // Quick fix for forbidden keywords
        if diagnostic.message.contains("'if' keyword") || diagnostic.message.contains("'else' keyword") {
            actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                title: "Convert to pattern matching with '?'".to_string(),
                kind: Some(CodeActionKind::QUICKFIX),
                diagnostics: Some(vec![diagnostic.clone()]),
                edit: None, // Would contain the actual conversion
                command: None,
                is_preferred: Some(true),
                disabled: None,
                data: None,
            }));
        }
        
        // Quick fix for function syntax
        if diagnostic.message.contains("function keywords") {
            actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                title: "Convert to Zen function syntax".to_string(),
                kind: Some(CodeActionKind::QUICKFIX),
                diagnostics: Some(vec![diagnostic.clone()]),
                edit: None,
                command: None,
                is_preferred: Some(true),
                disabled: None,
                data: None,
            }));
        }
        
        // Quick fix for variable declarations
        if diagnostic.message.contains("variable keywords") {
            actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                title: "Convert to Zen variable syntax".to_string(),
                kind: Some(CodeActionKind::QUICKFIX),
                diagnostics: Some(vec![diagnostic.clone()]),
                edit: None,
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

/// Build a complete symbol table from the AST
pub fn build_symbol_table(program: &Program, content: &str) -> SymbolTable {
    let mut table = SymbolTable::new();
    let lines: Vec<&str> = content.lines().collect();
    
    // Find actual line positions by searching for declarations in content
    for decl in program.declarations.iter() {
        match decl {
            Declaration::Function(func) => {
                // Find the actual line where this function is defined
                // Look for pattern like "functionName = (" or "functionName<T> = ("
                let mut found = false;
                for (line_idx, line) in lines.iter().enumerate() {
                    // Check if this line contains the function definition
                    if line.contains(&func.name) && line.contains("=") && line.contains("(") {
                        // Verify it's actually the function definition
                        if let Some(name_pos) = line.find(&func.name) {
                            let after_name = &line[name_pos + func.name.len()..];
                            // Check for pattern: name followed by optional generics, then =
                            if after_name.trim_start().starts_with('=') || 
                               after_name.trim_start().starts_with('<') {
                                let symbol = Symbol {
                                    name: func.name.clone(),
                                    kind: SymbolKind::FUNCTION,
                                    range: Range::new(
                                        Position::new(line_idx as u32, 0),
                                        Position::new(line_idx as u32 + func.body.len() as u32 + 1, 0),
                                    ),
                                    detail: Some(format!("fn {}({}) -> {:?}", 
                                        func.name,
                                        func.args.iter()
                                            .map(|(n, t)| format!("{}: {:?}", n, t))
                                            .collect::<Vec<_>>()
                                            .join(", "),
                                        func.return_type
                                    )),
                                    children: Vec::new(),
                                    definition_range: Range::new(
                                        Position::new(line_idx as u32, name_pos as u32),
                                        Position::new(line_idx as u32, (name_pos + func.name.len()) as u32),
                                    ),
                                    references: find_all_references(&func.name, content),
                                };
                                table.add_function(func.name.clone(), symbol);
                                found = true;
                                break;
                            }
                        }
                    }
                }
                
                // Fallback to simpler pattern if not found
                if !found {
                    let pattern = format!("{} =", func.name);
                    if let Some(line_idx) = lines.iter().position(|line| line.contains(&pattern)) {
                        let line = lines[line_idx];
                        let column = line.find(&func.name).unwrap_or(0);
                        
                        let symbol = Symbol {
                            name: func.name.clone(),
                            kind: SymbolKind::FUNCTION,
                            range: Range::new(
                                Position::new(line_idx as u32, 0),
                                Position::new(line_idx as u32 + 10, 0),
                            ),
                            detail: Some(format!("fn {}({}) -> {:?}", 
                                func.name,
                                func.args.iter()
                                    .map(|(n, t)| format!("{}: {:?}", n, t))
                                    .collect::<Vec<_>>()
                                    .join(", "),
                                func.return_type
                            )),
                            children: Vec::new(),
                            definition_range: Range::new(
                                Position::new(line_idx as u32, column as u32),
                                Position::new(line_idx as u32, (column + func.name.len()) as u32),
                            ),
                            references: find_all_references(&func.name, content),
                        };
                        table.add_function(func.name.clone(), symbol);
                    }
                }
            }
            Declaration::Struct(s) => {
                // Find the actual line where this struct is defined
                let pattern = format!("struct {}", s.name);
                if let Some(line_idx) = lines.iter().position(|line| line.contains(&pattern)) {
                    let line = lines[line_idx];
                    let column = line.find(&s.name).unwrap_or(0);
                    
                    let symbol = Symbol {
                        name: s.name.clone(),
                        kind: SymbolKind::STRUCT,
                        range: Range::new(
                            Position::new(line_idx as u32, 0),
                            Position::new(line_idx as u32 + s.fields.len() as u32 + 2, 0),
                        ),
                        detail: Some(format!("struct {} {{ {} fields }}", s.name, s.fields.len())),
                        children: Vec::new(),
                        definition_range: Range::new(
                            Position::new(line_idx as u32, column as u32),
                            Position::new(line_idx as u32, (column + s.name.len()) as u32),
                        ),
                        references: find_all_references(&s.name, content),
                    };
                    table.add_struct(s.name.clone(), symbol);
                }
            }
            Declaration::Enum(e) => {
                // Find the actual line where this enum is defined
                let pattern = format!("enum {}", e.name);
                if let Some(line_idx) = lines.iter().position(|line| line.contains(&pattern)) {
                    let line = lines[line_idx];
                    let column = line.find(&e.name).unwrap_or(0);
                    
                    let symbol = Symbol {
                        name: e.name.clone(),
                        kind: SymbolKind::ENUM,
                        range: Range::new(
                            Position::new(line_idx as u32, 0),
                            Position::new(line_idx as u32 + e.variants.len() as u32 + 2, 0),
                        ),
                        detail: Some(format!("enum {} {{ {} variants }}", e.name, e.variants.len())),
                        children: Vec::new(),
                        definition_range: Range::new(
                            Position::new(line_idx as u32, column as u32),
                            Position::new(line_idx as u32, (column + e.name.len()) as u32),
                        ),
                        references: find_all_references(&e.name, content),
                    };
                    table.add_enum(e.name.clone(), symbol);
                }
            }
            _ => {}
        }
    }
    
    table
}

/// Find all references to a given identifier in the content
fn find_all_references(identifier: &str, content: &str) -> Vec<Range> {
    let mut references = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    
    for (line_num, line) in lines.iter().enumerate() {
        let mut start = 0;
        while let Some(pos) = line[start..].find(identifier) {
            let abs_pos = start + pos;
            // Check for whole word match
            let before_ok = abs_pos == 0 || 
                !line.chars().nth(abs_pos - 1).unwrap_or(' ').is_alphanumeric();
            let after_ok = abs_pos + identifier.len() >= line.len() || 
                !line.chars().nth(abs_pos + identifier.len()).unwrap_or(' ').is_alphanumeric();
            
            if before_ok && after_ok {
                references.push(Range::new(
                    Position::new(line_num as u32, abs_pos as u32),
                    Position::new(line_num as u32, (abs_pos + identifier.len()) as u32),
                ));
            }
            start = abs_pos + 1;
        }
    }
    
    references
}

/// Enhanced go-to-definition with better accuracy
pub fn enhanced_goto_definition(
    program: &Program, 
    content: &str, 
    position: Position,
    uri: &tower_lsp::lsp_types::Url
) -> Option<Location> {
    let table = build_symbol_table(program, content);
    let lines: Vec<&str> = content.lines().collect();
    
    if position.line as usize >= lines.len() {
        return None;
    }
    
    let line = lines[position.line as usize];
    let identifier = extract_symbol_at_position(line, position.character as usize);
    
    if identifier.is_empty() {
        return None;
    }
    
    // Special handling for standard library modules
    if identifier.starts_with("@std") {
        // For now, we can't navigate to standard library sources
        // but we could potentially show documentation
        return None;
    }
    
    // Look up the symbol in our table
    if let Some(symbol) = table.find_symbol(&identifier) {
        return Some(Location {
            uri: uri.clone(),
            range: symbol.definition_range.clone(),
        });
    }
    
    // Check for local variables in the current scope
    // First, find which function we're in
    let mut current_function: Option<&crate::ast::Function> = None;
    let mut func_start_line = 0;
    
    for decl in &program.declarations {
        if let Declaration::Function(func) = decl {
            // Find where this function is defined in the source
            for (idx, src_line) in lines.iter().enumerate() {
                if src_line.contains(&func.name) && src_line.contains("=") && src_line.contains("(") {
                    if idx <= position.line as usize {
                        // Check if we're inside this function
                        // Estimate function end by looking for the closing brace
                        let mut brace_count = 0;
                        let mut in_function = false;
                        for check_idx in idx..lines.len() {
                            let check_line = lines[check_idx];
                            for ch in check_line.chars() {
                                if ch == '{' {
                                    brace_count += 1;
                                    in_function = true;
                                } else if ch == '}' {
                                    brace_count -= 1;
                                    if in_function && brace_count == 0 {
                                        // Found the end of the function
                                        if position.line as usize <= check_idx {
                                            current_function = Some(func);
                                            func_start_line = idx;
                                        }
                                        break;
                                    }
                                }
                            }
                            if in_function && brace_count == 0 {
                                break;
                            }
                        }
                    }
                    break;
                }
            }
        }
    }
    
    // Check function parameters if we're inside a function
    if let Some(func) = current_function {
        for (param_name, _) in &func.args {
            if param_name == &identifier {
                // Find the parameter in the function signature
                let func_line = lines[func_start_line];
                if let Some(param_pos) = func_line.find(param_name) {
                    return Some(Location {
                        uri: uri.clone(),
                        range: Range::new(
                            Position::new(func_start_line as u32, param_pos as u32),
                            Position::new(func_start_line as u32, (param_pos + param_name.len()) as u32),
                        ),
                    });
                }
            }
        }
    }
    
    // Also check for variable declarations (improved matching)
    for (line_idx, line_content) in lines.iter().enumerate() {
        // Skip lines after the current position (we want the first/nearest declaration)
        if line_idx > position.line as usize {
            break;
        }
        
        // More precise pattern matching for variable declarations
        let trimmed = line_content.trim();
        
        // Check for immutable variable declaration patterns
        if trimmed.starts_with(&format!("{} :=", identifier)) ||
            trimmed.starts_with(&format!("{}: ", identifier)) ||
            trimmed.contains(&format!(" {} :=", identifier)) ||
            trimmed.contains(&format!(" {}: ", identifier)) {
            if let Some(col) = line_content.find(&identifier) {
                // Verify it's a declaration context (followed by : or :=)
                let after_ident = &line_content[col + identifier.len()..];
                if after_ident.trim_start().starts_with(':') {
                    return Some(Location {
                        uri: uri.clone(),
                        range: Range::new(
                            Position::new(line_idx as u32, col as u32),
                            Position::new(line_idx as u32, (col + identifier.len()) as u32),
                        ),
                    });
                }
            }
        }
        
        // Check for mutable variable declaration patterns
        if trimmed.starts_with(&format!("{} ::=", identifier)) ||
            trimmed.starts_with(&format!("{} :: ", identifier)) ||
            trimmed.starts_with(&format!("{} :: ", identifier)) ||
            trimmed.contains(&format!(" {} ::=", identifier)) {
            if let Some(col) = line_content.find(&identifier) {
                // Verify it's a declaration context
                let after_ident = &line_content[col + identifier.len()..];
                if after_ident.trim_start().starts_with("::") {
                    return Some(Location {
                        uri: uri.clone(),
                        range: Range::new(
                            Position::new(line_idx as u32, col as u32),
                            Position::new(line_idx as u32, (col + identifier.len()) as u32),
                        ),
                    });
                }
            }
        }
    }
    
    None
}

/// Enhanced hover information with type details and documentation
pub fn enhanced_hover(program: &Program, content: &str, position: Position) -> Option<Hover> {
    let table = build_symbol_table(program, content);
    let lines: Vec<&str> = content.lines().collect();
    
    if position.line as usize >= lines.len() {
        return None;
    }
    
    let line = lines[position.line as usize];
    let identifier = extract_symbol_at_position(line, position.character as usize);
    
    if identifier.is_empty() {
        return None;
    }
    
    // Check for standard library modules
    if identifier.starts_with("@std") {
        let module_info = get_std_module_info(&identifier);
        return Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: module_info,
            }),
            range: Some(Range::new(
                Position::new(position.line, position.character.saturating_sub(identifier.len() as u32)),
                Position::new(position.line, position.character),
            )),
        });
    }
    
    // Look up the symbol in our table
    if let Some(symbol) = table.find_symbol(&identifier) {
        let mut hover_text = format!("**{}**\n\n", identifier);
        
        match symbol.kind {
            SymbolKind::FUNCTION => {
                hover_text.push_str("```zen\n");
                hover_text.push_str(symbol.detail.as_ref().map(|s| s.as_str()).unwrap_or(""));
                hover_text.push_str("\n```\n\n");
                hover_text.push_str("**Function**\n\n");
                hover_text.push_str(&format!("📍 Defined at line {}\n", symbol.definition_range.start.line + 1));
                hover_text.push_str(&format!("📊 References: {}\n", symbol.references.len()));
                
                // Add parameter documentation if available
                if let Some(func_decl) = program.declarations.iter().find_map(|d| {
                    if let Declaration::Function(f) = d {
                        if f.name == identifier { Some(f) } else { None }
                    } else { None }
                }) {
                    if !func_decl.args.is_empty() {
                        hover_text.push_str("\n**Parameters:**\n");
                        for (param_name, param_type) in &func_decl.args {
                            hover_text.push_str(&format!("- `{}: {:?}`\n", param_name, param_type));
                        }
                    }
                    hover_text.push_str(&format!("\n**Returns:** `{:?}`\n", func_decl.return_type));
                }
            }
            SymbolKind::STRUCT => {
                hover_text.push_str("```zen\n");
                hover_text.push_str(symbol.detail.as_ref().map(|s| s.as_str()).unwrap_or(""));
                hover_text.push_str("\n```\n\n");
                hover_text.push_str("**Struct Type**\n\n");
                hover_text.push_str(&format!("📍 Defined at line {}\n", symbol.definition_range.start.line + 1));
                hover_text.push_str("\n💡 **Usage:**\n");
                hover_text.push_str(&format!("- Constructor: `{} {{ field1: value1, field2: value2 }}`\n", identifier));
                hover_text.push_str(&format!("- UFCS methods: `value.method()` or `{}::method(value)`\n", identifier));
                
                // Add field documentation if available
                if let Some(struct_decl) = program.declarations.iter().find_map(|d| {
                    if let Declaration::Struct(s) = d {
                        if s.name == identifier { Some(s) } else { None }
                    } else { None }
                }) {
                    if !struct_decl.fields.is_empty() {
                        hover_text.push_str("\n**Fields:**\n");
                        for field in &struct_decl.fields {
                            hover_text.push_str(&format!("- `{}: {:?}`\n", field.name, field.type_));
                        }
                    }
                }
            }
            SymbolKind::ENUM => {
                hover_text.push_str("```zen\n");
                hover_text.push_str(symbol.detail.as_ref().map(|s| s.as_str()).unwrap_or(""));
                hover_text.push_str("\n```\n\n");
                hover_text.push_str("**Enum Type**\n\n");
                hover_text.push_str(&format!("📍 Defined at line {}\n", symbol.definition_range.start.line + 1));
                hover_text.push_str("\n💡 **Pattern Matching:**\n");
                hover_text.push_str("```zen\n");
                hover_text.push_str("value ? \n");
                hover_text.push_str("  Variant1 {handle1()}\n");
                hover_text.push_str("  Variant2(data) { handle2(data)}\n");
                hover_text.push_str("  _ => default()\n");
                hover_text.push_str("```\n");
                
                // Add variant documentation if available
                if let Some(enum_decl) = program.declarations.iter().find_map(|d| {
                    if let Declaration::Enum(e) = d {
                        if e.name == identifier { Some(e) } else { None }
                    } else { None }
                }) {
                    if !enum_decl.variants.is_empty() {
                        hover_text.push_str("\n**Variants:**\n");
                        for variant in &enum_decl.variants {
                            if let Some(payload) = &variant.payload {
                                hover_text.push_str(&format!("- `.{} -> {:?}`\n", variant.name, payload));
                            } else {
                                hover_text.push_str(&format!("- `.{}`\n", variant.name));
                            }
                        }
                    }
                }
            }
            _ => {
                if let Some(detail) = &symbol.detail {
                    hover_text.push_str(detail);
                }
            }
        }
        
        // Calculate the actual position of the identifier in the line
        let ident_start = line.find(&identifier).unwrap_or(position.character as usize);
        
        return Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: hover_text,
            }),
            range: Some(Range::new(
                Position::new(position.line, ident_start as u32),
                Position::new(position.line, (ident_start + identifier.len()) as u32),
            )),
        });
    }
    
    // Try to find variable declarations for type info
    for (_line_idx, line_content) in lines.iter().enumerate() {
        // Check for typed variable declarations
        if line_content.contains(&format!("{}: ", identifier)) ||
           line_content.contains(&format!("{} : ", identifier)) ||
           line_content.contains(&format!("{} :: ", identifier)) {
            // Extract type information
            if let Some(start) = line_content.find(&identifier) {
                let after_identifier = &line_content[start + identifier.len()..];
                if let Some(colon_pos) = after_identifier.find(':') {
                    let after_colon = &after_identifier[colon_pos + 1..].trim_start();
                    // Find the type name (up to '=' or next operator)
                    let type_end = after_colon.find('=').unwrap_or(after_colon.len());
                    let type_name = after_colon[..type_end].trim();
                    
                    if !type_name.is_empty() && !type_name.starts_with(':') {
                        let hover_text = format!(
                            "**{}**\n\n```zen\n{}: {}\n```\n\n**Variable**\n\nType: `{}`",
                            identifier, identifier, type_name, type_name
                        );
                        return Some(Hover {
                            contents: HoverContents::Markup(MarkupContent {
                                kind: MarkupKind::Markdown,
                                value: hover_text,
                            }),
                            range: Some(Range::new(
                                Position::new(position.line, position.character),
                                Position::new(position.line, position.character + identifier.len() as u32),
                            )),
                        });
                    }
                }
            }
        }
        
        // Check for inferred variable types
        if line_content.contains(&format!("{} :=", identifier)) ||
           line_content.contains(&format!("{} ::=", identifier)) {
            let hover_text = format!(
                "**{}**\n\n```zen\n{}\n```\n\n**Variable**\n\nType: _(inferred)_\n\nUse `:` for explicit typing: `{}: Type = value`",
                identifier, line_content.trim(), identifier
            );
            return Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: hover_text,
                }),
                range: Some(Range::new(
                    Position::new(position.line, position.character),
                    Position::new(position.line, position.character + identifier.len() as u32),
                )),
            });
        }
    }
    
    // Provide generic hover for keywords
    match identifier.as_str() {
        "comptime" => Some(create_keyword_hover(
            "comptime",
            "Compile-time evaluation block",
            "Execute code at compile time. No imports allowed inside."
        )),
        "loop" => Some(create_keyword_hover(
            "loop",
            "Looping construct",
            "Infinite: `loop { ... }`\nConditional: `loop (cond) { ... }`\nRange: `(0..10).loop((i) => ...)`"
        )),
        "break" => Some(create_keyword_hover(
            "break",
            "Exit loop",
            "Exits the current loop. Can return a value: `break value`"
        )),
        "continue" => Some(create_keyword_hover(
            "continue",
            "Skip to next iteration",
            "Skips to the next loop iteration"
        )),
        "return" => Some(create_keyword_hover(
            "return",
            "Return from function",
            "Returns a value from the function. Last expression without `;` is implicit return."
        )),
        "defer" => Some(create_keyword_hover(
            "defer",
            "Defer execution",
            "Execute statement when leaving scope (LIFO order)"
        )),
        _ => None,
    }
}

fn create_keyword_hover(keyword: &str, title: &str, description: &str) -> Hover {
    let hover_text = format!("**{}** - {}\n\n{}\n\n📚 See LANGUAGE_SPEC.md for details", 
                            keyword, title, description);
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: hover_text,
        }),
        range: None,
    }
}

pub fn get_std_module_info(module: &str) -> String {
    match module {
        "@std.io" => {
            "**@std.io** - Input/Output Module\n\n\
            Functions:\n\
            - `print(msg: string)` - Print to stdout\n\
            - `println(msg: string)` - Print with newline\n\
            - `read_line() -> string` - Read from stdin\n\
            - `read_file(path: string) -> Result<string, Error>`".to_string()
        }
        "@std.mem" => {
            "**@std.mem** - Memory Management\n\n\
            Types:\n\
            - `Allocator` - Memory allocator interface\n\
            - `Ptr<T>` - Smart pointer type\n\
            Functions:\n\
            - `alloc<T>(count: usize) -> Ptr<T>`\n\
            - `free<T>(ptr: Ptr<T>)`".to_string()
        }
        "@std.math" => {
            "**@std.math** - Mathematics\n\n\
            Constants:\n\
            - `PI`, `E`, `TAU`\n\
            Functions:\n\
            - `sin`, `cos`, `tan`, `sqrt`, `pow`, `abs`, `min`, `max`".to_string()
        }
        _ => format!("**{}**\n\nStandard library module", module),
    }
}