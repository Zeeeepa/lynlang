use inkwell::builder::BuilderError;
use inkwell::support::LLVMString;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CompileError {
    SyntaxError(String, Option<Span>),
    UndeclaredVariable(String, Option<Span>),
    UndeclaredFunction(String, Option<Span>),
    TypeMismatch {
        expected: String,
        found: String,
        span: Option<Span>,
    },
    InvalidLoopCondition(String, Option<Span>),
    MissingReturnStatement(String, Option<Span>),
    InternalError(String, Option<Span>),
    UnsupportedFeature(String, Option<Span>),
    TypeError(String, Option<Span>),
    FileNotFound(String, Option<String>),
    ParseError(String, Option<Span>),
    ComptimeError(String),
    // Enhanced error types for better LSP diagnostics
    UnexpectedToken {
        expected: Vec<String>,
        found: String,
        span: Option<Span>,
    },
    InvalidPattern(String, Option<Span>),
    ImportError(String, Option<Span>),
    FFIError(String, Option<Span>),
    InvalidSyntax {
        message: String,
        suggestion: String,
        span: Option<Span>,
    },
    MissingTypeAnnotation(String, Option<Span>),
    DuplicateDeclaration {
        name: String,
        first_location: Option<Span>,
        duplicate_location: Option<Span>,
    },
}

impl From<BuilderError> for CompileError {
    fn from(err: BuilderError) -> Self {
        CompileError::InternalError(err.to_string(), None)
    }
}

impl From<String> for CompileError {
    fn from(err: String) -> Self {
        CompileError::InternalError(err, None)
    }
}

impl From<LLVMString> for CompileError {
    fn from(err: LLVMString) -> Self {
        CompileError::InternalError(err.to_string(), None)
    }
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CompileError::SyntaxError(msg, span) => write!(f, "Syntax Error: {}{}", msg, span.as_ref().map(|s| format!(" at line {} column {}", s.line, s.column)).unwrap_or_default()),
            CompileError::UndeclaredVariable(name, span) => write!(f, "Undeclared variable: '{}'{}", name, span.as_ref().map(|s| format!(" at line {} column {}", s.line, s.column)).unwrap_or_default()),
            CompileError::UndeclaredFunction(name, span) => write!(f, "Undeclared function: '{}'{}", name, span.as_ref().map(|s| format!(" at line {} column {}", s.line, s.column)).unwrap_or_default()),
            CompileError::TypeMismatch { expected, found, span } => write!(f, "Type mismatch: Expected {}, found {}{}", expected, found, span.as_ref().map(|s| format!(" at line {} column {}", s.line, s.column)).unwrap_or_default()),
            CompileError::InvalidLoopCondition(msg, span) => write!(f, "Invalid loop condition: {}{}", msg, span.as_ref().map(|s| format!(" at line {} column {}", s.line, s.column)).unwrap_or_default()),
            CompileError::MissingReturnStatement(func_name, span) => write!(f, "Missing return statement in function '{}'{}", func_name, span.as_ref().map(|s| format!(" at line {} column {}", s.line, s.column)).unwrap_or_default()),
            CompileError::InternalError(msg, span) => write!(f, "Internal Compiler Error: {}{}", msg, span.as_ref().map(|s| format!(" at line {} column {}", s.line, s.column)).unwrap_or_default()),
            CompileError::UnsupportedFeature(msg, _) => write!(f, "Unsupported feature: {}", msg),
            CompileError::TypeError(msg, span) => write!(f, "Type error: {}{}", msg, span.as_ref().map(|s| format!(" at line {} column {}", s.line, s.column)).unwrap_or_default()),
            CompileError::FileNotFound(path, detail) => write!(f, "File not found: {}{}", path, detail.as_ref().map(|d| format!(" ({})", d)).unwrap_or_default()),
            CompileError::ParseError(msg, span) => write!(f, "Parse error: {}{}", msg, span.as_ref().map(|s| format!(" at line {} column {}", s.line, s.column)).unwrap_or_default()),
            CompileError::ComptimeError(msg) => write!(f, "Compile-time error: {}", msg),
            CompileError::UnexpectedToken { expected, found, span } => {
                let expected_str = if expected.len() == 1 {
                    expected[0].clone()
                } else {
                    format!("one of {}", expected.join(", "))
                };
                write!(f, "Unexpected token: expected {}, found '{}'{}", expected_str, found, span.as_ref().map(|s| format!(" at line {} column {}", s.line, s.column)).unwrap_or_default())
            },
            CompileError::InvalidPattern(msg, span) => write!(f, "Invalid pattern: {}{}", msg, span.as_ref().map(|s| format!(" at line {} column {}", s.line, s.column)).unwrap_or_default()),
            CompileError::ImportError(msg, span) => write!(f, "Import error: {}{}", msg, span.as_ref().map(|s| format!(" at line {} column {}", s.line, s.column)).unwrap_or_default()),
            CompileError::FFIError(msg, span) => write!(f, "FFI error: {}{}", msg, span.as_ref().map(|s| format!(" at line {} column {}", s.line, s.column)).unwrap_or_default()),
            CompileError::InvalidSyntax { message, suggestion, span } => write!(f, "Invalid syntax: {}. Suggestion: {}{}", message, suggestion, span.as_ref().map(|s| format!(" at line {} column {}", s.line, s.column)).unwrap_or_default()),
            CompileError::MissingTypeAnnotation(name, span) => write!(f, "Missing type annotation for '{}'{}", name, span.as_ref().map(|s| format!(" at line {} column {}", s.line, s.column)).unwrap_or_default()),
            CompileError::DuplicateDeclaration { name, first_location, duplicate_location } => {
                write!(f, "Duplicate declaration of '{}'", name)?;
                if let Some(first) = first_location {
                    write!(f, ", first declared at line {} column {}", first.line, first.column)?;
                }
                if let Some(dup) = duplicate_location {
                    write!(f, ", duplicate at line {} column {}", dup.line, dup.column)?;
                }
                Ok(())
            },
        }
    }
}

impl CompileError {
    /// Extract position information from the error if available
    pub fn position(&self) -> Option<&Span> {
        match self {
            CompileError::SyntaxError(_, span) |
            CompileError::UndeclaredVariable(_, span) |
            CompileError::UndeclaredFunction(_, span) |
            CompileError::InvalidLoopCondition(_, span) |
            CompileError::MissingReturnStatement(_, span) |
            CompileError::InternalError(_, span) |
            CompileError::UnsupportedFeature(_, span) |
            CompileError::TypeError(_, span) |
            CompileError::ParseError(_, span) |
            CompileError::InvalidPattern(_, span) |
            CompileError::ImportError(_, span) |
            CompileError::FFIError(_, span) |
            CompileError::MissingTypeAnnotation(_, span) => span.as_ref(),
            CompileError::TypeMismatch { span, .. } |
            CompileError::UnexpectedToken { span, .. } |
            CompileError::InvalidSyntax { span, .. } => span.as_ref(),
            CompileError::DuplicateDeclaration { duplicate_location, .. } => duplicate_location.as_ref(),
            CompileError::FileNotFound(_, _) |
            CompileError::ComptimeError(_) => None,
        }
    }
    
    /// Get a detailed error message with suggestions for fixing
    pub fn detailed_message(&self, source_lines: &[&str]) -> String {
        let mut result = self.to_string();
        
        // Add context and suggestions based on error type
        match self {
            CompileError::SyntaxError(msg, _span) => {
                if msg.contains("if") || msg.contains("else") || msg.contains("match") {
                    result.push_str("\n\nNote: Zen uses the '?' operator for pattern matching instead of if/else/match keywords.");
                    result.push_str("\nExample: value ? | true => action1 | false => action2");
                }
            },
            CompileError::UndeclaredVariable(name, _) => {
                result.push_str(&format!("\n\nDid you mean to declare '{}'?", name));
                result.push_str("\n  - Use ':=' for immutable variables: name := value");
                result.push_str("\n  - Use '::=' for mutable variables: name ::= value");
            },
            CompileError::InvalidPattern(_msg, _) => {
                result.push_str("\n\nZen pattern matching syntax:");
                result.push_str("\n  value ? | pattern1 => result1");
                result.push_str("\n          | pattern2 => result2");
                result.push_str("\n          | _ => default");
            },
            CompileError::ImportError(msg, _) => {
                if msg.contains("comptime") {
                    result.push_str("\n\nImports are not allowed inside comptime blocks.");
                    result.push_str("\nMove import statements to module level.");
                } else if msg.contains("function") {
                    result.push_str("\n\nImports must be at module level, not inside functions.");
                }
            },
            CompileError::FFIError(_msg, _) => {
                result.push_str("\n\nFFI usage in Zen:");
                result.push_str("\n  lib := FFI.lib(\"library_name\")");
                result.push_str("\n    .path(\"/path/to/library\")");
                result.push_str("\n    .function(\"func_name\", signature)");
                result.push_str("\n    .build()");
            },
            _ => {}
        }
        
        // Add source context if available
        if let Some(span) = self.position() {
            if span.line > 0 && span.line <= source_lines.len() {
                result.push_str("\n\nSource context:");
                let start = span.line.saturating_sub(2).max(1);
                let end = (span.line + 1).min(source_lines.len());
                
                for i in start..=end {
                    let line = source_lines[i - 1];
                    if i == span.line {
                        result.push_str(&format!("\n> {} | {}", i, line));
                        result.push_str(&format!("\n  {} | {}{}", " ".repeat(i.to_string().len()), 
                                                 " ".repeat(span.column), "^--- error here"));
                    } else {
                        result.push_str(&format!("\n  {} | {}", i, line));
                    }
                }
            }
        }
        
        result
    }
}

impl std::error::Error for CompileError {}

pub type Result<T> = std::result::Result<T, CompileError>; 