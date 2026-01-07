use super::core::Parser;
use crate::ast::{AstType, Function, StructDefinition, StructField};
use crate::error::{CompileError, Result};
use crate::lexer::Token;

impl<'a> Parser<'a> {
    pub fn parse_struct(&mut self) -> Result<StructDefinition> {
        // Struct name
        let name = self.expect_identifier("struct name")?;

        // Parse generics if present: <T: Trait1 + Trait2, U, ...>
        let type_params = self.parse_type_parameters()?;

        // Expect and consume ':' for type definition
        self.expect_symbol(':')?;

        // Check if they're trying to use enum syntax (comma-separated) for a struct
        if matches!(&self.current_token, Token::Identifier(_))
            || self.current_token == Token::Symbol('.')
        {
            return Err(self.syntax_error(
                "Structs use curly braces for fields, not comma-separated variants. Use `MyStruct: { field1: Type1, field2: Type2 }` instead of `MyStruct: Field1, Field2`"
            ));
        }

        // Opening brace
        if self.current_token != Token::Symbol('{') {
            return Err(self.syntax_error(
                "Expected '{' for struct fields. Structs use curly braces: `MyStruct: { field: Type }`"
            ));
        }
        self.next_token();

        let mut fields = vec![];

        // Parse fields
        while self.current_token != Token::Symbol('}') {
            if self.current_token == Token::Eof {
                return Err(self.syntax_error("Unexpected end of file in struct definition"));
            }

            // Field name
            let field_name = self.expect_identifier("field name")?;

            // Check for mutability modifier (:: for mutable) or regular type annotation (:)
            let is_mutable = if self.try_consume_operator("::") {
                true
            } else if self.try_consume_symbol(':') {
                false
            } else {
                return Err(self.syntax_error("Expected ':' or '::' after field name"));
            };

            // Field type
            let field_type = self.parse_type()?;

            // Optional default value
            let default_value = if self.try_consume_operator("=") {
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
            if !self.try_consume_symbol(',') && self.current_token != Token::Symbol('}') {
                return Err(self.syntax_error("Expected ',' or '}' after field"));
            }
        }

        // Closing brace
        self.next_token();

        // Methods are defined via `impl` blocks, not inline in struct definitions.
        // This matches Rust's syntax: `impl MyStruct { fn method(&self) { ... } }`
        // The methods field is kept for AST compatibility but remains empty during parsing.
        let methods = Vec::new();

        Ok(StructDefinition {
            name,
            type_params,
            fields,
            methods,
        })
    }

    #[allow(dead_code)]
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

        // Parse generic type parameters if present: <T, U, ...>
        let type_params = self.parse_type_parameters()?;

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
            let param_type = if self.current_token == Token::Symbol(')')
                || self.current_token == Token::Symbol(',')
            {
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

        // Check visibility before moving name
        let is_public = !name.starts_with("__");

        Ok(Function {
            name,
            type_params,
            args: parameters,
            return_type,
            body,
            is_varargs: false,
            is_public,
        })
    }
}
