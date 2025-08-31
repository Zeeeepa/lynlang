use super::core::Parser;
use crate::ast::{Statement, Expression};
use crate::error::{CompileError, Result};
use crate::lexer::Token;

impl<'a> Parser<'a> {
    /// Parse a comptime block: comptime { ... }
    pub fn parse_comptime_block(&mut self) -> Result<Statement> {
        // Expect 'comptime' keyword (already consumed)
        
        // Expect '{'
        if self.current_token != Token::Symbol('{') {
            return Err(CompileError::SyntaxError(
                "Expected '{' after 'comptime'".to_string(),
                Some(self.current_span.clone()),
            ));
        }
        self.next_token();
        
        // Parse statements inside the comptime block
        let mut statements = Vec::new();
        
        while self.current_token != Token::Symbol('}') && self.current_token != Token::Eof {
            // Check for import attempts inside comptime block
            if let Token::Identifier(id) = &self.current_token {
                if self.peek_token == Token::Symbol(':') {
                    // Look ahead to check for := @std pattern
                    let saved_current = self.current_token.clone();
                    let saved_peek = self.peek_token.clone();
                    let saved_span = self.current_span.clone();
                    
                    self.next_token(); // move to :
                    if self.current_token == Token::Symbol(':') && self.peek_token == Token::Symbol('=') {
                        self.next_token(); // move to =
                        self.next_token(); // move past :=
                        
                        // Check if it's an import
                        if let Token::Identifier(import_id) = &self.current_token {
                            if import_id.starts_with("@std") || import_id == "build" {
                                return Err(CompileError::SyntaxError(
                                    "Imports are not allowed inside comptime blocks. Move imports to module level".to_string(),
                                    Some(saved_span),
                                ));
                            }
                        }
                    }
                    
                    // Restore state
                    self.current_token = saved_current;
                    self.peek_token = saved_peek;
                    self.current_span = saved_span;
                }
            }
            
            let stmt = self.parse_statement()?;
            statements.push(stmt);
            
            // Skip optional semicolons
            if self.current_token == Token::Symbol(';') {
                self.next_token();
            }
        }
        
        // Expect '}'
        if self.current_token != Token::Symbol('}') {
            return Err(CompileError::SyntaxError(
                "Expected '}' to close comptime block".to_string(),
                Some(self.current_span.clone()),
            ));
        }
        self.next_token();
        
        Ok(Statement::ComptimeBlock(statements))
    }
    
    /// Parse a comptime expression: comptime expr
    pub fn parse_comptime_expression(&mut self) -> Result<Expression> {
        // 'comptime' keyword already consumed
        let expr = self.parse_expression()?;
        Ok(Expression::Comptime(Box::new(expr)))
    }
    
    /// Check if we're in a comptime context and handle accordingly
    pub fn parse_comptime(&mut self) -> Result<Statement> {
        // Check what follows 'comptime'
        if self.peek_token == Token::Symbol('{') {
            self.next_token(); // consume 'comptime'
            self.parse_comptime_block()
        } else {
            // It's a comptime expression, wrap it in a statement
            self.next_token(); // consume 'comptime'
            let expr = self.parse_comptime_expression()?;
            Ok(Statement::Expression(expr))
        }
    }
}
