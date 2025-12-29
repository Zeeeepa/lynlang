// LSP Utility Functions

use crate::ast::AstType;
use crate::error::{CompileError, Span};
use crate::stdlib_types::StdlibTypeRegistry;
use lsp_types::*;

/// Convert a byte offset to LSP Position (line and character)
/// Returns (line, character) where line is 0-based and character is UTF-16 code unit offset
pub fn byte_offset_to_lsp_position(content: &str, byte_offset: usize) -> Position {
    let mut line = 0u32;
    let mut line_start_offset = 0usize;
    
    for (idx, ch) in content.char_indices() {
        if idx >= byte_offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            line_start_offset = idx + 1;
        }
    }
    
    // Calculate character offset within the line (in UTF-16 code units for LSP)
    let char_offset = if byte_offset >= line_start_offset {
        content[line_start_offset..byte_offset.min(content.len())]
            .chars()
            .map(|c| c.len_utf16() as u32)
            .sum()
    } else {
        0
    };
    
    Position {
        line,
        character: char_offset,
    }
}

/// Convert a compiler Span to LSP Range using source content for accurate multi-line handling
pub fn span_to_lsp_range(span: &Span, content: Option<&str>) -> Range {
    if let Some(content) = content {
        // Use accurate byte offset conversion
        let start = byte_offset_to_lsp_position(content, span.start);
        let end = byte_offset_to_lsp_position(content, span.end);
        Range { start, end }
    } else {
        // Fallback: use span's line/column for start, estimate end on same line
        let start = Position {
            line: if span.line > 0 { span.line as u32 - 1 } else { 0 },
            character: span.column as u32,
        };
        let end = Position {
            line: start.line,
            character: (span.column + (span.end.saturating_sub(span.start)).max(1)) as u32,
        };
        Range { start, end }
    }
}

// Convert CompileError to LSP Diagnostic (without source context)
pub fn compile_error_to_diagnostic(error: CompileError) -> Diagnostic {
    compile_error_to_diagnostic_with_content(error, None)
}

// Convert CompileError to LSP Diagnostic with source content for position inference
pub fn compile_error_to_diagnostic_with_content(
    error: CompileError,
    content: Option<&str>,
) -> Diagnostic {
    // Extract span and determine severity
    let (span, severity, code) = match &error {
        CompileError::ParseError(_, span) => {
            (span.clone(), DiagnosticSeverity::ERROR, Some("parse-error"))
        }
        CompileError::SyntaxError(_, span) => (
            span.clone(),
            DiagnosticSeverity::ERROR,
            Some("syntax-error"),
        ),
        CompileError::TypeError(_, span) => {
            (span.clone(), DiagnosticSeverity::ERROR, Some("type-error"))
        }
        CompileError::TypeMismatch { span, .. } => (
            span.clone(),
            DiagnosticSeverity::ERROR,
            Some("type-mismatch"),
        ),
        CompileError::UndeclaredVariable(_, span) => (
            span.clone(),
            DiagnosticSeverity::ERROR,
            Some("undeclared-variable"),
        ),
        CompileError::UndeclaredFunction(_, span) => (
            span.clone(),
            DiagnosticSeverity::ERROR,
            Some("undeclared-function"),
        ),
        CompileError::UnexpectedToken { span, .. } => (
            span.clone(),
            DiagnosticSeverity::ERROR,
            Some("unexpected-token"),
        ),
        CompileError::InvalidPattern(_, span) => (
            span.clone(),
            DiagnosticSeverity::ERROR,
            Some("invalid-pattern"),
        ),
        CompileError::InvalidSyntax { span, .. } => (
            span.clone(),
            DiagnosticSeverity::ERROR,
            Some("invalid-syntax"),
        ),
        CompileError::MissingTypeAnnotation(_, span) => (
            span.clone(),
            DiagnosticSeverity::WARNING,
            Some("missing-type"),
        ),
        CompileError::DuplicateDeclaration {
            duplicate_location, ..
        } => (
            duplicate_location.clone(),
            DiagnosticSeverity::ERROR,
            Some("duplicate-declaration"),
        ),
        CompileError::ImportError(_, span) => (
            span.clone(),
            DiagnosticSeverity::ERROR,
            Some("import-error"),
        ),
        CompileError::FFIError(_, span) => {
            (span.clone(), DiagnosticSeverity::ERROR, Some("ffi-error"))
        }
        CompileError::InvalidLoopCondition(_, span) => (
            span.clone(),
            DiagnosticSeverity::ERROR,
            Some("invalid-loop"),
        ),
        CompileError::MissingReturnStatement(_, span) => (
            span.clone(),
            DiagnosticSeverity::WARNING,
            Some("missing-return"),
        ),
        CompileError::InternalError(_, span) => (
            span.clone(),
            DiagnosticSeverity::ERROR,
            Some("internal-error"),
        ),
        CompileError::UnsupportedFeature(_, span) => (
            span.clone(),
            DiagnosticSeverity::WARNING,
            Some("unsupported-feature"),
        ),
        CompileError::FileNotFound(_, _) => {
            (None, DiagnosticSeverity::ERROR, Some("file-not-found"))
        }
        CompileError::ComptimeError(_) => (None, DiagnosticSeverity::ERROR, Some("comptime-error")),
        CompileError::BuildError(_) => (None, DiagnosticSeverity::ERROR, Some("build-error")),
        CompileError::FileError(_) => (None, DiagnosticSeverity::ERROR, Some("file-error")),
        CompileError::CyclicDependency(_) => {
            (None, DiagnosticSeverity::ERROR, Some("cyclic-dependency"))
        }
    };

    // Convert span to LSP range, or try to infer from error message and content
    let (start_pos, end_pos) = if let Some(span) = span {
        let range = span_to_lsp_range(&span, content);
        (range.start, range.end)
    } else if let Some(content) = content {
        infer_error_position(&error, content)
    } else {
        (
            Position {
                line: 0,
                character: 0,
            },
            Position {
                line: 0,
                character: 1,
            },
        )
    };

    Diagnostic {
        range: Range {
            start: start_pos,
            end: end_pos,
        },
        severity: Some(severity),
        code: code.map(|c| lsp_types::NumberOrString::String(c.to_string())),
        code_description: None,
        source: Some("zen-compiler".to_string()),
        message: format!("{}", error),
        related_information: None,
        tags: None,
        data: None,
    }
}

/// Try to infer error position from error message and source content
fn infer_error_position(error: &CompileError, content: &str) -> (Position, Position) {
    let search_terms = extract_search_terms(error);

    for term in search_terms {
        if let Some((line, col, len)) = find_in_content(&term, content) {
            return (
                Position {
                    line: line as u32,
                    character: col as u32,
                },
                Position {
                    line: line as u32,
                    character: (col + len) as u32,
                },
            );
        }
    }

    // Default to first line if nothing found
    (
        Position {
            line: 0,
            character: 0,
        },
        Position {
            line: 0,
            character: 1,
        },
    )
}

/// Extract searchable terms from error message
fn extract_search_terms(error: &CompileError) -> Vec<String> {
    let mut terms = Vec::new();

    match error {
        CompileError::TypeError(msg, _) => {
            // Try to extract function/variable names from error message
            // "Unknown function: foo" -> search for "foo"
            if let Some(idx) = msg.find("Unknown function: ") {
                let name = msg[idx + 18..].trim();
                // Remove any trailing punctuation
                let name = name.trim_end_matches(|c: char| !c.is_alphanumeric() && c != '_');
                if !name.is_empty() {
                    terms.push(format!("{}(", name)); // Function call
                    terms.push(name.to_string());
                }
            }
            // "'foo' is not a function" -> search for "foo("
            if msg.starts_with('\'') {
                if let Some(end_quote) = msg[1..].find('\'') {
                    let name = &msg[1..end_quote + 1];
                    if !name.is_empty() {
                        terms.push(format!("{}(", name)); // Function call
                        terms.push(name.to_string());
                    }
                }
            }
            // "Undeclared variable: 'foo'" -> search for "foo"
            if let Some(idx) = msg.find("Undeclared variable: '") {
                let start = idx + 22;
                if let Some(end) = msg[start..].find('\'') {
                    let name = &msg[start..start + end];
                    if !name.is_empty() {
                        terms.push(name.to_string());
                    }
                }
            }
        }
        CompileError::UndeclaredVariable(name, _) => {
            terms.push(name.clone());
        }
        CompileError::UndeclaredFunction(name, _) => {
            terms.push(format!("{}(", name));
            terms.push(name.clone());
        }
        _ => {}
    }

    terms
}

/// Find a term in content and return (line, column, length)
fn find_in_content(term: &str, content: &str) -> Option<(usize, usize, usize)> {
    for (line_num, line) in content.lines().enumerate() {
        // Skip comment lines
        let trimmed = line.trim();
        if trimmed.starts_with("//") {
            continue;
        }

        if let Some(col) = line.find(term) {
            return Some((line_num, col, term.len()));
        }
    }
    None
}

pub fn format_symbol_kind(kind: SymbolKind) -> &'static str {
    match kind {
        SymbolKind::FILE => "File",
        SymbolKind::MODULE => "Module",
        SymbolKind::NAMESPACE => "Namespace",
        SymbolKind::PACKAGE => "Package",
        SymbolKind::CLASS => "Class",
        SymbolKind::METHOD => "Method",
        SymbolKind::PROPERTY => "Property",
        SymbolKind::FIELD => "Field",
        SymbolKind::CONSTRUCTOR => "Constructor",
        SymbolKind::ENUM => "Enum",
        SymbolKind::INTERFACE => "Interface",
        SymbolKind::FUNCTION => "Function",
        SymbolKind::VARIABLE => "Variable",
        SymbolKind::CONSTANT => "Constant",
        SymbolKind::STRING => "String",
        SymbolKind::NUMBER => "Number",
        SymbolKind::BOOLEAN => "Boolean",
        SymbolKind::ARRAY => "Array",
        SymbolKind::OBJECT => "Object",
        SymbolKind::KEY => "Key",
        SymbolKind::NULL => "Null",
        SymbolKind::ENUM_MEMBER => "Enum Member",
        SymbolKind::STRUCT => "Struct",
        SymbolKind::EVENT => "Event",
        SymbolKind::OPERATOR => "Operator",
        SymbolKind::TYPE_PARAMETER => "Type Parameter",
        _ => "Unknown",
    }
}

pub fn symbol_kind_to_completion_kind(kind: SymbolKind) -> CompletionItemKind {
    match kind {
        SymbolKind::FUNCTION | SymbolKind::METHOD => CompletionItemKind::FUNCTION,
        SymbolKind::STRUCT | SymbolKind::CLASS => CompletionItemKind::STRUCT,
        SymbolKind::ENUM => CompletionItemKind::ENUM,
        SymbolKind::ENUM_MEMBER => CompletionItemKind::ENUM_MEMBER,
        SymbolKind::VARIABLE => CompletionItemKind::VARIABLE,
        SymbolKind::CONSTANT => CompletionItemKind::CONSTANT,
        SymbolKind::FIELD | SymbolKind::PROPERTY => CompletionItemKind::FIELD,
        SymbolKind::INTERFACE => CompletionItemKind::INTERFACE,
        SymbolKind::MODULE | SymbolKind::NAMESPACE => CompletionItemKind::MODULE,
        SymbolKind::TYPE_PARAMETER => CompletionItemKind::TYPE_PARAMETER,
        SymbolKind::CONSTRUCTOR => CompletionItemKind::CONSTRUCTOR,
        SymbolKind::EVENT => CompletionItemKind::EVENT,
        SymbolKind::OPERATOR => CompletionItemKind::OPERATOR,
        _ => CompletionItemKind::TEXT,
    }
}

pub fn format_type(ast_type: &AstType) -> String {
    match ast_type {
        AstType::I8 => "i8".to_string(),
        AstType::I16 => "i16".to_string(),
        AstType::I32 => "i32".to_string(),
        AstType::I64 => "i64".to_string(),
        AstType::U8 => "u8".to_string(),
        AstType::U16 => "u16".to_string(),
        AstType::U32 => "u32".to_string(),
        AstType::U64 => "u64".to_string(),
        AstType::Usize => "usize".to_string(),
        AstType::F32 => "f32".to_string(),
        AstType::F64 => "f64".to_string(),
        AstType::Bool => "bool".to_string(),
        AstType::StaticLiteral => "str".to_string(), // Internal string literal type
        AstType::StaticString => "StaticString".to_string(),
        AstType::Struct { name, .. } if StdlibTypeRegistry::is_string_type(name) => "String".to_string(),
        AstType::Void => "void".to_string(),
        t if t.is_immutable_ptr() => {
            if let Some(inner) = t.ptr_inner() {
                format!("Ptr<{}>", format_type(inner))
            } else {
                "Ptr<?>".to_string()
            }
        }
        t if t.is_mutable_ptr() => {
            if let Some(inner) = t.ptr_inner() {
                format!("MutPtr<{}>", format_type(inner))
            } else {
                "MutPtr<?>".to_string()
            }
        }
        t if t.is_raw_ptr() => {
            if let Some(inner) = t.ptr_inner() {
                format!("RawPtr<{}>", format_type(inner))
            } else {
                "RawPtr<?>".to_string()
            }
        }
        AstType::Ref(inner) => format!("&{}", format_type(inner)),
        AstType::Range {
            start_type,
            end_type,
            inclusive,
        } => {
            if *inclusive {
                format!("{}..={}", format_type(start_type), format_type(end_type))
            } else {
                format!("{}..{}", format_type(start_type), format_type(end_type))
            }
        }
        AstType::FunctionPointer {
            param_types,
            return_type,
        } => {
            format!(
                "fn({}) {}",
                param_types
                    .iter()
                    .map(|p| format_type(p))
                    .collect::<Vec<_>>()
                    .join(", "),
                format_type(return_type)
            )
        }
        AstType::EnumType { name } => name.clone(),
        AstType::StdModule => "module".to_string(),
        AstType::Array(elem) => format!("Array<{}>", format_type(elem)),
        AstType::Vec { element_type, size } => {
            format!("Vec<{}, {}>", format_type(element_type), size)
        }
        AstType::DynVec { element_types, .. } => {
            if element_types.len() == 1 {
                format!("DynVec<{}>", format_type(&element_types[0]))
            } else {
                "DynVec<...>".to_string()
            }
        }
        AstType::FixedArray { element_type, size } => {
            format!("[{}; {}]", format_type(element_type), size)
        }
        // Option and Result are now Generic types - handled in Generic match below
        AstType::Struct { name, .. } => name.clone(),
        AstType::Enum { name, .. } => name.clone(),
        AstType::Generic { name, type_args } => {
            if type_args.is_empty() {
                name.clone()
            } else {
                format!(
                    "{}<{}>",
                    name,
                    type_args
                        .iter()
                        .map(|p| format_type(p))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
        }
        AstType::Function { args, return_type } => {
            format!(
                "({}) {}",
                args.iter()
                    .map(|p| format_type(p))
                    .collect::<Vec<_>>()
                    .join(", "),
                format_type(return_type)
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_byte_offset_to_lsp_position_simple() {
        let content = "hello\nworld";
        
        let pos = byte_offset_to_lsp_position(content, 0);
        assert_eq!((pos.line, pos.character), (0, 0));
        
        let pos = byte_offset_to_lsp_position(content, 3);
        assert_eq!((pos.line, pos.character), (0, 3));
        
        let pos = byte_offset_to_lsp_position(content, 5);
        assert_eq!((pos.line, pos.character), (0, 5));
        
        let pos = byte_offset_to_lsp_position(content, 6);
        assert_eq!((pos.line, pos.character), (1, 0));
        
        let pos = byte_offset_to_lsp_position(content, 9);
        assert_eq!((pos.line, pos.character), (1, 3));
    }

    #[test]
    fn test_byte_offset_emoji_utf16_surrogate_pairs() {
        let content = "a😀b";
        
        let pos = byte_offset_to_lsp_position(content, 0);
        assert_eq!((pos.line, pos.character), (0, 0));
        
        let pos = byte_offset_to_lsp_position(content, 1);
        assert_eq!((pos.line, pos.character), (0, 1));
        
        let pos = byte_offset_to_lsp_position(content, 5);
        assert_eq!((pos.line, pos.character), (0, 3));
    }

    #[test]
    fn test_byte_offset_japanese_bmp_characters() {
        let content = "日本語";
        
        let pos = byte_offset_to_lsp_position(content, 0);
        assert_eq!((pos.line, pos.character), (0, 0));
        
        let pos = byte_offset_to_lsp_position(content, 3);
        assert_eq!((pos.line, pos.character), (0, 1));
        
        let pos = byte_offset_to_lsp_position(content, 6);
        assert_eq!((pos.line, pos.character), (0, 2));
    }

    #[test]
    fn test_byte_offset_multiline_mixed_unicode() {
        let content = "hello\n世界\n😀";
        
        let pos = byte_offset_to_lsp_position(content, 0);
        assert_eq!((pos.line, pos.character), (0, 0));
        
        let pos = byte_offset_to_lsp_position(content, 6);
        assert_eq!((pos.line, pos.character), (1, 0));
        
        let pos = byte_offset_to_lsp_position(content, 9);
        assert_eq!((pos.line, pos.character), (1, 1));
        
        let pos = byte_offset_to_lsp_position(content, 13);
        assert_eq!((pos.line, pos.character), (2, 0));
    }

    #[test]
    fn test_byte_offset_edge_cases() {
        let pos = byte_offset_to_lsp_position("", 0);
        assert_eq!((pos.line, pos.character), (0, 0));
        
        let pos = byte_offset_to_lsp_position("hi", 100);
        assert_eq!(pos.line, 0);
        
        let content = "a\n\nb";
        let pos = byte_offset_to_lsp_position(content, 2);
        assert_eq!((pos.line, pos.character), (1, 0));
        
        let pos = byte_offset_to_lsp_position(content, 3);
        assert_eq!((pos.line, pos.character), (2, 0));
    }

    #[test]
    fn test_span_to_lsp_range_with_content() {
        let content = "hello\nworld\ntest";
        let span = Span { start: 6, end: 11, line: 2, column: 0 };
        
        let range = span_to_lsp_range(&span, Some(content));
        assert_eq!((range.start.line, range.start.character), (1, 0));
        assert_eq!((range.end.line, range.end.character), (1, 5));
    }

    #[test]
    fn test_span_to_lsp_range_fallback_without_content() {
        let span = Span { start: 10, end: 15, line: 3, column: 5 };
        
        let range = span_to_lsp_range(&span, None);
        assert_eq!((range.start.line, range.start.character), (2, 5));
        assert_eq!((range.end.line, range.end.character), (2, 10));
    }

    #[test]
    fn test_span_to_lsp_range_multiline_span() {
        let content = "line1\nline2\nline3";
        let span = Span { start: 0, end: 11, line: 1, column: 0 };
        
        let range = span_to_lsp_range(&span, Some(content));
        assert_eq!((range.start.line, range.start.character), (0, 0));
        assert_eq!((range.end.line, range.end.character), (1, 5));
    }
}
