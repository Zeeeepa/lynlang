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

    // ========================================================================
    // PARSER HELPER METHODS - Reduce duplication across parser modules
    // ========================================================================

    /// Create a syntax error with the current span
    #[inline]
    pub fn syntax_error(&self, message: impl Into<String>) -> crate::error::CompileError {
        crate::error::CompileError::SyntaxError(message.into(), Some(self.current_span.clone()))
    }

    /// Expect current token to be a specific symbol, return error if not
    pub fn expect_symbol(&mut self, expected: char) -> crate::error::Result<()> {
        if self.current_token == Token::Symbol(expected) {
            self.next_token();
            Ok(())
        } else {
            Err(self.syntax_error(format!("Expected '{}', got {:?}", expected, self.current_token)))
        }
    }

    /// Expect current token to be a specific operator, return error if not
    pub fn expect_operator(&mut self, expected: &str) -> crate::error::Result<()> {
        if self.current_token == Token::Operator(expected.to_string()) {
            self.next_token();
            Ok(())
        } else {
            Err(self.syntax_error(format!("Expected '{}', got {:?}", expected, self.current_token)))
        }
    }

    /// Try to consume a symbol if present, return true if consumed
    pub fn try_consume_symbol(&mut self, symbol: char) -> bool {
        if self.current_token == Token::Symbol(symbol) {
            self.next_token();
            true
        } else {
            false
        }
    }

    /// Try to consume an operator if present, return true if consumed
    pub fn try_consume_operator(&mut self, op: &str) -> bool {
        if self.current_token == Token::Operator(op.to_string()) {
            self.next_token();
            true
        } else {
            false
        }
    }

    /// Extract identifier from current token, or return error
    pub fn expect_identifier(&mut self, context: &str) -> crate::error::Result<String> {
        if let Token::Identifier(name) = &self.current_token {
            let name = name.clone();
            self.next_token();
            Ok(name)
        } else {
            Err(self.syntax_error(format!("Expected {} (identifier), got {:?}", context, self.current_token)))
        }
    }

    /// Check if current token is a specific identifier keyword
    pub fn is_keyword(&self, keyword: &str) -> bool {
        matches!(&self.current_token, Token::Identifier(id) if id == keyword)
    }

    /// Consume a keyword if it matches, return true if consumed
    pub fn try_consume_keyword(&mut self, keyword: &str) -> bool {
        if self.is_keyword(keyword) {
            self.next_token();
            true
        } else {
            false
        }
    }

}
