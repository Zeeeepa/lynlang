//! Collections codegen - HashMap, HashSet, DynVec constructors

use crate::codegen::llvm::LLVMCompiler;
use crate::ast;
use crate::error::CompileError;
use inkwell::module::Linkage;
use inkwell::values::BasicValueEnum;

/// Compile HashMap.new() - creates a new HashMap with allocator
pub fn compile_hashmap_new<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    // HashMap REQUIRES an allocator per NO-GC design
    if args.is_empty() {
        return Err(CompileError::TypeError(
            "HashMap.new() requires an allocator argument for NO-GC memory management".to_string(),
            None,
        ));
    }

    // HashMap struct: { buckets_ptr, size, capacity, allocator_ptr }
    let ptr_type = compiler.context.ptr_type(inkwell::AddressSpace::default());
    let hashmap_struct_type = compiler.context.struct_type(
        &[
            ptr_type.into(),                    // buckets pointer
            compiler.context.i64_type().into(), // size
            compiler.context.i64_type().into(), // capacity
            ptr_type.into(),                    // allocator pointer
        ],
        false,
    );

    // Use provided allocator
    let allocator_ptr = compiler.compile_expression(&args[0])?;

    // Initial capacity
    let initial_capacity = compiler.context.i64_type().const_int(16, false);

    // Allocate buckets array
    let bucket_size = compiler.context.i64_type().const_int(32, false); // Each bucket is 32 bytes (for chaining)
    let total_size = compiler
        .builder
        .build_int_mul(initial_capacity, bucket_size, "total_size")?;

    // For now, always use malloc (allocator is just stored for future use)
    // TODO: Implement proper allocator interface
    let malloc_fn = compiler.module.get_function("malloc").unwrap_or_else(|| {
        let i64_type = compiler.context.i64_type();
        let fn_type = ptr_type.fn_type(&[i64_type.into()], false);
        compiler
            .module
            .add_function("malloc", fn_type, Some(Linkage::External))
    });
    let buckets_ptr = compiler
        .builder
        .build_call(malloc_fn, &[total_size.into()], "buckets")?
        .try_as_basic_value()
        .left()
        .ok_or_else(|| {
            CompileError::InternalError("malloc should return a pointer".to_string(), None)
        })?;

    // Initialize buckets to zero
    let memset_fn = compiler.module.get_function("memset").unwrap_or_else(|| {
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
            buckets_ptr.into(),
            compiler.context.i32_type().const_int(0, false).into(),
            total_size.into(),
        ],
        "memset_call",
    )?;

    // Create the HashMap struct
    let hashmap_alloca = compiler
        .builder
        .build_alloca(hashmap_struct_type, "hashmap")?;

    // Store buckets pointer
    let buckets_field = compiler.builder.build_struct_gep(
        hashmap_struct_type,
        hashmap_alloca,
        0,
        "buckets_field",
    )?;
    compiler.builder.build_store(buckets_field, buckets_ptr)?;

    // Store size (initially 0)
    let size_field =
        compiler
            .builder
            .build_struct_gep(hashmap_struct_type, hashmap_alloca, 1, "size_field")?;
    compiler
        .builder
        .build_store(size_field, compiler.context.i64_type().const_int(0, false))?;

    // Store capacity
    let capacity_field = compiler.builder.build_struct_gep(
        hashmap_struct_type,
        hashmap_alloca,
        2,
        "capacity_field",
    )?;
    compiler
        .builder
        .build_store(capacity_field, initial_capacity)?;

    // Load and return the hashmap struct
    // Store allocator pointer
    let allocator_field = compiler.builder.build_struct_gep(
        hashmap_struct_type,
        hashmap_alloca,
        3,
        "allocator_field",
    )?;
    compiler
        .builder
        .build_store(allocator_field, allocator_ptr)?;

    let result =
        compiler
            .builder
            .build_load(hashmap_struct_type, hashmap_alloca, "hashmap_value")?;
    Ok(result)
}

/// Compile HashMap.insert() - inserts a key-value pair
pub fn compile_hashmap_insert<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    hashmap_ptr: inkwell::values::PointerValue<'ctx>,
    key: BasicValueEnum<'ctx>,
    value: BasicValueEnum<'ctx>,
    key_type: &crate::ast::AstType,
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    // HashMap struct: { buckets_ptr, size, capacity, allocator_ptr }
    let ptr_type = compiler.context.ptr_type(inkwell::AddressSpace::default());
    let hashmap_struct_type = compiler.context.struct_type(
        &[
            ptr_type.into(),
            compiler.context.i64_type().into(),
            compiler.context.i64_type().into(),
            ptr_type.into(),
        ],
        false,
    );

    // Get buckets pointer and capacity
    let buckets_ptr_field = compiler.builder.build_struct_gep(
        hashmap_struct_type,
        hashmap_ptr,
        0,
        "buckets_ptr_field",
    )?;
    let buckets_ptr = compiler
        .builder
        .build_load(ptr_type, buckets_ptr_field, "buckets_ptr")?
        .into_pointer_value();

    let capacity_field = compiler.builder.build_struct_gep(
        hashmap_struct_type,
        hashmap_ptr,
        2,
        "capacity_field",
    )?;
    let capacity = compiler
        .builder
        .build_load(compiler.context.i64_type(), capacity_field, "capacity")?
        .into_int_value();

    // Compute hash of key (simple implementation using key value directly for integers)
    let hash = match key {
        BasicValueEnum::IntValue(int_val) => {
            // For integers, use the value directly as hash
            compiler.builder.build_int_z_extend(int_val, compiler.context.i64_type(), "hash")?
        }
        _ => {
            // For other types, use a simple pointer hash
            let key_as_int = compiler.builder.build_ptr_to_int(
                key.into_pointer_value(),
                compiler.context.i64_type(),
                "key_hash",
            )?;
            key_as_int
        }
    };

    // Compute bucket index: hash % capacity
    let bucket_index = compiler.builder.build_int_unsigned_rem(hash, capacity, "bucket_index")?;

    // Calculate bucket address
    // Each bucket is 32 bytes: { key (8), value (8), occupied (1), padding (15) }
    let bucket_size = compiler.context.i64_type().const_int(32, false);
    let bucket_offset = compiler.builder.build_int_mul(bucket_index, bucket_size, "bucket_offset")?;
    let bucket_addr = unsafe {
        compiler.builder.build_gep(
            compiler.context.i8_type(),
            buckets_ptr,
            &[bucket_offset],
            "bucket_addr",
        )?
    };

    // Store key at offset 0
    let key_ptr = bucket_addr;
    let key_to_store = match key {
        BasicValueEnum::IntValue(int_val) => {
            compiler.builder.build_int_z_extend(int_val, compiler.context.i64_type(), "key_i64")?
        }
        _ => {
            // For other types, store as pointer
            return Err(CompileError::UnsupportedFeature(
                "HashMap only supports integer keys currently".to_string(),
                None,
            ));
        }
    };
    compiler.builder.build_store(key_ptr, key_to_store)?;

    // Store value at offset 8
    let value_ptr = unsafe {
        compiler.builder.build_gep(
            compiler.context.i8_type(),
            bucket_addr,
            &[compiler.context.i64_type().const_int(8, false)],
            "value_ptr",
        )?
    };
    let value_to_store = match value {
        BasicValueEnum::IntValue(int_val) => {
            compiler.builder.build_int_z_extend(int_val, compiler.context.i64_type(), "value_i64")?
        }
        _ => {
            return Err(CompileError::UnsupportedFeature(
                "HashMap only supports integer values currently".to_string(),
                None,
            ));
        }
    };
    compiler.builder.build_store(value_ptr, value_to_store)?;

    // Set occupied flag at offset 16
    let occupied_ptr = unsafe {
        compiler.builder.build_gep(
            compiler.context.i8_type(),
            bucket_addr,
            &[compiler.context.i64_type().const_int(16, false)],
            "occupied_ptr",
        )?
    };
    compiler.builder.build_store(occupied_ptr, compiler.context.i8_type().const_int(1, false))?;

    // Increment size
    let size_field = compiler.builder.build_struct_gep(
        hashmap_struct_type,
        hashmap_ptr,
        1,
        "size_field",
    )?;
    let current_size = compiler
        .builder
        .build_load(compiler.context.i64_type(), size_field, "current_size")?
        .into_int_value();
    let new_size = compiler.builder.build_int_add(
        current_size,
        compiler.context.i64_type().const_int(1, false),
        "new_size",
    )?;
    compiler.builder.build_store(size_field, new_size)?;

    // Return void (as a dummy i64 value of 0)
    let _ = key_type; // Suppress warning
    Ok(compiler.context.i64_type().const_int(0, false).into())
}

/// Compile HashMap.get() - gets a value by key, returns Option<V>
pub fn compile_hashmap_get<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    hashmap_ptr: inkwell::values::PointerValue<'ctx>,
    key: BasicValueEnum<'ctx>,
    value_type: &crate::ast::AstType,
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    // HashMap struct: { buckets_ptr, size, capacity, allocator_ptr }
    let ptr_type = compiler.context.ptr_type(inkwell::AddressSpace::default());
    let hashmap_struct_type = compiler.context.struct_type(
        &[
            ptr_type.into(),
            compiler.context.i64_type().into(),
            compiler.context.i64_type().into(),
            ptr_type.into(),
        ],
        false,
    );

    // Get buckets pointer and capacity
    let buckets_ptr_field = compiler.builder.build_struct_gep(
        hashmap_struct_type,
        hashmap_ptr,
        0,
        "buckets_ptr_field",
    )?;
    let buckets_ptr = compiler
        .builder
        .build_load(ptr_type, buckets_ptr_field, "buckets_ptr")?
        .into_pointer_value();

    let capacity_field = compiler.builder.build_struct_gep(
        hashmap_struct_type,
        hashmap_ptr,
        2,
        "capacity_field",
    )?;
    let capacity = compiler
        .builder
        .build_load(compiler.context.i64_type(), capacity_field, "capacity")?
        .into_int_value();

    // Compute hash of key
    let hash = match key {
        BasicValueEnum::IntValue(int_val) => {
            compiler.builder.build_int_z_extend(int_val, compiler.context.i64_type(), "hash")?
        }
        _ => {
            return Err(CompileError::UnsupportedFeature(
                "HashMap only supports integer keys currently".to_string(),
                None,
            ));
        }
    };

    // Compute bucket index: hash % capacity
    let bucket_index = compiler.builder.build_int_unsigned_rem(hash, capacity, "bucket_index")?;

    // Calculate bucket address
    let bucket_size = compiler.context.i64_type().const_int(32, false);
    let bucket_offset = compiler.builder.build_int_mul(bucket_index, bucket_size, "bucket_offset")?;
    let bucket_addr = unsafe {
        compiler.builder.build_gep(
            compiler.context.i8_type(),
            buckets_ptr,
            &[bucket_offset],
            "bucket_addr",
        )?
    };

    // Check occupied flag at offset 16
    let occupied_ptr = unsafe {
        compiler.builder.build_gep(
            compiler.context.i8_type(),
            bucket_addr,
            &[compiler.context.i64_type().const_int(16, false)],
            "occupied_ptr",
        )?
    };
    let occupied = compiler
        .builder
        .build_load(compiler.context.i8_type(), occupied_ptr, "occupied")?
        .into_int_value();
    let is_occupied = compiler.builder.build_int_compare(
        inkwell::IntPredicate::NE,
        occupied,
        compiler.context.i8_type().const_int(0, false),
        "is_occupied",
    )?;

    // Build Option result
    // Option struct: { tag (i64), payload_ptr (ptr) }
    let option_struct_type = compiler.context.struct_type(
        &[
            compiler.context.i64_type().into(),
            ptr_type.into(),
        ],
        false,
    );
    let option_alloca = compiler.builder.build_alloca(option_struct_type, "option_result")?;

    // Create basic blocks
    let current_fn = compiler.builder.get_insert_block().unwrap().get_parent().unwrap();
    let found_block = compiler.context.append_basic_block(current_fn, "found");
    let not_found_block = compiler.context.append_basic_block(current_fn, "not_found");
    let merge_block = compiler.context.append_basic_block(current_fn, "merge");

    compiler.builder.build_conditional_branch(is_occupied, found_block, not_found_block)?;

    // Found block - return Some(value)
    compiler.builder.position_at_end(found_block);
    let value_ptr = unsafe {
        compiler.builder.build_gep(
            compiler.context.i8_type(),
            bucket_addr,
            &[compiler.context.i64_type().const_int(8, false)],
            "value_ptr",
        )?
    };
    let stored_value = compiler
        .builder
        .build_load(compiler.context.i64_type(), value_ptr, "stored_value")?;

    // Set tag to 0 (Some)
    let tag_field = compiler.builder.build_struct_gep(option_struct_type, option_alloca, 0, "tag_field")?;
    compiler.builder.build_store(tag_field, compiler.context.i64_type().const_int(0, false))?;

    // Allocate payload and store value
    let payload_alloca = compiler.builder.build_alloca(compiler.context.i64_type(), "payload")?;
    compiler.builder.build_store(payload_alloca, stored_value)?;
    let payload_field = compiler.builder.build_struct_gep(option_struct_type, option_alloca, 1, "payload_field")?;
    compiler.builder.build_store(payload_field, payload_alloca)?;
    compiler.builder.build_unconditional_branch(merge_block)?;

    // Not found block - return None
    compiler.builder.position_at_end(not_found_block);
    let tag_field_none = compiler.builder.build_struct_gep(option_struct_type, option_alloca, 0, "tag_field_none")?;
    compiler.builder.build_store(tag_field_none, compiler.context.i64_type().const_int(1, false))?;
    let payload_field_none = compiler.builder.build_struct_gep(option_struct_type, option_alloca, 1, "payload_field_none")?;
    let null_ptr = ptr_type.const_null();
    compiler.builder.build_store(payload_field_none, null_ptr)?;
    compiler.builder.build_unconditional_branch(merge_block)?;

    // Merge block
    compiler.builder.position_at_end(merge_block);
    let result = compiler.builder.build_load(option_struct_type, option_alloca, "option_result")?;

    let _ = value_type; // Suppress warning
    Ok(result)
}

/// Compile HashSet.new() - creates a new HashSet
pub fn compile_hashset_new<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    // HashSet REQUIRES an allocator per NO-GC design
    if args.is_empty() {
        return Err(CompileError::TypeError(
            "HashSet.new() requires an allocator argument for NO-GC memory management".to_string(),
            None,
        ));
    }

    // HashSet uses the same structure as HashMap (but without values)
    // HashSet struct: { buckets_ptr, size, capacity, allocator_ptr }
    let ptr_type = compiler.context.ptr_type(inkwell::AddressSpace::default());
    let hashset_struct_type = compiler.context.struct_type(
        &[
            ptr_type.into(),                    // buckets pointer
            compiler.context.i64_type().into(), // size
            compiler.context.i64_type().into(), // capacity
            ptr_type.into(),                    // allocator pointer
        ],
        false,
    );

    // Use provided allocator
    let allocator_ptr = compiler.compile_expression(&args[0])?;

    // Initial capacity
    let initial_capacity = compiler.context.i64_type().const_int(16, false);

    // Allocate buckets array
    let bucket_size = compiler.context.i64_type().const_int(16, false); // Each bucket is 16 bytes (just key + next pointer)
    let total_size = compiler
        .builder
        .build_int_mul(initial_capacity, bucket_size, "total_size")?;

    // Call malloc
    let malloc_fn = compiler.module.get_function("malloc").unwrap_or_else(|| {
        let i64_type = compiler.context.i64_type();
        let fn_type = ptr_type.fn_type(&[i64_type.into()], false);
        compiler
            .module
            .add_function("malloc", fn_type, Some(Linkage::External))
    });

    let buckets_ptr = compiler
        .builder
        .build_call(malloc_fn, &[total_size.into()], "buckets")?
        .try_as_basic_value()
        .left()
        .ok_or_else(|| {
            CompileError::InternalError("malloc should return a pointer".to_string(), None)
        })?;

    // Initialize buckets to zero
    let memset_fn = compiler.module.get_function("memset").unwrap_or_else(|| {
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
            buckets_ptr.into(),
            compiler.context.i32_type().const_int(0, false).into(),
            total_size.into(),
        ],
        "memset_call",
    )?;

    // Create the HashSet struct
    let hashset_alloca = compiler
        .builder
        .build_alloca(hashset_struct_type, "hashset")?;

    // Store buckets pointer
    let buckets_field = compiler.builder.build_struct_gep(
        hashset_struct_type,
        hashset_alloca,
        0,
        "buckets_field",
    )?;
    compiler.builder.build_store(buckets_field, buckets_ptr)?;

    // Store size (initially 0)
    let size_field =
        compiler
            .builder
            .build_struct_gep(hashset_struct_type, hashset_alloca, 1, "size_field")?;
    compiler
        .builder
        .build_store(size_field, compiler.context.i64_type().const_int(0, false))?;

    // Store capacity
    let capacity_field = compiler.builder.build_struct_gep(
        hashset_struct_type,
        hashset_alloca,
        2,
        "capacity_field",
    )?;
    compiler
        .builder
        .build_store(capacity_field, initial_capacity)?;

    // Store allocator pointer
    let allocator_field = compiler.builder.build_struct_gep(
        hashset_struct_type,
        hashset_alloca,
        3,
        "allocator_field",
    )?;
    compiler
        .builder
        .build_store(allocator_field, allocator_ptr)?;

    // Load and return the hashset struct
    let result =
        compiler
            .builder
            .build_load(hashset_struct_type, hashset_alloca, "hashset_value")?;
    Ok(result)
}

/// Compile DynVec.new() - creates a new DynVec with allocator
pub fn compile_dynvec_new<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    // DynVec REQUIRES an allocator per NO-GC design
    if args.is_empty() {
        return Err(CompileError::TypeError(
            "DynVec.new() requires an allocator argument for NO-GC memory management".to_string(),
            None,
        ));
    }

    // DynVec struct: { ptr, length, capacity, allocator }
    let ptr_type = compiler.context.ptr_type(inkwell::AddressSpace::default());
    let dynvec_struct_type = compiler.context.struct_type(
        &[
            ptr_type.into(),                    // data pointer
            compiler.context.i64_type().into(), // length
            compiler.context.i64_type().into(), // capacity
            ptr_type.into(),                    // allocator pointer
        ],
        false,
    );

    // Use provided allocator
    let allocator_ptr = compiler.compile_expression(&args[0])?;

    // Initial capacity
    let initial_capacity = compiler.context.i64_type().const_int(10, false);

    // Allocate memory for initial capacity (8 bytes per element for i64)
    let element_size = compiler.context.i64_type().const_int(8, false);
    let total_size =
        compiler
            .builder
            .build_int_mul(initial_capacity, element_size, "total_size")?;

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
        .build_call(malloc_fn, &[total_size.into()], "dynvec_data")?
        .try_as_basic_value()
        .left()
        .ok_or_else(|| {
            CompileError::InternalError("malloc should return a pointer".to_string(), None)
        })?;

    // Create the DynVec struct
    let dynvec_alloca = compiler
        .builder
        .build_alloca(dynvec_struct_type, "dynvec")?;

    // Store data pointer
    let data_field =
        compiler
            .builder
            .build_struct_gep(dynvec_struct_type, dynvec_alloca, 0, "data_field")?;
    compiler.builder.build_store(data_field, data_ptr)?;

    // Store length (initially 0)
    let length_field =
        compiler
            .builder
            .build_struct_gep(dynvec_struct_type, dynvec_alloca, 1, "length_field")?;
    compiler.builder.build_store(
        length_field,
        compiler.context.i64_type().const_int(0, false),
    )?;

    // Store capacity
    let capacity_field = compiler.builder.build_struct_gep(
        dynvec_struct_type,
        dynvec_alloca,
        2,
        "capacity_field",
    )?;
    compiler
        .builder
        .build_store(capacity_field, initial_capacity)?;

    // Store allocator pointer
    let allocator_field = compiler.builder.build_struct_gep(
        dynvec_struct_type,
        dynvec_alloca,
        3,
        "allocator_field",
    )?;
    compiler
        .builder
        .build_store(allocator_field, allocator_ptr)?;

    // Load and return the dynvec struct
    let result = compiler
        .builder
        .build_load(dynvec_struct_type, dynvec_alloca, "dynvec_value")?;
    Ok(result)
}
