//! Helper module for creating FFI types
//! Provides convenience functions for creating AstType values

use crate::ast::AstType;

pub fn void() -> AstType {
    AstType::Void
}

pub fn bool() -> AstType {
    AstType::Bool
}

pub fn i8() -> AstType {
    AstType::I8
}

pub fn i16() -> AstType {
    AstType::I16
}

pub fn i32() -> AstType {
    AstType::I32
}

pub fn i64() -> AstType {
    AstType::I64
}

pub fn u8() -> AstType {
    AstType::U8
}

pub fn u16() -> AstType {
    AstType::U16
}

pub fn u32() -> AstType {
    AstType::U32
}

pub fn u64() -> AstType {
    AstType::U64
}

pub fn f32() -> AstType {
    AstType::F32
}

pub fn f64() -> AstType {
    AstType::F64
}

pub fn usize() -> AstType {
    // Use platform-specific size
    #[cfg(target_pointer_width = "64")]
    return AstType::U64;
    #[cfg(target_pointer_width = "32")]
    return AstType::U32;
    #[cfg(not(any(target_pointer_width = "64", target_pointer_width = "32")))]
    return AstType::U64; // Default to 64-bit
}

pub fn isize() -> AstType {
    // Use platform-specific size
    #[cfg(target_pointer_width = "64")]
    return AstType::I64;
    #[cfg(target_pointer_width = "32")]
    return AstType::I32;
    #[cfg(not(any(target_pointer_width = "64", target_pointer_width = "32")))]
    return AstType::I64; // Default to 64-bit
}

pub fn string() -> AstType {
    crate::ast::resolve_string_struct_type()
}

pub fn c_string() -> AstType {
    // C string is a pointer to char (u8)
    AstType::ptr(AstType::U8)
}

pub fn raw_ptr(inner: AstType) -> AstType {
    // Use regular Pointer for raw pointers in FFI
    AstType::ptr(inner)
}

pub fn ptr(inner: AstType) -> AstType {
    // Use regular Pointer type
    AstType::ptr(inner)
}

pub fn array(size: usize, element_type: AstType) -> AstType {
    // Use FixedArray for arrays with known size
    AstType::FixedArray {
        element_type: Box::new(element_type),
        size,
    }
}

pub fn slice(element_type: AstType) -> AstType {
    // A slice is a dynamic array
    AstType::Array(Box::new(element_type))
}

pub fn function(params: Vec<AstType>, returns: AstType) -> AstType {
    AstType::FunctionPointer {
        param_types: params,
        return_type: Box::new(returns),
    }
}
