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

        let mut variants = vec![];
        let mut first_variant = true;

        // Parse variants - comma-separated only (enums use commas, structs use {})
        while self.current_token != Token::Eof {
            // Handle variant separators
            if !first_variant {
                // Only accept comma separator for enums
                if self.current_token == Token::Symbol(',') {
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
            // Zen standard: VariantName: Type
            let payload = if self.current_token == Token::Symbol(':') {
                self.next_token(); // consume ':'

                // Parse the payload type (could be a simple type or struct type)
                let payload_type = self.parse_type()?;
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
