//! Standard library function codegen
//! Split into modules by functionality

pub mod collections;
pub mod compiler;
pub mod core;
pub mod fs;
pub mod helpers;
pub mod io;
pub mod math;

// Re-export io functions for backward compatibility
pub use io::*;

// Re-export math functions
pub use math::compile_math_function;

// Re-export core functions
pub use core::{compile_core_assert, compile_core_panic};

// Re-export fs functions
pub use fs::{
    compile_fs_create_dir, compile_fs_exists, compile_fs_read_file, compile_fs_remove_file,
    compile_fs_write_file,
};

// Re-export helpers
pub use helpers::{create_result_err, create_result_ok, create_result_ok_void};

// Re-export collections functions
pub use collections::{compile_dynvec_new, compile_hashmap_new, compile_hashset_new};

// Re-export compiler intrinsics
pub use compiler::{
    // Inline C
    compile_inline_c,
    // Memory allocation
    compile_raw_allocate,
    compile_raw_deallocate,
    compile_raw_reallocate,
    // Pointer operations
    compile_raw_ptr_cast,
    compile_raw_ptr_offset,
    // External calls
    compile_call_external,
    // Library loading
    compile_dlerror,
    compile_get_symbol,
    compile_load_library,
    compile_unload_library,
    // Pointer utilities
    compile_is_null,
    compile_null_ptr,
    // Enum intrinsics
    compile_discriminant,
    compile_get_payload,
    compile_set_discriminant,
    compile_set_payload,
    // GEP intrinsics
    compile_gep,
    compile_gep_struct,
    // Load/store intrinsics
    compile_load,
    compile_store,
    // Pointer conversion
    compile_int_to_ptr,
    compile_ptr_to_int,
    // Sizeof
    compile_sizeof,
    // Memory operations
    compile_memcmp,
    compile_memcpy,
    compile_memmove,
    compile_memset,
    // Bitwise intrinsics
    compile_bswap16,
    compile_bswap32,
    compile_bswap64,
    compile_ctlz,
    compile_ctpop,
    compile_cttz,
};
