//! Integration tests for pointer operations and pattern matching
//! Tests use compiler intrinsics directly since stdlib import isn't fully working in tests

use inkwell::context::Context;
use zen::compiler::Compiler;
use zen::error::CompileError;
use zen::lexer::Lexer;
use zen::parser::Parser;

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
fn test_raw_pointer_allocation() {
    // Test raw pointer allocation and deallocation using compiler intrinsics
    let code = r#"
        main = () i32 {
            ptr = compiler.raw_allocate(64)
            null_ptr = compiler.null_ptr()
            is_valid = ptr != null_ptr ?
                | true { 1 }
                | false { 0 }
            compiler.raw_deallocate(ptr, 64)
            return is_valid
        }
    "#;

    match compile_code(code) {
        Ok(_) => {}
        Err(e) => panic!("Raw pointer allocation should compile: {:?}", e),
    }
}

#[test]
fn test_pointer_comparison() {
    // Test pointer comparison
    let code = r#"
        main = () i32 {
            ptr1 = compiler.raw_allocate(8)
            ptr2 = compiler.raw_allocate(8)
            null_ptr = compiler.null_ptr()

            // Compare pointers
            same = ptr1 == ptr2 ?
                | true { 1 }
                | false { 0 }

            is_null = ptr1 == null_ptr ?
                | true { 1 }
                | false { 0 }

            compiler.raw_deallocate(ptr1, 8)
            compiler.raw_deallocate(ptr2, 8)
            return same + is_null
        }
    "#;

    match compile_code(code) {
        Ok(_) => {}
        Err(e) => panic!("Pointer comparison should compile: {:?}", e),
    }
}

#[test]
fn test_option_pattern_matching() {
    // Test Option type pattern matching (builtin type)
    let code = r#"
        get_value = () i32 {
            val = Option.Some(42)
            result = val ?
                | .Some(x) { x }
                | .None { 0 }
            return result
        }

        main = () i32 {
            return get_value()
        }
    "#;

    match compile_code(code) {
        Ok(_) => {}
        Err(e) => panic!("Option pattern matching should compile: {:?}", e),
    }
}

#[test]
fn test_option_none() {
    // Test Option.None creation and matching
    let code = r#"
        main = () i32 {
            none_val = Option.None
            none_val ?
                | .Some(_) { return 1 }
                | .None { return 0 }
        }
    "#;

    match compile_code(code) {
        Ok(_) => {}
        Err(e) => panic!("Option.None should compile: {:?}", e),
    }
}

#[test]
fn test_result_pattern_matching() {
    // Test Result type pattern matching (builtin type)
    let code = r#"
        get_result = () i32 {
            val = Result.Ok(100)
            result = val ?
                | .Ok(x) { x }
                | .Err(_e) { 0 }
            return result
        }

        main = () i32 {
            return get_result()
        }
    "#;

    match compile_code(code) {
        Ok(_) => {}
        Err(e) => panic!("Result pattern matching should compile: {:?}", e),
    }
}

#[test]
fn test_result_error_simple() {
    // Test Result.Err creation and matching with simple returns
    let code = r#"
        main = () i32 {
            err_val = Result.Err(42)
            err_val ?
                | .Ok(_) { return 0 }
                | .Err(_e) { return 1 }
        }
    "#;

    match compile_code(code) {
        Ok(_) => {}
        Err(e) => panic!("Result.Err should compile: {:?}", e),
    }
}

#[test]
fn test_custom_enum_pattern() {
    // Test custom enum with pattern matching
    let code = r#"
        // Define a custom enum type
        MyPtr<T>:
            Valid: i64,
            Invalid

        // Test pattern matching on custom enum
        main = () i32 {
            ptr = MyPtr.Valid(12345)
            ptr ?
                | .Valid(addr) { return 1 }
                | .Invalid { return 0 }
        }
    "#;

    match compile_code(code) {
        Ok(_) => {}
        Err(e) => panic!("Custom enum pattern should compile: {:?}", e),
    }
}

#[test]
fn test_enum_with_instance_methods() {
    // Test enum type with instance methods - now works with type tracking fix!
    let code = r#"
        // Define enum type
        SafePtr<T>:
            Some: i64,
            None

        // Instance method - doesn't require static method resolution
        SafePtr<T>.is_valid = (self: SafePtr<T>) bool {
            self ?
                | .Some(_) { true }
                | .None { false }
        }

        main = () i32 {
            // Create using direct enum variant syntax instead of static method
            ptr = SafePtr.Some(12345)
            ptr.is_valid() ?
                | true { return 1 }
                | false { return 0 }
        }
    "#;

    match compile_code(code) {
        Ok(_) => {}
        Err(e) => panic!("Enum with instance methods should compile: {:?}", e),
    }
}

#[test]
fn test_pointer_arithmetic() {
    // Test pointer offset calculations using compiler intrinsics
    let code = r#"
        main = () i32 {
            // Allocate memory for 4 i32 values
            ptr = compiler.raw_allocate(32)

            // Calculate offset - gep(ptr, byte_offset)
            offset_ptr = compiler.gep(ptr, 8)

            // Clean up
            compiler.raw_deallocate(ptr, 32)
            return 0
        }
    "#;

    match compile_code(code) {
        Ok(_) => {}
        Err(e) => panic!("Pointer arithmetic should compile: {:?}", e),
    }
}

#[test]
fn test_sizeof_intrinsic() {
    // Test sizeof intrinsic
    let code = r#"
        main = () i32 {
            size = compiler.sizeof<i32>()
            // sizeof i32 should be 4
            return size
        }
    "#;

    match compile_code(code) {
        Ok(_) => {}
        Err(e) => panic!("sizeof intrinsic should compile: {:?}", e),
    }
}

#[test]
fn test_ptr_to_int_and_back() {
    // Test pointer to integer conversion
    let code = r#"
        main = () i32 {
            ptr = compiler.raw_allocate(8)
            addr = compiler.ptr_to_int(ptr)
            back = compiler.int_to_ptr(addr)

            // Compare original and converted back
            same = ptr == back ?
                | true { 1 }
                | false { 0 }

            compiler.raw_deallocate(ptr, 8)
            return same
        }
    "#;

    match compile_code(code) {
        Ok(_) => {}
        Err(e) => panic!("ptr_to_int/int_to_ptr should compile: {:?}", e),
    }
}
