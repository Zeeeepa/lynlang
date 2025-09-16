use super::core::Parser;
use crate::ast::{StructDefinition, StructField, AstType, Function, TypeParameter};
use crate::error::{CompileError, Result};
use crate::lexer::Token;

impl<'a> Parser<'a> {
    pub fn parse_struct(&mut self) -> Result<StructDefinition> {
        // Struct name
        let name = if let Token::Identifier(name) = &self.current_token {
            name.clone()
        } else {
            return Err(CompileError::SyntaxError(
                "Expected struct name".to_string(),
                Some(self.current_span.clone()),
            ));
        };
        self.next_token();

        // Parse generics if present: <T: Trait1 + Trait2, U, ...>
        let type_params = self.parse_type_parameters()?;

        // Expect and consume ':' for type definition
        if self.current_token != Token::Symbol(':') {
            return Err(CompileError::SyntaxError(
                "Expected ':' after struct name for type definition".to_string(),
                Some(self.current_span.clone()),
            ));
        }
        self.next_token();

        // Opening brace
        if self.current_token != Token::Symbol('{') {
            return Err(CompileError::SyntaxError(
                "Expected '{' for struct fields".to_string(),
                Some(self.current_span.clone()),
            ));
        }
        self.next_token();

        let mut fields = vec![];

        // Parse fields
        while self.current_token != Token::Symbol('}') {
            if self.current_token == Token::Eof {
                return Err(CompileError::SyntaxError(
                    "Unexpected end of file in struct definition".to_string(),
                    Some(self.current_span.clone()),
                ));
            }

            // Field name
            let field_name = if let Token::Identifier(name) = &self.current_token {
                name.clone()
            } else {
                return Err(CompileError::SyntaxError(
                    "Expected field name".to_string(),
                    Some(self.current_span.clone()),
                ));
            };
            self.next_token();

            // Check for mutability modifier (:: for mutable) or regular type annotation (:)
            let is_mutable = if self.current_token == Token::Operator("::".to_string()) {
                self.next_token();
                true
            } else if self.current_token == Token::Symbol(':') {
                self.next_token();
                false
            } else {
                return Err(CompileError::SyntaxError(
                    "Expected ':' or '::' after field name".to_string(),
                    Some(self.current_span.clone()),
                ));
            };

            // Field type
            let field_type = self.parse_type()?;

            // Optional default value
            let default_value = if self.current_token == Token::Operator("=".to_string()) {
                self.next_token();
                Some(self.parse_expression()?)
            } else {
                None
            };

            fields.push(StructField {
                name: field_name,
                type_: field_type,
                is_mutable,
                default_value,
            });

            // Comma separator (except for last field)
            if self.current_token == Token::Symbol(',') {
                self.next_token();
            } else if self.current_token != Token::Symbol('}') {
                return Err(CompileError::SyntaxError(
                    "Expected ',' or '}' after field".to_string(),
                    Some(self.current_span.clone()),
                ));
            }
        }

        // Closing brace
        self.next_token();

        // Parse methods (zero or more fn ...)
        // TODO: Implement method parsing with correct syntax
        let methods = Vec::new();

        Ok(StructDefinition { name, type_params, fields, methods })
    }

    fn parse_method(&mut self) -> Result<Function> {
        // Method name (after 'fn' keyword)
        let name = if let Token::Identifier(name) = &self.current_token {
            name.clone()
        } else {
            return Err(CompileError::SyntaxError(
                "Expected method name".to_string(),
                Some(self.current_span.clone()),
            ));
        };
        self.next_token();

        // Parameters in parentheses
        if self.current_token != Token::Symbol('(') {
            return Err(CompileError::SyntaxError(
                "Expected '(' for method parameters".to_string(),
                Some(self.current_span.clone()),
            ));
        }
        self.next_token();

        let mut parameters = Vec::new();
        while self.current_token != Token::Symbol(')') {
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

            // Parameter type (optional - if next token is ')' or ',', skip type parsing)
            let param_type = if self.current_token == Token::Symbol(')') || self.current_token == Token::Symbol(',') {
                // No explicit type, use a default (could be inferred later)
                AstType::I32 // Default type for now
            } else {
                self.parse_type()?
            };
            parameters.push((param_name, param_type));

            // Comma or closing parenthesis
            if self.current_token == Token::Symbol(',') {
                self.next_token();
            } else if self.current_token != Token::Symbol(')') {
                return Err(CompileError::SyntaxError(
                    "Expected ',' or ')' after parameter".to_string(),
                    Some(self.current_span.clone()),
                ));
            }
        }
        self.next_token(); // consume ')'

        // Return type
        let return_type = self.parse_type()?;

        // Function body
        if self.current_token != Token::Symbol('{') {
            return Err(CompileError::SyntaxError(
                "Expected '{' for method body".to_string(),
                Some(self.current_span.clone()),
            ));
        }
        self.next_token();

        let mut body = Vec::new();
        while self.current_token != Token::Symbol('}') {
            body.push(self.parse_statement()?);
        }
        self.next_token(); // consume '}'

        Ok(Function {
            name,
            type_params: Vec::new(), // TODO: Parse generic type parameters for methods
            args: parameters,
            return_type,
            body,
        })
    }
}
