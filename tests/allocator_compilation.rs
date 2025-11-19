//! Integration tests for allocator interface compilation and functionality
//! Tests that stdlib allocator modules compile and work correctly
//!
//! NOTE: These tests call compiler.raw_allocate directly without importing @std
//! The @std module system is still being implemented. When it's ready, tests
//! should be updated to use: { compiler } = @std

use zen::compiler::Compiler;
use zen::error::CompileError;
use zen::lexer::Lexer;
use zen::parser::Parser;
use inkwell::context::Context;

/// Test helper - compile Zen code and verify it generates valid LLVM IR
fn compile_code(code: &str) -> Result<(), CompileError> {
    let context = Context::create();
    let compiler = Compiler::new(&context);
    
    let lexer = Lexer::new(code);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program()?;
    
    // Compile to LLVM IR
    compiler.get_module(&program)?;
    Ok(())
}

#[test]
fn test_gpa_allocator_basic() {
    let code = r#"
        main = () i32 {
            alloc = compiler.raw_allocate(100)
            compiler.raw_deallocate(alloc, 100)
            return 0
        }
    "#;
    
    match compile_code(code) {
        Ok(_) => {},
        Err(e) => panic!("GPA allocator basic test should compile: {:?}", e),
    }
}

#[test]
fn test_allocator_allocate_array() {
    let code = r#"
        main = () i32 {
            size = 10
            total = size * 8
            alloc = compiler.raw_allocate(total)
            compiler.raw_deallocate(alloc, total)
            return 0
        }
    "#;
    
    match compile_code(code) {
        Ok(_) => {},
        Err(e) => panic!("Allocator array test should compile: {:?}", e),
    }
}

#[test]
fn test_allocator_reallocate() {
    let code = r#"
        main = () i32 {
            ptr = compiler.raw_allocate(50)
            new_ptr = compiler.raw_reallocate(ptr, 50, 100)
            compiler.raw_deallocate(new_ptr, 100)
            return 0
        }
    "#;
    
    match compile_code(code) {
        Ok(_) => {},
        Err(e) => panic!("Allocator reallocate test should compile: {:?}", e),
    }
}

#[test]
fn test_allocator_with_null_check() {
    let code = r#"
        main = () i32 {
            ptr = compiler.raw_allocate(100)
            null_ptr = compiler.null_ptr()
            is_null = ptr == null_ptr ?
                | true { 1 }
                | false { 0 }
            compiler.raw_deallocate(ptr, 100)
            return is_null
        }
    "#;
    
    match compile_code(code) {
        Ok(_) => {},
        Err(e) => panic!("Allocator null check test should compile: {:?}", e),
    }
}

#[test]
fn test_gpa_allocate_multiple() {
    let code = r#"
        main = () i32 {
            ptr1 = compiler.raw_allocate(100)
            ptr2 = compiler.raw_allocate(200)
            ptr3 = compiler.raw_allocate(50)
            
            compiler.raw_deallocate(ptr1, 100)
            compiler.raw_deallocate(ptr2, 200)
            compiler.raw_deallocate(ptr3, 50)
            
            return 0
        }
    "#;
    
    match compile_code(code) {
        Ok(_) => {},
        Err(e) => panic!("GPA allocate multiple test should compile: {:?}", e),
    }
}

#[test]
fn test_allocator_with_pointer_arithmetic() {
    let code = r#"
        main = () i32 {
            base = compiler.raw_allocate(1000)
            offset = compiler.gep(base, 100)
            another_offset = compiler.gep(offset, 50)
            compiler.raw_deallocate(base, 1000)
            return 0
        }
    "#;
    
    match compile_code(code) {
        Ok(_) => {},
        Err(e) => panic!("Allocator with pointer arithmetic should compile: {:?}", e),
    }
}

#[test]
fn test_allocator_loop_allocations() {
    let code = r#"
        main = () i32 {
            i:: i32 = 0
            loop {
                cond = i < 10 ?
                    | true { true }
                    | false { break }
                ptr = compiler.raw_allocate(50)
                compiler.raw_deallocate(ptr, 50)
                i = i + 1
            }
            return 0
        }
    "#;
    
    match compile_code(code) {
        Ok(_) => {},
        Err(e) => panic!("Allocator in loop should compile: {:?}", e),
    }
}

#[test]
fn test_allocator_conditional_allocation() {
    let code = r#"
        main = () i32 {
            size = 100
            ptr = size > 50 ?
                | true { compiler.raw_allocate(size) }
                | false { compiler.null_ptr() }
            
            ptr != compiler.null_ptr() ?
                | true { compiler.raw_deallocate(ptr, size) }
                | false { }
            
            return 0
        }
    "#;
    
    match compile_code(code) {
        Ok(_) => {},
        Err(e) => panic!("Allocator with conditional allocation should compile: {:?}", e),
    }
}

#[test]
fn test_allocator_overflow_check() {
    let code = r#"
        main = () i32 {
            count = 1000000
            item_size = 8
            total_size = count * item_size
            
            // Check for overflow
            overflow = total_size < count ?
                | true { 1 }
                | false { 0 }
            
            ptr = compiler.raw_allocate(total_size)
            compiler.raw_deallocate(ptr, total_size)
            
            return overflow
        }
    "#;
    
    match compile_code(code) {
        Ok(_) => {},
        Err(e) => panic!("Allocator overflow check should compile: {:?}", e),
    }
}

#[test]
fn test_allocator_with_type_casting() {
    let code = r#"
        main = () i32 {
            // Allocate for i32 array
            count = 10
            item_size = 4
            total_size = count * item_size
            
            ptr = compiler.raw_allocate(total_size)
            typed_ptr = compiler.raw_ptr_cast(ptr)
            compiler.raw_deallocate(ptr, total_size)
            
            return 0
        }
    "#;
    
    match compile_code(code) {
        Ok(_) => {},
        Err(e) => panic!("Allocator with type casting should compile: {:?}", e),
    }
}

#[test]
fn test_allocator_string_usage() {
    // This documents how String would use the allocator
    let code = r#"
        main = () i32 {
            // String internally allocates memory
            s = "hello world"
            // String should deallocate when it goes out of scope
            return 0
        }
    "#;
    
    match compile_code(code) {
        Ok(_) => {},
        Err(e) => panic!("Allocator with string usage should compile: {:?}", e),
    }
}
