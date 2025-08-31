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
            // Check for import pattern: identifier := @std...
            if let Token::Identifier(_name) = &self.current_token {
                if let Token::Operator(op) = &self.peek_token {
                    if op == ":=" {
                        // Look ahead to check if it's an import
                        let saved_current = self.current_token.clone();
                        let saved_peek = self.peek_token.clone();
                        let saved_lex_pos = self.lexer.position;
                        let saved_lex_read = self.lexer.read_position;
                        let saved_lex_char = self.lexer.current_char;
                        
                        self.next_token(); // Move to :=
                        self.next_token(); // Move past :=
                        
                        let is_import = match &self.current_token {
                            Token::Identifier(id) => {
                                // Check for @std patterns (could be @std.core, @std.build, etc.)
                                id.starts_with("@std") || id == "build" || id.starts_with("@")
                            }
                            Token::Symbol('@') => {
                                // In case @ is treated as a separate symbol
                                true
                            }
                            _ => false
                        };
                        
                        // Restore state
                        self.current_token = saved_current;
                        self.peek_token = saved_peek;
                        self.lexer.position = saved_lex_pos;
                        self.lexer.read_position = saved_lex_read;
                        self.lexer.current_char = saved_lex_char;
                        
                        if is_import {
                            return Err(CompileError::SyntaxError(
                                "Import statements are not allowed inside comptime blocks. Move imports to module level.".to_string(),
                                Some(self.current_span.clone()),
                            ));
                        }
                    }
                }
            }
            
            let stmt = self.parse_statement()?;
            
            // Check if the parsed statement contains an import
            if let Statement::VariableDeclaration { initializer, .. } = &stmt {
                if let Some(expr) = initializer {
                    // Check for @std patterns or build.import calls
                    let is_import = match expr {
                        Expression::MemberAccess { object, .. } => {
                            if let Expression::Identifier(id) = &**object {
                                id.starts_with("@std") || id == "@std"
                            } else {
                                false
                            }
                        }
                        Expression::FunctionCall { name, .. } => {
                            name.contains("import") || name == "build.import"
                        }
                        Expression::Identifier(id) => {
                            id.starts_with("@std")
                        }
                        _ => false
                    };
                    
                    if is_import {
                        return Err(CompileError::SyntaxError(
                            "Import statements are not allowed inside comptime blocks. Move imports to module level.".to_string(),
                            Some(self.current_span.clone()),
                        ));
                    }
                }
            }
            
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
