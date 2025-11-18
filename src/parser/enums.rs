use super::core::Parser;
use crate::ast::{EnumDefinition, EnumVariant};
use crate::error::{CompileError, Result};
use crate::lexer::Token;

impl<'a> Parser<'a> {
    pub fn parse_enum(&mut self) -> Result<EnumDefinition> {
        // Enum name
        let name = if let Token::Identifier(name) = &self.current_token {
            name.clone()
        } else {
            return Err(CompileError::SyntaxError(
                "Expected enum name".to_string(),
                Some(self.current_span.clone()),
            ));
        };
        self.next_token();

        // Parse optional type parameters
        let type_params = if self.current_token == Token::Operator("<".to_string()) {
            self.parse_type_parameters()?
        } else {
            Vec::new()
        };

        // Expect ':' for type definition
        if self.current_token != Token::Symbol(':') {
            return Err(CompileError::SyntaxError(
                "Expected ':' after enum name for type definition".to_string(),
                Some(self.current_span.clone()),
            ));
        }
        self.next_token();

        // Enums must NOT use curly braces - that's for structs!
        if self.current_token == Token::Symbol('{') {
            return Err(CompileError::SyntaxError(
                "Enums use pipe (|) or comma separators, not curly braces. Use 'MyEnum: Variant1 | Variant2' syntax".to_string(),
                Some(self.current_span.clone()),
            ));
        }

        let mut variants = vec![];
        let mut first_variant = true;

        // Parse variants - support both | separator and comma separator
        while self.current_token != Token::Eof {
            // Handle variant separators
            if !first_variant {
                // Check for | or comma separator
                if self.current_token == Token::Pipe {
                    self.next_token();
                } else if self.current_token == Token::Symbol(',') {
                    self.next_token();
                } else {
                    // No separator found, done with variants
                    break;
                }
            }
            first_variant = false;

            // Check for shorthand enum variant syntax: .VariantName
            let variant_name = if self.current_token == Token::Symbol('.') {
                self.next_token(); // consume '.'
                if let Token::Identifier(name) = &self.current_token {
                    let name = name.clone();
                    self.next_token();
                    name
                } else {
                    return Err(CompileError::SyntaxError(
                        "Expected variant name after '.'".to_string(),
                        Some(self.current_span.clone()),
                    ));
                }
            } else if let Token::Identifier(name) = &self.current_token {
                let name = name.clone();
                self.next_token();
                name
            } else {
                // If not a variant, break (we're done parsing variants)
                break;
            };

            // Check for payload type
            // Support two syntaxes:
            // 1. VariantName: Type  (Zen standard - colon syntax)
            // 2. VariantName(Type)  (Rust-style parentheses)
            // Note: Enums are sum types - they don't use curly braces (that's for structs)
            let payload = if self.current_token == Token::Symbol(':') {
                self.next_token(); // consume ':'

                // Parse the payload type (could be a simple type or struct type)
                let payload_type = self.parse_type()?;
                Some(payload_type)
            } else if self.current_token == Token::Symbol('(') {
                self.next_token();

                // Check if this is a named field (field: type) or just a type
                if let Token::Identifier(_field_name) = &self.current_token {
                    // Look ahead to see if there's a colon
                    if self.peek_token == Token::Symbol(':') {
                        // Named field syntax - skip the field name for now
                        self.next_token(); // skip field name
                        self.next_token(); // skip ':'
                    }
                }

                // Parse payload type
                let payload_type = self.parse_type()?;

                // Closing parenthesis
                if self.current_token != Token::Symbol(')') {
                    return Err(CompileError::SyntaxError(
                        "Expected ')' after payload type".to_string(),
                        Some(self.current_span.clone()),
                    ));
                }
                self.next_token();

                Some(payload_type)
            } else {
                None
            };

            variants.push(EnumVariant {
                name: variant_name,
                payload,
            });
        }

        if variants.is_empty() {
            return Err(CompileError::SyntaxError(
                "Enum must have at least one variant".to_string(),
                Some(self.current_span.clone()),
            ));
        }

        Ok(EnumDefinition {
            name,
            type_params,
            variants,
            methods: Vec::new(),
            required_traits: Vec::new(), // Will be populated by .requires() statements
        })
    }
}
