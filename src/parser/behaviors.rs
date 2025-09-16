use crate::ast::{BehaviorDefinition, BehaviorMethod, TraitDefinition, TraitMethod, TraitImplementation, TraitRequirement, Parameter, TypeParameter, TraitConstraint};
use crate::lexer::Token;
use crate::parser::core::Parser;
use crate::error::{CompileError, Result};

impl<'a> Parser<'a> {
    pub fn parse_type_parameters(&mut self) -> Result<Vec<TypeParameter>> {
        // Parse generic type parameters if present: <T: Trait1 + Trait2, U, ...>
        let mut type_params = Vec::new();
        if self.current_token == Token::Operator("<".to_string()) {
            self.next_token();
            loop {
                if let Token::Identifier(gen) = &self.current_token {
                    let name = gen.clone();
                    self.next_token();
                    
                    // Parse optional trait constraints: T: Trait1 + Trait2
                    let mut constraints = Vec::new();
                    if self.current_token == Token::Symbol(':') {
                        self.next_token(); // consume ':'
                        
                        // Parse first trait constraint
                        if let Token::Identifier(trait_name) = &self.current_token {
                            constraints.push(TraitConstraint {
                                trait_name: trait_name.clone(),
                            });
                            self.next_token();
                            
                            // Parse additional constraints with '+' operator
                            while self.current_token == Token::Operator("+".to_string()) {
                                self.next_token(); // consume '+'
                                if let Token::Identifier(trait_name) = &self.current_token {
                                    constraints.push(TraitConstraint {
                                        trait_name: trait_name.clone(),
                                    });
                                    self.next_token();
                                } else {
                                    return Err(CompileError::SyntaxError(
                                        "Expected trait name after '+'".to_string(),
                                        Some(self.current_span.clone()),
                                    ));
                                }
                            }
                        } else {
                            return Err(CompileError::SyntaxError(
                                "Expected trait name after ':'".to_string(),
                                Some(self.current_span.clone()),
                            ));
                        }
                    }
                    
                    type_params.push(TypeParameter {
                        name,
                        constraints,
                    });
                    
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
        Ok(type_params)
    }
    
    fn parse_parameter(&mut self) -> Result<Parameter> {
        // Check for mutability
        let is_mutable = if self.current_token == Token::Identifier("mut".to_string()) {
            self.next_token();
            true
        } else {
            false
        };
        
        // Parse parameter name
        let name = if let Token::Identifier(n) = &self.current_token {
            n.clone()
        } else {
            return Err(CompileError::SyntaxError(
                format!("Expected parameter name, got {:?}", self.current_token),
                Some(self.current_span.clone()),
            ));
        };
        self.next_token();
        
        // Expect ':' for type annotation
        if self.current_token != Token::Symbol(':') {
            return Err(CompileError::SyntaxError(
                format!("Expected ':' after parameter name, got {:?}", self.current_token),
                Some(self.current_span.clone()),
            ));
        }
        self.next_token();
        
        // Parse parameter type
        let type_ = self.parse_type()?;
        
        Ok(Parameter {
            name,
            type_,
            is_mutable,
        })
    }
    
    pub fn parse_behavior(&mut self) -> Result<BehaviorDefinition> {
        // Parse behavior name
        let name = if let Token::Identifier(n) = &self.current_token {
            n.clone()
        } else {
            return Err(CompileError::SyntaxError(
                format!("Expected behavior name, got {:?}", self.current_token),
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
                format!("Expected ':' after behavior name for type definition, got {:?}", self.current_token),
                Some(self.current_span.clone()),
            ));
        }
        self.next_token();
        
        // Expect 'behavior' identifier
        if !matches!(&self.current_token, Token::Identifier(name) if name == "behavior") {
            return Err(CompileError::SyntaxError(
                format!("Expected 'behavior' identifier, got {:?}", self.current_token),
                Some(self.current_span.clone()),
            ));
        }
        self.next_token();
        
        // Expect '{'
        if self.current_token != Token::Symbol('{') {
            return Err(CompileError::SyntaxError(
                format!("Expected '{{' after 'behavior', got {:?}", self.current_token),
                Some(self.current_span.clone()),
            ));
        }
        self.next_token();
        
        // Parse behavior methods
        let mut methods = Vec::new();
        
        while self.current_token != Token::Symbol('}') && self.current_token != Token::Eof {
            let method = self.parse_behavior_method()?;
            methods.push(method);
            
            // Handle comma separator
            if self.current_token == Token::Symbol(',') {
                self.next_token();
            }
        }
        
        // Expect '}'
        if self.current_token != Token::Symbol('}') {
            return Err(CompileError::SyntaxError(
                "Expected '}' to close behavior definition".to_string(),
                Some(self.current_span.clone()),
            ));
        }
        self.next_token();
        
        Ok(BehaviorDefinition {
            name,
            type_params,
            methods,
        })
    }
    
    fn parse_behavior_method(&mut self) -> Result<BehaviorMethod> {
        // Parse method name
        let name = if let Token::Identifier(n) = &self.current_token {
            n.clone()
        } else {
            return Err(CompileError::SyntaxError(
                format!("Expected method name, got {:?}", self.current_token),
                Some(self.current_span.clone()),
            ));
        };
        self.next_token();
        
        // Expect ':' for method type definition
        if self.current_token != Token::Symbol(':') {
            return Err(CompileError::SyntaxError(
                format!("Expected ':' after method name for type definition, got {:?}", self.current_token),
                Some(self.current_span.clone()),
            ));
        }
        self.next_token();
        
        // Expect '('
        if self.current_token != Token::Symbol('(') {
            return Err(CompileError::SyntaxError(
                format!("Expected '(' after '=', got {:?}", self.current_token),
                Some(self.current_span.clone()),
            ));
        }
        self.next_token();
        
        // Parse parameters
        let mut params = Vec::new();
        
        if self.current_token != Token::Symbol(')') {
            loop {
                let param = self.parse_parameter()?;
                params.push(param);
                
                if self.current_token == Token::Symbol(',') {
                    self.next_token();
                } else {
                    break;
                }
            }
        }
        
        // Expect ')'
        if self.current_token != Token::Symbol(')') {
            return Err(CompileError::SyntaxError(
                "Expected ')' after parameters".to_string(),
                Some(self.current_span.clone()),
            ));
        }
        self.next_token();
        
        // Parse return type
        let return_type = self.parse_type()?;
        
        Ok(BehaviorMethod {
            name,
            params,
            return_type,
        })
    }
    
    pub fn parse_trait(&mut self) -> Result<TraitDefinition> {
        // Parse trait name
        let name = if let Token::Identifier(n) = &self.current_token {
            n.clone()
        } else {
            return Err(CompileError::SyntaxError(
                format!("Expected trait name, got {:?}", self.current_token),
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
                format!("Expected ':' after trait name for type definition, got {:?}", self.current_token),
                Some(self.current_span.clone()),
            ));
        }
        self.next_token();
        
        // Expect '{'
        if self.current_token != Token::Symbol('{') {
            return Err(CompileError::SyntaxError(
                format!("Expected '{{' after ':', got {:?}", self.current_token),
                Some(self.current_span.clone()),
            ));
        }
        self.next_token();
        
        // Parse trait methods
        let mut methods = Vec::new();
        
        while self.current_token != Token::Symbol('}') && self.current_token != Token::Eof {
            let method = self.parse_trait_method()?;
            methods.push(method);
            
            // Handle comma separator
            if self.current_token == Token::Symbol(',') {
                self.next_token();
            }
        }
        
        // Expect '}'
        if self.current_token != Token::Symbol('}') {
            return Err(CompileError::SyntaxError(
                "Expected '}' to close trait definition".to_string(),
                Some(self.current_span.clone()),
            ));
        }
        self.next_token();
        
        Ok(TraitDefinition {
            name,
            type_params,
            methods,
        })
    }
    
    fn parse_trait_method(&mut self) -> Result<TraitMethod> {
        // Parse method name
        let name = if let Token::Identifier(n) = &self.current_token {
            n.clone()
        } else {
            return Err(CompileError::SyntaxError(
                format!("Expected method name, got {:?}", self.current_token),
                Some(self.current_span.clone()),
            ));
        };
        self.next_token();
        
        // Expect ':' for method type definition
        if self.current_token != Token::Symbol(':') {
            return Err(CompileError::SyntaxError(
                format!("Expected ':' after method name for type definition, got {:?}", self.current_token),
                Some(self.current_span.clone()),
            ));
        }
        self.next_token();
        
        // Expect '('
        if self.current_token != Token::Symbol('(') {
            return Err(CompileError::SyntaxError(
                format!("Expected '(' after ':', got {:?}", self.current_token),
                Some(self.current_span.clone()),
            ));
        }
        self.next_token();
        
        // Parse parameters
        let mut params = Vec::new();
        
        if self.current_token != Token::Symbol(')') {
            loop {
                let param = self.parse_parameter()?;
                params.push(param);
                
                if self.current_token == Token::Symbol(',') {
                    self.next_token();
                } else {
                    break;
                }
            }
        }
        
        // Expect ')'
        if self.current_token != Token::Symbol(')') {
            return Err(CompileError::SyntaxError(
                "Expected ')' after parameters".to_string(),
                Some(self.current_span.clone()),
            ));
        }
        self.next_token();
        
        // Parse return type
        let return_type = self.parse_type()?;
        
        Ok(TraitMethod {
            name,
            params,
            return_type,
        })
    }
    
    
    /// Parse a function within an impl block context with a pre-parsed name
    pub fn parse_impl_function_with_name(&mut self, name: String) -> Result<crate::ast::Function> {
        use crate::ast::Function;
        
        // Parse generic type parameters if present: <T: Constraint, U, ...>
        let type_params = self.parse_type_parameters()?;
        
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
        
        // Check for return type (it should be present for impl functions)
        let return_type = if self.current_token != Token::Symbol('{') {
            // If it's not '{', then we have a return type
            self.parse_type()?
        } else {
            // Default to void/unit type if no return type specified
            crate::ast::AstType::Void
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
    
    /// Parse a function within an impl block context
    /// This is different from parse_function() because the function name and '=' have already been consumed
    pub fn parse_impl_function(&mut self) -> Result<crate::ast::Function> {
        use crate::ast::Function;
        
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
        
        // Parse generic type parameters if present: <T: Constraint, U, ...>
        let type_params = self.parse_type_parameters()?;
        
        // Expect '=' (function signature separator)
        if self.current_token != Token::Operator("=".to_string()) {
            return Err(CompileError::SyntaxError(
                "Expected '=' after function name".to_string(),
                Some(self.current_span.clone()),
            ));
        }
        self.next_token();
        
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
        
        // Check for return type (it should be present for impl functions)
        let return_type = if self.current_token != Token::Symbol('{') {
            // If it's not '{', then we have a return type
            self.parse_type()?
        } else {
            // Default to void/unit type if no return type specified
            crate::ast::AstType::Void
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
    
    pub fn parse_trait_implementation(&mut self, type_name: String) -> Result<TraitImplementation> {
        // Parse: Type.implements(Trait, { methods })
        // Current token is '(' after 'implements'
        if self.current_token != Token::Symbol('(') {
            return Err(CompileError::SyntaxError(
                format!("Expected '(' after 'implements', got {:?}", self.current_token),
                Some(self.current_span.clone()),
            ));
        }
        self.next_token(); // consume '('
        
        // Parse trait name
        let trait_name = if let Token::Identifier(name) = &self.current_token {
            name.clone()
        } else {
            return Err(CompileError::SyntaxError(
                format!("Expected trait name, got {:?}", self.current_token),
                Some(self.current_span.clone()),
            ));
        };
        self.next_token(); // consume trait name
        
        // Expect comma
        if self.current_token != Token::Symbol(',') {
            return Err(CompileError::SyntaxError(
                format!("Expected ',' after trait name, got {:?}", self.current_token),
                Some(self.current_span.clone()),
            ));
        }
        self.next_token(); // consume ','
        
        // Expect opening brace for methods
        if self.current_token != Token::Symbol('{') {
            return Err(CompileError::SyntaxError(
                format!("Expected '{{' for trait methods, got {:?}", self.current_token),
                Some(self.current_span.clone()),
            ));
        }
        self.next_token(); // consume '{'
        
        // Parse methods
        let mut methods = Vec::new();
        while self.current_token != Token::Symbol('}') && self.current_token != Token::Eof {
            // Parse method: name = (params) return_type { body }
            let method_name = if let Token::Identifier(name) = &self.current_token {
                name.clone()
            } else {
                return Err(CompileError::SyntaxError(
                    format!("Expected method name, got {:?}", self.current_token),
                    Some(self.current_span.clone()),
                ));
            };
            self.next_token(); // consume method name
            
            // Expect '='
            if self.current_token != Token::Operator("=".to_string()) {
                return Err(CompileError::SyntaxError(
                    format!("Expected '=' after method name, got {:?}", self.current_token),
                    Some(self.current_span.clone()),
                ));
            }
            self.next_token(); // consume '='
            
            // Parse method as a function (name has already been parsed)
            let method = self.parse_impl_function_with_name(method_name)?;
            methods.push(method);
            
            // Check for comma between methods
            if self.current_token == Token::Symbol(',') {
                self.next_token();
            }
        }
        
        // Expect closing brace
        if self.current_token != Token::Symbol('}') {
            return Err(CompileError::SyntaxError(
                format!("Expected '}}' to close trait methods, got {:?}", self.current_token),
                Some(self.current_span.clone()),
            ));
        }
        self.next_token(); // consume '}'
        
        // Expect closing parenthesis
        if self.current_token != Token::Symbol(')') {
            return Err(CompileError::SyntaxError(
                format!("Expected ')' to close implements call, got {:?}", self.current_token),
                Some(self.current_span.clone()),
            ));
        }
        self.next_token(); // consume ')'
        
        Ok(TraitImplementation {
            type_name,
            trait_name,
            type_params: Vec::new(), // TODO: Add support for generic type parameters
            methods,
        })
    }
    
    pub fn parse_trait_requirement(&mut self, type_name: String) -> Result<TraitRequirement> {
        // Parse: Type.requires(Trait)
        // Current token is '(' after 'requires'
        if self.current_token != Token::Symbol('(') {
            return Err(CompileError::SyntaxError(
                format!("Expected '(' after 'requires', got {:?}", self.current_token),
                Some(self.current_span.clone()),
            ));
        }
        self.next_token(); // consume '('
        
        // Parse trait name
        let trait_name = if let Token::Identifier(name) = &self.current_token {
            name.clone()
        } else {
            return Err(CompileError::SyntaxError(
                format!("Expected trait name, got {:?}", self.current_token),
                Some(self.current_span.clone()),
            ));
        };
        self.next_token(); // consume trait name
        
        // Expect closing parenthesis
        if self.current_token != Token::Symbol(')') {
            return Err(CompileError::SyntaxError(
                format!("Expected ')' after trait name, got {:?}", self.current_token),
                Some(self.current_span.clone()),
            ));
        }
        self.next_token(); // consume ')'
        
        Ok(TraitRequirement {
            type_name,
            trait_name,
        })
    }
}