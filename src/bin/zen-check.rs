use std::env;
use std::fs;
use std::process;
use zen::lexer::Lexer;
use zen::parser::Parser;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        eprintln!("Usage: zen-check <file.zen> [files...]");
        eprintln!("       zen-check *.zen");
        process::exit(1);
    }
    
    let mut has_errors = false;
    let mut total_files = 0;
    let mut files_with_errors = 0;
    
    for file_path in &args[1..] {
        // Handle glob patterns
        let paths = if file_path.contains('*') {
            glob::glob(file_path)
                .expect("Failed to read glob pattern")
                .filter_map(Result::ok)
                .collect::<Vec<_>>()
        } else {
            vec![std::path::PathBuf::from(file_path)]
        };
        
        for path in paths {
            total_files += 1;
            let file_str = path.to_string_lossy();
            
            match fs::read_to_string(&path) {
                Ok(content) => {
                    println!("Checking {}...", file_str);
                    
                    // Check for import placement errors
                    if let Some(error) = check_import_placement(&content) {
                        eprintln!("  ✗ {}", error);
                        has_errors = true;
                        files_with_errors += 1;
                        continue;
                    }
                    
                    // Try to parse the file
                    let lexer = Lexer::new(&content);
                    let mut parser = Parser::new(lexer);
                    
                    match parser.parse_program() {
                        Ok(_) => {
                            println!("  ✓ No syntax errors found");
                        }
                        Err(e) => {
                            eprintln!("  ✗ Syntax error: {}", e);
                            has_errors = true;
                            files_with_errors += 1;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("  ✗ Error reading file: {}", e);
                    has_errors = true;
                    files_with_errors += 1;
                }
            }
        }
    }
    
    // Print summary if checking multiple files
    if total_files > 1 {
        println!("\n=== Summary ===");
        println!("Files checked: {}", total_files);
        println!("Files with errors: {}", files_with_errors);
        println!("Files without errors: {}", total_files - files_with_errors);
    }
    
    process::exit(if has_errors { 1 } else { 0 });
}

/// Check for imports inside comptime blocks
fn check_import_placement(content: &str) -> Option<String> {
    let lines: Vec<&str> = content.lines().collect();
    let mut in_comptime = false;
    let mut brace_depth = 0;
    
    for (line_num, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        
        // Check for comptime block start
        if trimmed.starts_with("comptime") && line.contains('{') {
            in_comptime = true;
            brace_depth = 1;
        }
        
        // Track brace depth in comptime
        if in_comptime {
            for ch in line.chars() {
                match ch {
                    '{' => brace_depth += 1,
                    '}' => {
                        brace_depth -= 1;
                        if brace_depth == 0 {
                            in_comptime = false;
                        }
                    }
                    _ => {}
                }
            }
        }
        
        // Check for imports in comptime
        if in_comptime {
            if trimmed.contains("@std") || trimmed.contains("build.import") {
                return Some(format!(
                    "Line {}: Import statements are not allowed inside comptime blocks\n    {}",
                    line_num + 1,
                    trimmed
                ));
            }
        }
    }
    
    None
}