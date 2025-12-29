use super::super::lexer::{Lexer, Token};
use crate::error::Span;

pub struct Parser<'a> {
    pub(crate) lexer: Lexer<'a>,
    pub(crate) current_token: Token,
    pub(crate) peek_token: Token,
    pub(crate) current_span: Span,
    pub(crate) peek_span: Span,
}

impl<'a> Parser<'a> {
    pub fn new(mut lexer: Lexer<'a>) -> Self {
        let current_token_with_span = lexer.next_token_with_span();
        let peek_token_with_span = lexer.next_token_with_span();
        Parser {
            lexer,
            current_token: current_token_with_span.token,
            peek_token: peek_token_with_span.token,
            current_span: current_token_with_span.span,
            peek_span: peek_token_with_span.span,
        }
    }

    pub fn next_token(&mut self) {
        let token_with_span = self.lexer.next_token_with_span();
        self.current_token = self.peek_token.clone();
        self.peek_token = token_with_span.token;
        self.current_span = self.peek_span.clone();
        self.peek_span = token_with_span.span;
    }

    #[allow(dead_code)]
    pub fn debug_current_token(&self) -> &Token {
        &self.current_token
    }

    #[allow(dead_code)]
    pub fn debug_peek_token(&self) -> &Token {
        &self.peek_token
    }

    /// Check if the current position looks like an import statement
    #[allow(dead_code)]
    pub fn is_import_statement(&self) -> bool {
        // Check for @std or @builtin imports
        if self.current_token == Token::AtStd || self.current_token == Token::AtBuiltin {
            return true;
        }

        // Check for identifiers that might be followed by := @std or .import()
        if let Token::Identifier(_) = self.current_token {
            if self.peek_token == Token::Symbol(':') {
                // Might be := @std style import
                return true;
            }
        }

        false
    }

    /// Check if a statement contains an import
    #[allow(dead_code)]
    pub fn is_import_in_statement(&self, stmt: &crate::ast::Statement) -> bool {
        use crate::ast::Statement;

        match stmt {
            Statement::VariableDeclaration { initializer, .. } => {
                if let Some(expr) = initializer {
                    self.is_import_expression(expr)
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Peek at the token after peek_token (two tokens ahead)
    /// This is a simplified version - in a full implementation we'd cache this
    pub fn peek_peek_token(&mut self) -> Option<Token> {
        // Save current state (including line/column for accurate error reporting)
        let saved_pos = self.lexer.position;
        let saved_read_pos = self.lexer.read_position;
        let saved_char = self.lexer.current_char;
        let saved_line = self.lexer.line;
        let saved_column = self.lexer.column;

        // Advance past current and peek tokens
        let _ = self.lexer.next_token_with_span();
        let _ = self.lexer.next_token_with_span();
        let next_next = self.lexer.next_token_with_span();

        // Restore state
        self.lexer.position = saved_pos;
        self.lexer.read_position = saved_read_pos;
        self.lexer.current_char = saved_char;
        self.lexer.line = saved_line;
        self.lexer.column = saved_column;

        Some(next_next.token)
    }

    /// Check if an expression is an import expression
    pub fn is_import_expression(&self, expr: &crate::ast::Expression) -> bool {
        use crate::ast::Expression;

        match expr {
            Expression::MemberAccess { object, member } => {
                // Check if it's @std.something or build.import()
                if member == "import" {
                    return true;
                }
                if let Expression::Identifier(name) = &**object {
                    name == "@std" || name.starts_with("@std")
                } else {
                    false
                }
            }
            Expression::FunctionCall { name, .. } => {
                // Check for .import() calls
                name == "import" || name.ends_with(".import")
            }
            Expression::Identifier(name) => {
                // Check for direct @std references
                name == "@std" || name.starts_with("@std")
            }
            _ => false,
        }
    }
}
