# Compiler Intrinsics Reference

**Status**: Production Ready (Core Set)
**Implemented**: 25+ intrinsics with working LLVM codegen
**Access**: `@builtin.*` (raw) or `@std.compiler.*` (wrapped)

## Overview

Zen provides low-level intrinsics that map directly to LLVM instructions or libc functions.

**Access Patterns:**
- `@builtin.function()` - Raw intrinsics (internal use only, in stdlib/compiler/compiler.zen)
- `@std.compiler.function()` - Safe wrappers (recommended for user code)

## Quick Reference Table

| Category | Working ✅ | Stubs/Unimplemented ❌ |
|----------|-----------|----------------------|
| Memory Allocation | `raw_allocate`, `raw_deallocate`, `raw_reallocate` | - |
| Memory Operations | `memcpy`, `memmove`, `memset`, `memcmp` | - |
| Pointer Arithmetic | `gep`, `gep_struct`, `null_ptr`, `raw_ptr_offset`, `raw_ptr_cast` | - |
| Memory Access | `load`, `store` | - |
| Enum Operations | `discriminant`, `set_discriminant`, `get_payload` | `set_payload` (partial) |
| Type Conversion | `ptr_to_int`, `int_to_ptr` | `trunc_*`, `sitofp_*`, `uitofp_*` |
| Bitwise | `bswap16`, `bswap32`, `bswap64`, `ctlz`, `cttz`, `ctpop` | - |
| Type Introspection | `sizeof<T>` | `alignof` |
| Atomic | - | `atomic_load`, `atomic_store`, `atomic_add`, `atomic_sub`, `atomic_cas`, `atomic_xchg`, `fence` |
| Overflow | - | `add_overflow`, `sub_overflow`, `mul_overflow` |
| Debug | - | `unreachable`, `trap`, `debugtrap` |
| FFI | - | `inline_c` (stub), `load_library`, `get_symbol`, `unload_library` |

---

## Memory Allocation

### raw_allocate

```zen
ptr = @std.compiler.raw_allocate(1024)
```

Allocates `size` bytes of raw memory using malloc.

- **Params**: `size: usize`
- **Returns**: `RawPtr<u8>` - Pointer to allocated memory, null if allocation fails
- **Safety**: No initialization, no bounds checking

### raw_deallocate

```zen
@std.compiler.raw_deallocate(ptr, 1024)
```

Deallocates memory previously allocated by `raw_allocate`.

- **Params**: `ptr: RawPtr<u8>`, `size: usize`
- **Returns**: `void`
- **Safety**: Caller must ensure correct size and validity

### raw_reallocate

```zen
new_ptr = @std.compiler.raw_reallocate(ptr, 1024, 2048)
```

Reallocates memory to new size, preserving existing data.

- **Params**: `ptr: RawPtr<u8>`, `old_size: usize`, `new_size: usize`
- **Returns**: `RawPtr<u8>` - Pointer to reallocated memory
- **Safety**: Old pointer becomes invalid after call

---

## Memory Operations

### memcpy

```zen
@std.compiler.memcpy(dest, src, 256)
```

Copy bytes from source to destination. Regions must not overlap.

- **Params**: `dest: RawPtr<u8>`, `src: RawPtr<u8>`, `size: usize`
- **Returns**: `void`
- **Safety**: Undefined behavior if regions overlap

### memmove

```zen
@std.compiler.memmove(dest, src, 256)
```

Copy bytes from source to destination. Safe for overlapping regions.

- **Params**: `dest: RawPtr<u8>`, `src: RawPtr<u8>`, `size: usize`
- **Returns**: `void`

### memset

```zen
@std.compiler.memset(ptr, 0, 1024)
```

Set all bytes in memory range to a value.

- **Params**: `dest: RawPtr<u8>`, `value: u8`, `size: usize`
- **Returns**: `void`

### memcmp

```zen
result = @std.compiler.memcmp(ptr1, ptr2, 64)
```

Compare bytes in memory.

- **Params**: `ptr1: RawPtr<u8>`, `ptr2: RawPtr<u8>`, `size: usize`
- **Returns**: `i32` - 0 if equal, negative if ptr1 < ptr2, positive if ptr1 > ptr2

---

## Pointer Arithmetic

### gep

```zen
elem_ptr = @std.compiler.gep(base_ptr, 40)
```

GetElementPointer - performs byte-level pointer arithmetic.

- **Params**: `base_ptr: RawPtr<u8>`, `offset: i64` (signed, can be negative)
- **Returns**: `RawPtr<u8>` - New pointer at computed location
- **Safety**: No bounds checking

### gep_struct

```zen
field_ptr = @std.compiler.gep_struct(struct_ptr, 2)
```

Struct field access using GetElementPointer.

- **Params**: `struct_ptr: RawPtr<u8>`, `field_index: i32`
- **Returns**: `RawPtr<u8>` - Pointer to field location
- **Note**: Assumes 8-byte field alignment

### null_ptr / nullptr

```zen
ptr = @std.compiler.null_ptr()
```

Returns a null pointer (address 0).

- **Returns**: `RawPtr<u8>`
- **Use**: Sentinel values, optional pointer fields

### raw_ptr_offset (deprecated)

```zen
new_ptr = @std.compiler.raw_ptr_offset(ptr, 64)
```

Use `gep` instead. Offset a pointer by byte count.

### raw_ptr_cast

```zen
new_ptr = @std.compiler.raw_ptr_cast(ptr)
```

Reinterprets a pointer type. Zero-cost operation (affects type checking only).

---

## Memory Access

### load

```zen
value = @std.compiler.load(ptr)
```

Load a value from a pointer.

- **Params**: `ptr: RawPtr<u8>`
- **Returns**: Generic `T` based on context
- **Safety**: No type checking, no bounds checking

### store

```zen
@std.compiler.store(ptr, value)
```

Store a value to a pointer.

- **Params**: `ptr: RawPtr<u8>`, `value: T`
- **Returns**: `void`
- **Safety**: No type checking, no bounds checking

---

## Enum Operations

### discriminant

```zen
tag = @std.compiler.discriminant(@ptr_from(opt))
```

Reads the discriminant (variant tag) from an enum value.

- **Params**: `enum_ptr: RawPtr<u8>`
- **Returns**: `i32` - Variant tag
- **Layout**: Discriminant is at offset 0, i32

**Standard Discriminants:**
| Type | Variant | Discriminant |
|------|---------|--------------|
| Option | Some | 0 |
| Option | None | 1 |
| Result | Ok | 0 |
| Result | Err | 1 |

### set_discriminant

```zen
@std.compiler.set_discriminant(@ptr_from(opt), 0)
```

Sets the discriminant (variant tag) of an enum.

- **Params**: `enum_ptr: RawPtr<u8>`, `discriminant: i32`
- **Returns**: `void`
- **Safety**: Doesn't validate discriminant value

### get_payload

```zen
payload_ptr = @std.compiler.get_payload(@ptr_from(opt))
```

Returns a pointer to the payload data within an enum.

- **Params**: `enum_ptr: RawPtr<u8>`
- **Returns**: `RawPtr<u8>` - Pointer to payload (offset 8 bytes from enum start)

### set_payload

```zen
@std.compiler.set_payload(@ptr_from(opt), payload_ptr)
```

Copies payload data into an enum's payload field.

- **Status**: Placeholder - needs size information from type system

---

## Type Conversion

### ptr_to_int

```zen
addr = @std.compiler.ptr_to_int(ptr)
```

Convert a pointer to an integer address.

- **Params**: `ptr: RawPtr<u8>`
- **Returns**: `i64`

### int_to_ptr

```zen
ptr = @std.compiler.int_to_ptr(addr)
```

Convert an integer address to a pointer.

- **Params**: `addr: i64`
- **Returns**: `RawPtr<u8>`

### trunc_f64_i64 / trunc_f32_i32

```zen
int_val = @std.compiler.trunc_f64_i64(3.14159)
```

Truncate floating point to integer.

### sitofp_i64_f64 / uitofp_u64_f64

```zen
float_val = @std.compiler.sitofp_i64_f64(42)
```

Convert integer to floating point (signed/unsigned).

---

## Bitwise Operations

### bswap16 / bswap32 / bswap64

```zen
swapped = @std.compiler.bswap32(0x12345678)
```

Byte-swap for endian conversion.

- **Returns**: Same type with bytes reversed

### ctlz

```zen
zeros = @std.compiler.ctlz(value)
```

Count leading zeros.

- **Params**: `value: u64`
- **Returns**: `u64` - Number of leading zero bits

### cttz

```zen
zeros = @std.compiler.cttz(value)
```

Count trailing zeros.

- **Params**: `value: u64`
- **Returns**: `u64` - Number of trailing zero bits

### ctpop

```zen
bits = @std.compiler.ctpop(value)
```

Population count (count set bits).

- **Params**: `value: u64`
- **Returns**: `u64` - Number of bits set to 1

---

## Atomic Operations

All atomic operations provide sequential consistency by default.

### atomic_load

```zen
value = @std.compiler.atomic_load(ptr)
```

Atomically load a value.

### atomic_store

```zen
@std.compiler.atomic_store(ptr, value)
```

Atomically store a value.

### atomic_add / atomic_sub

```zen
old_value = @std.compiler.atomic_add(ptr, 1)
```

Atomically add/subtract and return old value.

### atomic_cas

```zen
success = @std.compiler.atomic_cas(ptr, expected, new_value)
```

Compare-and-swap. Returns `true` if swap succeeded.

- **Params**: `ptr: RawPtr<u64>`, `expected: u64`, `new_value: u64`
- **Returns**: `bool`

### atomic_xchg

```zen
old_value = @std.compiler.atomic_xchg(ptr, new_value)
```

Atomically exchange and return old value.

### fence

```zen
@std.compiler.fence()
```

Memory fence for synchronization.

---

## Overflow-Checked Arithmetic

Returns a struct with result and overflow flag.

### add_overflow / sub_overflow / mul_overflow

```zen
result = @std.compiler.add_overflow(a, b)
result.overflow ? {
    | true { handle_overflow() }
    | false { use(result.result) }
}
```

- **Params**: `a: i64`, `b: i64`
- **Returns**: `{ result: i64, overflow: bool }`

---

## Type Introspection

### sizeof

```zen
size = @std.compiler.sizeof<MyStruct>()
```

Returns the size of a type in bytes.

- **Returns**: `usize`

### alignof

```zen
align = @std.compiler.alignof<MyStruct>()
```

Returns the alignment of a type in bytes.

- **Returns**: `usize`

---

## Debug Operations

### unreachable

```zen
@std.compiler.unreachable()
```

Marks code as unreachable. Undefined behavior if reached.

### trap

```zen
@std.compiler.trap()
```

Triggers a trap/abort.

### debugtrap

```zen
@std.compiler.debugtrap()
```

Triggers a debug trap (breakpoint).

---

## FFI (Foreign Function Interface)

> ⚠️ **All FFI intrinsics are currently stubs/placeholders**

### inline_c

```zen
@std.compiler.inline_c("printf(\"Hello from C!\\n\");")
```

**Status**: ❌ STUB - Returns void, does nothing. Requires Clang integration to compile C code to LLVM IR.

### load_library

```zen
lib = @std.compiler.load_library("libfoo.so")
```

**Status**: ❌ STUB - Returns error "not yet fully implemented - requires platform-specific FFI".

### get_symbol

```zen
func_ptr = @std.compiler.get_symbol(lib, "my_function")
```

**Status**: ❌ STUB - Returns error "not yet fully implemented".

### unload_library

```zen
@std.compiler.unload_library(lib)
```

**Status**: ❌ STUB - Returns error "not yet fully implemented".

---

## Practical Examples

### Custom Bump Allocator

```zen
BumpAllocator: {
    base: RawPtr<u8>,
    capacity: usize,
    used:: usize,
}

allocate = (alloc: MutPtr<BumpAllocator>, size: usize) RawPtr<u8> {
    alloc.used + size > alloc.capacity ? {
        | true { return @std.compiler.null_ptr() }
        | false { }
    }
    ptr = @std.compiler.gep(alloc.base, alloc.used)
    alloc.used = alloc.used + size
    return ptr
}
```

### Safe Atomic Counter

```zen
AtomicCounter: {
    value: RawPtr<u64>,
}

increment = (counter: MutPtr<AtomicCounter>) u64 {
    return @std.compiler.atomic_add(counter.val.value, 1)
}

get = (counter: Ptr<AtomicCounter>) u64 {
    return @std.compiler.atomic_load(counter.value)
}
```

### Zero Memory

```zen
zero_memory = (ptr: RawPtr<u8>, size: usize) void {
    @std.compiler.memset(ptr, 0, size)
}
```

### Endian Conversion

```zen
host_to_network_u32 = (value: u32) u32 {
    return @std.compiler.bswap32(value)
}
```

---

## Safety Considerations

These are low-level intrinsics that bypass Zen's safety guarantees.

| Risk | Affected Intrinsics |
|------|---------------------|
| No Bounds Checking | `gep`, `load`, `store`, `memcpy`, `memmove` |
| No Type Safety | `raw_ptr_cast`, `load`, `store`, `int_to_ptr` |
| No Initialization | `raw_allocate` |
| Undefined Behavior | `unreachable`, overlapping `memcpy` |
| Data Races | Non-atomic operations on shared data |

### Best Practices

1. **Wrap in safe abstractions**: Create Zen-level wrappers with bounds checking
2. **Validate inputs**: Check bounds and validity before use
3. **Document invariants**: Clear comments on memory layout assumptions
4. **Use proper types**: Avoid raw pointer manipulation in user code
5. **Prefer atomic ops**: Use atomic intrinsics for shared data

---

## Performance Characteristics

| Intrinsic | Cost | Notes |
|-----------|------|-------|
| `raw_allocate` | Function call | Calls malloc |
| `raw_deallocate` | Function call | Calls free |
| `gep` | 1 LLVM inst | GEP instruction |
| `null_ptr` | 0 cost | Constant |
| `load`/`store` | 1 LLVM inst | Memory access |
| `memcpy`/`memmove` | O(n) | libc call, often optimized |
| `bswap*` | 1-2 LLVM inst | Native instruction on most CPUs |
| `ctlz`/`cttz`/`ctpop` | 1 LLVM inst | Native instruction |
| `atomic_*` | 1 LLVM inst | Includes memory barrier |
| `fence` | 1 LLVM inst | Memory fence |

---

## Implementation Details

### Source Files

| File | Purpose |
|------|---------|
| `src/stdlib_metadata/compiler.rs` | Intrinsic type definitions |
| `src/codegen/llvm/intrinsics.rs` | LLVM code generation |
| `src/typechecker/intrinsics.rs` | Type checking |
| `docs/INTRINSICS_REFERENCE.md` | This documentation |

### Testing

```bash
cargo test --test intrinsics_tests
cargo test --test enum_intrinsics  
cargo test --test gep_intrinsics
```

---

## Implementation Priority

### Completed ✅
- [x] `memcpy`, `memmove`, `memset`, `memcmp` - Bulk memory operations
- [x] `sizeof<T>()` with actual type sizes - Type introspection
- [x] `bswap16`, `bswap32`, `bswap64` - Endian conversion
- [x] `ctlz`, `cttz`, `ctpop` - Bit manipulation

### Medium Priority (In Progress)
- [ ] `set_payload` with proper copying - Required for enum construction
- [ ] Atomic operations - Required for thread-safe code
- [ ] Overflow-checked arithmetic - Required for safe math
- [ ] Type conversion intrinsics - Required for numeric casting

### Future Work
- [ ] SIMD intrinsics (vector operations)
- [ ] Platform-specific intrinsics (x86, ARM)
- [ ] Full FFI implementation with Clang integration
- [ ] Bounds-checked variants for development builds
