//! Compiler intrinsics codegen - inline_c, raw_allocate, etc.

use super::super::super::LLVMCompiler;
use crate::ast;
use crate::error::CompileError;
use inkwell::module::Linkage;
use inkwell::values::BasicValueEnum;

/// Compile compiler.inline_c() - Inline C code compilation
pub fn compile_inline_c<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    if args.len() != 1 {
        return Err(CompileError::TypeError(
            format!("compiler.inline_c expects 1 argument (C code string), got {}", args.len()),
            None,
        ));
    }

    // For now, inline.c() is a placeholder that returns void
    // Full implementation would:
    // 1. Parse the C code string
    // 2. Resolve ${variable} interpolations
    // 3. Compile C code to LLVM IR using Clang or direct LLVM
    // 4. Insert the IR into the current function
    
    // TODO: Implement full inline C compilation
    // This is a complex feature that requires:
    // - C parser/compiler integration (Clang)
    // - String interpolation resolution
    // - LLVM IR insertion
    
    // For now, return void
    Ok(compiler.context.i32_type().const_zero().into())
}

/// Compile compiler.raw_allocate() - Raw memory allocation
pub fn compile_raw_allocate<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    if args.len() != 1 {
        return Err(CompileError::TypeError(
            format!("compiler.raw_allocate expects 1 argument (size), got {}", args.len()),
            None,
        ));
    }

    let size = compiler.compile_expression(&args[0])?;
    let ptr_type = compiler.context.ptr_type(inkwell::AddressSpace::default());
    
    // Get or declare malloc
    let malloc_fn = compiler.module.get_function("malloc").unwrap_or_else(|| {
        let i64_type = compiler.context.i64_type();
        let fn_type = ptr_type.fn_type(&[i64_type.into()], false);
        compiler.module.add_function("malloc", fn_type, Some(Linkage::External))
    });

    // Convert size to i64 if needed
    let size_i64 = if size.is_int_value() {
        let int_val = size.into_int_value();
        if int_val.get_type().get_bit_width() != 64 {
            compiler.builder.build_int_z_extend(int_val, compiler.context.i64_type(), "size_extend")?
        } else {
            int_val
        }
    } else {
        return Err(CompileError::TypeError(
            "compiler.raw_allocate size must be an integer".to_string(),
            None,
        ));
    };

    let ptr = compiler.builder.build_call(malloc_fn, &[size_i64.into()], "allocated_ptr")?
        .try_as_basic_value()
        .left()
        .ok_or_else(|| CompileError::InternalError(
            "malloc should return a pointer".to_string(),
            None,
        ))?;

    Ok(ptr)
}

/// Compile compiler.raw_deallocate() - Raw memory deallocation
pub fn compile_raw_deallocate<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    if args.len() != 2 {
        return Err(CompileError::TypeError(
            format!("compiler.raw_deallocate expects 2 arguments (ptr, size), got {}", args.len()),
            None,
        ));
    }

    let ptr = compiler.compile_expression(&args[0])?;
    let _size = compiler.compile_expression(&args[1])?; // Size kept for allocator compatibility

    // Get or declare free
    let free_fn = compiler.module.get_function("free").unwrap_or_else(|| {
        let ptr_type = compiler.context.ptr_type(inkwell::AddressSpace::default());
        let fn_type = compiler.context.void_type().fn_type(&[ptr_type.into()], false);
        compiler.module.add_function("free", fn_type, Some(Linkage::External))
    });

    // Call free (size is ignored in standard free, but kept for allocator compatibility)
    compiler.builder.build_call(free_fn, &[ptr.into()], "free_call")?;

    // Return void
    Ok(compiler.context.i32_type().const_zero().into())
}

/// Compile compiler.raw_reallocate() - Raw memory reallocation
pub fn compile_raw_reallocate<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    if args.len() != 3 {
        return Err(CompileError::TypeError(
            format!("compiler.raw_reallocate expects 3 arguments (ptr, old_size, new_size), got {}", args.len()),
            None,
        ));
    }

    let ptr = compiler.compile_expression(&args[0])?;
    let _old_size = compiler.compile_expression(&args[1])?;
    let new_size = compiler.compile_expression(&args[2])?;

    // Get or declare realloc
    let ptr_type = compiler.context.ptr_type(inkwell::AddressSpace::default());
    let realloc_fn = compiler.module.get_function("realloc").unwrap_or_else(|| {
        let i64_type = compiler.context.i64_type();
        let fn_type = ptr_type.fn_type(&[ptr_type.into(), i64_type.into()], false);
        compiler.module.add_function("realloc", fn_type, Some(Linkage::External))
    });

    // Convert new_size to i64 if needed
    let new_size_i64 = if new_size.is_int_value() {
        let int_val = new_size.into_int_value();
        if int_val.get_type().get_bit_width() != 64 {
            compiler.builder.build_int_z_extend(int_val, compiler.context.i64_type(), "size_extend")?
        } else {
            int_val
        }
    } else {
        return Err(CompileError::TypeError(
            "compiler.raw_reallocate new_size must be an integer".to_string(),
            None,
        ));
    };

    let new_ptr = compiler.builder.build_call(realloc_fn, &[ptr.into(), new_size_i64.into()], "realloc_ptr")?
        .try_as_basic_value()
        .left()
        .ok_or_else(|| CompileError::InternalError(
            "realloc should return a pointer".to_string(),
            None,
        ))?;

    Ok(new_ptr)
}

/// Compile compiler.raw_ptr_offset() - Pointer arithmetic
pub fn compile_raw_ptr_offset<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    if args.len() != 2 {
        return Err(CompileError::TypeError(
            format!("compiler.raw_ptr_offset expects 2 arguments (ptr, offset), got {}", args.len()),
            None,
        ));
    }

    let ptr = compiler.compile_expression(&args[0])?;
    let offset = compiler.compile_expression(&args[1])?;

    // Convert offset to i64
    let offset_i64 = if offset.is_int_value() {
        let int_val = offset.into_int_value();
        if int_val.get_type().get_bit_width() != 64 {
            // Sign extend for signed offsets
            compiler.builder.build_int_s_extend(int_val, compiler.context.i64_type(), "offset_extend")?
        } else {
            int_val
        }
    } else {
        return Err(CompileError::TypeError(
            "compiler.raw_ptr_offset offset must be an integer".to_string(),
            None,
        ));
    };

    // Convert offset to bytes (assuming u8 pointer)
    let i8_type = compiler.context.i8_type();
    let offset_ptr = unsafe {
        compiler.builder.build_gep(i8_type, ptr.into_pointer_value(), &[offset_i64], "offset_ptr")?
    };

    Ok(offset_ptr.into())
}

/// Compile compiler.raw_ptr_cast() - Pointer type casting
pub fn compile_raw_ptr_cast<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    if args.len() != 1 {
        return Err(CompileError::TypeError(
            format!("compiler.raw_ptr_cast expects 1 argument (ptr), got {}", args.len()),
            None,
        ));
    }

    // Pointer casting is a no-op at LLVM level (all pointers are the same)
    // This is mainly for type checking in Zen
    let ptr = compiler.compile_expression(&args[0])?;
    Ok(ptr)
}

/// Compile compiler.call_external() - Call external C function
pub fn compile_call_external<'ctx>(
    _compiler: &mut LLVMCompiler<'ctx>,
    _args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    // This is a placeholder - full implementation would require:
    // - Function signature information
    // - Argument marshalling
    // - Return value handling
    
    Err(CompileError::InternalError(
        "compiler.call_external() not yet fully implemented - use inline.c() for direct C calls".to_string(),
        None,
    ))
}

/// Compile compiler.load_library() - Load dynamic library
pub fn compile_load_library<'ctx>(
    _compiler: &mut LLVMCompiler<'ctx>,
    _args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    // This requires platform-specific library loading (dlopen, LoadLibrary, etc.)
    // For now, return a placeholder
    
    Err(CompileError::InternalError(
        "compiler.load_library() not yet fully implemented - requires platform-specific FFI".to_string(),
        None,
    ))
}

/// Compile compiler.get_symbol() - Get symbol from library
pub fn compile_get_symbol<'ctx>(
    _compiler: &mut LLVMCompiler<'ctx>,
    _args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    // This requires platform-specific symbol lookup (dlsym, GetProcAddress, etc.)
    
    Err(CompileError::InternalError(
        "compiler.get_symbol() not yet fully implemented - requires platform-specific FFI".to_string(),
        None,
    ))
}

/// Compile compiler.unload_library() - Unload dynamic library
pub fn compile_unload_library<'ctx>(
    _compiler: &mut LLVMCompiler<'ctx>,
    _args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    // This requires platform-specific library unloading (dlclose, FreeLibrary, etc.)
    
    Err(CompileError::InternalError(
        "compiler.unload_library() not yet fully implemented - requires platform-specific FFI".to_string(),
        None,
    ))
}

/// Compile compiler.null_ptr() - Get null pointer
pub fn compile_null_ptr<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    _args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    let ptr_type = compiler.context.ptr_type(inkwell::AddressSpace::default());
    let null_ptr = ptr_type.const_null();
    Ok(null_ptr.into())
}
