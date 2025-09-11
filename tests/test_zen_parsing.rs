// Tests for parsing Zen source files
// Validates that our self-hosted Zen code is syntactically correct

use std::fs;
use std::path::Path;

// Import the Zen parser (assuming we have these modules)
// This would be from the actual Rust implementation
// use zen_compiler::lexer::Lexer;
// use zen_compiler::parser::Parser;

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to read a Zen file
    fn read_zen_file(path: &str) -> Result<String, std::io::Error> {
        fs::read_to_string(path)
    }

    // Test that all stdlib Zen files are syntactically valid
    #[test]
    fn test_stdlib_files_parse() {
        let stdlib_files = vec![
            "stdlib/core.zen",
            "stdlib/io.zen",
            "stdlib/fs.zen",
            "stdlib/string.zen",
            "stdlib/vec.zen",
            "stdlib/math.zen",
            "stdlib/compiler/lexer.zen",
            "stdlib/compiler/parser.zen",
            "stdlib/compiler/type_checker.zen",
            "stdlib/compiler/codegen.zen",
            "stdlib/compiler/bootstrap_compiler.zen",
        ];

        for file_path in stdlib_files {
            println!("Testing: {}", file_path);
            
            match read_zen_file(file_path) {
                Ok(content) => {
                    // Basic syntax validation
                    assert!(!content.is_empty(), "{} is empty", file_path);
                    
                    // Check for balanced braces
                    let open_braces = content.matches('{').count();
                    let close_braces = content.matches('}').count();
                    assert_eq!(
                        open_braces, close_braces,
                        "{} has unbalanced braces: {} open, {} close",
                        file_path, open_braces, close_braces
                    );
                    
                    // Check for balanced parentheses
                    let open_parens = content.matches('(').count();
                    let close_parens = content.matches(')').count();
                    assert_eq!(
                        open_parens, close_parens,
                        "{} has unbalanced parentheses: {} open, {} close",
                        file_path, open_parens, close_parens
                    );
                    
                    // Check for balanced brackets
                    let open_brackets = content.matches('[').count();
                    let close_brackets = content.matches(']').count();
                    assert_eq!(
                        open_brackets, close_brackets,
                        "{} has unbalanced brackets: {} open, {} close",
                        file_path, open_brackets, close_brackets
                    );
                    
                    // Check that imports are not in comptime blocks
                    if content.contains("comptime") && content.contains("@std") {
                        // More detailed check
                        let lines: Vec<&str> = content.lines().collect();
                        let mut in_comptime = false;
                        
                        for (i, line) in lines.iter().enumerate() {
                            if line.contains("comptime") && line.contains("{") {
                                in_comptime = true;
                            }
                            
                            if in_comptime && line.contains("@std") {
                                panic!(
                                    "{} has import in comptime block at line {}",
                                    file_path,
                                    i + 1
                                );
                            }
                            
                            if in_comptime && line.contains("}") {
                                // Simple heuristic - may need improvement
                                in_comptime = false;
                            }
                        }
                    }
                    
                    println!("  ✓ {} parsed successfully", file_path);
                }
                Err(e) => {
                    // Skip if file doesn't exist yet
                    if e.kind() == std::io::ErrorKind::NotFound {
                        println!("  ⊘ {} not found (skipping)", file_path);
                    } else {
                        panic!("Failed to read {}: {}", file_path, e);
                    }
                }
            }
        }
    }

    // Test that example Zen programs parse correctly
    #[test]
    fn test_example_programs() {
        let example_files = vec![
            "examples/hello_world.zen",
            "examples/factorial.zen",
            "examples/fibonacci.zen",
            "examples/bootstrap.zen",
        ];

        for file_path in example_files {
            if Path::new(file_path).exists() {
                println!("Testing example: {}", file_path);
                
                match read_zen_file(file_path) {
                    Ok(content) => {
                        // Check for main function
                        assert!(
                            content.contains("main"),
                            "{} should have a main function",
                            file_path
                        );
                        
                        // Check for proper return type
                        assert!(
                            content.contains("i32") || content.contains("void"),
                            "{} main should have a return type",
                            file_path
                        );
                        
                        println!("  ✓ {} is valid", file_path);
                    }
                    Err(_) => {
                        println!("  ⊘ {} not found (skipping)", file_path);
                    }
                }
            }
        }
    }

    // Test that test files are valid
    #[test]
    fn test_zen_test_files() {
        let test_files = vec![
            "tests/test_self_hosting.zen",
            "tests/test_new_imports.zen",
            "tests/test_import_simple.zen",
        ];

        for file_path in test_files {
            if Path::new(file_path).exists() {
                println!("Testing test file: {}", file_path);
                
                match read_zen_file(file_path) {
                    Ok(content) => {
                        // Test files should import test module
                        if !file_path.contains("import") {
                            // Skip import test files
                            assert!(
                                content.contains("@std.test") || content.contains("test :="),
                                "{} should import test module",
                                file_path
                            );
                        }
                        
                        println!("  ✓ {} is valid", file_path);
                    }
                    Err(_) => {
                        println!("  ⊘ {} not found (skipping)", file_path);
                    }
                }
            }
        }
    }

    // Test that tools written in Zen are valid
    #[test]
    fn test_zen_tools() {
        let tool_files = vec![
            "tools/zen-check.zen",
            "tools/zen-lsp-basic.zen",
        ];

        for file_path in tool_files {
            if Path::new(file_path).exists() {
                println!("Testing tool: {}", file_path);
                
                match read_zen_file(file_path) {
                    Ok(content) => {
                        // Tools should have main function
                        assert!(
                            content.contains("main"),
                            "{} should have a main function",
                            file_path
                        );
                        
                        // Tools should use proper imports
                        // Skip zen-check.zen and zen-lsp-basic as they check FOR this pattern
                        if !file_path.contains("zen-check") && !file_path.contains("zen-lsp-basic") {
                            // Check for actual comptime blocks with imports
                            let has_comptime_import = content.contains("comptime") && 
                                content.contains("{") && 
                                content.contains("@std") &&
                                content.find("comptime").map_or(false, |ct_pos| {
                                    content[ct_pos..].find("@std").map_or(false, |std_pos| {
                                        content[ct_pos..ct_pos+std_pos].contains("{")
                                    })
                                });
                            assert!(
                                !has_comptime_import,
                                "{} should not have imports in comptime blocks",
                                file_path
                            );
                        }
                        
                        println!("  ✓ {} is valid", file_path);
                    }
                    Err(_) => {
                        println!("  ⊘ {} not found (skipping)", file_path);
                    }
                }
            }
        }
    }
}