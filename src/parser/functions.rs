use super::core::Parser;
use crate::ast::{Function, TypeParameter};
use crate::error::{CompileError, Result};
use crate::lexer::Token;

impl<'a> Parser<'a> {
    pub fn parse_function(&mut self) -> Result<Function> {
        // Function name
        let name = if let Token::Identifier(name) = &self.current_token {
            name.clone()
        } else {
            return Err(CompileError::SyntaxError(
                "Expected function name".to_string(),
                Some(self.current_span.clone()),
            ));
        };
        self.next_token();
        
        // Parse generic type parameters if present: <T, U, ...>
        let mut type_params = Vec::new();
        if self.current_token == Token::Operator("<".to_string()) {
            self.next_token();
            loop {
                if let Token::Identifier(gen) = &self.current_token {
                    type_params.push(TypeParameter {
                        name: gen.clone(),
                        constraints: Vec::new(),
                    });
                    self.next_token();
                    
                    if self.current_token == Token::Operator(">".to_string()) {
                        self.next_token();
                        break;
                    } else if self.current_token == Token::Symbol(',') {
                        self.next_token();
                    } else {
                        return Err(CompileError::SyntaxError(
                            "Expected ',' or '>' in generic parameters".to_string(),
                            Some(self.current_span.clone()),
                        ));
                    }
                } else {
                    return Err(CompileError::SyntaxError(
                        "Expected generic parameter name".to_string(),
                        Some(self.current_span.clone()),
                    ));
                }
            }
        }
        
        // Check for ':' or '=' for function definition
        // Both syntaxes are valid: name : (params) or name = (params)
        let uses_equals_syntax = if self.current_token == Token::Symbol(':') {
            self.next_token();
            false
        } else if self.current_token == Token::Operator("=".to_string()) {
            self.next_token();
            true
        } else {
            return Err(CompileError::SyntaxError(
                "Expected ':' or '=' after function name for type definition".to_string(),
                Some(self.current_span.clone()),
            ));
        };
        
        // Parameters
        if self.current_token != Token::Symbol('(') {
            return Err(CompileError::SyntaxError(
                "Expected '(' for function parameters".to_string(),
                Some(self.current_span.clone()),
            ));
        }
        self.next_token();
        
        let mut args = vec![];
        if self.current_token != Token::Symbol(')') {
            loop {
                // Parameter name
                let param_name = if let Token::Identifier(name) = &self.current_token {
                    name.clone()
                } else {
                    return Err(CompileError::SyntaxError(
                        "Expected parameter name".to_string(),
                        Some(self.current_span.clone()),
                    ));
                };
                self.next_token();
                
                // Parameter type
                if self.current_token != Token::Symbol(':') {
                    return Err(CompileError::SyntaxError(
                        "Expected ':' after parameter name".to_string(),
                        Some(self.current_span.clone()),
                    ));
                }
                self.next_token();
                
                let param_type = self.parse_type()?;
                args.push((param_name, param_type));
                
                if self.current_token == Token::Symbol(')') {
                    break;
                }
                if self.current_token != Token::Symbol(',') {
                    return Err(CompileError::SyntaxError(
                        "Expected ',' or ')' in parameter list".to_string(),
                        Some(self.current_span.clone()),
                    ));
                }
                self.next_token();
            }
        }
        self.next_token(); // consume ')'
        
        // Parse return type (required in zen, comes directly after parentheses)
        let return_type = if uses_equals_syntax {
            // For '=' syntax, return type comes directly after ')'
            if self.current_token == Token::Symbol('{') {
                // No return type specified, default to void
                crate::ast::AstType::Void
            } else {
                // Parse the return type
                self.parse_type()?
            }
        } else {
            // For ':' syntax, we may have '=' before the body
            if self.current_token == Token::Operator("=".to_string()) {
                // If we see '=' immediately, default to void
                self.next_token();
                crate::ast::AstType::Void
            } else {
                // Parse the return type
                let ret_type = self.parse_type()?;
                // After return type, expect '=' before body
                if self.current_token != Token::Operator("=".to_string()) {
                    return Err(CompileError::SyntaxError(
                        "Expected '=' after return type before function body".to_string(),
                        Some(self.current_span.clone()),
                    ));
                }
                self.next_token(); // consume '='
                ret_type
            }
        };
        
        // Function body
        if self.current_token != Token::Symbol('{') {
            return Err(CompileError::SyntaxError(
                "Expected '{' for function body".to_string(),
                Some(self.current_span.clone()),
            ));
        }
        self.next_token();
        
        let mut body = vec![];
        while self.current_token != Token::Symbol('}') && self.current_token != Token::Eof {
            body.push(self.parse_statement()?);
        }
        
        if self.current_token != Token::Symbol('}') {
            return Err(CompileError::SyntaxError(
                "Expected '}' to close function body".to_string(),
                Some(self.current_span.clone()),
            ));
        }
        self.next_token();
        
        Ok(Function {
            name,
            type_params,
            args,
            return_type,
            body,
        })
    }
}
