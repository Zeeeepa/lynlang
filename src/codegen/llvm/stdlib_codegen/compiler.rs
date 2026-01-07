//! Compiler intrinsics codegen - true LLVM primitives that must be in Rust

use crate::ast::{self, AstType};
use crate::codegen::llvm::{LLVMCompiler, Type};
use crate::error::CompileError;
use inkwell::intrinsics::Intrinsic;
use inkwell::module::Linkage;
use inkwell::types::IntType;
use inkwell::values::{BasicValueEnum, FunctionValue, IntValue, PointerValue};
use inkwell::AddressSpace;

// =============================================================================
// Helpers
// =============================================================================

/// Convert any integer to i64, extending or truncating as needed
fn to_i64<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    val: BasicValueEnum<'ctx>,
    signed: bool,
) -> Result<IntValue<'ctx>, CompileError> {
    if !val.is_int_value() {
        return Err(CompileError::TypeError("Expected integer".to_string(), None));
    }
    let int_val = val.into_int_value();
    let bits = int_val.get_type().get_bit_width();
    let i64_type = compiler.context.i64_type();
    Ok(if bits == 64 {
        int_val
    } else if bits < 64 {
        if signed {
            compiler.builder.build_int_s_extend(int_val, i64_type, "sext")?
        } else {
            compiler.builder.build_int_z_extend(int_val, i64_type, "zext")?
        }
    } else {
        compiler.builder.build_int_truncate(int_val, i64_type, "trunc")?
    })
}

/// Convert integer to specific bit width
fn to_int_width<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    val: IntValue<'ctx>,
    target: IntType<'ctx>,
    signed: bool,
) -> Result<IntValue<'ctx>, CompileError> {
    let src_bits = val.get_type().get_bit_width();
    let dst_bits = target.get_bit_width();
    Ok(if src_bits == dst_bits {
        val
    } else if src_bits < dst_bits {
        if signed {
            compiler.builder.build_int_s_extend(val, target, "sext")?
        } else {
            compiler.builder.build_int_z_extend(val, target, "zext")?
        }
    } else {
        compiler.builder.build_int_truncate(val, target, "trunc")?
    })
}

/// Get or declare a libc function
fn get_or_declare_fn<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    name: &str,
    ret_type: Option<inkwell::types::BasicTypeEnum<'ctx>>,
    param_types: &[inkwell::types::BasicMetadataTypeEnum<'ctx>],
) -> FunctionValue<'ctx> {
    compiler.module.get_function(name).unwrap_or_else(|| {
        let fn_type = match ret_type {
            Some(t) => match t {
                inkwell::types::BasicTypeEnum::IntType(i) => i.fn_type(param_types, false),
                inkwell::types::BasicTypeEnum::PointerType(p) => p.fn_type(param_types, false),
                _ => compiler.context.void_type().fn_type(param_types, false),
            },
            None => compiler.context.void_type().fn_type(param_types, false),
        };
        compiler.module.add_function(name, fn_type, Some(Linkage::External))
    })
}

/// Call an LLVM intrinsic (bswap, ctlz, cttz, ctpop)
fn call_int_intrinsic<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    name: &str,
    val: IntValue<'ctx>,
    extra_args: &[BasicValueEnum<'ctx>],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    let intrinsic = Intrinsic::find(name)
        .ok_or_else(|| CompileError::InternalError(format!("{} intrinsic not found", name), None))?;
    let int_type = val.get_type();
    let intrinsic_fn = intrinsic
        .get_declaration(&compiler.module, &[int_type.into()])
        .ok_or_else(|| CompileError::InternalError(format!("Failed to get {} declaration", name), None))?;

    let mut args: Vec<BasicValueEnum> = vec![val.into()];
    args.extend_from_slice(extra_args);
    let args_meta: Vec<_> = args.iter().map(|a| (*a).into()).collect();

    compiler
        .builder
        .build_call(intrinsic_fn, &args_meta, "intrinsic")?
        .try_as_basic_value()
        .left()
        .ok_or_else(|| CompileError::InternalError("Intrinsic should return value".to_string(), None))
}

/// Extract data pointer from String struct or return pointer directly
fn extract_string_ptr<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    value: BasicValueEnum<'ctx>,
) -> Result<PointerValue<'ctx>, CompileError> {
    match value {
        BasicValueEnum::PointerValue(ptr) => Ok(ptr),
        BasicValueEnum::StructValue(s) => {
            let ptr = compiler.builder.build_extract_value(s, 0, "str_data")?;
            match ptr {
                BasicValueEnum::PointerValue(p) => Ok(p),
                _ => Err(CompileError::InternalError("String field 0 not a pointer".to_string(), None)),
            }
        }
        _ => Err(CompileError::InternalError(format!("Expected String, got {:?}", value.get_type()), None)),
    }
}

fn ptr_type<'ctx>(compiler: &LLVMCompiler<'ctx>) -> inkwell::types::PointerType<'ctx> {
    compiler.context.ptr_type(AddressSpace::default())
}

fn require_args(args: &[ast::Expression], expected: usize, name: &str) -> Result<(), CompileError> {
    if args.len() != expected {
        Err(CompileError::TypeError(format!("{} expects {} args, got {}", name, expected, args.len()), None))
    } else {
        Ok(())
    }
}

// =============================================================================
// Memory Allocation (libc wrappers)
// =============================================================================

pub fn compile_raw_allocate<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    require_args(args, 1, "raw_allocate")?;
    let size_val = compiler.compile_expression(&args[0])?;
    let size = to_i64(compiler, size_val, false)?;
    let malloc = get_or_declare_fn(compiler, "malloc", Some(ptr_type(compiler).into()), &[compiler.context.i64_type().into()]);
    Ok(compiler.builder.build_call(malloc, &[size.into()], "ptr")?.try_as_basic_value().left().unwrap())
}

pub fn compile_raw_deallocate<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    require_args(args, 2, "raw_deallocate")?;
    let ptr = compiler.compile_expression(&args[0])?;
    let _size = compiler.compile_expression(&args[1])?;
    let free = get_or_declare_fn(compiler, "free", None, &[ptr_type(compiler).into()]);
    compiler.builder.build_call(free, &[ptr.into()], "")?;
    Ok(compiler.context.i32_type().const_zero().into())
}

pub fn compile_raw_reallocate<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    require_args(args, 3, "raw_reallocate")?;
    let ptr = compiler.compile_expression(&args[0])?;
    let _old = compiler.compile_expression(&args[1])?;
    let new_size_val = compiler.compile_expression(&args[2])?;
    let new_size = to_i64(compiler, new_size_val, false)?;
    let realloc = get_or_declare_fn(compiler, "realloc", Some(ptr_type(compiler).into()), &[ptr_type(compiler).into(), compiler.context.i64_type().into()]);
    Ok(compiler.builder.build_call(realloc, &[ptr.into(), new_size.into()], "ptr")?.try_as_basic_value().left().unwrap())
}

// =============================================================================
// Pointer Operations
// =============================================================================

pub fn compile_raw_ptr_offset<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    require_args(args, 2, "raw_ptr_offset")?;
    let ptr = compiler.compile_expression(&args[0])?.into_pointer_value();
    let offset_val = compiler.compile_expression(&args[1])?;
    let offset = to_i64(compiler, offset_val, true)?;
    let result = unsafe { compiler.builder.build_gep(compiler.context.i8_type(), ptr, &[offset], "offset")? };
    Ok(result.into())
}

pub fn compile_raw_ptr_cast<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    require_args(args, 1, "raw_ptr_cast")?;
    compiler.compile_expression(&args[0]) // No-op at LLVM level
}

pub fn compile_null_ptr<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    _args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    Ok(ptr_type(compiler).const_null().into())
}

pub fn compile_is_null<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    require_args(args, 1, "is_null")?;
    let ptr = compiler.compile_expression(&args[0])?.into_pointer_value();
    let null = ptr_type(compiler).const_null();
    Ok(compiler.builder.build_int_compare(inkwell::IntPredicate::EQ, ptr, null, "is_null")?.into())
}

pub fn compile_ptr_to_int<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    require_args(args, 1, "ptr_to_int")?;
    let ptr = compiler.compile_expression(&args[0])?.into_pointer_value();
    Ok(compiler.builder.build_ptr_to_int(ptr, compiler.context.i64_type(), "p2i")?.into())
}

pub fn compile_int_to_ptr<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    require_args(args, 1, "int_to_ptr")?;
    let addr = compiler.compile_expression(&args[0])?.into_int_value();
    Ok(compiler.builder.build_int_to_ptr(addr, ptr_type(compiler), "i2p")?.into())
}

// =============================================================================
// Dynamic Library Loading
// =============================================================================

pub fn compile_load_library<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    require_args(args, 1, "load_library")?;
    let path_val = compiler.compile_expression(&args[0])?;
    let path = extract_string_ptr(compiler, path_val)?;
    let dlopen = get_or_declare_fn(compiler, "dlopen", Some(ptr_type(compiler).into()), &[ptr_type(compiler).into(), compiler.context.i32_type().into()]);
    let rtld_lazy = compiler.context.i32_type().const_int(1, false);
    Ok(compiler.builder.build_call(dlopen, &[path.into(), rtld_lazy.into()], "handle")?.try_as_basic_value().left().unwrap())
}

pub fn compile_get_symbol<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    require_args(args, 2, "get_symbol")?;
    let handle = compiler.compile_expression(&args[0])?;
    let name_val = compiler.compile_expression(&args[1])?;
    let name = extract_string_ptr(compiler, name_val)?;
    let dlsym = get_or_declare_fn(compiler, "dlsym", Some(ptr_type(compiler).into()), &[ptr_type(compiler).into(), ptr_type(compiler).into()]);
    Ok(compiler.builder.build_call(dlsym, &[handle.into(), name.into()], "sym")?.try_as_basic_value().left().unwrap())
}

pub fn compile_unload_library<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    require_args(args, 1, "unload_library")?;
    let handle = compiler.compile_expression(&args[0])?;
    let dlclose = get_or_declare_fn(compiler, "dlclose", Some(compiler.context.i32_type().into()), &[ptr_type(compiler).into()]);
    Ok(compiler.builder.build_call(dlclose, &[handle.into()], "result")?.try_as_basic_value().left().unwrap())
}

pub fn compile_dlerror<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    _args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    let dlerror = get_or_declare_fn(compiler, "dlerror", Some(ptr_type(compiler).into()), &[]);
    Ok(compiler.builder.build_call(dlerror, &[], "err")?.try_as_basic_value().left().unwrap())
}

// =============================================================================
// Enum Introspection
// =============================================================================

pub fn compile_discriminant<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    require_args(args, 1, "discriminant")?;
    let ptr = compiler.compile_expression(&args[0])?.into_pointer_value();
    Ok(compiler.builder.build_load(compiler.context.i32_type(), ptr, "disc")?)
}

pub fn compile_set_discriminant<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    require_args(args, 2, "set_discriminant")?;
    let ptr = compiler.compile_expression(&args[0])?.into_pointer_value();
    let disc = compiler.compile_expression(&args[1])?;
    compiler.builder.build_store(ptr, disc)?;
    Ok(compiler.context.i32_type().const_zero().into())
}

pub fn compile_get_payload<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    require_args(args, 1, "get_payload")?;
    let ptr = compiler.compile_expression(&args[0])?.into_pointer_value();
    // Payload after 4-byte discriminant
    let offset = compiler.context.i32_type().const_int(4, false);
    let payload = unsafe { compiler.builder.build_gep(compiler.context.i8_type(), ptr, &[offset], "payload")? };
    Ok(payload.into())
}

pub fn compile_set_payload<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    require_args(args, 2, "set_payload")?;
    let _ptr = compiler.compile_expression(&args[0])?;
    let _payload = compiler.compile_expression(&args[1])?;
    // TODO: needs size information for proper copy
    Ok(compiler.context.i32_type().const_zero().into())
}

// =============================================================================
// GEP / Load / Store
// =============================================================================

pub fn compile_gep<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    require_args(args, 2, "gep")?;
    let ptr = compiler.compile_expression(&args[0])?.into_pointer_value();
    let offset_val = compiler.compile_expression(&args[1])?;
    let offset = to_i64(compiler, offset_val, true)?;
    let result = unsafe { compiler.builder.build_gep(compiler.context.i8_type(), ptr, &[offset], "gep")? };
    Ok(result.into())
}

pub fn compile_gep_struct<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    require_args(args, 2, "gep_struct")?;
    let ptr = compiler.compile_expression(&args[0])?.into_pointer_value();
    let idx = compiler.compile_expression(&args[1])?.into_int_value();
    // Approximate: field_index * 8 bytes
    let idx_i32 = to_int_width(compiler, idx, compiler.context.i32_type(), false)?;
    let offset = compiler.builder.build_int_mul(idx_i32, compiler.context.i32_type().const_int(8, false), "off")?;
    let offset_i64 = to_int_width(compiler, offset, compiler.context.i64_type(), true)?;
    let result = unsafe { compiler.builder.build_gep(compiler.context.i8_type(), ptr, &[offset_i64], "gep_s")? };
    Ok(result.into())
}

pub fn compile_load<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
    type_arg: Option<&AstType>,
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    require_args(args, 1, "load")?;
    let ptr = compiler.compile_expression(&args[0])?.into_pointer_value();
    let load_type = match type_arg {
        Some(ty) => compiler.to_llvm_type(ty)?,
        None => Type::Basic(compiler.context.i32_type().into()),
    };
    let basic = match load_type {
        Type::Basic(b) => b,
        _ => return Err(CompileError::TypeError("load: need basic type".to_string(), None)),
    };
    Ok(compiler.builder.build_load(basic, ptr, "val")?)
}

pub fn compile_store<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
    _type_arg: Option<&AstType>,
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    require_args(args, 2, "store")?;
    let ptr = compiler.compile_expression(&args[0])?.into_pointer_value();
    let val = compiler.compile_expression(&args[1])?;
    compiler.builder.build_store(ptr, val)?;
    Ok(compiler.context.i32_type().const_zero().into())
}

// =============================================================================
// Sizeof
// =============================================================================

pub fn compile_sizeof<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    type_arg: Option<&AstType>,
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    let size: u64 = match type_arg {
        Some(ty) => match ty {
            AstType::I8 | AstType::U8 | AstType::Bool => 1,
            AstType::I16 | AstType::U16 => 2,
            AstType::I32 | AstType::U32 | AstType::F32 => 4,
            AstType::I64 | AstType::U64 | AstType::F64 | AstType::Usize => 8,
            AstType::Void => 0,
            t if t.is_ptr_type() => 8,
            AstType::Struct { fields, .. } => fields.iter().map(|(_, ft)| match ft {
                AstType::I8 | AstType::U8 => 1,
                AstType::I16 | AstType::U16 => 2,
                AstType::I32 | AstType::U32 | AstType::F32 => 4,
                _ => 8,
            }).sum(),
            _ => 8,
        },
        None => 8,
    };
    Ok(compiler.context.i64_type().const_int(size, false).into())
}

// =============================================================================
// Memory Operations (libc)
// =============================================================================

pub fn compile_memset<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    require_args(args, 3, "memset")?;
    let dest = compiler.compile_expression(&args[0])?;
    let val_expr = compiler.compile_expression(&args[1])?.into_int_value();
    let val = to_int_width(compiler, val_expr, compiler.context.i8_type(), false)?;
    let size_val = compiler.compile_expression(&args[2])?;
    let size = to_i64(compiler, size_val, false)?;
    let memset = get_or_declare_fn(compiler, "memset", Some(ptr_type(compiler).into()),
        &[ptr_type(compiler).into(), compiler.context.i8_type().into(), compiler.context.i64_type().into()]);
    compiler.builder.build_call(memset, &[dest.into(), val.into(), size.into()], "")?;
    Ok(compiler.context.i32_type().const_zero().into())
}

pub fn compile_memcpy<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    require_args(args, 3, "memcpy")?;
    let dest = compiler.compile_expression(&args[0])?;
    let src = compiler.compile_expression(&args[1])?;
    let size_val = compiler.compile_expression(&args[2])?;
    let size = to_i64(compiler, size_val, false)?;
    let memcpy = get_or_declare_fn(compiler, "memcpy", Some(ptr_type(compiler).into()),
        &[ptr_type(compiler).into(), ptr_type(compiler).into(), compiler.context.i64_type().into()]);
    compiler.builder.build_call(memcpy, &[dest.into(), src.into(), size.into()], "")?;
    Ok(compiler.context.i32_type().const_zero().into())
}

pub fn compile_memmove<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    require_args(args, 3, "memmove")?;
    let dest = compiler.compile_expression(&args[0])?;
    let src = compiler.compile_expression(&args[1])?;
    let size_val = compiler.compile_expression(&args[2])?;
    let size = to_i64(compiler, size_val, false)?;
    let memmove = get_or_declare_fn(compiler, "memmove", Some(ptr_type(compiler).into()),
        &[ptr_type(compiler).into(), ptr_type(compiler).into(), compiler.context.i64_type().into()]);
    compiler.builder.build_call(memmove, &[dest.into(), src.into(), size.into()], "")?;
    Ok(compiler.context.i32_type().const_zero().into())
}

pub fn compile_memcmp<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    require_args(args, 3, "memcmp")?;
    let p1 = compiler.compile_expression(&args[0])?;
    let p2 = compiler.compile_expression(&args[1])?;
    let size_val = compiler.compile_expression(&args[2])?;
    let size = to_i64(compiler, size_val, false)?;
    let memcmp = get_or_declare_fn(compiler, "memcmp", Some(compiler.context.i32_type().into()),
        &[ptr_type(compiler).into(), ptr_type(compiler).into(), compiler.context.i64_type().into()]);
    Ok(compiler.builder.build_call(memcmp, &[p1.into(), p2.into(), size.into()], "cmp")?.try_as_basic_value().left().unwrap())
}

// =============================================================================
// Bitwise Intrinsics
// =============================================================================

fn compile_bswap<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
    bits: u32,
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    require_args(args, 1, &format!("bswap{}", bits))?;
    let val = compiler.compile_expression(&args[0])?.into_int_value();
    let target_type = match bits {
        16 => compiler.context.i16_type(),
        32 => compiler.context.i32_type(),
        64 => compiler.context.i64_type(),
        _ => return Err(CompileError::InternalError("Invalid bswap width".to_string(), None)),
    };
    let converted = to_int_width(compiler, val, target_type, false)?;
    call_int_intrinsic(compiler, &format!("llvm.bswap.i{}", bits), converted, &[])
}

pub fn compile_bswap16<'ctx>(compiler: &mut LLVMCompiler<'ctx>, args: &[ast::Expression]) -> Result<BasicValueEnum<'ctx>, CompileError> {
    compile_bswap(compiler, args, 16)
}

pub fn compile_bswap32<'ctx>(compiler: &mut LLVMCompiler<'ctx>, args: &[ast::Expression]) -> Result<BasicValueEnum<'ctx>, CompileError> {
    compile_bswap(compiler, args, 32)
}

pub fn compile_bswap64<'ctx>(compiler: &mut LLVMCompiler<'ctx>, args: &[ast::Expression]) -> Result<BasicValueEnum<'ctx>, CompileError> {
    compile_bswap(compiler, args, 64)
}

fn compile_bit_count<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
    intrinsic_name: &str,
    has_zero_poison: bool,
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    require_args(args, 1, intrinsic_name)?;
    let val = compiler.compile_expression(&args[0])?.into_int_value();
    let val_i64 = to_int_width(compiler, val, compiler.context.i64_type(), false)?;
    let extra = if has_zero_poison {
        vec![compiler.context.bool_type().const_zero().into()]
    } else {
        vec![]
    };
    call_int_intrinsic(compiler, &format!("llvm.{}.i64", intrinsic_name), val_i64, &extra)
}

pub fn compile_ctlz<'ctx>(compiler: &mut LLVMCompiler<'ctx>, args: &[ast::Expression]) -> Result<BasicValueEnum<'ctx>, CompileError> {
    compile_bit_count(compiler, args, "ctlz", true)
}

pub fn compile_cttz<'ctx>(compiler: &mut LLVMCompiler<'ctx>, args: &[ast::Expression]) -> Result<BasicValueEnum<'ctx>, CompileError> {
    compile_bit_count(compiler, args, "cttz", true)
}

pub fn compile_ctpop<'ctx>(compiler: &mut LLVMCompiler<'ctx>, args: &[ast::Expression]) -> Result<BasicValueEnum<'ctx>, CompileError> {
    compile_bit_count(compiler, args, "ctpop", false)
}

// =============================================================================
// Inline C Compilation
// =============================================================================

pub fn compile_inline_c<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    use std::io::Write;
    use std::process::Command;

    require_args(args, 1, "inline_c")?;

    // Extract C code from string literal argument
    let c_code = match &args[0] {
        ast::Expression::String(s) => s.clone(),
        _ => {
            return Err(CompileError::TypeError(
                "inline_c requires a string literal argument".to_string(),
                None,
            ))
        }
    };

    // Create temp files
    let temp_dir = std::env::temp_dir();
    let id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let c_file = temp_dir.join(format!("zen_inline_{}.c", id));
    let bc_file = temp_dir.join(format!("zen_inline_{}.bc", id));

    // Write C code to temp file
    let mut file = std::fs::File::create(&c_file).map_err(|e| {
        CompileError::InternalError(format!("Failed to create temp C file: {}", e), None)
    })?;
    file.write_all(c_code.as_bytes()).map_err(|e| {
        CompileError::InternalError(format!("Failed to write C code: {}", e), None)
    })?;
    drop(file);

    // Compile C to LLVM bitcode using clang
    let output = Command::new("clang")
        .args([
            "-emit-llvm",
            "-c",
            "-O2",
            "-o",
            bc_file.to_str().unwrap_or(""),
            c_file.to_str().unwrap_or(""),
        ])
        .output()
        .map_err(|e| {
            CompileError::InternalError(
                format!("Failed to run clang (is it installed?): {}", e),
                None,
            )
        })?;

    // Clean up C file
    let _ = std::fs::remove_file(&c_file);

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let _ = std::fs::remove_file(&bc_file);
        return Err(CompileError::InternalError(
            format!("Clang compilation failed:\n{}", stderr),
            None,
        ));
    }

    // Load the bitcode and link into current module
    let memory_buffer = inkwell::memory_buffer::MemoryBuffer::create_from_file(&bc_file)
        .map_err(|e| {
            let _ = std::fs::remove_file(&bc_file);
            CompileError::InternalError(format!("Failed to load bitcode: {:?}", e), None)
        })?;

    let inline_module = compiler
        .context
        .create_module_from_ir(memory_buffer)
        .map_err(|e| {
            let _ = std::fs::remove_file(&bc_file);
            CompileError::InternalError(format!("Failed to parse bitcode: {:?}", e), None)
        })?;

    // Clean up bitcode file
    let _ = std::fs::remove_file(&bc_file);

    // Link the inline module into our main module
    compiler
        .module
        .link_in_module(inline_module)
        .map_err(|e| {
            CompileError::InternalError(format!("Failed to link inline C module: {:?}", e), None)
        })?;

    Ok(compiler.context.i32_type().const_zero().into())
}

pub fn compile_call_external<'ctx>(
    _compiler: &mut LLVMCompiler<'ctx>,
    _args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    Err(CompileError::InternalError(
        "call_external not implemented - use inline_c to define C functions, then call them directly".to_string(),
        None,
    ))
}
