use super::core::Parser;
use crate::ast::AstType;
use crate::error::Result;
use crate::lexer::Token;
use crate::well_known::well_known;

impl<'a> Parser<'a> {
    pub fn parse_type(&mut self) -> Result<AstType> {
        match &self.current_token {
            Token::Identifier(type_name) => {
                let mut type_name = type_name.clone();
                self.next_token();

                // Check for member access in type names (e.g., FFI.Library)
                while self.try_consume_symbol('.') {
                    let member = self.expect_identifier("identifier after '.' in type name")?;
                    type_name = format!("{}.{}", type_name, member);
                }

                match type_name.as_str() {
                    "i8" => Ok(AstType::I8),
                    "i16" => Ok(AstType::I16),
                    "i32" => Ok(AstType::I32),
                    "i64" => Ok(AstType::I64),
                    "u8" => Ok(AstType::U8),
                    "u16" => Ok(AstType::U16),
                    "u32" => Ok(AstType::U32),
                    "u64" => Ok(AstType::U64),
                    "usize" => Ok(AstType::Usize),
                    "f32" => Ok(AstType::F32),
                    "f64" => Ok(AstType::F64),
                    "bool" => Ok(AstType::Bool),
                    "StringLiteral" | "StaticString" => Ok(AstType::StaticString), // User-facing: string literals (compile-time, no allocator)
                    // String is a struct type - return as Generic to be resolved later
                    // IMPORTANT: Do NOT call resolve_string_struct_type() here as it causes
                    // circular dependency with stdlib_types() OnceLock initialization
                    "String" => Ok(AstType::Generic {
                        name: "String".to_string(),
                        type_args: vec![],
                    }),
                    "void" => Ok(AstType::Void),
                    "ptr" => Ok(AstType::ptr(AstType::Void)),
                    // Well-known pointer types (Ptr, MutPtr, RawPtr)
                    _ if well_known().is_ptr(&type_name) => {
                        let wk = well_known();
                        if self.try_consume_operator("<") {
                            let inner_type = self.parse_type()?;
                            self.expect_operator(">")?;
                            if wk.is_immutable_ptr(&type_name) {
                                Ok(AstType::ptr(inner_type))
                            } else if wk.is_mutable_ptr(&type_name) {
                                Ok(AstType::mut_ptr(inner_type))
                            } else {
                                Ok(AstType::raw_ptr(inner_type))
                            }
                        } else {
                            Err(self.syntax_error(format!("{} type requires type argument: {}<T>", type_name, type_name)))
                        }
                    }
                    "Vec" => {
                        // Vec<T> - Generic vector type OR Vec<T, size> - fixed-size array
                        if !self.try_consume_operator("<") {
                            return Err(self.syntax_error("Vec type requires type arguments: Vec<T, size>"));
                        }

                        // Parse element type
                        let element_type = self.parse_type()?;

                        // Check if it's Vec<T> (generic) or Vec<T, size> (fixed-size)
                        if self.try_consume_operator(">") {
                            // Vec<T> - generic vector, treat as user-defined generic type
                            return Ok(AstType::Generic {
                                name: "Vec".to_string(),
                                type_args: vec![element_type],
                            });
                        }

                        // Expect comma for Vec<T, size>
                        if !self.try_consume_symbol(',') {
                            return Err(self.syntax_error("Expected '>' or ',' in Vec type"));
                        }

                        // Parse size (must be integer literal)
                        let size = match &self.current_token {
                            Token::Integer(size_str) => {
                                size_str.parse::<usize>().map_err(|_| {
                                    self.syntax_error(format!("Invalid Vec size: {}", size_str))
                                })?
                            }
                            _ => {
                                return Err(self.syntax_error("Expected integer literal for Vec size"));
                            }
                        };
                        self.next_token();

                        self.expect_operator(">")?;

                        Ok(AstType::Vec {
                            element_type: Box::new(element_type),
                            size,
                        })
                    }
                    "DynVec" => {
                        // DynVec<T> or DynVec<T1, T2, ...> - Dynamic vector with optional mixed types
                        if !self.try_consume_operator("<") {
                            return Err(self.syntax_error(
                                "DynVec type requires type arguments: DynVec<T> or DynVec<T1, T2, ...>"
                            ));
                        }
                        let mut element_types = Vec::new();

                        loop {
                            element_types.push(self.parse_type()?);

                            if self.try_consume_operator(">") {
                                break;
                            } else if !self.try_consume_symbol(',') {
                                return Err(self.syntax_error("Expected ',' or '>' in DynVec type arguments"));
                            }
                        }

                        Ok(AstType::DynVec {
                            element_types,
                            allocator_type: None, // Allocator is specified at construction time
                        })
                    }
                    "str" | "string" => {
                        // Provide helpful error for common mistake
                        Err(self.syntax_error(
                            "Use 'StringLiteral' for string literals or 'String' for dynamic strings ('str' and 'string' are not valid types)"
                        ))
                    }
                    _ => {
                        // Check for generic type instantiation (e.g., List<T>)
                        if self.try_consume_operator("<") {
                            let mut type_args = Vec::new();

                            loop {
                                type_args.push(self.parse_type()?);

                                if self.try_consume_operator(">") {
                                    break;
                                } else if !self.try_consume_symbol(',') {
                                    return Err(self.syntax_error("Expected ',' or '>' in type arguments"));
                                }
                            }

                            Ok(AstType::Generic {
                                name: type_name,
                                type_args,
                            })
                        } else {
                            // Could be a custom type (struct, enum, etc.) or type parameter
                            Ok(AstType::Generic {
                                name: type_name,
                                type_args: vec![],
                            })
                        }
                    }
                }
            }
            Token::Symbol('[') => {
                // Array type: [T] (dynamic array) or [T; N] (fixed-size array)
                self.next_token();
                let element_type = self.parse_type()?;

                // Check for semicolon to determine if it's a fixed-size array
                if self.try_consume_symbol(';') {
                    // Parse the size (must be an integer literal for now)
                    match &self.current_token {
                        Token::Integer(size_str) => {
                            let size = size_str.parse::<usize>().map_err(|_| {
                                self.syntax_error(format!("Invalid array size: {}", size_str))
                            })?;
                            self.next_token();

                            self.expect_symbol(']')?;
                            Ok(AstType::FixedArray {
                                element_type: Box::new(element_type),
                                size,
                            })
                        }
                        _ => Err(self.syntax_error("Expected integer literal for array size")),
                    }
                } else if self.try_consume_symbol(']') {
                    // Dynamic array [T]
                    Ok(AstType::Array(Box::new(element_type)))
                } else {
                    Err(self.syntax_error("Expected ']' or ';' in array type"))
                }
            }
            Token::Symbol('*') => {
                // Pointer type: *T or function pointer *(params) return_type
                self.next_token();
                self.parse_pointer_or_function_pointer()
            }
            Token::Operator(op) if op == "*" => {
                // Pointer type: *T (operator version)
                self.next_token();
                self.parse_pointer_or_function_pointer()
            }
            Token::Symbol('(') => {
                // Function type: (params) ReturnType (for behaviors)
                self.next_token();
                let mut param_types = Vec::new();

                // Parse parameter types
                while self.current_token != Token::Symbol(')') {
                    // Check if it's a parameter with name (name: Type) or just a type
                    if let Token::Identifier(param_name) = &self.current_token {
                        let name = param_name.clone();

                        // Special case for "self" parameter - doesn't need a type
                        if name == "self" {
                            self.next_token();
                            // "self" is implicitly the type being defined
                            param_types.push(AstType::Generic {
                                name: "Self".to_string(),
                                type_args: vec![],
                            });
                        } else {
                            // Look ahead to see if this is "name: Type" or just "Type"
                            let saved_state = self.lexer.save_state();
                            let saved_current_token = self.current_token.clone();
                            let saved_peek_token = self.peek_token.clone();
                            let saved_span = self.current_span.clone();

                            self.next_token();

                            // Check if next token is colon (named parameter)
                            if self.try_consume_symbol(':') {
                                param_types.push(self.parse_type()?);
                            } else {
                                // Not a named parameter, restore and parse as type
                                self.lexer.restore_state(saved_state);
                                self.current_token = saved_current_token;
                                self.peek_token = saved_peek_token;
                                self.current_span = saved_span;
                                param_types.push(self.parse_type()?);
                            }
                        }
                    } else {
                        // Just parse the type directly
                        param_types.push(self.parse_type()?);
                    }

                    if !self.try_consume_symbol(',') && self.current_token != Token::Symbol(')') {
                        return Err(self.syntax_error("Expected ',' or ')' in function type parameters"));
                    }
                }

                self.next_token(); // Skip ')'

                // Parse return type
                let return_type = self.parse_type()?;

                Ok(AstType::FunctionPointer {
                    param_types,
                    return_type: Box::new(return_type),
                })
            }
            Token::Symbol('&') => {
                // Reference type: &T
                self.next_token();
                let referenced_type = self.parse_type()?;
                Ok(AstType::Ref(Box::new(referenced_type)))
            }
            Token::Symbol('{') => {
                // Anonymous struct type: {field1: Type1, field2: Type2}
                self.next_token(); // consume '{'

                let mut fields = Vec::new();

                while self.current_token != Token::Symbol('}') && self.current_token != Token::Eof {
                    // Parse field name
                    let field_name = self.expect_identifier("field name in anonymous struct type")?;

                    // Expect ':'
                    self.expect_symbol(':')?;

                    // Parse field type
                    let field_type = self.parse_type()?;
                    fields.push((field_name, field_type));

                    // Check for comma or end of struct
                    if !self.try_consume_symbol(',') && self.current_token != Token::Symbol('}') {
                        return Err(self.syntax_error("Expected ',' or '}' in anonymous struct type"));
                    }
                }

                if self.current_token != Token::Symbol('}') {
                    return Err(self.syntax_error("Expected '}' to close anonymous struct type"));
                }
                self.next_token(); // consume '}'

                Ok(AstType::Struct {
                    name: String::new(), // Anonymous struct has no name
                    fields,
                })
            }
            _ => Err(self.syntax_error(format!("Unexpected token in type: {:?}", self.current_token))),
        }
    }

    /// Helper for parsing pointer or function pointer types
    fn parse_pointer_or_function_pointer(&mut self) -> Result<AstType> {
        // Check if it's a function pointer
        if self.current_token == Token::Symbol('(') {
            self.next_token();
            let mut param_types = Vec::new();

            // Parse parameter types
            while self.current_token != Token::Symbol(')') {
                param_types.push(self.parse_type()?);

                if !self.try_consume_symbol(',') && self.current_token != Token::Symbol(')') {
                    return Err(self.syntax_error("Expected ',' or ')' in function pointer parameters"));
                }
            }

            self.next_token(); // Skip ')'

            // Parse return type
            let return_type = self.parse_type()?;

            Ok(AstType::FunctionPointer {
                param_types,
                return_type: Box::new(return_type),
            })
        } else {
            // Regular pointer
            let pointee_type = self.parse_type()?;
            Ok(AstType::ptr(pointee_type))
        }
    }

    pub fn parse_type_alias(&mut self) -> Result<crate::ast::TypeAlias> {
        use crate::ast::{TypeAlias, TypeParameter};

        // Capture span at the start of the type alias definition
        let start_span = self.current_span.clone();

        // Skip 'type' keyword
        self.next_token();

        // Get alias name
        let name = self.expect_identifier("type alias name")?;

        // Parse optional generic type parameters
        let mut type_params = Vec::new();
        if self.try_consume_operator("<") {
            loop {
                let param_name = self.expect_identifier("type parameter name")?;
                type_params.push(TypeParameter {
                    name: param_name,
                    constraints: Vec::new(),
                });

                if self.try_consume_operator(">") {
                    break;
                } else if !self.try_consume_symbol(',') {
                    return Err(self.syntax_error("Expected ',' or '>' in type parameters"));
                }
            }
        }

        // Expect ':' for type definition
        self.expect_symbol(':')?;

        // Parse the target type
        let target_type = self.parse_type()?;

        Ok(TypeAlias {
            name,
            type_params,
            target_type,
            span: Some(start_span),
        })
    }
}
