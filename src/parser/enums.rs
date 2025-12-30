use super::core::Parser;
use crate::ast::{EnumDefinition, EnumVariant};
use crate::error::Result;
use crate::lexer::Token;

impl<'a> Parser<'a> {
    pub fn parse_enum(&mut self) -> Result<EnumDefinition> {
        // Enum name
        let name = self.expect_identifier("enum name")?;

        // Parse optional type parameters
        let type_params = self.parse_type_parameters()?;

        // Expect ':' for type definition
        self.expect_symbol(':')?;

        // Check if they're trying to use struct syntax (curly braces) for an enum
        if self.current_token == Token::Symbol('{') {
            return Err(self.syntax_error(
                "Enums use comma-separated variants, not curly braces. Use `MyEnum: Variant1, Variant2` instead of `MyEnum: { Variant1, Variant2 }`"
            ));
        }

        let mut variants = vec![];
        let mut first_variant = true;

        // Parse variants - comma-separated only (enums use commas, structs use {})
        while self.current_token != Token::Eof {
            // Handle variant separators
            if !first_variant {
                // Check for pipe syntax (old syntax) and provide helpful error
                if self.current_token == Token::Pipe {
                    return Err(self.syntax_error(
                        "Enums use comma-separated variants, not pipes. Use `MyEnum: Variant1, Variant2` instead of `MyEnum: Variant1 | Variant2`"
                    ));
                }
                // Only accept comma separator for enums
                if !self.try_consume_symbol(',') {
                    // No separator found, done with variants
                    break;
                }
            }
            first_variant = false;

            // Check for shorthand enum variant syntax: .VariantName
            let variant_name = if self.try_consume_symbol('.') {
                self.expect_identifier("variant name after '.'")?
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
            let payload = if self.try_consume_symbol(':') {
                // Parse the payload type (could be a simple type or struct type)
                Some(self.parse_type()?)
            } else {
                None
            };

            variants.push(EnumVariant {
                name: variant_name,
                payload,
            });
        }

        if variants.is_empty() {
            return Err(self.syntax_error(format!(
                "Enum `{}` must have at least one variant. Use `{}: Variant1, Variant2` syntax",
                name, name
            )));
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
