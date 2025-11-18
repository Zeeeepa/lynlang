# @std.compiler - Compiler Intrinsics

The `@std.compiler` module provides **minimal low-level primitives** that are the foundation for building all higher-level features in Zen.

## Architecture Philosophy

**Compiler Level**: Only exposes minimal primitives
- Memory operations
- Pointer operations  
- Function calling
- Library loading

**Zen Level**: Builds everything else
- Allocators (GPA, AsyncPool)
- FFI libraries
- Collections
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

✅ **Implemented**:
- `raw_allocate`, `raw_deallocate`, `raw_reallocate`
- `raw_ptr_offset`, `raw_ptr_cast`
- `null_ptr`
- `inline_c` (placeholder - returns void for now)

⏳ **Placeholder** (returns errors):
- `load_library`
- `get_symbol`
- `unload_library`
- `call_external`

## See Also

- `examples/compiler_intrinsics.zen` - Example usage
- `stdlib/memory/gpa.zen` - GPA allocator built from primitives
- `stdlib/ffi/ffi.zen` - FFI library built from primitives

