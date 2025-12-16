//! Behavioral tests that compile and RUN code to verify correct output.
//!
//! These tests catch semantic bugs that syntax/IR-only tests miss:
//! - Type mismatches that produce wrong values (not crashes)
//! - Memory corruption from stack layout issues
//! - Logic errors in codegen
//! - Silent data corruption
//!
//! Philosophy: Test what the user cares about - does the program produce correct output?

use std::process::{Command, Stdio};
use std::path::Path;
use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};

use zen::compiler::Compiler;
use zen::lexer::Lexer;
use zen::parser::Parser;
use inkwell::context::Context;
use inkwell::targets::{CodeModel, FileType, RelocMode, Target, TargetMachine, InitializationConfig};
use inkwell::OptimizationLevel;

/// Global counter for unique test IDs (thread-safe)
static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Result of running compiled code
#[derive(Debug)]
pub struct RunResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

/// Compile Zen source code to a temporary executable and run it.
/// Returns the exit code and captured stdout/stderr.
fn compile_and_run(source: &str) -> Result<RunResult, String> {
    // Initialize LLVM
    Target::initialize_native(&InitializationConfig::default())
        .map_err(|e| format!("LLVM init failed: {}", e))?;

    let context = Context::create();
    let compiler = Compiler::new(&context);

    // Parse
    let lexer = Lexer::new(source);
    let mut parser = Parser::new(lexer);
    let program = parser
        .parse_program()
        .map_err(|e| format!("Parse error: {}", e))?;

    // Compile to LLVM module
    let module = compiler
        .get_module(&program)
        .map_err(|e| format!("Compilation error: {}", e))?;

    // Create temp file paths with unique IDs to avoid conflicts in parallel tests
    let test_id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let thread_id = std::thread::current().id();
    let obj_path = format!("/tmp/zen_test_{:?}_{}.o", thread_id, test_id);
    let exe_path = format!("/tmp/zen_test_{:?}_{}", thread_id, test_id);

    // Get target machine
    let target_triple = TargetMachine::get_default_triple();
    let target = Target::from_triple(&target_triple)
        .map_err(|e| format!("Failed to get target: {}", e))?;

    let target_machine = target
        .create_target_machine(
            &target_triple,
            "generic",
            "",
            OptimizationLevel::None,
            RelocMode::Default,
            CodeModel::Default,
        )
        .ok_or_else(|| "Failed to create target machine".to_string())?;

    // Write object file
    target_machine
        .write_to_file(&module, FileType::Object, Path::new(&obj_path))
        .map_err(|e| format!("Failed to write object file: {}", e))?;

    // Link
    let link_status = Command::new("cc")
        .arg(&obj_path)
        .arg("-o")
        .arg(&exe_path)
        .arg("-no-pie")
        .arg("-lm")
        .status()
        .map_err(|e| format!("Failed to link: {}", e))?;

    if !link_status.success() {
        fs::remove_file(&obj_path).ok();
        return Err("Linking failed".to_string());
    }

    // Clean up object file
    fs::remove_file(&obj_path).ok();

    // Verify executable exists
    if !Path::new(&exe_path).exists() {
        return Err(format!("Executable was not created at {}", exe_path));
    }

    // Run the executable
    let output = Command::new(&exe_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("Failed to run executable: {}", e))?;

    // Clean up executable
    fs::remove_file(&exe_path).ok();

    // Handle signals (e.g., segfault = SIGSEGV = signal 11)
    let exit_code = output.status.code().unwrap_or_else(|| {
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            if let Some(signal) = output.status.signal() {
                return -(signal as i32); // Return negative signal number
            }
        }
        -1
    });

    Ok(RunResult {
        exit_code,
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    })
}

/// Helper to compile and run, asserting success
fn run_expecting_success(source: &str) -> RunResult {
    match compile_and_run(source) {
        Ok(result) => {
            if result.exit_code < 0 {
                let signal = -result.exit_code;
                let signal_name = match signal {
                    11 => "SIGSEGV (segmentation fault)",
                    6 => "SIGABRT (abort)",
                    8 => "SIGFPE (floating point exception)",
                    _ => "unknown signal",
                };
                panic!(
                    "Program crashed with signal {} ({})!\nstdout: {}\nstderr: {}",
                    signal, signal_name, result.stdout, result.stderr
                );
            }
            result
        }
        Err(e) => panic!("Compilation/run failed: {}", e),
    }
}

// ============================================================================
// REGRESSION TESTS - Tests for specific bugs that were found and fixed
// ============================================================================

/// Regression test for: i64/i32 type mismatch causing stack corruption
/// Bug: Integer literals were i64 but alloca was i32, causing adjacent variables
/// to be overwritten when storing 8 bytes into 4-byte slot.
/// Fixed in: coercing_store in variables.rs
#[test]
fn test_regression_i64_i32_stack_corruption() {
    let source = r#"
        { io } = @std.io

        main = () i32 {
            a = 5
            b = 3
            msg = "${a}"
            // After string interpolation, 'a' should still be 5
            // Bug caused 'a' to become 0 due to stack corruption
            a == 5 ?
                | true { return 0 }
                | false { return 1 }
        }
    "#;

    let result = run_expecting_success(source);
    assert_eq!(
        result.exit_code, 0,
        "Variable 'a' was corrupted! Expected 5, got something else.\nstdout: {}\nstderr: {}",
        result.stdout, result.stderr
    );
}

/// Verify multiple variables maintain their values after string operations
#[test]
fn test_multiple_variables_after_string_interpolation() {
    let source = r#"
        { io } = @std.io

        main = () i32 {
            a = 10
            b = 20
            c = 30
            msg = "${a} ${b} ${c}"

            // All variables should maintain their values
            sum = a + b + c
            sum == 60 ?
                | true { return 0 }
                | false { return 1 }
        }
    "#;

    let result = run_expecting_success(source);
    assert_eq!(
        result.exit_code, 0,
        "Variables were corrupted after string interpolation.\nstdout: {}\nstderr: {}",
        result.stdout, result.stderr
    );
}

// ============================================================================
// ARITHMETIC TESTS
// ============================================================================

#[test]
fn test_basic_arithmetic() {
    let source = r#"
        main = () i32 {
            a = 10
            b = 3
            sum = a + b
            diff = a - b
            prod = a * b
            quot = a / b

            // 10 + 3 = 13, 10 - 3 = 7, 10 * 3 = 30, 10 / 3 = 3
            // sum + diff + prod + quot = 13 + 7 + 30 + 3 = 53
            result = sum + diff + prod + quot
            result == 53 ?
                | true { return 0 }
                | false { return 1 }
        }
    "#;

    let result = run_expecting_success(source);
    assert_eq!(result.exit_code, 0, "Basic arithmetic failed");
}

#[test]
fn test_negative_numbers() {
    let source = r#"
        main = () i32 {
            a = 5
            b = 10
            diff = a - b  // -5

            diff == -5 ?
                | true { return 0 }
                | false { return 1 }
        }
    "#;

    let result = run_expecting_success(source);
    assert_eq!(result.exit_code, 0, "Negative number handling failed");
}

// ============================================================================
// CONTROL FLOW TESTS
// ============================================================================

#[test]
fn test_conditional_branches() {
    let source = r#"
        main = () i32 {
            x = 42
            x == 42 ?
                | true { return 0 }
                | false { return 1 }
        }
    "#;

    let result = run_expecting_success(source);
    assert_eq!(result.exit_code, 0, "Conditional branch failed");
}

#[test]
fn test_conditional_false_branch() {
    let source = r#"
        main = () i32 {
            x = 10
            x == 42 ?
                | true { return 1 }
                | false { return 0 }
        }
    "#;

    let result = run_expecting_success(source);
    assert_eq!(result.exit_code, 0, "Conditional false branch failed");
}

// ============================================================================
// FUNCTION TESTS
// ============================================================================

#[test]
fn test_function_call_and_return() {
    let source = r#"
        add = (a: i32, b: i32) i32 {
            return a + b
        }

        main = () i32 {
            result = add(3, 4)
            result == 7 ?
                | true { return 0 }
                | false { return 1 }
        }
    "#;

    let result = run_expecting_success(source);
    assert_eq!(result.exit_code, 0, "Function call failed");
}

#[test]
fn test_nested_function_calls() {
    let source = r#"
        double = (x: i32) i32 {
            return x * 2
        }

        quadruple = (x: i32) i32 {
            return double(double(x))
        }

        main = () i32 {
            result = quadruple(5)  // 5 * 2 * 2 = 20
            result == 20 ?
                | true { return 0 }
                | false { return 1 }
        }
    "#;

    let result = run_expecting_success(source);
    assert_eq!(result.exit_code, 0, "Nested function calls failed");
}

// ============================================================================
// STRUCT TESTS
// ============================================================================

#[test]
fn test_struct_field_access() {
    let source = r#"
        Point: {
            x: i32,
            y: i32
        }

        main = () i32 {
            p = Point { x: 10, y: 20 }
            sum = p.x + p.y
            sum == 30 ?
                | true { return 0 }
                | false { return 1 }
        }
    "#;

    let result = run_expecting_success(source);
    assert_eq!(result.exit_code, 0, "Struct field access failed");
}

#[test]
fn test_nested_struct_field_access() {
    let source = r#"
        Inner: {
            value: i32
        }

        Outer: {
            inner: Inner
        }

        main = () i32 {
            obj = Outer {
                inner: Inner { value: 42 }
            }
            obj.inner.value == 42 ?
                | true { return 0 }
                | false { return 1 }
        }
    "#;

    let result = run_expecting_success(source);
    assert_eq!(result.exit_code, 0, "Nested struct field access failed");
}

/// Test for struct field assignment type mismatch
/// Note: Direct struct field assignment (p.x = 10) is currently not supported
/// This test uses mutable variable reassignment instead
#[test]
fn test_struct_mutable_reassignment() {
    let source = r#"
        Point: {
            x: i32,
            y: i32
        }

        main = () i32 {
            // Test that struct values work correctly with different field values
            p1 = Point { x: 10, y: 20 }
            p2 = Point { x: 100, y: 200 }

            sum1 = p1.x + p1.y  // 30
            sum2 = p2.x + p2.y  // 300

            total = sum1 + sum2  // 330
            total == 330 ?
                | true { return 0 }
                | false { return 1 }
        }
    "#;

    let result = run_expecting_success(source);
    assert_eq!(result.exit_code, 0, "Struct mutable reassignment failed");
}

// ============================================================================
// OUTPUT VERIFICATION TESTS
// ============================================================================

#[test]
fn test_println_output() {
    let source = r#"
        { io } = @std.io

        main = () i32 {
            io.println("hello")
            return 0
        }
    "#;

    let result = run_expecting_success(source);
    assert_eq!(result.exit_code, 0);
    assert!(
        result.stdout.contains("hello"),
        "Expected 'hello' in output, got: {}",
        result.stdout
    );
}

#[test]
fn test_string_interpolation_output() {
    let source = r#"
        { io } = @std.io

        main = () i32 {
            x = 42
            io.println("value: ${x}")
            return 0
        }
    "#;

    let result = run_expecting_success(source);
    assert_eq!(result.exit_code, 0);
    assert!(
        result.stdout.contains("value: 42"),
        "Expected 'value: 42' in output, got: {}",
        result.stdout
    );
}
