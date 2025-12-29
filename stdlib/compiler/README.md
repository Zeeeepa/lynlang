# @std.compiler - Compiler Intrinsics

The `@std.compiler` module provides **minimal low-level primitives** that are the foundation for building all higher-level features in Zen.

## Architecture

```
┌─────────────────────────────────────────────────────┐
│  Layer 1: Rust/LLVM Intrinsics                     │
│                                                     │
│  src/stdlib_metadata/compiler.rs                   │
│  └── Type signatures (57 defined)                 │
│                                                     │
│  src/codegen/llvm/stdlib_codegen/compiler.rs       │
│  └── LLVM IR generation (13 implemented)          │
│                                                     │
│  Accessed via: @builtin.*                          │
└─────────────────────────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────┐
│  Layer 2: stdlib/compiler/compiler.zen             │
│  (THE ONLY file that uses @builtin.*)              │
│                                                     │
│  Wraps raw intrinsics with Zen types              │
│  Provides safe abstractions                        │
│                                                     │
│  Accessed via: { compiler } = @std                 │
└─────────────────────────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────┐
│  Layer 3: Other Stdlib Modules                     │
│                                                     │
│  stdlib/memory/gpa.zen                              │
│  stdlib/core/ptr.zen                                │
│  stdlib/vec.zen                                     │
│                                                     │
│  Import: { compiler } = @std                       │
│  Use: compiler.raw_allocate(size)                  │
└─────────────────────────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────┐
│  Layer 4: User Code                                 │
│                                                     │
│  { gpa } = @std.memory                              │
│  { Vec } = @std                                    │
│  // Uses safe abstractions, NOT raw intrinsics    │
└─────────────────────────────────────────────────────┘
```

## Key Symbols

| Symbol | Meaning | Used By |
|--------|---------|---------|
| `@builtin.*` | Raw Rust/LLVM intrinsics | ONLY compiler.zen |
| `@std.compiler` | This module (compiler.zen) | Other stdlib modules |
| `@std.*` | Standard library modules | User code |

## Philosophy

**Compiler Level (Rust)**: Only exposes minimal primitives
- Memory operations (malloc/free/realloc wrappers)
- Pointer arithmetic (LLVM GEP)
- Enum introspection (discriminant/payload)
- Type introspection (sizeof - incomplete)

**Zen Level (stdlib/)**: Builds everything else
- Allocators (GPA wraps raw_allocate)
- Safe pointers (Ptr<T> wraps RawPtr)
- Collections (Vec, String use allocators)
- All standard library features

## Available Primitives

### Memory Management

```zen
{ compiler } = @std

// Allocate raw memory
ptr = compiler.raw_allocate(size: usize) RawPtr<u8>

// Deallocate raw memory
compiler.raw_deallocate(ptr: RawPtr<u8>, size: usize) void

// Reallocate raw memory
new_ptr = compiler.raw_reallocate(ptr: RawPtr<u8>, old_size: usize, new_size: usize) RawPtr<u8>
```

### Pointer Operations

```zen
// Pointer arithmetic
offset_ptr = compiler.raw_ptr_offset(ptr: RawPtr<u8>, offset: i64) RawPtr<u8>

// Pointer type casting
typed_ptr = compiler.raw_ptr_cast(ptr: RawPtr<u8>) RawPtr<u8>

// Get null pointer
null = compiler.null_ptr() RawPtr<u8>
```

### Inline C Code

```zen
// Compile C code inline (with string interpolation)
compiler.inline_c("""
    memcpy(${dst}, ${src}, ${len});
""")
```

### Library Loading (Placeholder - Not Yet Implemented)

```zen
// Load dynamic library
lib = compiler.load_library(path: string) RawPtr<u8>

// Get symbol from library
func_ptr = compiler.get_symbol(lib: RawPtr<u8>, name: string) RawPtr<u8>

// Unload library
compiler.unload_library(lib: RawPtr<u8>) void

// Call external function
result = compiler.call_external(func_ptr: RawPtr<u8>, args: RawPtr<u8>) RawPtr<u8>
```

## Building Features in Zen

### Example: GPA Allocator

```zen
// stdlib/memory/gpa.zen
{ compiler } = @std

GPA: {}

GPA.allocate = (self: GPA, size: usize) RawPtr<u8> {
    return compiler.raw_allocate(size)
}

GPA.deallocate = (self: GPA, ptr: RawPtr<u8>, size: usize) void {
    compiler.raw_deallocate(ptr, size)
}
```

### Example: FFI Library

```zen
// stdlib/ffi/ffi.zen
{ compiler } = @std

load_library = (path: string) Result<LibraryHandle, FFIError> {
    handle = compiler.load_library(path)
    handle == compiler.null_ptr() ?
        | true { Result:Err(FFIError:LibraryNotFound(path)) }
        | false { Result:Ok(handle) }
}
```

## Implementation Status

**13 of 57 intrinsics have working LLVM codegen.**

✅ **Fully Implemented (13)**:
- Memory: `raw_allocate`, `raw_deallocate`, `raw_reallocate`
- Pointers: `gep`, `gep_struct`, `raw_ptr_offset`, `raw_ptr_cast`, `null_ptr`
- Conversion: `ptr_to_int`, `int_to_ptr`
- Memory Access: `load<T>`, `store<T>`
- Enum: `discriminant`, `set_discriminant`, `get_payload`

❌ **Stubs (return void or errors)**:
- FFI: `inline_c`, `load_library`, `get_symbol`, `unload_library`, `call_external`

❌ **Defined but No Codegen (36)**:
- Memory: `memcpy`, `memmove`, `memset`, `memcmp`
- Atomic: `atomic_load`, `atomic_store`, `atomic_add`, `atomic_sub`, `atomic_cas`, `atomic_xchg`, `fence`
- Bitwise: `bswap16`, `bswap32`, `bswap64`, `ctlz`, `cttz`, `ctpop`
- Overflow: `add_overflow`, `sub_overflow`, `mul_overflow`
- Type: `sizeof` (hardcoded 8), `alignof`
- Debug: `unreachable`, `trap`, `debugtrap`

## Source Files

| File | Purpose |
|------|---------|
| `src/stdlib_metadata/compiler.rs` | Type signatures (57 intrinsics) |
| `src/codegen/llvm/stdlib_codegen/compiler.rs` | LLVM codegen (13 working) |
| `src/codegen/llvm/functions/calls.rs` | Intrinsic dispatcher |
| `docs/INTRINSICS_REFERENCE.md` | Full documentation |

## See Also

- `docs/INTRINSICS_REFERENCE.md` - Complete reference with examples
- `stdlib/memory/gpa.zen` - GPA allocator uses raw_allocate/deallocate
- `stdlib/core/ptr.zen` - Ptr<T> uses pointer intrinsics
- `stdlib/ffi/ffi.zen` - FFI wrappers (stubs)

