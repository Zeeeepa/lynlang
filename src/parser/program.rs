//! Program-level parsing - exports, imports, and declaration detection
//! Extracted from statements.rs to reduce file size

use super::core::Parser;
use crate::ast::Declaration;
use crate::error::{CompileError, Result};
use crate::lexer::Token;

impl<'a> Parser<'a> {
    /// Parse an @export declaration
    pub fn parse_export(&mut self) -> Result<Declaration> {
        self.next_token(); // consume '@export'
        
        // Check for @export * (export all public symbols)
        if self.current_token == Token::Operator("*".to_string()) {
            self.next_token(); // consume '*'
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
        self.next_token(); // consume '{'
        
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
        self.next_token(); // consume '}'
        
        Ok(Declaration::Export {
            symbols: exported_symbols,
        })
    }

    /// Parse a destructuring import: { name, name } = @std
    pub fn parse_destructuring_import_declaration(&mut self) -> Result<Vec<Declaration>> {
        self.next_token(); // consume '{'
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
        self.next_token(); // consume '}'

        // Expect '=' operator
        if self.current_token != Token::Operator("=".to_string()) {
            return Err(CompileError::SyntaxError(
                "Expected '=' after destructuring pattern".to_string(),
                Some(self.current_span.clone()),
            ));
        }
        self.next_token(); // consume '='

        // Expect @std or @std.module reference
        if self.current_token == Token::AtStd {
            let mut module_path = "@std".to_string();
            self.next_token();

            // Handle @std.module syntax
            while self.current_token == Token::Symbol('.') {
                self.next_token(); // consume '.'
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
                    self.next_token(); // consume '.'
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
}

