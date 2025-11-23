//! Compiler intrinsics codegen - inline_c, raw_allocate, etc.

use super::super::{LLVMCompiler, Type};
use crate::ast;
use crate::error::CompileError;
use inkwell::module::Linkage;
use inkwell::values::BasicValueEnum;
use inkwell::types::{BasicType, BasicTypeEnum};

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

/// Compile compiler.discriminant() - Get enum discriminant
/// Reads the discriminant (tag) from an enum value
/// Enums are laid out as: [i32 discriminant][padding][payload]
pub fn compile_discriminant<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    if args.len() != 1 {
        return Err(CompileError::TypeError(
            format!("compiler.discriminant expects 1 argument, got {}", args.len()),
            None,
        ));
    }

    let enum_ptr = compiler.compile_expression(&args[0])?;
    let i32_type = compiler.context.i32_type();
    
    // Cast to i32* to access discriminant
    let enum_ptr_cast = compiler.builder.build_pointer_cast(
        enum_ptr.into_pointer_value(),
        compiler.context.ptr_type(inkwell::AddressSpace::default()),
        "enum_ptr_i32",
    )?;
    
    // Load discriminant (first i32)
    let discriminant = compiler.builder.build_load(i32_type, enum_ptr_cast, "discriminant")?;
    
    Ok(discriminant)
}

/// Compile compiler.set_discriminant() - Set enum discriminant
/// Writes the discriminant (tag) to an enum value
pub fn compile_set_discriminant<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    if args.len() != 2 {
        return Err(CompileError::TypeError(
            format!("compiler.set_discriminant expects 2 arguments, got {}", args.len()),
            None,
        ));
    }

    let enum_ptr = compiler.compile_expression(&args[0])?;
    let discriminant = compiler.compile_expression(&args[1])?;
    
    let i32_type = compiler.context.i32_type();
    
    // Cast to i32* to access discriminant
    let enum_ptr_cast = compiler.builder.build_pointer_cast(
        enum_ptr.into_pointer_value(),
        compiler.context.ptr_type(inkwell::AddressSpace::default()),
        "enum_ptr_i32",
    )?;
    
    // Store discriminant
    compiler.builder.build_store(enum_ptr_cast, discriminant)?;
    
    // Return void
    Ok(compiler.context.i32_type().const_zero().into())
}

/// Compile compiler.get_payload() - Get enum payload
/// Returns a pointer to the payload data within an enum
pub fn compile_get_payload<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    if args.len() != 1 {
        return Err(CompileError::TypeError(
            format!("compiler.get_payload expects 1 argument, got {}", args.len()),
            None,
        ));
    }

    let enum_ptr = compiler.compile_expression(&args[0])?;
    let enum_ptr_val = enum_ptr.into_pointer_value();
    
    // Payload starts after the discriminant (i32)
    // Use GEP to skip the discriminant field
    let i8_type = compiler.context.i8_type();
    
    let payload_offset = unsafe {
        compiler.builder.build_gep(
            i8_type,
            enum_ptr_val,
            &[compiler.context.i32_type().const_int(4, false)],
            "payload_ptr",
        )?
    };
    
    Ok(payload_offset.into())
}

/// Compile compiler.set_payload() - Set enum payload
/// Copies payload data into the enum's payload field
pub fn compile_set_payload<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    if args.len() != 2 {
        return Err(CompileError::TypeError(
            format!("compiler.set_payload expects 2 arguments, got {}", args.len()),
            None,
        ));
    }

    let _enum_ptr = compiler.compile_expression(&args[0])?;
    let _payload_ptr = compiler.compile_expression(&args[1])?;
    
    // For now, we return void and expect the caller to handle the copy
    // In a real implementation, we'd need to know the payload size
    // This is typically handled at a higher level where types are known
    
    // TODO: Implement proper payload copying with size information
    // For now, just return void to satisfy the interface
    
    Ok(compiler.context.i32_type().const_zero().into())
}

/// Compile compiler.gep() - Byte-level pointer arithmetic (GetElementPointer)
/// Performs byte-offset pointer arithmetic with optional bounds checking
pub fn compile_gep<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    if args.len() != 2 {
        return Err(CompileError::TypeError(
            format!("compiler.gep expects 2 arguments (ptr, offset), got {}", args.len()),
            None,
        ));
    }

    let base_ptr = compiler.compile_expression(&args[0])?;
    let offset = compiler.compile_expression(&args[1])?;

    // Convert offset to i64 if needed
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
            "compiler.gep offset must be an integer".to_string(),
            None,
        ));
    };

    // Use GEP for byte-level pointer arithmetic
    // Treat as i8 for byte-level access
    let i8_type = compiler.context.i8_type();
    let result_ptr = unsafe {
        compiler.builder.build_gep(i8_type, base_ptr.into_pointer_value(), &[offset_i64], "gep_result")?
    };

    Ok(result_ptr.into())
}

/// Compile compiler.gep_struct() - Struct field access via pointer
/// Accesses fields of a struct using field index
/// This is a type-aware variant of GEP for struct field access
pub fn compile_gep_struct<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    if args.len() != 2 {
        return Err(CompileError::TypeError(
            format!("compiler.gep_struct expects 2 arguments (struct_ptr, field_index), got {}", args.len()),
            None,
        ));
    }

    let struct_ptr = compiler.compile_expression(&args[0])?;
    let field_index = compiler.compile_expression(&args[1])?;

    // Convert field_index to u32 if needed
    let field_idx_u32 = if field_index.is_int_value() {
        let int_val = field_index.into_int_value();
        // Ensure we have a valid u32 field index
        if int_val.get_type().get_bit_width() != 32 {
            compiler.builder.build_int_truncate(int_val, compiler.context.i32_type(), "field_idx_trunc")?
        } else {
            int_val
        }
    } else {
        return Err(CompileError::TypeError(
            "compiler.gep_struct field_index must be an integer".to_string(),
            None,
        ));
    };

    // For struct field access, we need the struct type
    // Since we're working with raw pointers, we approximate by using byte offsets
    // A proper implementation would require type information
    
    // As a placeholder, treat field_index as a byte offset multiplied by alignment
    // Typical alignment is 8 bytes for most types
    let field_offset = compiler.builder.build_int_mul(
        field_idx_u32,
        compiler.context.i32_type().const_int(8, false),
        "field_offset_bytes"
    )?;
    
    let field_offset_i64 = compiler.builder.build_int_s_extend(
        field_offset,
        compiler.context.i64_type(),
        "field_offset_i64"
    )?;

    // Use GEP for the computed offset
    let i8_type = compiler.context.i8_type();
    let result_ptr = unsafe {
        compiler.builder.build_gep(
            i8_type,
            struct_ptr.into_pointer_value(),
            &[field_offset_i64],
            "gep_struct_result"
        )?
    };

    Ok(result_ptr.into())
}

/// Compile compiler.load<T>() - Load value from pointer
/// Generic function that loads a value of type T from a pointer
pub fn compile_load<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
    type_arg: Option<&ast::AstType>,
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    if args.len() != 1 {
        return Err(CompileError::TypeError(
            format!("compiler.load expects 1 argument (ptr), got {}", args.len()),
            None,
        ));
    }

    let ptr = compiler.compile_expression(&args[0])?;
    
    // Get the type to load - either from type_arg or default to i32
    let load_type = if let Some(ty) = type_arg {
        compiler.to_llvm_type(ty)?
    } else {
        // Default to i32 if type not specified
        Type::Basic(compiler.context.i32_type().into())
    };

    // Get the basic type for loading
    let basic_type = match load_type {
        Type::Basic(b) => b,
        _ => return Err(CompileError::TypeError(
            "compiler.load can only load basic types".to_string(),
            None,
        )),
    };

    // Cast pointer to pointer-to-T
    let ptr_type = match basic_type {
        BasicTypeEnum::IntType(t) => t.ptr_type(inkwell::AddressSpace::default()),
        BasicTypeEnum::FloatType(t) => t.ptr_type(inkwell::AddressSpace::default()),
        BasicTypeEnum::PointerType(t) => t.ptr_type(inkwell::AddressSpace::default()),
        BasicTypeEnum::StructType(t) => t.ptr_type(inkwell::AddressSpace::default()),
        _ => compiler.context.ptr_type(inkwell::AddressSpace::default()),
    };
    
    let typed_ptr = compiler.builder.build_pointer_cast(
        ptr.into_pointer_value(),
        ptr_type,
        "typed_ptr"
    )?;

    // Load the value
    let loaded = compiler.builder.build_load(basic_type, typed_ptr, "loaded_value")?;
    Ok(loaded)
}

/// Compile compiler.store<T>() - Store value to pointer
/// Generic function that stores a value of type T to a pointer
pub fn compile_store<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
    type_arg: Option<&ast::AstType>,
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    if args.len() != 2 {
        return Err(CompileError::TypeError(
            format!("compiler.store expects 2 arguments (ptr, value), got {}", args.len()),
            None,
        ));
    }

    let ptr = compiler.compile_expression(&args[0])?;
    let value = compiler.compile_expression(&args[1])?;

    // Get the type to store - either from type_arg or infer from value type
    let store_type = if let Some(ty) = type_arg {
        compiler.to_llvm_type(ty)?
    } else {
        // Infer from value type
        match value {
            BasicValueEnum::IntValue(iv) => Type::Basic(iv.get_type().into()),
            BasicValueEnum::FloatValue(fv) => Type::Basic(fv.get_type().into()),
            BasicValueEnum::PointerValue(_) => Type::Basic(compiler.context.ptr_type(inkwell::AddressSpace::default()).into()),
            BasicValueEnum::StructValue(sv) => Type::Struct(sv.get_type()),
            _ => Type::Basic(compiler.context.i32_type().into()),
        }
    };

    // Get the basic type for storing and pointer type
    let (basic_type, ptr_type) = match store_type {
        Type::Basic(b) => {
            let ptr_ty = match b {
                BasicTypeEnum::IntType(t) => t.ptr_type(inkwell::AddressSpace::default()),
                BasicTypeEnum::FloatType(t) => t.ptr_type(inkwell::AddressSpace::default()),
                BasicTypeEnum::PointerType(t) => t.ptr_type(inkwell::AddressSpace::default()),
                BasicTypeEnum::StructType(t) => t.ptr_type(inkwell::AddressSpace::default()),
                _ => compiler.context.ptr_type(inkwell::AddressSpace::default()),
            };
            (b, ptr_ty)
        },
        Type::Struct(st) => {
            let ptr_ty = st.ptr_type(inkwell::AddressSpace::default());
            // Convert struct to BasicTypeEnum for build_store
            (st.as_basic_type_enum(), ptr_ty)
        },
        _ => return Err(CompileError::TypeError(
            "compiler.store can only store basic types or structs".to_string(),
            None,
        )),
    };
    
    let typed_ptr = compiler.builder.build_pointer_cast(
        ptr.into_pointer_value(),
        ptr_type,
        "typed_ptr"
    )?;

    // Store the value
    compiler.builder.build_store(typed_ptr, value)?;

    // Return void
    Ok(compiler.context.i32_type().const_zero().into())
}

/// Compile compiler.ptr_to_int() - Convert pointer to integer
pub fn compile_ptr_to_int<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    if args.len() != 1 {
        return Err(CompileError::TypeError(
            format!("compiler.ptr_to_int expects 1 argument (ptr), got {}", args.len()),
            None,
        ));
    }

    let ptr = compiler.compile_expression(&args[0])?;
    let ptr_val = ptr.into_pointer_value();
    let i64_type = compiler.context.i64_type();
    let int_val = compiler.builder.build_ptr_to_int(ptr_val, i64_type, "ptr_to_int")?;
    Ok(int_val.into())
}

/// Compile compiler.int_to_ptr() - Convert integer to pointer
pub fn compile_int_to_ptr<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    if args.len() != 1 {
        return Err(CompileError::TypeError(
            format!("compiler.int_to_ptr expects 1 argument (addr), got {}", args.len()),
            None,
        ));
    }

    let addr = compiler.compile_expression(&args[0])?;
    let int_val = addr.into_int_value();
    let ptr_type = compiler.context.ptr_type(inkwell::AddressSpace::default());
    let ptr_val = compiler.builder.build_int_to_ptr(int_val, ptr_type, "int_to_ptr")?;
    Ok(ptr_val.into())
}
