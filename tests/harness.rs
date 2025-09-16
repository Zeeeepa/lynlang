// Test harness for Zen language files
// Automatically discovers and runs .zen test files

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Test categories for organizing tests
#[derive(Debug)]
enum TestCategory {
    /// Should compile successfully
    CompilePass,
    /// Should fail to compile with specific error
    CompileFail,
    /// Should compile and run successfully
    RuntimePass,
    /// Should compile but fail at runtime
    RuntimeFail,
}

/// A single test case
#[derive(Debug)]
struct ZenTest {
    path: PathBuf,
    category: TestCategory,
    expected_error: Option<String>,
}

impl ZenTest {
    fn from_path(path: PathBuf) -> Option<Self> {
        // Determine test category based on path
        let path_str = path.to_string_lossy();
        
        let category = if path_str.contains("compile_pass") {
            TestCategory::CompilePass
        } else if path_str.contains("compile_fail") {
            TestCategory::CompileFail
        } else if path_str.contains("runtime_pass") {
            TestCategory::RuntimePass
        } else if path_str.contains("runtime_fail") {
            TestCategory::RuntimeFail
        } else {
            // Default: try to compile and run
            TestCategory::RuntimePass
        };
        
        // Extract expected error from comments if compile_fail
        let expected_error = if matches!(category, TestCategory::CompileFail) {
            extract_expected_error(&path)
        } else {
            None
        };
        
        Some(ZenTest {
            path,
            category,
            expected_error,
        })
    }
    
    /// Run this test
    fn run(&self) -> Result<(), String> {
        let compiler_path = get_compiler_path()?;
        let test_name = self.path.file_name()
            .unwrap()
            .to_string_lossy();
        
        match self.category {
            TestCategory::CompilePass => {
                self.run_compile_test(&compiler_path, true)
            }
            TestCategory::CompileFail => {
                self.run_compile_test(&compiler_path, false)
            }
            TestCategory::RuntimePass => {
                self.run_runtime_test(&compiler_path, true)
            }
            TestCategory::RuntimeFail => {
                self.run_runtime_test(&compiler_path, false)
            }
        }
    }
    
    fn run_compile_test(&self, compiler: &Path, should_pass: bool) -> Result<(), String> {
        let output = Command::new(compiler)
            .arg(&self.path)
            .arg("-o")
            .arg("/tmp/zen_test_output")
            .output()
            .map_err(|e| format!("Failed to run compiler: {}", e))?;
        
        let success = output.status.success();
        
        if should_pass && !success {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Expected compilation to pass, but it failed:\n{}", stderr));
        }
        
        if !should_pass && success {
            return Err("Expected compilation to fail, but it succeeded".to_string());
        }
        
        if !should_pass {
            // Check for expected error message
            if let Some(expected) = &self.expected_error {
                let stderr = String::from_utf8_lossy(&output.stderr);
                if !stderr.contains(expected) {
                    return Err(format!(
                        "Expected error '{}' not found in:\n{}",
                        expected, stderr
                    ));
                }
            }
        }
        
        // Clean up
        let _ = fs::remove_file("/tmp/zen_test_output");
        
        Ok(())
    }
    
    fn run_runtime_test(&self, compiler: &Path, should_pass: bool) -> Result<(), String> {
        // First compile
        let compile_output = Command::new(compiler)
            .arg(&self.path)
            .arg("-o")
            .arg("/tmp/zen_test_output")
            .output()
            .map_err(|e| format!("Failed to run compiler: {}", e))?;
        
        if !compile_output.status.success() {
            let stderr = String::from_utf8_lossy(&compile_output.stderr);
            return Err(format!("Compilation failed:\n{}", stderr));
        }
        
        // Then run
        let run_output = Command::new("/tmp/zen_test_output")
            .output()
            .map_err(|e| format!("Failed to run compiled program: {}", e))?;
        
        let success = run_output.status.success();
        
        if should_pass && !success {
            let stderr = String::from_utf8_lossy(&run_output.stderr);
            return Err(format!("Expected program to run successfully, but it failed:\n{}", stderr));
        }
        
        if !should_pass && success {
            return Err("Expected program to fail at runtime, but it succeeded".to_string());
        }
        
        // Clean up
        let _ = fs::remove_file("/tmp/zen_test_output");
        
        Ok(())
    }
}

/// Get path to the Zen compiler
fn get_compiler_path() -> Result<PathBuf, String> {
    let release_path = PathBuf::from("target/release/zen");
    if release_path.exists() {
        return Ok(release_path);
    }
    
    let debug_path = PathBuf::from("target/debug/zen");
    if debug_path.exists() {
        return Ok(debug_path);
    }
    
    Err("Zen compiler not found. Please build with 'cargo build'".to_string())
}

/// Extract expected error message from test file comments
fn extract_expected_error(path: &Path) -> Option<String> {
    let content = fs::read_to_string(path).ok()?;
    
    // Look for "// ERROR: " comments
    for line in content.lines() {
        if let Some(error) = line.strip_prefix("// ERROR: ") {
            return Some(error.to_string());
        }
    }
    
    None
}

/// Discover all test files
fn discover_tests() -> Vec<ZenTest> {
    let mut tests = Vec::new();
    
    // Test directories to search
    let test_dirs = [
        "tests/language/compile_pass",
        "tests/language/compile_fail",
        "tests/language/runtime_pass",
        "tests/language/runtime_fail",
        "tests", // Also check root tests directory
    ];
    
    for dir in &test_dirs {
        let path = Path::new(dir);
        if !path.exists() {
            continue;
        }
        
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("zen") {
                    // Skip work-in-progress or broken tests
                    let name = path.file_name().unwrap().to_string_lossy();
                    if name.contains("_wip.") || name.contains("_broken.") {
                        continue;
                    }
                    
                    // Only include zen_test_* files from root tests directory
                    if dir == &"tests" && !name.starts_with("zen_test_") {
                        continue;
                    }
                    
                    if let Some(test) = ZenTest::from_path(path) {
                        tests.push(test);
                    }
                }
            }
        }
    }
    
    tests.sort_by(|a, b| a.path.cmp(&b.path));
    tests
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_zen_language_suite() {
        // Ensure compiler is built
        let compiler = match get_compiler_path() {
            Ok(path) => path,
            Err(e) => {
                eprintln!("Skipping Zen tests: {}", e);
                return;
            }
        };
        
        println!("Using compiler: {:?}", compiler);
        
        let tests = discover_tests();
        println!("Found {} Zen test files", tests.len());
        
        let mut passed = 0;
        let mut failed = 0;
        let mut failures = Vec::new();
        
        for test in tests {
            let test_name = test.path.display().to_string();
            print!("Testing {} ... ", test_name);
            
            match test.run() {
                Ok(()) => {
                    println!("ok");
                    passed += 1;
                }
                Err(e) => {
                    println!("FAILED");
                    failures.push((test_name.clone(), e));
                    failed += 1;
                }
            }
        }
        
        println!("\nTest results: {} passed, {} failed", passed, failed);
        
        if !failures.is_empty() {
            println!("\nFailures:");
            for (name, error) in failures {
                println!("\n{}: {}", name, error);
            }
            panic!("{} tests failed", failed);
        }
    }
}