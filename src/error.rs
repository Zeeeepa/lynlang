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
    BuildError(String),
    FileError(String),
    CyclicDependency(String),
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
            CompileError::SyntaxError(msg, span) => write!(f, "Syntax Error: {}{}", msg, span.as_ref().map(|s| format!(" at line {} column {}", s.line, s.column + 1)).unwrap_or_default()),
            CompileError::UndeclaredVariable(name, span) => write!(f, "Undeclared variable: '{}'{}", name, span.as_ref().map(|s| format!(" at line {} column {}", s.line, s.column + 1)).unwrap_or_default()),
            CompileError::UndeclaredFunction(name, span) => write!(f, "Undeclared function: '{}'{}", name, span.as_ref().map(|s| format!(" at line {} column {}", s.line, s.column + 1)).unwrap_or_default()),
            CompileError::TypeMismatch { expected, found, span } => write!(f, "Type mismatch: Expected {}, found {}{}", expected, found, span.as_ref().map(|s| format!(" at line {} column {}", s.line, s.column + 1)).unwrap_or_default()),
            CompileError::InvalidLoopCondition(msg, span) => write!(f, "Invalid loop condition: {}{}", msg, span.as_ref().map(|s| format!(" at line {} column {}", s.line, s.column + 1)).unwrap_or_default()),
            CompileError::MissingReturnStatement(func_name, span) => write!(f, "Missing return statement in function '{}'{}", func_name, span.as_ref().map(|s| format!(" at line {} column {}", s.line, s.column + 1)).unwrap_or_default()),
            CompileError::InternalError(msg, span) => write!(f, "Internal Compiler Error: {}{}", msg, span.as_ref().map(|s| format!(" at line {} column {}", s.line, s.column + 1)).unwrap_or_default()),
            CompileError::UnsupportedFeature(msg, _) => write!(f, "Unsupported feature: {}", msg),
            CompileError::TypeError(msg, span) => write!(f, "Type error: {}{}", msg, span.as_ref().map(|s| format!(" at line {} column {}", s.line, s.column + 1)).unwrap_or_default()),
            CompileError::FileNotFound(path, detail) => write!(f, "File not found: {}{}", path, detail.as_ref().map(|d| format!(" ({})", d)).unwrap_or_default()),
            CompileError::ParseError(msg, span) => write!(f, "Parse error: {}{}", msg, span.as_ref().map(|s| format!(" at line {} column {}", s.line, s.column + 1)).unwrap_or_default()),
            CompileError::ComptimeError(msg) => write!(f, "Compile-time error: {}", msg),
            CompileError::UnexpectedToken { expected, found, span } => {
                let expected_str = if expected.len() == 1 {
                    expected[0].clone()
                } else {
                    format!("one of {}", expected.join(", "))
                };
                write!(f, "Unexpected token: expected {}, found '{}'{}", expected_str, found, span.as_ref().map(|s| format!(" at line {} column {}", s.line, s.column + 1)).unwrap_or_default())
            },
            CompileError::InvalidPattern(msg, span) => write!(f, "Invalid pattern: {}{}", msg, span.as_ref().map(|s| format!(" at line {} column {}", s.line, s.column + 1)).unwrap_or_default()),
            CompileError::ImportError(msg, span) => write!(f, "Import error: {}{}", msg, span.as_ref().map(|s| format!(" at line {} column {}", s.line, s.column + 1)).unwrap_or_default()),
            CompileError::FFIError(msg, span) => write!(f, "FFI error: {}{}", msg, span.as_ref().map(|s| format!(" at line {} column {}", s.line, s.column + 1)).unwrap_or_default()),
            CompileError::InvalidSyntax { message, suggestion, span } => write!(f, "Invalid syntax: {}. Suggestion: {}{}", message, suggestion, span.as_ref().map(|s| format!(" at line {} column {}", s.line, s.column + 1)).unwrap_or_default()),
            CompileError::MissingTypeAnnotation(name, span) => write!(f, "Missing type annotation for '{}'{}", name, span.as_ref().map(|s| format!(" at line {} column {}", s.line, s.column + 1)).unwrap_or_default()),
            CompileError::DuplicateDeclaration { name, first_location, duplicate_location } => {
                write!(f, "Duplicate declaration of '{}'", name)?;
                if let Some(first) = first_location {
                    write!(f, ", first declared at line {} column {}", first.line, first.column + 1)?;
                }
                if let Some(dup) = duplicate_location {
                    write!(f, ", duplicate at line {} column {}", dup.line, dup.column + 1)?;
                }
                Ok(())
            },
            CompileError::BuildError(msg) => write!(f, "Build error: {}", msg),
            CompileError::FileError(msg) => write!(f, "File error: {}", msg),
            CompileError::CyclicDependency(module) => write!(f, "Cyclic dependency detected: {}", module),
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
            CompileError::ComptimeError(_) |
            CompileError::BuildError(_) |
            CompileError::FileError(_) |
            CompileError::CyclicDependency(_) => None,
        }
    }
    
    /// Get a detailed error message with suggestions for fixing
    pub fn detailed_message(&self, source_lines: &[&str]) -> String {
        let mut result = self.to_string();
        
        // Add source code context if we have position information
        if let Some(span) = self.position() {
            if span.line > 0 && span.line <= source_lines.len() {
                let line_idx = span.line - 1;
                let line = source_lines[line_idx];
                
                // Show context with error location
                result.push_str("\n\n📍 Error Location:");
                result.push_str(&format!("\n   {} | {}", span.line, line));
                
                // Add pointer to exact column (convert from 0-based to display position)
                let prefix_len = format!("   {} | ", span.line).len();
                let pointer = " ".repeat(prefix_len + span.column) + "^";
                
                // For multi-character errors, show the full span
                if span.end > span.start {
                    let error_len = span.end - span.start;
                    let underline = "^".repeat(error_len.min(line.len().saturating_sub(span.column)));
                    result.push_str(&format!("\n{}{}\n", " ".repeat(prefix_len + span.column), underline));
                } else {
                    result.push_str(&format!("\n{}\n", pointer));
                }
                
                // Add surrounding context (lines before and after)
                if line_idx > 0 {
                    let prev_line = source_lines[line_idx - 1];
                    result.insert_str(result.find("📍 Error Location:").unwrap() + 18,
                        &format!("\n   {} | {}", span.line - 1, prev_line));
                }
                if line_idx + 1 < source_lines.len() {
                    let next_line = source_lines[line_idx + 1];
                    result.push_str(&format!("   {} | {}", span.line + 1, next_line));
                }
            }
        }
        
        // Add context and suggestions based on error type
        match self {
            CompileError::SyntaxError(msg, _span) => {
                result.push_str("\n\n💡 Suggestions:");
                if msg.contains("if") || msg.contains("else") || msg.contains("match") {
                    result.push_str("\n  • Zen uses the '?' operator for pattern matching instead of if/else/match keywords.");
                    result.push_str("\n  • Example: value ? | true => action1 | false => action2");
                    result.push_str("\n  • For boolean checks: condition ? { do_something() }");
                } else if msg.contains("function") {
                    result.push_str("\n  • Function syntax in Zen: name = (params) ReturnType { body }");
                    result.push_str("\n  • Example: add = (a: i32, b: i32) i32 { a + b }");
                    result.push_str("\n  • No parameters: greet = () void { print(\"Hello\") }");
                    result.push_str("\n  • With generics: identity<T> = (value: T) T { value }");
                } else if msg.contains("struct") {
                    result.push_str("\n\nStruct syntax in Zen:");
                    result.push_str("\n  StructName = { field1: Type, field2: Type }");
                } else if msg.contains("loop") {
                    result.push_str("\n\nLoop syntax in Zen:");
                    result.push_str("\n  loop (condition) { body }  // conditional loop");
                    result.push_str("\n  loop { body }              // infinite loop");
                } else if msg.contains("variable") || msg.contains("binding") {
                    result.push_str("\n\nVariable declaration in Zen:");
                    result.push_str("\n  • Immutable: name = value  or  name: Type = value");
                    result.push_str("\n  • Mutable: name ::= value  or  name:: Type = value");
                    result.push_str("\n  • Example: counter ::= 0  // mutable counter");
                } else if msg.contains("pattern") {
                    result.push_str("\n\nPattern matching in Zen:");
                    result.push_str("\n  • Basic: value ? | pattern => result");
                    result.push_str("\n  • Multiple: value ? | 1  { \"one\" | 2  { \"two\" | _  { \"other\" } } }");
                    result.push_str("\n  • With binding: value ? | Some(x) { use_x(x) }");
                } else if msg.contains("import") || msg.contains("module") {
                    result.push_str("\n\nModule system in Zen:");
                    result.push_str("\n  • Import: io := @std.build.import(\"io\")");
                    result.push_str("\n  • Destructure: { Vec, HashMap } := @std.build.import(\"collections\")");
                    result.push_str("\n  • Note: Imports must be at module level, not inside functions");
                }
            },
            CompileError::UndeclaredVariable(name, _) => {
                result.push_str(&format!("\n\nDid you mean to declare '{}'?", name));
                result.push_str("\n  - Use ':=' for immutable variables: name := value");
                result.push_str("\n  - Use '::=' for mutable variables: name ::= value");
                result.push_str("\n  - Use ': Type =' for typed immutable: name: Type = value");
                result.push_str("\n  - Use ':: Type =' for typed mutable: name:: Type = value");
            },
            CompileError::InvalidPattern(msg, _) => {
                result.push_str("\n\nZen pattern matching syntax:");
                result.push_str("\n  value ? | pattern1 { result1 }");
                result.push_str("\n          | pattern2 { result2 }");
                result.push_str("\n          | _  { default }");
                result.push_str("\n\nPatterns can include:");
                result.push_str("\n  - Literals: 42, \"hello\"");
                result.push_str("\n  - Ranges: 0..=10");
                result.push_str("\n  - Destructuring: Ok(val)");
                result.push_str("\n  - Guards: x -> x > 0");
                
                if msg.contains("bool") {
                    result.push_str("\n\nBool patterns (special syntax):");
                    result.push_str("\n  condition ? { true_branch }  // simple bool check");
                    result.push_str("\n  condition ? | 1  { true_branch }");
                    result.push_str("\n              | 0  { false_branch }");
                }
            },
            CompileError::ImportError(msg, _) => {
                if msg.contains("comptime") {
                    result.push_str("\n\nImports are not allowed inside comptime blocks.");
                    result.push_str("\nMove import statements to module level.");
                } else if msg.contains("function") {
                    result.push_str("\n\nImports must be at module level, not inside functions.");
                } else {
                    result.push_str("\n\nImport syntax in Zen:");
                    result.push_str("\n  build := @std.build");
                    result.push_str("\n  io := build.import(\"io\")");
                    result.push_str("\n  { Vec, HashMap } := build.import(\"collections\")");
                }
            },
            CompileError::FFIError(msg, _) => {
                result.push_str("\n\nFFI usage in Zen:");
                result.push_str("\n  lib := FFI.lib(\"library_name\")");
                result.push_str("\n    .path(\"/path/to/library\")");
                result.push_str("\n    .function(\"func_name\", signature)");
                result.push_str("\n    .constant(\"CONST_NAME\", type)");
                result.push_str("\n    .build()");
                
                if msg.contains("signature") {
                    result.push_str("\n\nFunction signature example:");
                    result.push_str("\n  FnSignature::new(vec![types::i32()], types::void())");
                }
            },
            CompileError::TypeMismatch { expected, found, .. } => {
                result.push_str(&format!("\n\nType conversion needed:"));
                result.push_str(&format!("\n  Expected: {}", expected));
                result.push_str(&format!("\n  Found: {}", found));
                
                // Suggest common conversions
                if expected == "i32" && found == "f64" {
                    result.push_str("\n  Use: value as i32");
                } else if expected == "string" && found.contains("str") {
                    result.push_str("\n  Use: value.to_string()");
                }
            },
            CompileError::UnexpectedToken { expected, found, .. } => {
                result.push_str("\n\nExpected tokens:");
                for exp in expected {
                    result.push_str(&format!("\n  - {}", exp));
                }
                result.push_str(&format!("\nFound: '{}'", found));
                
                // Common fixes
                if expected.contains(&"=".to_string()) {
                    result.push_str("\n\nDid you forget '=' in a function or variable declaration?");
                } else if expected.contains(&")".to_string()) {
                    result.push_str("\n\nCheck for unclosed parentheses");
                } else if expected.contains(&"}".to_string()) {
                    result.push_str("\n\nCheck for unclosed braces");
                }
            },
            CompileError::MissingTypeAnnotation(name, _) => {
                result.push_str(&format!("\n\nVariable '{}' needs a type annotation.", name));
                result.push_str("\n\nOptions:");
                result.push_str("\n  1. Use type inference: name := value");
                result.push_str("\n  2. Explicit type: name: Type = value");
                result.push_str("\n  3. Mutable with type: name:: Type = value");
            },
            CompileError::DuplicateDeclaration { name, .. } => {
                result.push_str(&format!("\n\n'{}' has already been declared.", name));
                result.push_str("\n\nPossible solutions:");
                result.push_str("\n  1. Use a different name");
                result.push_str("\n  2. Remove the duplicate declaration");
                result.push_str("\n  3. Use shadowing in a nested scope");
            },
            CompileError::InvalidLoopCondition(msg, _) => {
                result.push_str("\n\nValid loop forms:");
                result.push_str("\n  loop { ... }           // infinite loop");
                result.push_str("\n  loop (condition) { ... } // conditional loop");
                result.push_str("\n  (0..10).loop((i) => { ... }) // range iteration");
                
                if msg.contains("bool") {
                    result.push_str("\n\nLoop condition must evaluate to bool");
                }
            },
            CompileError::MissingReturnStatement(func, _) => {
                result.push_str(&format!("\n\nFunction '{}' must return a value.", func));
                result.push_str("\n\nOptions:");
                result.push_str("\n  1. Add explicit return: return value");
                result.push_str("\n  2. Use implicit return (no semicolon on last expression)");
                result.push_str("\n  3. Change return type to void if no value needed");
            },
            CompileError::ComptimeError(msg) => {
                result.push_str("\n\n💡 Comptime Requirements:");
                result.push_str("\n  • Code must be deterministic");
                result.push_str("\n  • No side effects allowed");
                result.push_str("\n  • Only pure computations");
                result.push_str("\n  • No imports in comptime blocks");
                
                if msg.contains("import") {
                    result.push_str("\n\n⚠️ Imports must be at module level, outside comptime blocks");
                }
            },
            CompileError::InvalidSyntax { suggestion, .. } => {
                result.push_str("\n\n💡 Suggestion: ");
                result.push_str(suggestion);
            },
            _ => {}
        }
        
        result
    }
}

impl std::error::Error for CompileError {}

pub type Result<T> = std::result::Result<T, CompileError>; 