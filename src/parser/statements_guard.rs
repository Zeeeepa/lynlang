// Guards for common patterns from other languages that provide helpful error messages
// when users try to use syntax from other languages in Zen.

use crate::error::{CompileError, Result, Span};
use crate::lexer::Token;

/// Check if a token is a reserved keyword from another language that Zen doesn't use.
/// Returns an error with a helpful message if it is, or None if it's a valid identifier.
pub fn check_statement_keyword_guard(token: &Token, span: &Span) -> Option<Result<()>> {
    if let Token::Identifier(id) = token {
        match id.as_str() {
            // Variable declaration keywords
            "const" => Some(Err(CompileError::SyntaxError(
                "Zen doesn't use 'const'. Use '=' for immutable bindings (x = 5) or '::=' for mutable (x ::= 5)".to_string(),
                Some(span.clone()),
            ))),
            "let" => Some(Err(CompileError::SyntaxError(
                "Zen doesn't use 'let'. Use '=' for immutable bindings (x = 5) or '::=' for mutable (x ::= 5)".to_string(),
                Some(span.clone()),
            ))),
            "var" => Some(Err(CompileError::SyntaxError(
                "Zen doesn't use 'var'. Use '=' for immutable bindings (x = 5) or '::=' for mutable (x ::= 5)".to_string(),
                Some(span.clone()),
            ))),

            // Control flow keywords
            "if" => Some(Err(CompileError::SyntaxError(
                "Zen doesn't use 'if'. Use '?' for conditionals: condition ? { true_branch } or value ? | pattern => result".to_string(),
                Some(span.clone()),
            ))),
            "else" => Some(Err(CompileError::SyntaxError(
                "Zen doesn't use 'else'. Use pattern matching with '?': condition ? | true => a | false => b".to_string(),
                Some(span.clone()),
            ))),
            "while" => Some(Err(CompileError::SyntaxError(
                "Zen doesn't use 'while'. Use 'loop condition { }' for conditional loops".to_string(),
                Some(span.clone()),
            ))),
            "for" => Some(Err(CompileError::SyntaxError(
                "Zen doesn't use 'for' loops. Use functional iteration: collection.each((item) { ... })".to_string(),
                Some(span.clone()),
            ))),
            "do" => Some(Err(CompileError::SyntaxError(
                "Zen doesn't use 'do'. Use 'loop { }' for loops or blocks directly".to_string(),
                Some(span.clone()),
            ))),
            "goto" => Some(Err(CompileError::SyntaxError(
                "Zen doesn't support 'goto'. Use structured control flow with loops and pattern matching".to_string(),
                Some(span.clone()),
            ))),

            // Pattern matching keywords
            "match" => Some(Err(CompileError::SyntaxError(
                "Zen doesn't use 'match'. Use '?' for pattern matching: value ? | pattern => result".to_string(),
                Some(span.clone()),
            ))),
            "switch" => Some(Err(CompileError::SyntaxError(
                "Zen doesn't use 'switch'. Use '?' for pattern matching: value ? | pattern => result".to_string(),
                Some(span.clone()),
            ))),
            "case" => Some(Err(CompileError::SyntaxError(
                "Zen doesn't use 'case'. Use '?' pattern matching with '|': value ? | pattern => result".to_string(),
                Some(span.clone()),
            ))),

            // Function definition keywords
            "fn" | "func" | "function" => Some(Err(CompileError::SyntaxError(
                "Zen doesn't use 'fn'/'func'/'function'. Define functions as: name = (params) ReturnType { body }".to_string(),
                Some(span.clone()),
            ))),
            "def" => Some(Err(CompileError::SyntaxError(
                "Zen doesn't use 'def'. Define functions as: name = (params) ReturnType { body }".to_string(),
                Some(span.clone()),
            ))),
            "lambda" => Some(Err(CompileError::SyntaxError(
                "Zen doesn't use 'lambda'. Define closures as: (params) { body } or (params) ReturnType { body }".to_string(),
                Some(span.clone()),
            ))),

            // Object creation keywords
            "new" => Some(Err(CompileError::SyntaxError(
                "Zen doesn't use 'new'. Create instances with: Type { field: value } or Type.init()".to_string(),
                Some(span.clone()),
            ))),

            // Error handling keywords
            "try" => Some(Err(CompileError::SyntaxError(
                "Zen doesn't use 'try/catch'. Use '?' for error handling: result ? | .Ok(v) => v | .Err(e) => handle(e)".to_string(),
                Some(span.clone()),
            ))),
            "catch" | "except" => Some(Err(CompileError::SyntaxError(
                "Zen doesn't use 'catch/except'. Use '?' pattern matching for error handling".to_string(),
                Some(span.clone()),
            ))),
            "throw" | "raise" => Some(Err(CompileError::SyntaxError(
                "Zen doesn't use 'throw/raise'. Return error types: return .Err(error)".to_string(),
                Some(span.clone()),
            ))),

            // Async keywords
            "async" | "await" => Some(Err(CompileError::SyntaxError(
                "Zen doesn't use 'async/await'. Use the async runtime: task.spawn(() { ... })".to_string(),
                Some(span.clone()),
            ))),
            "yield" => Some(Err(CompileError::SyntaxError(
                "Zen doesn't use 'yield'. Use iterators or callbacks for lazy evaluation".to_string(),
                Some(span.clone()),
            ))),

            // Null/nil keywords
            "null" | "nil" | "nullptr" | "None" => Some(Err(CompileError::SyntaxError(
                "Zen doesn't use 'null/nil/None'. Use Option type: .Some(value) or .None".to_string(),
                Some(span.clone()),
            ))),

            // Self reference keywords
            "this" => Some(Err(CompileError::SyntaxError(
                "Zen uses '@this' instead of 'this' for self-reference in methods".to_string(),
                Some(span.clone()),
            ))),
            // Note: 'self' is allowed as a regular identifier/parameter name in Zen

            // Visibility/storage keywords
            "static" => Some(Err(CompileError::SyntaxError(
                "Zen doesn't use 'static'. Top-level bindings are module-scoped. Use 'comptime' for compile-time values".to_string(),
                Some(span.clone()),
            ))),
            "private" | "protected" => Some(Err(CompileError::SyntaxError(
                "Zen doesn't use 'private/protected'. Use 'pub' for public visibility, otherwise items are module-private".to_string(),
                Some(span.clone()),
            ))),
            "public" => Some(Err(CompileError::SyntaxError(
                "Zen uses 'pub' instead of 'public' for visibility".to_string(),
                Some(span.clone()),
            ))),

            // Inheritance keywords
            "extends" | "inherits" => Some(Err(CompileError::SyntaxError(
                "Zen doesn't use inheritance. Use composition and traits: Type.implements(Trait, { ... })".to_string(),
                Some(span.clone()),
            ))),
            "super" => Some(Err(CompileError::SyntaxError(
                "Zen doesn't use 'super'. Use composition instead of inheritance".to_string(),
                Some(span.clone()),
            ))),
            "abstract" | "virtual" => Some(Err(CompileError::SyntaxError(
                "Zen doesn't use 'abstract/virtual'. Define traits for polymorphism".to_string(),
                Some(span.clone()),
            ))),
            "final" | "sealed" => Some(Err(CompileError::SyntaxError(
                "Zen doesn't use 'final/sealed'. Types are not inheritable by default".to_string(),
                Some(span.clone()),
            ))),

            // Type introspection keywords
            "typeof" | "instanceof" => Some(Err(CompileError::SyntaxError(
                "Zen doesn't use 'typeof/instanceof'. Use pattern matching: value ? | Type(v) => ...".to_string(),
                Some(span.clone()),
            ))),
            "sizeof" => Some(Err(CompileError::SyntaxError(
                "Zen uses @builtin.sizeof(Type) for size information".to_string(),
                Some(span.clone()),
            ))),

            _ => None,
        }
    } else {
        None
    }
}

/// Check if a token is a reserved keyword from another language for top-level declarations.
/// Returns an error with a helpful message if it is, or None if it's a valid identifier.
pub fn check_declaration_keyword_guard(token: &Token, span: &Span) -> Option<Result<()>> {
    if let Token::Identifier(id) = token {
        match id.as_str() {
            // Variable declaration keywords at top level
            "const" => Some(Err(CompileError::SyntaxError(
                "Zen doesn't use 'const'. Use '=' for immutable bindings (x = 5) or ':=' for compile-time constants (x := 5)".to_string(),
                Some(span.clone()),
            ))),
            "let" => Some(Err(CompileError::SyntaxError(
                "Zen doesn't use 'let'. Use '=' for immutable bindings or '::=' for mutable bindings".to_string(),
                Some(span.clone()),
            ))),
            "var" => Some(Err(CompileError::SyntaxError(
                "Zen doesn't use 'var'. Use '=' for immutable bindings or '::=' for mutable bindings".to_string(),
                Some(span.clone()),
            ))),

            // Function definition keywords
            "fn" | "func" | "function" => Some(Err(CompileError::SyntaxError(
                "Zen doesn't use 'fn'/'func'/'function'. Define functions as: name = (params) ReturnType { body }".to_string(),
                Some(span.clone()),
            ))),

            // Type definition keywords
            "struct" => Some(Err(CompileError::SyntaxError(
                "Zen doesn't use 'struct' keyword. Define structs as: Name : { field: Type, ... }".to_string(),
                Some(span.clone()),
            ))),
            "enum" => Some(Err(CompileError::SyntaxError(
                "Zen doesn't use 'enum' keyword. Define enums as: Name : .Variant1 | .Variant2".to_string(),
                Some(span.clone()),
            ))),
            "trait" | "interface" => Some(Err(CompileError::SyntaxError(
                "Zen doesn't use 'trait'/'interface' keyword. Define traits as: Name : { method: (Self) ReturnType }".to_string(),
                Some(span.clone()),
            ))),
            "impl" => Some(Err(CompileError::SyntaxError(
                "Zen doesn't use standalone 'impl'. Use Type.impl = { ... } to implement methods".to_string(),
                Some(span.clone()),
            ))),
            "class" => Some(Err(CompileError::SyntaxError(
                "Zen doesn't use 'class'. Define structs as: Name : { field: Type, ... } and methods with Name.impl = { ... }".to_string(),
                Some(span.clone()),
            ))),

            // Import keywords
            "import" | "use" | "require" => Some(Err(CompileError::SyntaxError(
                "Zen doesn't use 'import'/'use'/'require'. Import modules as: name = @std.module or { a, b } = @std".to_string(),
                Some(span.clone()),
            ))),
            "package" | "module" | "mod" => Some(Err(CompileError::SyntaxError(
                "Zen doesn't use 'package/module/mod'. Files are modules. Import with: name = @std.module".to_string(),
                Some(span.clone()),
            ))),
            "from" => Some(Err(CompileError::SyntaxError(
                "Zen doesn't use 'from'. Import modules as: { a, b } = @std.module".to_string(),
                Some(span.clone()),
            ))),

            _ => None,
        }
    } else {
        None
    }
}
