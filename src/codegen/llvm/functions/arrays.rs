use crate::ast;
use crate::codegen::llvm::LLVMCompiler;
use crate::error::CompileError;
use inkwell::module::Linkage;
use inkwell::types::{BasicTypeEnum, StructType};
use inkwell::values::{BasicValueEnum, PointerValue};
use inkwell::AddressSpace;

// --- Type Helpers ---

/// Get the standard Array struct type: { ptr, length, capacity, allocator }
fn array_struct_type<'ctx>(compiler: &LLVMCompiler<'ctx>) -> StructType<'ctx> {
    let ptr_type = compiler.context.ptr_type(AddressSpace::default());
    compiler.context.struct_type(
        &[
            ptr_type.into(),
            compiler.context.i64_type().into(),
            compiler.context.i64_type().into(),
            ptr_type.into(),
        ],
        false,
    )
}

/// Get Option struct type: { discriminant: i64, payload: T }
fn option_struct_type<'ctx>(
    compiler: &LLVMCompiler<'ctx>,
    payload_type: BasicTypeEnum<'ctx>,
) -> StructType<'ctx> {
    compiler.context.struct_type(
        &[compiler.context.i64_type().into(), payload_type],
        false,
    )
}

/// Convert AstType to LLVM BasicTypeEnum for use as array element/Option payload
fn ast_to_payload_type<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    element_type: &ast::AstType,
) -> Result<BasicTypeEnum<'ctx>, CompileError> {
    let llvm_type = compiler.to_llvm_type(element_type)?;
    match llvm_type {
        crate::codegen::llvm::Type::Basic(basic) => Ok(basic),
        crate::codegen::llvm::Type::Pointer(_) => {
            Ok(compiler.context.ptr_type(AddressSpace::default()).into())
        }
        crate::codegen::llvm::Type::Struct(s) => Ok(s.into()),
        crate::codegen::llvm::Type::Function(_) => Err(CompileError::TypeError(
            "Cannot store function types in arrays".to_string(),
            None,
        )),
        crate::codegen::llvm::Type::Void => Err(CompileError::TypeError(
            "Cannot store void types in arrays".to_string(),
            None,
        )),
    }
}

/// Create a zero value for a given type (for None payloads, etc.)
fn zero_value<'ctx>(
    _compiler: &LLVMCompiler<'ctx>,
    ty: BasicTypeEnum<'ctx>,
) -> BasicValueEnum<'ctx> {
    if ty.is_int_type() {
        ty.into_int_type().const_zero().into()
    } else if ty.is_float_type() {
        ty.into_float_type().const_zero().into()
    } else if ty.is_pointer_type() {
        ty.into_pointer_type().const_null().into()
    } else {
        ty.const_zero()
    }
}

// --- Memory Helpers ---

fn get_or_declare_malloc<'ctx>(compiler: &mut LLVMCompiler<'ctx>) -> inkwell::values::FunctionValue<'ctx> {
    compiler.module.get_function("malloc").unwrap_or_else(|| {
        let ptr_type = compiler.context.ptr_type(AddressSpace::default());
        let fn_type = ptr_type.fn_type(&[compiler.context.i64_type().into()], false);
        compiler.module.add_function("malloc", fn_type, Some(Linkage::External))
    })
}

fn get_or_declare_memset<'ctx>(compiler: &mut LLVMCompiler<'ctx>) -> inkwell::values::FunctionValue<'ctx> {
    compiler.module.get_function("memset").unwrap_or_else(|| {
        let ptr_type = compiler.context.ptr_type(AddressSpace::default());
        let fn_type = ptr_type.fn_type(
            &[ptr_type.into(), compiler.context.i32_type().into(), compiler.context.i64_type().into()],
            false,
        );
        compiler.module.add_function("memset", fn_type, Some(Linkage::External))
    })
}

/// Allocate memory using malloc
fn malloc<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    size: inkwell::values::IntValue<'ctx>,
    name: &str,
) -> Result<PointerValue<'ctx>, CompileError> {
    let malloc_fn = get_or_declare_malloc(compiler);
    compiler
        .builder
        .build_call(malloc_fn, &[size.into()], name)?
        .try_as_basic_value()
        .left()
        .map(|v| v.into_pointer_value())
        .ok_or_else(|| CompileError::InternalError("malloc returned void".to_string(), compiler.get_current_span()))
}

/// Box a value for storage in an array (allocates heap memory and stores value)
fn box_value_for_array<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    value: BasicValueEnum<'ctx>,
) -> Result<PointerValue<'ctx>, CompileError> {
    if value.is_pointer_value() {
        return Ok(value.into_pointer_value());
    }

    let (alloc_size, store_value): (u64, BasicValueEnum) = if value.is_int_value() {
        let int_val = value.into_int_value();
        let val_i64 = if int_val.get_type() == compiler.context.i64_type() {
            int_val
        } else {
            compiler.builder.build_int_s_extend(int_val, compiler.context.i64_type(), "box_i64")?
        };
        (8, val_i64.into())
    } else if value.is_float_value() {
        (8, value)
    } else if value.is_struct_value() {
        (32, value) // Conservative size for small structs
    } else {
        (8, value)
    };

    let size = compiler.context.i64_type().const_int(alloc_size, false);
    let ptr = malloc(compiler, size, "boxed_val")?;
    compiler.builder.build_store(ptr, store_value)?;
    Ok(ptr)
}

// --- Array Field Access ---

struct ArrayFields<'ctx> {
    data_ptr: PointerValue<'ctx>,
    length: inkwell::values::IntValue<'ctx>,
    length_ptr: PointerValue<'ctx>,
}

fn get_array_fields<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    array_ptr: PointerValue<'ctx>,
    suffix: &str,
) -> Result<ArrayFields<'ctx>, CompileError> {
    let struct_type = array_struct_type(compiler);
    let ptr_type = compiler.context.ptr_type(AddressSpace::default());

    let data_ptr_ptr = compiler.builder.build_struct_gep(struct_type, array_ptr, 0, &format!("data_ptr_{}", suffix))?;
    let data_ptr = compiler.builder.build_load(ptr_type, data_ptr_ptr, &format!("data_{}", suffix))?.into_pointer_value();

    let length_ptr = compiler.builder.build_struct_gep(struct_type, array_ptr, 1, &format!("len_ptr_{}", suffix))?;
    let length = compiler.builder.build_load(compiler.context.i64_type(), length_ptr, &format!("len_{}", suffix))?.into_int_value();

    Ok(ArrayFields { data_ptr, length, length_ptr })
}

// --- Public API ---

pub fn is_allocator_type(_compiler: &LLVMCompiler, _expr: &ast::Expression) -> bool {
    false // TODO: proper type checking
}

/// Compile Array.new(allocator, capacity?, default_value?)
pub fn compile_array_new<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    if args.is_empty() {
        return Err(CompileError::TypeError(
            "Array.new() requires an allocator argument for NO-GC memory management".to_string(),
            None,
        ));
    }

    let struct_type = array_struct_type(compiler);
    let allocator_ptr = compiler.compile_expression(&args[0])?;
    let remaining = &args[1..];

    // Determine capacity
    let capacity = if remaining.is_empty() {
        compiler.context.i64_type().const_int(10, false)
    } else if remaining.len() == 2 {
        let cap_val = compiler.compile_expression(&remaining[0])?.into_int_value();
        if cap_val.get_type() == compiler.context.i64_type() {
            cap_val
        } else {
            compiler.builder.build_int_z_extend(cap_val, compiler.context.i64_type(), "cap_i64")?
        }
    } else {
        return Err(CompileError::TypeError(
            format!("Array.new expects allocator, capacity, and default value. Got {} args", args.len()),
            None,
        ));
    };

    // Compile default value if provided
    if remaining.len() == 2 {
        let _default_val = compiler.compile_expression(&remaining[1])?;
    }

    // Allocate data buffer
    let element_size = compiler.context.i64_type().const_int(8, false);
    let total_size = compiler.builder.build_int_mul(capacity, element_size, "total_size")?;
    let data_ptr = malloc(compiler, total_size, "array_data")?;

    // Zero-initialize if we have capacity > default
    if remaining.len() == 2 {
        let memset_fn = get_or_declare_memset(compiler);
        compiler.builder.build_call(
            memset_fn,
            &[data_ptr.into(), compiler.context.i32_type().const_zero().into(), total_size.into()],
            "memset",
        )?;
    }

    // Build struct
    let array_alloca = compiler.builder.build_alloca(struct_type, "array")?;
    let fields: [(u32, BasicValueEnum); 4] = [
        (0, data_ptr.into()),
        (1, compiler.context.i64_type().const_zero().into()),
        (2, capacity.into()),
        (3, allocator_ptr),
    ];
    for (idx, val) in fields {
        let ptr = compiler.builder.build_struct_gep(struct_type, array_alloca, idx, "")?;
        compiler.builder.build_store(ptr, val)?;
    }

    Ok(compiler.builder.build_load(struct_type, array_alloca, "array_val")?)
}

/// Compile Array.push by pointer (modifies in place)
pub fn compile_array_push_by_ptr<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    array_ptr: PointerValue<'ctx>,
    value: BasicValueEnum<'ctx>,
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    let fields = get_array_fields(compiler, array_ptr, "push")?;
    let ptr_type = compiler.context.ptr_type(AddressSpace::default());

    // Calculate element address and store boxed value
    let element_ptr = unsafe {
        compiler.builder.build_gep(ptr_type, fields.data_ptr, &[fields.length], "elem_ptr")?
    };
    let boxed = box_value_for_array(compiler, value)?;
    compiler.builder.build_store(element_ptr, boxed)?;

    // Increment length
    let new_len = compiler.builder.build_int_add(fields.length, compiler.context.i64_type().const_int(1, false), "new_len")?;
    compiler.builder.build_store(fields.length_ptr, new_len)?;

    Ok(compiler.context.struct_type(&[], false).const_zero().into())
}

/// Compile Array.push (value version)
pub fn compile_array_push<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    array_val: BasicValueEnum<'ctx>,
    value: BasicValueEnum<'ctx>,
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    let struct_type = array_struct_type(compiler);
    let array_ptr = compiler.builder.build_alloca(struct_type, "array_ptr")?;
    compiler.builder.build_store(array_ptr, array_val)?;

    compile_array_push_by_ptr(compiler, array_ptr, value)?;
    Ok(array_val)
}

/// Compile Array.get(index) -> Option<T>
pub fn compile_array_get<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    array_val: BasicValueEnum<'ctx>,
    index_val: BasicValueEnum<'ctx>,
    element_type: &ast::AstType,
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    compiler.track_generic_type("Option_Some_Type".to_string(), element_type.clone());

    let struct_type = array_struct_type(compiler);
    let payload_type = ast_to_payload_type(compiler, element_type)?;
    let option_type = option_struct_type(compiler, payload_type);
    let ptr_type = compiler.context.ptr_type(AddressSpace::default());

    compiler.inline_counter += 1;
    let id = compiler.inline_counter;

    // Store array to get pointer
    let array_ptr = compiler.builder.build_alloca(struct_type, &format!("get_arr_{}", id))?;
    compiler.builder.build_store(array_ptr, array_val)?;

    // Get index as i64
    let index = index_val.into_int_value();
    let index_i64 = if index.get_type() == compiler.context.i64_type() {
        index
    } else {
        compiler.builder.build_int_s_extend(index, compiler.context.i64_type(), &format!("idx_{}", id))?
    };

    let fields = get_array_fields(compiler, array_ptr, &format!("get_{}", id))?;

    // Bounds check
    let zero = compiler.context.i64_type().const_zero();
    let ge_zero = compiler.builder.build_int_compare(inkwell::IntPredicate::SGE, index_i64, zero, "ge0")?;
    let lt_len = compiler.builder.build_int_compare(inkwell::IntPredicate::SLT, index_i64, fields.length, "ltlen")?;
    let in_bounds = compiler.builder.build_and(ge_zero, lt_len, "inbounds")?;

    let current_fn = compiler.builder.get_insert_block().unwrap().get_parent().unwrap();
    let in_bb = compiler.context.append_basic_block(current_fn, &format!("get_in_{}", id));
    let out_bb = compiler.context.append_basic_block(current_fn, &format!("get_out_{}", id));
    let merge_bb = compiler.context.append_basic_block(current_fn, &format!("get_merge_{}", id));

    compiler.builder.build_conditional_branch(in_bounds, in_bb, out_bb)?;

    // In-bounds: load value and return Some
    compiler.builder.position_at_end(in_bb);
    let elem_ptr = unsafe { compiler.builder.build_gep(ptr_type, fields.data_ptr, &[index_i64], "eptr")? };
    let val_ptr = compiler.builder.build_load(ptr_type, elem_ptr, "vptr")?.into_pointer_value();
    let value = compiler.builder.build_load(payload_type, val_ptr, "val")?;

    let some_alloca = compiler.builder.build_alloca(option_type, "some")?;
    compiler.builder.build_store(
        compiler.builder.build_struct_gep(option_type, some_alloca, 0, "")?,
        compiler.context.i64_type().const_zero(),
    )?;
    compiler.builder.build_store(
        compiler.builder.build_struct_gep(option_type, some_alloca, 1, "")?,
        value,
    )?;
    let some_val = compiler.builder.build_load(option_type, some_alloca, "some_v")?;
    compiler.builder.build_unconditional_branch(merge_bb)?;
    let some_bb = compiler.builder.get_insert_block().unwrap();

    // Out-of-bounds: return None
    compiler.builder.position_at_end(out_bb);
    let none_alloca = compiler.builder.build_alloca(option_type, "none")?;
    compiler.builder.build_store(
        compiler.builder.build_struct_gep(option_type, none_alloca, 0, "")?,
        compiler.context.i64_type().const_int(1, false),
    )?;
    compiler.builder.build_store(
        compiler.builder.build_struct_gep(option_type, none_alloca, 1, "")?,
        zero_value(compiler, payload_type),
    )?;
    let none_val = compiler.builder.build_load(option_type, none_alloca, "none_v")?;
    compiler.builder.build_unconditional_branch(merge_bb)?;
    let none_bb = compiler.builder.get_insert_block().unwrap();

    // Merge with PHI
    compiler.builder.position_at_end(merge_bb);
    let phi = compiler.builder.build_phi(option_type, "result")?;
    phi.add_incoming(&[(&some_val, some_bb), (&none_val, none_bb)]);

    Ok(phi.as_basic_value())
}

/// Compile Array.len()
pub fn compile_array_len<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    array_val: BasicValueEnum<'ctx>,
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    let struct_type = array_struct_type(compiler);

    compiler.inline_counter += 1;
    let id = compiler.inline_counter;

    let array_ptr = compiler.builder.build_alloca(struct_type, &format!("len_arr_{}", id))?;
    compiler.builder.build_store(array_ptr, array_val)?;

    let length_ptr = compiler.builder.build_struct_gep(struct_type, array_ptr, 1, "len_ptr")?;
    Ok(compiler.builder.build_load(compiler.context.i64_type(), length_ptr, "len")?)
}

/// Compile Array.set(index, value)
pub fn compile_array_set<'ctx>(
    compiler: &mut LLVMCompiler,
    array_val: BasicValueEnum<'ctx>,
    index_val: BasicValueEnum<'ctx>,
    value: BasicValueEnum<'ctx>,
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    let struct_type = array_struct_type(compiler);

    compiler.inline_counter += 1;
    let id = compiler.inline_counter;

    let array_ptr = compiler.builder.build_alloca(struct_type, &format!("set_arr_{}", id))?;
    compiler.builder.build_store(array_ptr, array_val)?;

    let index = index_val.into_int_value();
    let index_i64 = if index.get_type() == compiler.context.i64_type() {
        index
    } else {
        compiler.builder.build_int_s_extend(index, compiler.context.i64_type(), "idx")?
    };

    let fields = get_array_fields(compiler, array_ptr, "set")?;

    let elem_ptr = unsafe {
        compiler.builder.build_gep(compiler.context.i64_type(), fields.data_ptr, &[index_i64], "eptr")?
    };

    let store_val = if value.is_int_value() {
        let iv = value.into_int_value();
        if iv.get_type() == compiler.context.i64_type() {
            iv
        } else {
            compiler.builder.build_int_s_extend(iv, compiler.context.i64_type(), "v64")?
        }
    } else {
        return Err(CompileError::TypeError(
            "Array.set currently only supports integer values".to_string(),
            None,
        ));
    };

    compiler.builder.build_store(elem_ptr, store_val)?;
    Ok(array_val)
}

/// Compile Array.pop by pointer -> Option<T>
pub fn compile_array_pop_by_ptr<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    array_ptr: PointerValue<'ctx>,
    element_type: &ast::AstType,
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    compiler.track_generic_type("Option_Some_Type".to_string(), element_type.clone());

    let payload_type = ast_to_payload_type(compiler, element_type)?;
    let option_type = compiler.context.struct_type(
        &[compiler.context.i64_type().into(), compiler.context.ptr_type(AddressSpace::default()).into()],
        false,
    );
    let ptr_type = compiler.context.ptr_type(AddressSpace::default());

    compiler.inline_counter += 1;
    let id = compiler.inline_counter;

    let value_ptr = compiler.builder.build_alloca(payload_type, &format!("pop_val_{}", id))?;
    let fields = get_array_fields(compiler, array_ptr, &format!("pop_{}", id))?;

    // Check empty
    let zero = compiler.context.i64_type().const_zero();
    let is_empty = compiler.builder.build_int_compare(inkwell::IntPredicate::EQ, fields.length, zero, "empty")?;

    let current_fn = compiler.builder.get_insert_block().unwrap().get_parent().unwrap();
    let empty_bb = compiler.context.append_basic_block(current_fn, &format!("pop_empty_{}", id));
    let nonempty_bb = compiler.context.append_basic_block(current_fn, &format!("pop_nonempty_{}", id));
    let merge_bb = compiler.context.append_basic_block(current_fn, &format!("pop_merge_{}", id));

    compiler.builder.build_conditional_branch(is_empty, empty_bb, nonempty_bb)?;

    // Empty: return None
    compiler.builder.position_at_end(empty_bb);
    compiler.builder.build_store(value_ptr, zero_value(compiler, payload_type))?;
    let none_val = option_type.const_named_struct(&[
        compiler.context.i64_type().const_int(1, false).into(),
        ptr_type.const_null().into(),
    ]);
    compiler.builder.build_unconditional_branch(merge_bb)?;

    // Non-empty: pop and return Some
    compiler.builder.position_at_end(nonempty_bb);
    let one = compiler.context.i64_type().const_int(1, false);
    let new_len = compiler.builder.build_int_sub(fields.length, one, "new_len")?;

    let elem_ptr_ptr = unsafe { compiler.builder.build_gep(ptr_type, fields.data_ptr, &[new_len], "epp")? };
    compiler.builder.build_store(fields.length_ptr, new_len)?;

    let elem_val_ptr = compiler.builder.build_load(ptr_type, elem_ptr_ptr, "evp")?.into_pointer_value();
    let elem_val = compiler.builder.build_load(payload_type, elem_val_ptr, "ev")?;
    compiler.builder.build_store(value_ptr, elem_val)?;

    let some_alloca = compiler.builder.build_alloca(option_type, "some")?;
    compiler.builder.build_store(
        compiler.builder.build_struct_gep(option_type, some_alloca, 0, "")?,
        zero,
    )?;
    compiler.builder.build_store(
        compiler.builder.build_struct_gep(option_type, some_alloca, 1, "")?,
        value_ptr,
    )?;
    let some_val = compiler.builder.build_load(option_type, some_alloca, "some_v")?;
    compiler.builder.build_unconditional_branch(merge_bb)?;

    // Merge
    compiler.builder.position_at_end(merge_bb);
    let phi = compiler.builder.build_phi(option_type, "result")?;
    phi.add_incoming(&[(&none_val, empty_bb), (&some_val, nonempty_bb)]);

    Ok(phi.as_basic_value())
}

/// Compile Array.pop (value version)
pub fn compile_array_pop<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    array_val: BasicValueEnum<'ctx>,
    element_type: &ast::AstType,
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    let struct_type = array_struct_type(compiler);
    let array_ptr = compiler.builder.build_alloca(struct_type, "pop_arr")?;
    compiler.builder.build_store(array_ptr, array_val)?;
    compile_array_pop_by_ptr(compiler, array_ptr, element_type)
}
