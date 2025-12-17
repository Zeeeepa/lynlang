//! Program-level parsing - exports, imports, and declaration detection
//! Extracted from statements.rs to reduce file size

use super::core::Parser;
use crate::ast::{Declaration, Statement};
use crate::error::{CompileError, Result};
use crate::lexer::Token;

impl<'a> Parser<'a> {
    /// Parse an @export declaration
    pub fn parse_export(&mut self) -> Result<Declaration> {
        self.next_token();
        
        // Check for @export * (export all public symbols)
        if self.current_token == Token::Operator("*".to_string()) {
            self.next_token();
            return Ok(Declaration::Export {
                symbols: vec!["*".to_string()], // Special marker for "export all"
            });
        }
        
        if self.current_token != Token::Symbol('{') {
            return Err(CompileError::SyntaxError(
                "Expected '{' or '*' after @export".to_string(),
                Some(self.current_span.clone()),
            ));
        }
        self.next_token();
        
        let mut exported_symbols = vec![];
        
        // Parse exported symbol names
        while self.current_token != Token::Symbol('}') && self.current_token != Token::Eof {
            if let Token::Identifier(name) = &self.current_token {
                exported_symbols.push(name.clone());
                self.next_token();
                
                if self.current_token == Token::Symbol(',') {
                    self.next_token();
                } else if self.current_token != Token::Symbol('}') {
                    return Err(CompileError::SyntaxError(
                        "Expected ',' or '}' in @export list".to_string(),
                        Some(self.current_span.clone()),
                    ));
                }
            } else {
                return Err(CompileError::SyntaxError(
                    "Expected identifier in @export list".to_string(),
                    Some(self.current_span.clone()),
                ));
            }
        }
        
        if self.current_token != Token::Symbol('}') {
            return Err(CompileError::SyntaxError(
                "Expected '}' to close @export".to_string(),
                Some(self.current_span.clone()),
            ));
        }
        self.next_token();
        
        Ok(Declaration::Export {
            symbols: exported_symbols,
        })
    }

    /// Parse a destructuring import: { name, name } = @std
    pub fn parse_destructuring_import_declaration(&mut self) -> Result<Vec<Declaration>> {
        self.next_token();
        let mut imported_names = vec![];

        // Parse imported names
        while self.current_token != Token::Symbol('}') && self.current_token != Token::Eof {
            if let Token::Identifier(name) = &self.current_token {
                imported_names.push(name.clone());
                self.next_token();

                if self.current_token == Token::Symbol(',') {
                    self.next_token();
                } else if self.current_token != Token::Symbol('}') {
                    return Err(CompileError::SyntaxError(
                        "Expected ',' or '}' in destructuring import".to_string(),
                        Some(self.current_span.clone()),
                    ));
                }
            } else {
                return Err(CompileError::SyntaxError(
                    "Expected identifier in destructuring import".to_string(),
                    Some(self.current_span.clone()),
                ));
            }
        }

        if self.current_token != Token::Symbol('}') {
            return Err(CompileError::SyntaxError(
                "Expected '}' to close destructuring import".to_string(),
                Some(self.current_span.clone()),
            ));
        }
        self.next_token();

        // Expect '=' operator
        if self.current_token != Token::Operator("=".to_string()) {
            return Err(CompileError::SyntaxError(
                "Expected '=' after destructuring pattern".to_string(),
                Some(self.current_span.clone()),
            ));
        }
        self.next_token();

        // Expect @std or @std.module reference
        if self.current_token == Token::AtStd {
            let mut module_path = "@std".to_string();
            self.next_token();

            // Handle @std.module syntax
            while self.current_token == Token::Symbol('.') {
                self.next_token();
                if let Token::Identifier(member) = &self.current_token {
                    module_path.push('.');
                    module_path.push_str(member);
                    self.next_token();
                } else {
                    return Err(CompileError::SyntaxError(
                        "Expected identifier after '.'".to_string(),
                        Some(self.current_span.clone()),
                    ));
                }
            }

            // Create imports from the specified module
            // Map common stdlib symbols to their actual module paths
            let mut declarations = vec![];
            for name in imported_names {
                let actual_module_path = if module_path == "@std" {
                    // Map common symbols to their actual module locations
                    match name.as_str() {
                        "Option" | "Some" | "None" => "@std.core.option".to_string(),
                        "Result" | "Ok" | "Err" => "@std.core.result".to_string(),
                        "io" | "println" | "print" => "@std.io".to_string(),
                        _ => format!("{}.{}", module_path, name),
                    }
                } else {
                    format!("{}.{}", module_path, name)
                };
                declarations.push(Declaration::ModuleImport {
                    alias: name.clone(),
                    module_path: actual_module_path,
                });
            }
            Ok(declarations)
        } else if let Token::Identifier(module) = &self.current_token {
            if module.starts_with("@std") {
                let mut module_path = module.clone();
                self.next_token();

                // Handle @std.module syntax
                while self.current_token == Token::Symbol('.') {
                    self.next_token();
                    if let Token::Identifier(member) = &self.current_token {
                        module_path.push('.');
                        module_path.push_str(member);
                        self.next_token();
                    } else {
                        return Err(CompileError::SyntaxError(
                            "Expected identifier after '.'".to_string(),
                            Some(self.current_span.clone()),
                        ));
                    }
                }

                // Create imports from the specified module
                let mut declarations = vec![];
                for name in imported_names {
                    declarations.push(Declaration::ModuleImport {
                        alias: name.clone(),
                        module_path: format!("{}.{}", module_path, name),
                    });
                }
                Ok(declarations)
            } else {
                Err(CompileError::SyntaxError(
                    "Expected '@std' or '@std.module' after '=' in destructuring import"
                        .to_string(),
                    Some(self.current_span.clone()),
                ))
            }
        } else {
            Err(CompileError::SyntaxError(
                "Expected module reference after '=' in destructuring import".to_string(),
                Some(self.current_span.clone()),
            ))
        }
    }

    /// Check if the current position represents a module import after :=
    /// Returns true if this is @std, @std.xxx, or build.import pattern
    pub fn is_module_import_after_colon_assign(&mut self) -> bool {
        if self.current_token == Token::AtStd {
            return true;
        }
        
        if let Token::Identifier(id) = &self.current_token {
            if id.starts_with("@std") {
                return true;
            }
            if id == "build" {
               
                let saved_state = self.lexer.save_state();
                let saved_current = self.current_token.clone();
                let saved_peek = self.peek_token.clone();

                self.next_token();
                let is_import = self.current_token == Token::Symbol('.')
                    && {
                        self.next_token();
                        matches!(&self.current_token, Token::Identifier(name) if name == "import")
                    };

               
                self.lexer.restore_state(saved_state);
                self.current_token = saved_current;
                self.peek_token = saved_peek;

                return is_import;
            }
        }
        false
    }

    /// Parse a module import after := has been consumed
    /// Handles both @std.module and build.import("module") patterns
    pub fn parse_module_import_after_colon_assign(&mut self, alias: String) -> Result<Declaration> {
        if let Token::Identifier(id) = &self.current_token {
            if id == "build" {
               
                self.next_token();
                if self.current_token != Token::Symbol('.') {
                    return Err(CompileError::SyntaxError(
                        "Expected '.' after 'build'".to_string(),
                        Some(self.current_span.clone()),
                    ));
                }
                self.next_token();

                if !matches!(&self.current_token, Token::Identifier(name) if name == "import") {
                    return Err(CompileError::SyntaxError(
                        "Expected 'import' after 'build.'".to_string(),
                        Some(self.current_span.clone()),
                    ));
                }
                self.next_token();

                if self.current_token != Token::Symbol('(') {
                    return Err(CompileError::SyntaxError(
                        "Expected '(' after 'build.import'".to_string(),
                        Some(self.current_span.clone()),
                    ));
                }
                self.next_token();

                let module_name = if let Token::StringLiteral(name) = &self.current_token {
                    name.clone()
                } else {
                    return Err(CompileError::SyntaxError(
                        "Expected string literal for module name".to_string(),
                        Some(self.current_span.clone()),
                    ));
                };
                self.next_token();

                if self.current_token != Token::Symbol(')') {
                    return Err(CompileError::SyntaxError(
                        "Expected ')' after module name".to_string(),
                        Some(self.current_span.clone()),
                    ));
                }
                self.next_token();

                Ok(Declaration::ModuleImport {
                    alias,
                    module_path: format!("std.{}", module_name),
                })
            } else {
               
                let module_path = id.clone();
                self.next_token();

               
                let full_path = if self.current_token == Token::Symbol('.') {
                    let mut path = module_path;
                    while self.current_token == Token::Symbol('.') {
                        self.next_token();
                        if let Token::Identifier(member) = &self.current_token {
                            path.push('.');
                            path.push_str(member);
                            self.next_token();
                        } else {
                            break;
                        }
                    }
                    path
                } else {
                    module_path
                };

                Ok(Declaration::ModuleImport {
                    alias,
                    module_path: full_path,
                })
            }
        } else {
            Err(CompileError::SyntaxError(
                "Expected module path after :=".to_string(),
                Some(self.current_span.clone()),
            ))
        }
    }

    /// Parse a constant declaration from a statement
    pub fn parse_constant_from_statement(&mut self) -> Result<Declaration> {
        let stmt = self.parse_statement()?;
        if let Statement::VariableDeclaration {
            name,
            type_,
            initializer,
            ..
        } = stmt
        {
            if let Some(init) = initializer {
                Ok(Declaration::Constant {
                    name,
                    type_,
                    value: init,
                })
            } else {
                Err(CompileError::SyntaxError(
                    "Constant declaration requires an initializer".to_string(),
                    Some(self.current_span.clone()),
                ))
            }
        } else {
            Err(CompileError::SyntaxError(
                "Expected variable declaration".to_string(),
                Some(self.current_span.clone()),
            ))
        }
    }

    /// Parse a top-level mutable variable declaration: name :: Type = value
    pub fn parse_top_level_mutable_var(&mut self, name: String) -> Result<Declaration> {
        self.next_token();
        let type_ = self.parse_type()?;

        if self.current_token != Token::Operator("=".to_string()) {
            return Err(CompileError::SyntaxError(
                "Expected '=' after type in mutable variable declaration".to_string(),
                Some(self.current_span.clone()),
            ));
        }
        self.next_token();

        let value = self.parse_expression()?;

       
        if self.current_token == Token::Symbol(';') {
            self.next_token();
        }

        Ok(Declaration::Constant {
            name,
            value,
            type_: Some(type_),
        })
    }

    /// Parse type parameters from current position, returning the full type string
    /// e.g., for "Type<T, U>" returns "Type<T,U>"
    pub fn parse_type_name_with_generics(&mut self) -> Result<String> {
        let mut type_name = if let Token::Identifier(name) = &self.current_token {
            name.clone()
        } else {
            return Err(CompileError::SyntaxError(
                "Expected type name".to_string(),
                Some(self.current_span.clone()),
            ));
        };
        self.next_token();

       
        if self.current_token == Token::Operator("<".to_string()) {
            type_name.push('<');
            self.next_token();
            loop {
                if let Token::Identifier(param) = &self.current_token {
                    type_name.push_str(param);
                    self.next_token();
                } else {
                    break;
                }
                if self.current_token == Token::Symbol(',') {
                    type_name.push(',');
                    self.next_token();
                } else if self.current_token == Token::Operator(">".to_string()) {
                    type_name.push('>');
                    self.next_token();
                    break;
                } else {
                    break;
                }
            }
        }

        Ok(type_name)
    }

    /// Parse impl block: Type.impl = { ... }
    pub fn parse_impl_block_declaration(&mut self, type_name: String) -> Result<Declaration> {
        self.next_token();
        if self.current_token != Token::Operator("=".to_string()) {
            return Err(CompileError::SyntaxError(
                "Expected '=' after 'impl'".to_string(),
                Some(self.current_span.clone()),
            ));
        }
        self.next_token();
        Ok(Declaration::ImplBlock(self.parse_impl_block(type_name)?))
    }

    /// Parse method definition: Type.method = (params) ReturnType { ... }
    pub fn parse_method_definition(&mut self, type_name: String, method_name: String) -> Result<Declaration> {
        let full_function_name = format!("{}.{}", type_name, method_name);
        let mut func = self.parse_function()?;
        func.name = full_function_name;
        Ok(Declaration::Function(func))
    }

    /// Check if peek token indicates a generic declaration (has '<')
    pub fn peek_is_generic(&self) -> bool {
        self.peek_token == Token::Operator("<".to_string())
    }

    /// Skip past generic parameters and return depth (0 if successful)
    pub fn skip_generic_params(&mut self) -> i32 {
        self.next_token();
        self.next_token();
        let mut depth = 1;
        while depth > 0 && self.current_token != Token::Eof {
            if self.current_token == Token::Operator("<".to_string()) {
                depth += 1;
            } else if self.current_token == Token::Operator(">".to_string()) {
                depth -= 1;
            }
            if depth > 0 {
                self.next_token();
            }
        }
        if depth == 0 {
            self.next_token();
        }
        depth
    }

    /// Determine declaration type after ':' and optional generics
    pub fn classify_declaration_after_colon(&self) -> DeclarationKind {
        if matches!(&self.current_token, Token::Identifier(name) if name == "behavior") {
            return DeclarationKind::Behavior;
        }
        if self.current_token == Token::Symbol('{') {
            return DeclarationKind::StructOrTrait;
        }
        if self.current_token == Token::Symbol('(') {
            return DeclarationKind::Function;
        }
       
        if matches!(&self.current_token, Token::Identifier(_)) 
            || matches!(&self.current_token, Token::Symbol('.')) {
            return DeclarationKind::Enum;
        }
        DeclarationKind::Unknown
    }

    /// Check if current '{' starts a trait (has method signatures) or struct (has fields)
    pub fn is_trait_definition(&mut self) -> bool {
        let saved_state = self.lexer.save_state();
        let saved_current = self.current_token.clone();
        let saved_peek = self.peek_token.clone();

        self.next_token();

       
        let looks_like_trait = if let Token::Identifier(_) = &self.current_token {
            self.next_token();
            self.current_token == Token::Symbol(':') && {
                self.next_token();
                self.current_token == Token::Symbol('(')
            }
        } else {
            false
        };

       
        self.lexer.restore_state(saved_state);
        self.current_token = saved_current;
        self.peek_token = saved_peek;

        looks_like_trait
    }
}

/// Classification of declaration types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DeclarationKind {
    Behavior,
    StructOrTrait,
    Function,
    Enum,
    Unknown,
}

