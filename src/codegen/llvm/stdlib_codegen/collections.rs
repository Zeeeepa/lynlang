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
