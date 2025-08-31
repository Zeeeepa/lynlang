// Zen Language REPL Implementation
use crate::ast::{Expression, Statement, Program, Declaration};
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::typechecker::TypeChecker;
use anyhow::{Result, anyhow};
use colored::*;
use rustyline::error::ReadlineError;
use rustyline::{Editor, Config};
use std::collections::HashMap;

pub struct Repl {
    editor: Editor<()>,
    line_number: usize,
    variables: HashMap<String, String>,
    functions: HashMap<String, String>,
    debug_mode: bool,
    multiline_buffer: String,
    in_multiline: bool,
}

impl Repl {
    pub fn new() -> Result<Self> {
        let config = Config::builder()
            .history_ignore_space(true)
            .completion_type(rustyline::CompletionType::List)
            .build();
            
        let editor = Editor::<()>::with_config(config)?;
        
        Ok(Repl {
            editor,
            line_number: 1,
            variables: HashMap::new(),
            functions: HashMap::new(),
            debug_mode: false,
            multiline_buffer: String::new(),
            in_multiline: false,
        })
    }
    
    pub fn run(&mut self) -> Result<()> {
        self.print_banner();
        
        loop {
            let prompt = self.get_prompt();
            let readline = self.editor.readline(&prompt);
            
            match readline {
                Ok(line) => {
                    if line.trim().is_empty() {
                        if self.in_multiline {
                            // Execute multiline buffer
                            let input = self.multiline_buffer.clone();
                            self.multiline_buffer.clear();
                            self.in_multiline = false;
                            self.evaluate(&input);
                            self.line_number += 1;
                        }
                        continue;
                    }
                    
                    // Add to history
                    let _ = self.editor.add_history_entry(line.as_str());
                    
                    // Check for continuation
                    if line.trim().ends_with('\\') {
                        self.in_multiline = true;
                        let without_slash = &line[..line.len()-1];
                        self.multiline_buffer.push_str(without_slash);
                        self.multiline_buffer.push('\n');
                        continue;
                    }
                    
                    // Build complete input
                    let complete_input = if self.in_multiline {
                        let mut result = self.multiline_buffer.clone();
                        result.push_str(&line);
                        self.multiline_buffer.clear();
                        self.in_multiline = false;
                        result
                    } else {
                        line.clone()
                    };
                    
                    // Process command or evaluate
                    if !self.process_command(&complete_input) {
                        self.evaluate(&complete_input);
                    }
                    
                    self.line_number += 1;
                }
                Err(ReadlineError::Interrupted) => {
                    println!("{}", "^C".yellow());
                    self.multiline_buffer.clear();
                    self.in_multiline = false;
                }
                Err(ReadlineError::Eof) => {
                    println!("{}", "Goodbye!".yellow());
                    break;
                }
                Err(err) => {
                    eprintln!("Error: {:?}", err);
                    break;
                }
            }
        }
        
        Ok(())
    }
    
    fn print_banner(&self) {
        println!("{}", "╔═══════════════════════════════════════╗".cyan());
        println!("{}", format!("║      {}       ║", 
            "Zen Language REPL v1.0".yellow().bold()).cyan());
        println!("{}", "╚═══════════════════════════════════════╝".cyan());
        println!();
        println!("Type 'help' for commands, 'exit' to quit");
        println!();
    }
    
    fn get_prompt(&self) -> String {
        if self.in_multiline {
            format!("{}", "... ".blue())
        } else {
            format!("{}[{}]> ", "zen".blue(), self.line_number)
        }
    }
    
    fn process_command(&mut self, input: &str) -> bool {
        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.is_empty() {
            return false;
        }
        
        match parts[0] {
            "help" => {
                self.print_help();
                true
            }
            "exit" | "quit" => {
                println!("{}", "Goodbye!".yellow());
                std::process::exit(0);
            }
            "clear" => {
                print!("\x1b[2J\x1b[H");
                true
            }
            "vars" => {
                self.show_variables();
                true
            }
            "funcs" => {
                self.show_functions();
                true
            }
            "debug" => {
                if parts.len() > 1 {
                    match parts[1] {
                        "on" => {
                            self.debug_mode = true;
                            println!("{}", "Debug mode enabled".green());
                        }
                        "off" => {
                            self.debug_mode = false;
                            println!("{}", "Debug mode disabled".green());
                        }
                        _ => {
                            println!("Debug mode: {}", 
                                if self.debug_mode { "on" } else { "off" });
                        }
                    }
                } else {
                    println!("Debug mode: {}", 
                        if self.debug_mode { "on" } else { "off" });
                }
                true
            }
            "reset" => {
                self.variables.clear();
                self.functions.clear();
                self.line_number = 1;
                println!("{}", "REPL state reset".green());
                true
            }
            _ => false,
        }
    }
    
    fn print_help(&self) {
        println!("{}", "Available Commands:".green().bold());
        println!("  help          - Show this help message");
        println!("  exit/quit     - Exit the REPL");
        println!("  clear         - Clear the screen");
        println!("  vars          - Show all variables");
        println!("  funcs         - Show all functions");
        println!("  debug on/off  - Toggle debug mode");
        println!("  reset         - Reset REPL state");
        println!();
        println!("{}", "Multi-line Input:".green().bold());
        println!("  Use \\ at end of line to continue");
        println!("  Empty line to execute multi-line input");
        println!();
    }
    
    fn show_variables(&self) {
        if self.variables.is_empty() {
            println!("No variables defined");
        } else {
            println!("{}", "Variables:".green().bold());
            for (name, value) in &self.variables {
                println!("  {} = {}", name, value);
            }
        }
    }
    
    fn show_functions(&self) {
        if self.functions.is_empty() {
            println!("No functions defined");
        } else {
            println!("{}", "Functions:".green().bold());
            for (name, sig) in &self.functions {
                println!("  {}{}", name, sig);
            }
        }
    }
    
    fn evaluate(&mut self, input: &str) {
        // Wrap input in a simple program structure for REPL
        let wrapped_input = if input.contains('=') && !input.contains("==") {
            // Variable assignment or function definition
            input.to_string()
        } else {
            // Expression - wrap in a print statement
            format!("io.print({})", input)
        };
        
        // Tokenize
        let mut lexer = Lexer::new(&wrapped_input);
        let tokens = match lexer.tokenize() {
            Ok(tokens) => tokens,
            Err(e) => {
                eprintln!("{}: {}", "Lexer error".red(), e);
                return;
            }
        };
        
        if self.debug_mode {
            println!("{}: {:?}", "Tokens".magenta(), tokens);
        }
        
        // Parse
        let mut parser = Parser::new(tokens);
        let ast = match parser.parse() {
            Ok(ast) => ast,
            Err(e) => {
                eprintln!("{}: {}", "Parser error".red(), e);
                return;
            }
        };
        
        if self.debug_mode {
            println!("{}: {:#?}", "AST".magenta(), ast);
        }
        
        // Type check
        let mut type_checker = TypeChecker::new();
        match type_checker.check(&ast) {
            Ok(_) => {
                if self.debug_mode {
                    println!("{}", "Type check passed".green());
                }
            }
            Err(e) => {
                eprintln!("{}: {}", "Type error".red(), e);
                return;
            }
        }
        
        // For now, just print success
        // In a real implementation, we'd interpret or JIT compile the code
        println!("{} {}", "=>".green(), "Expression evaluated successfully");
        
        // Update state (simplified - just track that something was defined)
        self.update_state(&ast);
    }
    
    fn update_state(&mut self, program: &Program) {
        for decl in &program.declarations {
            match decl {
                Declaration::Function(func) => {
                    let sig = format!("({}) {}", 
                        func.parameters.iter()
                            .map(|p| format!("{}: {}", p.name, format!("{:?}", p.type_)))
                            .collect::<Vec<_>>()
                            .join(", "),
                        format!("{:?}", func.return_type)
                    );
                    self.functions.insert(func.name.clone(), sig);
                }
                Declaration::Global { name, type_, .. } => {
                    self.variables.insert(name.clone(), format!("{:?}", type_));
                }
                _ => {}
            }
        }
    }
}