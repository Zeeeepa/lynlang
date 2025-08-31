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

    pub fn debug_current_token(&self) -> &Token {
        &self.current_token
    }

    pub fn debug_peek_token(&self) -> &Token {
        &self.peek_token
    }
    
    /// Check if the current position looks like an import statement
    pub fn is_import_statement(&self) -> bool {
        // Check for @std imports
        if let Token::Symbol('@') = self.current_token {
            if let Token::Identifier(ref s) = self.peek_token {
                return s == "std";
            }
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
    pub fn is_import_in_statement(&self, stmt: &crate::ast::Statement) -> bool {
        use crate::ast::{Statement, Expression};
        
        match stmt {
            Statement::VariableDeclaration { initializer, .. } => {
                if let Some(expr) = initializer {
                    self.is_import_expression(expr)
                } else {
                    false
                }
            }
            _ => false
        }
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
                    name.starts_with("@std")
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
                name.starts_with("@std")
            }
            _ => false
        }
    }
}
