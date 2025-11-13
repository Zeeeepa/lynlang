// LSP Utility Functions

use lsp_types::*;
use crate::ast::AstType;
use crate::error::CompileError;

// Convert CompileError to LSP Diagnostic
pub fn compile_error_to_diagnostic(error: CompileError) -> Diagnostic {
    // Extract span and determine severity
    let (span, severity, code) = match &error {
        CompileError::ParseError(_, span) => (span.clone(), DiagnosticSeverity::ERROR, Some("parse-error")),
        CompileError::SyntaxError(_, span) => (span.clone(), DiagnosticSeverity::ERROR, Some("syntax-error")),
        CompileError::TypeError(_, span) => (span.clone(), DiagnosticSeverity::ERROR, Some("type-error")),
        CompileError::TypeMismatch { span, .. } => (span.clone(), DiagnosticSeverity::ERROR, Some("type-mismatch")),
        CompileError::UndeclaredVariable(_, span) => (span.clone(), DiagnosticSeverity::ERROR, Some("undeclared-variable")),
        CompileError::UndeclaredFunction(_, span) => (span.clone(), DiagnosticSeverity::ERROR, Some("undeclared-function")),
        CompileError::UnexpectedToken { span, .. } => (span.clone(), DiagnosticSeverity::ERROR, Some("unexpected-token")),
        CompileError::InvalidPattern(_, span) => (span.clone(), DiagnosticSeverity::ERROR, Some("invalid-pattern")),
        CompileError::InvalidSyntax { span, .. } => (span.clone(), DiagnosticSeverity::ERROR, Some("invalid-syntax")),
        CompileError::MissingTypeAnnotation(_, span) => (span.clone(), DiagnosticSeverity::WARNING, Some("missing-type")),
        CompileError::DuplicateDeclaration { duplicate_location, .. } => (duplicate_location.clone(), DiagnosticSeverity::ERROR, Some("duplicate-declaration")),
        CompileError::ImportError(_, span) => (span.clone(), DiagnosticSeverity::ERROR, Some("import-error")),
        CompileError::FFIError(_, span) => (span.clone(), DiagnosticSeverity::ERROR, Some("ffi-error")),
        CompileError::InvalidLoopCondition(_, span) => (span.clone(), DiagnosticSeverity::ERROR, Some("invalid-loop")),
        CompileError::MissingReturnStatement(_, span) => (span.clone(), DiagnosticSeverity::WARNING, Some("missing-return")),
        CompileError::InternalError(_, span) => (span.clone(), DiagnosticSeverity::ERROR, Some("internal-error")),
        CompileError::UnsupportedFeature(_, span) => (span.clone(), DiagnosticSeverity::WARNING, Some("unsupported-feature")),
        CompileError::FileNotFound(_, _) => (None, DiagnosticSeverity::ERROR, Some("file-not-found")),
        CompileError::ComptimeError(_) => (None, DiagnosticSeverity::ERROR, Some("comptime-error")),
        CompileError::BuildError(_) => (None, DiagnosticSeverity::ERROR, Some("build-error")),
        CompileError::FileError(_) => (None, DiagnosticSeverity::ERROR, Some("file-error")),
        CompileError::CyclicDependency(_) => (None, DiagnosticSeverity::ERROR, Some("cyclic-dependency")),
    };

    // Convert span to LSP range
    let (start_pos, end_pos) = if let Some(span) = span {
        let start = Position {
            line: if span.line > 0 { span.line as u32 - 1 } else { 0 },
            character: span.column as u32,
        };
        let end = Position {
            line: if span.line > 0 { span.line as u32 - 1 } else { 0 },
            character: (span.column + (span.end - span.start).max(1)) as u32,
        };
        (start, end)
    } else {
        (Position { line: 0, character: 0 }, Position { line: 0, character: 1 })
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
        AstType::StaticLiteral => "str".to_string(),  // Internal string literal type
        AstType::StaticString => "StaticString".to_string(),
        AstType::Struct { name, .. } if name == "String" => "String".to_string(),
        AstType::Void => "void".to_string(),
        AstType::Ptr(inner) => format!("Ptr<{}>", format_type(inner)),
        AstType::MutPtr(inner) => format!("MutPtr<{}>", format_type(inner)),
        AstType::RawPtr(inner) => format!("RawPtr<{}>", format_type(inner)),
        AstType::Ref(inner) => format!("&{}", format_type(inner)),
        AstType::Range { start_type, end_type, inclusive } => {
            if *inclusive {
                format!("{}..={}", format_type(start_type), format_type(end_type))
            } else {
                format!("{}..{}", format_type(start_type), format_type(end_type))
            }
        }
        AstType::FunctionPointer { param_types, return_type } => {
            format!("fn({}) {}",
                param_types.iter().map(|p| format_type(p)).collect::<Vec<_>>().join(", "),
                format_type(return_type))
        }
        AstType::EnumType { name } => name.clone(),
        AstType::StdModule => "module".to_string(),
        AstType::Array(elem) => format!("Array<{}>", format_type(elem)),
        AstType::Vec { element_type, size } => format!("Vec<{}, {}>", format_type(element_type), size),
        AstType::DynVec { element_types, .. } => {
            if element_types.len() == 1 {
                format!("DynVec<{}>", format_type(&element_types[0]))
            } else {
                "DynVec<...>".to_string()
            }
        },
        AstType::FixedArray { element_type, size } => format!("[{}; {}]", format_type(element_type), size),
        AstType::Option(inner) => format!("Option<{}>", format_type(inner)),
        AstType::Result { ok_type, err_type } => format!("Result<{}, {}>", format_type(ok_type), format_type(err_type)),
        AstType::Struct { name, .. } => name.clone(),
        AstType::Enum { name, .. } => name.clone(),
        AstType::Generic { name, type_args } => {
            if type_args.is_empty() {
                name.clone()
            } else {
                format!("{}<{}>", name,
                    type_args.iter().map(|p| format_type(p)).collect::<Vec<_>>().join(", "))
            }
        }
        AstType::Function { args, return_type } => {
            format!("({}) {}",
                args.iter().map(|p| format_type(p)).collect::<Vec<_>>().join(", "),
                format_type(return_type))
        }
    }
}
