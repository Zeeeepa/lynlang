use crate::codegen::llvm::LLVMCompiler;
use crate::ast;
use crate::error::CompileError;
use inkwell::{
    module::Linkage,
    values::{BasicValueEnum, PointerValue},
};

pub fn is_allocator_type(_compiler: &LLVMCompiler, _expr: &ast::Expression) -> bool {
    // Check if expression type is Allocator
    // For now, return false - this needs proper type checking
    false
}

/// Compile Array.new(allocator, capacity, default_value) - creates a new array
pub fn compile_array_new<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    // Array REQUIRES an allocator per NO-GC design
    if args.is_empty() {
        return Err(CompileError::TypeError(
            "Array.new() requires an allocator argument for NO-GC memory management".to_string(),
            None,
        ));
    }

    // Array struct: { ptr, length, capacity, allocator }
    let ptr_type = compiler.context.ptr_type(inkwell::AddressSpace::default());
    let array_struct_type = compiler.context.struct_type(
        &[
            ptr_type.into(),                    // data pointer
            compiler.context.i64_type().into(), // length
            compiler.context.i64_type().into(), // capacity
            ptr_type.into(),                    // allocator pointer
        ],
        false,
    );

    // First arg must be allocator
    let allocator_ptr = compiler.compile_expression(&args[0])?;
    let remaining_args = &args[1..];

    // Allow Array.new() with no arguments for generic arrays - use default capacity of 10
    if remaining_args.is_empty() {
        // Create empty array with default capacity of 10
        let default_capacity = compiler.context.i64_type().const_int(10, false);

        // Allocate memory for 10 elements (80 bytes for i64 elements)
        let element_size = compiler.context.i64_type().const_int(8, false);
        let total_size =
            compiler
                .builder
                .build_int_mul(default_capacity, element_size, "total_size")?;

        // For now, always use malloc (allocator is just stored for future use)
        // TODO: Implement proper allocator interface
        let malloc_fn = compiler.module.get_function("malloc").unwrap_or_else(|| {
            let i64_type = compiler.context.i64_type();
            let fn_type = ptr_type.fn_type(&[i64_type.into()], false);
            compiler
                .module
                .add_function("malloc", fn_type, Some(Linkage::External))
        });
        let data_ptr = compiler
            .builder
            .build_call(malloc_fn, &[total_size.into()], "array_data")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| CompileError::InternalError("malloc returned void".to_string(), None))?;

        // Create the array struct
        let array_val = compiler.builder.build_alloca(array_struct_type, "array")?;

        // Set data pointer
        let data_field_ptr =
            compiler
                .builder
                .build_struct_gep(array_struct_type, array_val, 0, "data_ptr")?;
        compiler.builder.build_store(data_field_ptr, data_ptr)?;

        // Set length to 0 (empty array)
        let length_field_ptr =
            compiler
                .builder
                .build_struct_gep(array_struct_type, array_val, 1, "length_ptr")?;
        compiler.builder.build_store(
            length_field_ptr,
            compiler.context.i64_type().const_int(0, false),
        )?;

        // Set capacity to 10
        let capacity_field_ptr =
            compiler
                .builder
                .build_struct_gep(array_struct_type, array_val, 2, "capacity_ptr")?;
        compiler
            .builder
            .build_store(capacity_field_ptr, default_capacity)?;

        // Store allocator pointer
        let allocator_field = compiler.builder.build_struct_gep(
            array_struct_type,
            array_val,
            3,
            "allocator_field",
        )?;
        compiler
            .builder
            .build_store(allocator_field, allocator_ptr)?;

        // Return the array struct
        return Ok(compiler
            .builder
            .build_load(array_struct_type, array_val, "array_loaded")?
            .into());
    }

    // remaining_args contains capacity and default value (if provided)
    if remaining_args.len() != 2 {
        return Err(CompileError::TypeError(
            format!(
                "Array.new expects allocator (optional), capacity, and default value. Got {} args",
                args.len()
            ),
            None,
        ));
    }

    // Compile the capacity argument
    let capacity_val = compiler.compile_expression(&remaining_args[0])?;
    let capacity_raw = capacity_val.into_int_value();

    // Cast capacity to i64 if needed
    let capacity = if capacity_raw.get_type() == compiler.context.i64_type() {
        capacity_raw
    } else {
        compiler.builder.build_int_z_extend(
            capacity_raw,
            compiler.context.i64_type(),
            "capacity_i64",
        )?
    };

    // Compile the default value (could be Option.None)
    let _default_val = compiler.compile_expression(&remaining_args[1])?;

    // Array struct already created above, reuse the same structure
    // (Already defined above with allocator field)

    // Allocate memory for the array data
    // For now, allocate as i64 array (8 bytes per element)
    let element_size = compiler.context.i64_type().const_int(8, false);
    let total_size = compiler
        .builder
        .build_int_mul(capacity, element_size, "total_size")?;

    // Always use malloc for now (allocator is stored for future use)
    // TODO: Implement proper allocator interface
    let malloc_fn = compiler.module.get_function("malloc").unwrap_or_else(|| {
        let i64_type = compiler.context.i64_type();
        let fn_type = ptr_type.fn_type(&[i64_type.into()], false);
        compiler
            .module
            .add_function("malloc", fn_type, Some(Linkage::External))
    });
    let data_ptr = compiler
        .builder
        .build_call(malloc_fn, &[total_size.into()], "array_data")?
        .try_as_basic_value()
        .left()
        .ok_or_else(|| {
            CompileError::InternalError("malloc should return a pointer".to_string(), None)
        })?;

    // Initialize the array to zeros
    let memset_fn = compiler.module.get_function("memset").unwrap_or_else(|| {
        let _i8_type = compiler.context.i8_type();
        let i32_type = compiler.context.i32_type();
        let i64_type = compiler.context.i64_type();
        let fn_type = ptr_type.fn_type(&[ptr_type.into(), i32_type.into(), i64_type.into()], false);
        compiler
            .module
            .add_function("memset", fn_type, Some(Linkage::External))
    });

    compiler.builder.build_call(
        memset_fn,
        &[
            data_ptr.into(),
            compiler.context.i32_type().const_int(0, false).into(),
            total_size.into(),
        ],
        "memset_call",
    )?;

    // Create the Array struct
    let array_alloca = compiler.builder.build_alloca(array_struct_type, "array")?;

    // Store data pointer
    let data_ptr_field =
        compiler
            .builder
            .build_struct_gep(array_struct_type, array_alloca, 0, "data_ptr_field")?;
    compiler.builder.build_store(data_ptr_field, data_ptr)?;

    // Store length (initially 0 for empty array with capacity)
    let length_field =
        compiler
            .builder
            .build_struct_gep(array_struct_type, array_alloca, 1, "length_field")?;
    compiler.builder.build_store(
        length_field,
        compiler.context.i64_type().const_int(0, false),
    )?;

    // Store capacity
    let capacity_field =
        compiler
            .builder
            .build_struct_gep(array_struct_type, array_alloca, 2, "capacity_field")?;
    compiler.builder.build_store(capacity_field, capacity)?;

    // Store allocator pointer
    let allocator_field =
        compiler
            .builder
            .build_struct_gep(array_struct_type, array_alloca, 3, "allocator_field")?;
    compiler
        .builder
        .build_store(allocator_field, allocator_ptr)?;

    // Load and return the array struct
    let result = compiler
        .builder
        .build_load(array_struct_type, array_alloca, "array_value")?;
    Ok(result)
}

/// Compile Array.push(value) by pointer - modifies array in place
pub fn compile_array_push_by_ptr<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    array_ptr: PointerValue<'ctx>,
    value: BasicValueEnum<'ctx>,
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    // Array struct type: { ptr, length, capacity, allocator }
    let array_struct_type = compiler.context.struct_type(
        &[
            compiler
                .context
                .ptr_type(inkwell::AddressSpace::default())
                .into(),
            compiler.context.i64_type().into(),
            compiler.context.i64_type().into(),
            compiler
                .context
                .ptr_type(inkwell::AddressSpace::default())
                .into(),
        ],
        false,
    );

    // Get current length
    let length_ptr =
        compiler
            .builder
            .build_struct_gep(array_struct_type, array_ptr, 1, "length_ptr")?;
    let current_length = compiler
        .builder
        .build_load(compiler.context.i64_type(), length_ptr, "current_length")?
        .into_int_value();

    // Get data pointer
    let data_ptr_ptr =
        compiler
            .builder
            .build_struct_gep(array_struct_type, array_ptr, 0, "data_ptr_ptr")?;
    let data_ptr = compiler
        .builder
        .build_load(
            compiler.context.ptr_type(inkwell::AddressSpace::default()),
            data_ptr_ptr,
            "data_ptr",
        )?
        .into_pointer_value();

    // Calculate element address (arrays store pointers)
    let ptr_type = compiler.context.ptr_type(inkwell::AddressSpace::default());
    let element_ptr = unsafe {
        compiler
            .builder
            .build_gep(ptr_type, data_ptr, &[current_length], "element_ptr")?
    };

    // Store the value - Arrays use generic pointer storage
    // We store values as pointers for flexibility
    let value_to_store = if value.is_pointer_value() {
        // Already a pointer (e.g., strings, structs passed by reference)
        value.into_pointer_value()
    } else if value.is_int_value() {
        // For integers, allocate memory and store the value
        let int_val = value.into_int_value();
        let alloc_size = compiler.context.i64_type().const_int(8, false);
        let malloc_fn = compiler.module.get_function("malloc").ok_or_else(|| {
            CompileError::InternalError("No malloc function declared".to_string(), None)
        })?;
        let ptr = compiler
            .builder
            .build_call(malloc_fn, &[alloc_size.into()], "int_ptr")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| CompileError::InternalError("malloc failed".to_string(), None))?
            .into_pointer_value();

        // Store the integer value (extend to i64 if needed)
        let value_i64 = if int_val.get_type() == compiler.context.i64_type() {
            int_val
        } else {
            compiler.builder.build_int_s_extend(
                int_val,
                compiler.context.i64_type(),
                "value_i64",
            )?
        };
        compiler.builder.build_store(ptr, value_i64)?;
        ptr
    } else if value.is_float_value() {
        // For floats, allocate memory and store the value
        let float_val = value.into_float_value();
        let alloc_size = compiler.context.i64_type().const_int(8, false);
        let malloc_fn = compiler.module.get_function("malloc").ok_or_else(|| {
            CompileError::InternalError("No malloc function declared".to_string(), None)
        })?;
        let ptr = compiler
            .builder
            .build_call(malloc_fn, &[alloc_size.into()], "float_ptr")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| CompileError::InternalError("malloc failed".to_string(), None))?
            .into_pointer_value();
        compiler.builder.build_store(ptr, float_val)?;
        ptr
    } else if value.is_struct_value() {
        // For structs (like Option.Some), allocate memory and store the struct
        let struct_val = value.into_struct_value();

        // Calculate struct size (simplified - in reality need proper size calculation)
        // For now use a conservative estimate
        let alloc_size = compiler.context.i64_type().const_int(32, false); // Conservative size for small structs
        let malloc_fn = compiler.module.get_function("malloc").ok_or_else(|| {
            CompileError::InternalError("No malloc function declared".to_string(), None)
        })?;
        let ptr = compiler
            .builder
            .build_call(malloc_fn, &[alloc_size.into()], "struct_ptr")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| CompileError::InternalError("malloc failed".to_string(), None))?
            .into_pointer_value();

        // Store the struct value
        compiler.builder.build_store(ptr, struct_val)?;
        ptr
    } else {
        // Unknown value type - try to allocate and store as-is
        let alloc_size = compiler.context.i64_type().const_int(8, false);
        let malloc_fn = compiler.module.get_function("malloc").ok_or_else(|| {
            CompileError::InternalError("No malloc function declared".to_string(), None)
        })?;
        let ptr = compiler
            .builder
            .build_call(malloc_fn, &[alloc_size.into()], "value_ptr")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| CompileError::InternalError("malloc failed".to_string(), None))?
            .into_pointer_value();
        compiler.builder.build_store(ptr, value)?;
        ptr
    };

    compiler.builder.build_store(element_ptr, value_to_store)?;

    // Increment length
    let new_length = compiler.builder.build_int_add(
        current_length,
        compiler.context.i64_type().const_int(1, false),
        "new_length",
    )?;
    compiler.builder.build_store(length_ptr, new_length)?;

    // Return void/unit type for push
    Ok(compiler.context.struct_type(&[], false).const_zero().into())
}

/// Compile Array.push(value) - adds an element to the array
pub fn compile_array_push<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    array_val: BasicValueEnum<'ctx>,
    value: BasicValueEnum<'ctx>,
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    // Array struct type: { ptr, length, capacity, allocator }
    let array_struct_type = compiler.context.struct_type(
        &[
            compiler
                .context
                .ptr_type(inkwell::AddressSpace::default())
                .into(),
            compiler.context.i64_type().into(),
            compiler.context.i64_type().into(),
            compiler
                .context
                .ptr_type(inkwell::AddressSpace::default())
                .into(),
        ],
        false,
    );

    // Store array to get a pointer
    let array_ptr = compiler
        .builder
        .build_alloca(array_struct_type, "array_ptr")?;
    compiler.builder.build_store(array_ptr, array_val)?;

    // Get current length
    let length_ptr =
        compiler
            .builder
            .build_struct_gep(array_struct_type, array_ptr, 1, "length_ptr")?;
    let current_length = compiler
        .builder
        .build_load(compiler.context.i64_type(), length_ptr, "current_length")?
        .into_int_value();

    // Get capacity
    let capacity_ptr =
        compiler
            .builder
            .build_struct_gep(array_struct_type, array_ptr, 2, "capacity_ptr")?;
    let _capacity = compiler
        .builder
        .build_load(compiler.context.i64_type(), capacity_ptr, "capacity")?
        .into_int_value();

    // TODO: Check if we need to resize (for now, assume we have capacity)

    // Get data pointer
    let data_ptr_ptr =
        compiler
            .builder
            .build_struct_gep(array_struct_type, array_ptr, 0, "data_ptr_ptr")?;
    let data_ptr = compiler
        .builder
        .build_load(
            compiler.context.ptr_type(inkwell::AddressSpace::default()),
            data_ptr_ptr,
            "data_ptr",
        )?
        .into_pointer_value();

    // Calculate element address (arrays store pointers)
    let ptr_type = compiler.context.ptr_type(inkwell::AddressSpace::default());
    let element_ptr = unsafe {
        compiler
            .builder
            .build_gep(ptr_type, data_ptr, &[current_length], "element_ptr")?
    };

    // Store the value - Arrays use generic pointer storage
    // We store values as pointers for flexibility
    let value_to_store = if value.is_pointer_value() {
        // Already a pointer (e.g., strings, structs passed by reference)
        value.into_pointer_value()
    } else if value.is_int_value() {
        // For integers, allocate memory and store the value
        let int_val = value.into_int_value();
        let alloc_size = compiler.context.i64_type().const_int(8, false);
        let malloc_fn = compiler.module.get_function("malloc").ok_or_else(|| {
            CompileError::InternalError("No malloc function declared".to_string(), None)
        })?;
        let ptr = compiler
            .builder
            .build_call(malloc_fn, &[alloc_size.into()], "int_ptr")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| CompileError::InternalError("malloc failed".to_string(), None))?
            .into_pointer_value();

        // Store the integer value (extend to i64 if needed)
        let value_i64 = if int_val.get_type() == compiler.context.i64_type() {
            int_val
        } else {
            compiler.builder.build_int_s_extend(
                int_val,
                compiler.context.i64_type(),
                "value_i64",
            )?
        };
        compiler.builder.build_store(ptr, value_i64)?;
        ptr
    } else if value.is_float_value() {
        // For floats, allocate memory and store the value
        let float_val = value.into_float_value();
        let alloc_size = compiler.context.i64_type().const_int(8, false);
        let malloc_fn = compiler.module.get_function("malloc").ok_or_else(|| {
            CompileError::InternalError("No malloc function declared".to_string(), None)
        })?;
        let ptr = compiler
            .builder
            .build_call(malloc_fn, &[alloc_size.into()], "float_ptr")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| CompileError::InternalError("malloc failed".to_string(), None))?
            .into_pointer_value();
        compiler.builder.build_store(ptr, float_val)?;
        ptr
    } else if value.is_struct_value() {
        // For structs (like Option.Some), allocate memory and store the struct
        let struct_val = value.into_struct_value();

        // Calculate struct size (simplified - in reality need proper size calculation)
        // For now use a conservative estimate
        let alloc_size = compiler.context.i64_type().const_int(32, false); // Conservative size for small structs
        let malloc_fn = compiler.module.get_function("malloc").ok_or_else(|| {
            CompileError::InternalError("No malloc function declared".to_string(), None)
        })?;
        let ptr = compiler
            .builder
            .build_call(malloc_fn, &[alloc_size.into()], "struct_ptr")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| CompileError::InternalError("malloc failed".to_string(), None))?
            .into_pointer_value();

        // Store the struct value
        compiler.builder.build_store(ptr, struct_val)?;
        ptr
    } else {
        // Unknown value type - try to allocate and store as-is
        let alloc_size = compiler.context.i64_type().const_int(8, false);
        let malloc_fn = compiler.module.get_function("malloc").ok_or_else(|| {
            CompileError::InternalError("No malloc function declared".to_string(), None)
        })?;
        let ptr = compiler
            .builder
            .build_call(malloc_fn, &[alloc_size.into()], "value_ptr")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| CompileError::InternalError("malloc failed".to_string(), None))?
            .into_pointer_value();
        compiler.builder.build_store(ptr, value)?;
        ptr
    };

    compiler.builder.build_store(element_ptr, value_to_store)?;

    // Increment length
    let new_length = compiler.builder.build_int_add(
        current_length,
        compiler.context.i64_type().const_int(1, false),
        "new_length",
    )?;
    compiler.builder.build_store(length_ptr, new_length)?;

    // Return the array
    Ok(array_val)
}

/// Compile Array.get(index) - gets an element from the array
///
/// This function properly handles generic Array<T> by using the actual element type T.
pub fn compile_array_get<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    array_val: BasicValueEnum<'ctx>,
    index_val: BasicValueEnum<'ctx>,
    element_type: &ast::AstType,
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    // Set the generic type context for pattern matching with the actual element type
    compiler.track_generic_type("Option_Some_Type".to_string(), element_type.clone());

    // Array struct type: { ptr, length, capacity, allocator }
    let array_struct_type = compiler.context.struct_type(
        &[
            compiler
                .context
                .ptr_type(inkwell::AddressSpace::default())
                .into(),
            compiler.context.i64_type().into(),
            compiler.context.i64_type().into(),
            compiler
                .context
                .ptr_type(inkwell::AddressSpace::default())
                .into(),
        ],
        false,
    );

    // Option type: { discriminant: i64, payload: T }
    // Generate payload type based on actual Array<T> element type
    let element_llvm_type = compiler.to_llvm_type(element_type)?;
    let payload_type = match element_llvm_type {
        crate::codegen::llvm::Type::Basic(basic) => basic,
        crate::codegen::llvm::Type::Pointer(_) => {
            // Pointer types - use generic opaque pointer
            compiler.context.ptr_type(inkwell::AddressSpace::default()).into()
        }
        crate::codegen::llvm::Type::Struct(struct_type) => {
            // Struct types can be stored directly
            struct_type.into()
        }
        crate::codegen::llvm::Type::Function(_) => {
            return Err(CompileError::TypeError(
                "Cannot store function types in arrays".to_string(),
                None,
            ));
        }
        crate::codegen::llvm::Type::Void => {
            return Err(CompileError::TypeError(
                "Cannot store void types in arrays".to_string(),
                None,
            ));
        }
    };

    let option_type = compiler.context.struct_type(
        &[
            compiler.context.i64_type().into(), // discriminant
            payload_type.into(),                // payload (actual element type T)
        ],
        false,
    );

    // Use inline_counter for unique naming
    compiler.inline_counter += 1;
    let unique_id = compiler.inline_counter;

    // Store array to get a pointer
    let array_ptr = compiler
        .builder
        .build_alloca(array_struct_type, &format!("array_get_ptr_{}", unique_id))?;
    compiler.builder.build_store(array_ptr, array_val)?;

    // Get index as int
    let index = index_val.into_int_value();
    let index_i64 = if index.get_type() == compiler.context.i64_type() {
        index
    } else {
        compiler.builder.build_int_s_extend(
            index,
            compiler.context.i64_type(),
            &format!("get_index_i64_{}", unique_id),
        )?
    };

    // Get length to check bounds
    let length_ptr = compiler.builder.build_struct_gep(
        array_struct_type,
        array_ptr,
        1,
        &format!("get_length_ptr_{}", unique_id),
    )?;
    let length = compiler
        .builder
        .build_load(
            compiler.context.i64_type(),
            length_ptr,
            &format!("get_length_{}", unique_id),
        )?
        .into_int_value();

    // Check if index is within bounds (index >= 0 && index < length)
    let zero = compiler.context.i64_type().const_zero();
    let index_ge_zero = compiler.builder.build_int_compare(
        inkwell::IntPredicate::SGE,
        index_i64,
        zero,
        &format!("get_index_ge_zero_{}", unique_id),
    )?;
    let index_lt_len = compiler.builder.build_int_compare(
        inkwell::IntPredicate::SLT,
        index_i64,
        length,
        &format!("get_index_lt_len_{}", unique_id),
    )?;
    let in_bounds = compiler.builder.build_and(
        index_ge_zero,
        index_lt_len,
        &format!("get_in_bounds_{}", unique_id),
    )?;

    // Create blocks for in-bounds and out-of-bounds cases
    let current_fn = compiler
        .builder
        .get_insert_block()
        .unwrap()
        .get_parent()
        .unwrap();
    let in_bounds_bb = compiler
        .context
        .append_basic_block(current_fn, &format!("get_in_bounds_{}", unique_id));
    let out_bounds_bb = compiler
        .context
        .append_basic_block(current_fn, &format!("get_out_bounds_{}", unique_id));
    let merge_bb = compiler
        .context
        .append_basic_block(current_fn, &format!("get_merge_{}", unique_id));

    compiler
        .builder
        .build_conditional_branch(in_bounds, in_bounds_bb, out_bounds_bb)?;

    // In-bounds case: load value and return Some
    compiler.builder.position_at_end(in_bounds_bb);

    // Get data pointer
    let data_ptr_ptr = compiler.builder.build_struct_gep(
        array_struct_type,
        array_ptr,
        0,
        &format!("get_data_ptr_ptr_{}", unique_id),
    )?;
    let data_ptr = compiler
        .builder
        .build_load(
            compiler.context.ptr_type(inkwell::AddressSpace::default()),
            data_ptr_ptr,
            &format!("get_data_ptr_{}", unique_id),
        )?
        .into_pointer_value();

    // Calculate element address (arrays store pointers)
    let ptr_type = compiler.context.ptr_type(inkwell::AddressSpace::default());
    let element_ptr = unsafe {
        compiler.builder.build_gep(
            ptr_type,
            data_ptr,
            &[index_i64],
            &format!("get_elem_ptr_{}", unique_id),
        )?
    };

    // Load the pointer from the array
    let value_ptr = compiler
        .builder
        .build_load(
            ptr_type,
            element_ptr,
            &format!("get_elem_ptr_val_{}", unique_id),
        )?
        .into_pointer_value();

    // Load the actual value with the correct element type
    let value = compiler.builder.build_load(
        payload_type,
        value_ptr,
        &format!("get_elem_val_{}", unique_id),
    )?;

    // Create Some(value) - allocate and build struct
    let some_alloca = compiler
        .builder
        .build_alloca(option_type, &format!("some_alloca_{}", unique_id))?;
    let disc_ptr = compiler.builder.build_struct_gep(
        option_type,
        some_alloca,
        0,
        &format!("some_disc_ptr_{}", unique_id),
    )?;
    let payload_ptr = compiler.builder.build_struct_gep(
        option_type,
        some_alloca,
        1,
        &format!("some_payload_ptr_{}", unique_id),
    )?;

    compiler
        .builder
        .build_store(disc_ptr, compiler.context.i64_type().const_int(0, false))?; // 0 for Some
    compiler.builder.build_store(payload_ptr, value)?;

    let some_val = compiler.builder.build_load(
        option_type,
        some_alloca,
        &format!("some_val_{}", unique_id),
    )?;

    compiler.builder.build_unconditional_branch(merge_bb)?;
    let some_bb = compiler.builder.get_insert_block().unwrap();

    // Out-of-bounds case: return None
    compiler.builder.position_at_end(out_bounds_bb);

    let none_alloca = compiler
        .builder
        .build_alloca(option_type, &format!("none_alloca_{}", unique_id))?;
    let disc_ptr = compiler.builder.build_struct_gep(
        option_type,
        none_alloca,
        0,
        &format!("none_disc_ptr_{}", unique_id),
    )?;
    let payload_ptr = compiler.builder.build_struct_gep(
        option_type,
        none_alloca,
        1,
        &format!("none_payload_ptr_{}", unique_id),
    )?;

    compiler
        .builder
        .build_store(disc_ptr, compiler.context.i64_type().const_int(1, false))?; // 1 for None
    // Store a zero/null value for the payload with the correct type
    let zero_payload = if payload_type.is_int_type() {
        payload_type.into_int_type().const_zero().into()
    } else if payload_type.is_float_type() {
        payload_type.into_float_type().const_zero().into()
    } else if payload_type.is_pointer_type() {
        payload_type.into_pointer_type().const_null().into()
    } else {
        payload_type.const_zero()
    };
    compiler.builder.build_store(payload_ptr, zero_payload)?; // dummy payload

    let none_val = compiler.builder.build_load(
        option_type,
        none_alloca,
        &format!("none_val_{}", unique_id),
    )?;

    compiler.builder.build_unconditional_branch(merge_bb)?;
    let none_bb = compiler.builder.get_insert_block().unwrap();

    // Merge block: use PHI node
    compiler.builder.position_at_end(merge_bb);
    let phi = compiler
        .builder
        .build_phi(option_type, &format!("get_result_{}", unique_id))?;
    phi.add_incoming(&[(&some_val, some_bb), (&none_val, none_bb)]);

    Ok(phi.as_basic_value())
}

/// Compile Array.len() - returns the current length of the array
pub fn compile_array_len<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    array_val: BasicValueEnum<'ctx>,
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    // Array struct type: { ptr, length, capacity, allocator }
    let array_struct_type = compiler.context.struct_type(
        &[
            compiler
                .context
                .ptr_type(inkwell::AddressSpace::default())
                .into(),
            compiler.context.i64_type().into(),
            compiler.context.i64_type().into(),
            compiler
                .context
                .ptr_type(inkwell::AddressSpace::default())
                .into(),
        ],
        false,
    );

    // Use inline_counter for unique naming
    compiler.inline_counter += 1;
    let unique_id = compiler.inline_counter;

    // Store array to get a pointer
    let array_ptr = compiler
        .builder
        .build_alloca(array_struct_type, &format!("array_len_ptr_{}", unique_id))?;
    compiler.builder.build_store(array_ptr, array_val)?;

    // Get length field (index 1)
    let length_ptr = compiler.builder.build_struct_gep(
        array_struct_type,
        array_ptr,
        1,
        &format!("len_field_ptr_{}", unique_id),
    )?;

    let length = compiler.builder.build_load(
        compiler.context.i64_type(),
        length_ptr,
        &format!("len_value_{}", unique_id),
    )?;

    Ok(length)
}

/// Compile Array.set(index, value) - sets an element at the given index
pub fn compile_array_set<'ctx>(
    compiler: &mut LLVMCompiler,
    array_val: BasicValueEnum<'ctx>,
    index_val: BasicValueEnum<'ctx>,
    value: BasicValueEnum<'ctx>,
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    // Array struct type: { ptr, length, capacity, allocator }
    let array_struct_type = compiler.context.struct_type(
        &[
            compiler
                .context
                .ptr_type(inkwell::AddressSpace::default())
                .into(),
            compiler.context.i64_type().into(),
            compiler.context.i64_type().into(),
            compiler
                .context
                .ptr_type(inkwell::AddressSpace::default())
                .into(),
        ],
        false,
    );

    // Use inline_counter for unique naming
    compiler.inline_counter += 1;
    let unique_id = compiler.inline_counter;

    // Store array to get a pointer
    let array_ptr = compiler
        .builder
        .build_alloca(array_struct_type, &format!("array_set_ptr_{}", unique_id))?;
    compiler.builder.build_store(array_ptr, array_val)?;

    // Get index as int
    let index = index_val.into_int_value();
    let index_i64 = if index.get_type() == compiler.context.i64_type() {
        index
    } else {
        compiler.builder.build_int_s_extend(
            index,
            compiler.context.i64_type(),
            &format!("set_index_i64_{}", unique_id),
        )?
    };

    // Get data pointer
    let data_ptr_ptr = compiler.builder.build_struct_gep(
        array_struct_type,
        array_ptr,
        0,
        &format!("set_data_ptr_ptr_{}", unique_id),
    )?;
    let data_ptr = compiler
        .builder
        .build_load(
            compiler.context.ptr_type(inkwell::AddressSpace::default()),
            data_ptr_ptr,
            &format!("set_data_ptr_{}", unique_id),
        )?
        .into_pointer_value();

    // Calculate element address
    let element_ptr = unsafe {
        compiler.builder.build_gep(
            compiler.context.i64_type(),
            data_ptr,
            &[index_i64],
            &format!("set_elem_ptr_{}", unique_id),
        )?
    };

    // Store the value (convert to i64 if needed)
    let value_to_store = if value.is_int_value() {
        let int_val = value.into_int_value();
        if int_val.get_type() == compiler.context.i64_type() {
            int_val
        } else {
            compiler.builder.build_int_s_extend(
                int_val,
                compiler.context.i64_type(),
                &format!("set_value_i64_{}", unique_id),
            )?
        }
    } else {
        return Err(CompileError::TypeError(
            "Array.set currently only supports integer values".to_string(),
            None,
        ));
    };

    compiler.builder.build_store(element_ptr, value_to_store)?;

    // Return the array itself for chaining
    Ok(array_val)
}

/// Compile Array.pop() by pointer - modifies array in place and returns Option<T>
///
/// This function properly handles generic Array<T> by using the actual element type T.
pub fn compile_array_pop_by_ptr<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    array_ptr: PointerValue<'ctx>,
    element_type: &ast::AstType,
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    // Set the generic type context so pattern matching knows the actual Option<T> type
    compiler.track_generic_type("Option_Some_Type".to_string(), element_type.clone());
    // Array struct type: { ptr, length, capacity, allocator }
    let array_struct_type = compiler.context.struct_type(
        &[
            compiler
                .context
                .ptr_type(inkwell::AddressSpace::default())
                .into(),
            compiler.context.i64_type().into(),
            compiler.context.i64_type().into(),
            compiler
                .context
                .ptr_type(inkwell::AddressSpace::default())
                .into(),
        ],
        false,
    );

    // Option struct type: { discriminant: i64, payload: T }
    // Generate payload type based on actual Array<T> element type
    let element_llvm_type = compiler.to_llvm_type(element_type)?;
    let payload_type = match element_llvm_type {
        crate::codegen::llvm::Type::Basic(basic) => basic,
        crate::codegen::llvm::Type::Pointer(_) => {
            // Pointer types - use generic opaque pointer
            compiler.context.ptr_type(inkwell::AddressSpace::default()).into()
        }
        crate::codegen::llvm::Type::Struct(struct_type) => {
            // Struct types can be stored directly
            struct_type.into()
        }
        crate::codegen::llvm::Type::Function(_) => {
            return Err(CompileError::TypeError(
                "Cannot store function types in arrays".to_string(),
                None,
            ));
        }
        crate::codegen::llvm::Type::Void => {
            return Err(CompileError::TypeError(
                "Cannot store void types in arrays".to_string(),
                None,
            ));
        }
    };

    let option_type = compiler.context.struct_type(
        &[
            compiler.context.i64_type().into(),
            compiler
                .context
                .ptr_type(inkwell::AddressSpace::default())
                .into(), // We still use pointers for the Option payload internally
        ],
        false,
    );

    // Use inline_counter for unique naming
    compiler.inline_counter += 1;
    let unique_id = compiler.inline_counter;

    // Allocate space for the return value at function scope with the actual element type
    let value_ptr = compiler.builder.build_alloca(
        payload_type,
        &format!("pop_return_val_{}", unique_id),
    )?;

    // Get length field
    let length_ptr = compiler.builder.build_struct_gep(
        array_struct_type,
        array_ptr,
        1,
        &format!("pop_len_ptr_{}", unique_id),
    )?;
    let length = compiler
        .builder
        .build_load(
            compiler.context.i64_type(),
            length_ptr,
            &format!("pop_len_{}", unique_id),
        )?
        .into_int_value();

    // Check if array is empty
    let zero = compiler.context.i64_type().const_zero();
    let is_empty = compiler.builder.build_int_compare(
        inkwell::IntPredicate::EQ,
        length,
        zero,
        &format!("pop_is_empty_{}", unique_id),
    )?;

    // Create blocks for empty and non-empty cases
    let current_fn = compiler
        .builder
        .get_insert_block()
        .unwrap()
        .get_parent()
        .unwrap();
    let empty_bb = compiler
        .context
        .append_basic_block(current_fn, &format!("pop_empty_{}", unique_id));
    let nonempty_bb = compiler
        .context
        .append_basic_block(current_fn, &format!("pop_nonempty_{}", unique_id));
    let merge_bb = compiler
        .context
        .append_basic_block(current_fn, &format!("pop_merge_{}", unique_id));

    compiler
        .builder
        .build_conditional_branch(is_empty, empty_bb, nonempty_bb)?;

    // Empty case: return None
    compiler.builder.position_at_end(empty_bb);

    // Store a dummy zero value to value_ptr in the None case with the correct type
    let zero_value = if payload_type.is_int_type() {
        payload_type.into_int_type().const_zero().into()
    } else if payload_type.is_float_type() {
        payload_type.into_float_type().const_zero().into()
    } else if payload_type.is_pointer_type() {
        payload_type.into_pointer_type().const_null().into()
    } else {
        payload_type.const_zero()
    };
    compiler.builder.build_store(value_ptr, zero_value)?;

    let none_val = {
        let discriminant = compiler.context.i64_type().const_int(1, false); // 1 for None (matching registration)
        let null_ptr = compiler
            .context
            .ptr_type(inkwell::AddressSpace::default())
            .const_null();
        option_type.const_named_struct(&[discriminant.into(), null_ptr.into()])
    };
    compiler.builder.build_unconditional_branch(merge_bb)?;

    // Non-empty case: get last element and return Some(value)
    compiler.builder.position_at_end(nonempty_bb);

    // Decrement length AFTER getting the element
    let one = compiler.context.i64_type().const_int(1, false);
    let new_length =
        compiler
            .builder
            .build_int_sub(length, one, &format!("pop_new_len_{}", unique_id))?;

    // Get data pointer
    let data_ptr_ptr = compiler.builder.build_struct_gep(
        array_struct_type,
        array_ptr,
        0,
        &format!("pop_data_ptr_ptr_{}", unique_id),
    )?;
    let data_ptr = compiler
        .builder
        .build_load(
            compiler.context.ptr_type(inkwell::AddressSpace::default()),
            data_ptr_ptr,
            &format!("pop_data_ptr_{}", unique_id),
        )?
        .into_pointer_value();

    // Get element at new_length position (which is the last element, since we haven't decremented length yet)
    // Arrays store pointers, so we need to get the pointer at this index
    let ptr_type = compiler.context.ptr_type(inkwell::AddressSpace::default());
    let element_ptr_ptr = unsafe {
        compiler.builder.build_gep(
            ptr_type,
            data_ptr,
            &[new_length], // This is now correct - it points to the last element
            &format!("pop_elem_ptr_ptr_{}", unique_id),
        )?
    };

    // NOW store the new length after we've gotten the element
    compiler.builder.build_store(length_ptr, new_length)?;

    // Load the pointer to the actual value from the array
    let element_value_ptr = compiler.builder.build_load(
        ptr_type,
        element_ptr_ptr,
        &format!("pop_elem_val_ptr_{}", unique_id),
    )?.into_pointer_value();

    // Load the actual value with the correct element type
    let element_value = compiler.builder.build_load(
        payload_type,
        element_value_ptr,
        &format!("pop_val_{}", unique_id),
    )?;

    // Store the value to the pre-allocated pointer (allocated at function scope)
    compiler.builder.build_store(value_ptr, element_value)?;

    // Create Some(value) - following the same pattern as compile_enum_variant
    let some_alloca = compiler
        .builder
        .build_alloca(option_type, &format!("pop_some_alloca_{}", unique_id))?;

    // Store discriminant - Some is 0 (matching registration)
    let tag_ptr = compiler.builder.build_struct_gep(
        option_type,
        some_alloca,
        0,
        &format!("pop_tag_ptr_{}", unique_id),
    )?;
    compiler
        .builder
        .build_store(tag_ptr, compiler.context.i64_type().const_zero())?;

    // Store payload pointer
    let payload_ptr = compiler.builder.build_struct_gep(
        option_type,
        some_alloca,
        1,
        &format!("pop_payload_ptr_{}", unique_id),
    )?;
    compiler.builder.build_store(payload_ptr, value_ptr)?;

    // Load the struct
    let some_val = compiler.builder.build_load(
        option_type,
        some_alloca,
        &format!("pop_some_val_{}", unique_id),
    )?;
    compiler.builder.build_unconditional_branch(merge_bb)?;

    // Merge block
    compiler.builder.position_at_end(merge_bb);
    let phi = compiler
        .builder
        .build_phi(option_type, &format!("pop_result_{}", unique_id))?;
    phi.add_incoming(&[(&none_val, empty_bb), (&some_val, nonempty_bb)]);

    Ok(phi.as_basic_value())
}

/// Compile Array.pop() - removes and returns the last element as Option<T>
/// This version takes a value and creates a temporary pointer
pub fn compile_array_pop<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    array_val: BasicValueEnum<'ctx>,
    element_type: &ast::AstType,
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    // Array struct type: { ptr, length, capacity, allocator }
    let array_struct_type = compiler.context.struct_type(
        &[
            compiler
                .context
                .ptr_type(inkwell::AddressSpace::default())
                .into(),
            compiler.context.i64_type().into(),
            compiler.context.i64_type().into(),
            compiler
                .context
                .ptr_type(inkwell::AddressSpace::default())
                .into(),
        ],
        false,
    );

    // Store array to get a pointer
    let array_ptr = compiler
        .builder
        .build_alloca(array_struct_type, "array_pop_temp")?;
    compiler.builder.build_store(array_ptr, array_val)?;

    // Call the pointer version
    compile_array_pop_by_ptr(compiler, array_ptr, element_type)
}
