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
            // Check for import attempts inside comptime blocks
            if let Token::Identifier(name) = &self.current_token {
                if self.peek_token == Token::Operator(":=".to_string()) {
                    // Look ahead to see if this is an import
                    let saved_current = self.current_token.clone();
                    let saved_peek = self.peek_token.clone();
                    let saved_pos = self.lexer.position;
                    let saved_read = self.lexer.read_position;
                    let saved_char = self.lexer.current_char;
                    
                    self.next_token(); // Move to :=
                    self.next_token(); // Move past :=
                    
                    let is_import = if let Token::Identifier(id) = &self.current_token {
                        id.starts_with("@") || id == "build"
                    } else {
                        false
                    };
                    
                    // Restore state
                    self.current_token = saved_current;
                    self.peek_token = saved_peek;
                    self.lexer.position = saved_pos;
                    self.lexer.read_position = saved_read;
                    self.lexer.current_char = saved_char;
                    
                    if is_import {
                        return Err(CompileError::SyntaxError(
                            "Import statements are not allowed inside comptime blocks. Move imports to module level.".to_string(),
                            Some(self.current_span.clone()),
                        ));
                    }
                }
            }
            
            // Parse regular statement
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
