# Zen Standard Library Design

## Overview

The Zen stdlib has a three-layer architecture:

| Layer | Location | Purpose |
|-------|----------|---------|
| **Zen Source** | `stdlib/` | User-facing `.zen` files |
| **Rust Metadata** | `src/stdlib_metadata/` | Type checking & signatures |
| **LLVM Codegen** | `src/codegen/llvm/stdlib_codegen/` | Runtime implementations |

---

## Module Structure

```
stdlib/
├── std.zen              # Entry point, re-exports all modules
├── core/                # Core types
│   ├── option.zen       # Option<T> enum
│   ├── result.zen       # Result<T, E> enum
│   ├── ptr.zen          # Ptr<T> safe pointer wrapper
│   └── propagate.zen    # Error propagation helpers
├── memory/              # Memory management
│   ├── allocator.zen    # Allocator trait
│   └── gpa.zen          # General Purpose Allocator
├── string.zen           # Dynamic String type
├── vec.zen              # Vec<T> growable array
├── io/io.zen            # IO operations
├── math/math.zen        # Math functions
├── collections/         # Data structures
│   ├── stack.zen
│   ├── queue.zen
│   ├── set.zen
│   └── hashmap.zen
├── time.zen             # Duration, Instant, sleep
├── random.zen           # PRNG
├── error.zen            # Error types
├── testing/runner.zen   # Test utilities
├── fs/fs.zen            # File system (stubs)
├── net/net.zen          # Networking (stubs)
└── ffi/ffi.zen          # C FFI
```

---

## Compiler Intrinsics

Defined in `src/stdlib_metadata/compiler.rs`, used via `@std.compiler`.

> **Note**: Only 13 intrinsics have working LLVM codegen. See `docs/INTRINSICS_REFERENCE.md` for full status.

### Working ✅ - Memory (3)
```
raw_allocate(size: usize) -> RawPtr<u8>
raw_deallocate(ptr: RawPtr<u8>, size: usize) -> void
raw_reallocate(ptr: RawPtr<u8>, old_size: usize, new_size: usize) -> RawPtr<u8>
```

### Working ✅ - Pointer (5)
```
gep(base: RawPtr<u8>, offset: i64) -> RawPtr<u8>           # GetElementPointer
gep_struct(ptr: RawPtr<u8>, field: i32) -> RawPtr<u8>      # Struct field access
raw_ptr_cast(ptr: RawPtr<u8>) -> RawPtr<u8>                # Type reinterpret
ptr_to_int(ptr: RawPtr<u8>) -> i64                         # Pointer to integer
int_to_ptr(addr: i64) -> RawPtr<u8>                        # Integer to pointer
```

### Working ✅ - Enum (3 of 4)
```
discriminant(enum_val: RawPtr<u8>) -> i32           # Read variant tag
set_discriminant(ptr: RawPtr<u8>, tag: i32) -> void # Write variant tag
get_payload(enum_val: RawPtr<u8>) -> RawPtr<u8>     # Extract payload pointer
```
- `set_payload` is partial (needs size info)

### Working ✅ - Memory Access (2)
```
load<T>(ptr: RawPtr<u8>) -> T                       # Generic load
store<T>(ptr: RawPtr<u8>, value: T) -> void         # Generic store
```

### NOT Working ❌ - FFI (all stubs)
```
inline_c(code: StaticString) -> void                        # Returns void, does nothing
load_library(path: StaticString) -> RawPtr<u8>              # Returns error
get_symbol(lib: RawPtr<u8>, symbol: StaticString) -> RawPtr<u8>  # Returns error
unload_library(lib: RawPtr<u8>) -> void                     # Returns error
```

### NOT Working ❌ - Also defined but no codegen
- `memcpy`, `memmove`, `memset`, `memcmp`
- `atomic_*` (all 7 atomic operations)
- `sizeof<T>()` (hardcoded to 8), `alignof<T>()`
- `bswap*`, `ctlz`, `cttz`, `ctpop`
- `add_overflow`, `sub_overflow`, `mul_overflow`

---

## Core Types

### Option<T>
```zen
Option<T>:
    Some: T,
    None

// Functions
option_is_some(opt: Option<T>) bool
option_is_none(opt: Option<T>) bool
option_unwrap(opt: Option<T>) T
option_map(opt: Option<T>, f: (T) U) Option<U>
```

### Result<T, E>
```zen
Result<T, E>:
    Ok: T,
    Err: E

// Functions
result_is_ok(res: Result<T, E>) bool
result_is_err(res: Result<T, E>) bool
result_unwrap(res: Result<T, E>) T
```

### Ptr<T>
```zen
Ptr<T>:
    Some: *u8,
    None

// Functions
ptr_allocate(size: usize) Ptr<T>
ptr_at(p: Ptr<T>, index: usize) Ptr<T>
ptr_free(p: Ptr<T>, size: usize)
```

---

## Memory Management

### Allocator Interface
```zen
Allocator: {
    allocate: (self, size: usize) RawPtr<u8>,
    deallocate: (self, ptr: RawPtr<u8>, size: usize) void,
    reallocate: (self, ptr: RawPtr<u8>, old_size: usize, new_size: usize) RawPtr<u8>
}
```

### GPA (General Purpose Allocator)
```zen
GPA: { id: i32 }

gpa_new() GPA
gpa_allocate(alloc: GPA, size: usize) RawPtr<u8>
gpa_deallocate(alloc: GPA, ptr: RawPtr<u8>, size: usize)
gpa_reallocate(alloc: GPA, ptr: RawPtr<u8>, old_size: usize, new_size: usize) RawPtr<u8>
default_allocator() GPA
```

---

## Collections

### Vec<T>
```zen
Vec<T>: {
    data: Ptr<T>,
    len: usize,
    capacity: usize,
    allocator: Allocator
}

Vec<T>.new(allocator: Allocator) Vec<T>
Vec<T>.push(self: MutPtr<Vec<T>>, elem: T)
Vec<T>.pop(self: MutPtr<Vec<T>>)
Vec<T>.get(self: Vec<T>, index: usize) Option<T>
Vec<T>.len(self: Vec<T>) usize
Vec<T>.free(self: MutPtr<Vec<T>>)
```

### String
```zen
String: {
    data: Ptr<u8>,
    len: usize,
    capacity: usize,
    allocator: Allocator
}

String.new(allocator: Allocator) String
String.push(self: MutPtr<String>, byte: u8)
String.len(self: String) usize
String.at(self: String, index: usize) u8
String.free(self: MutPtr<String>)
```

---

## Implementation Status

| Module | Zen Source | Metadata | Codegen | Status |
|--------|------------|----------|---------|--------|
| core/option | ✅ Done | ✅ Done | N/A | **Working** |
| core/result | ✅ Done | ✅ Done | N/A | **Working** |
| core/ptr | ✅ Done | ✅ Done | N/A | Needs testing |
| memory/gpa | ✅ Done | ✅ Done | ✅ Done | **Working** |
| io | Stub | ✅ Done | Partial | print/println work |
| math | Stub | ✅ Done | ✅ Done | **Working** |
| string | ✅ Done | ✅ Done | N/A | **Working** |
| vec | ✅ Done | ✅ Done | N/A | **Working** |
| collections/hashmap | ❌ Stub | - | - | TODO placeholders |
| collections/stack | ❌ Empty | - | - | Placeholder only |
| collections/queue | ⚠️ Partial | - | - | Depends on Vec |
| collections/set | ⚠️ Partial | - | - | Depends on Vec |
| fs | Stub | ✅ Done | ❌ No | Not implemented |
| net | Stub | ✅ Done | ❌ No | Not implemented |
| ffi | Stub | ✅ Done | ❌ Stubs | inline_c returns void |

---

## Design Principles

1. **No Null Pointers** - Use `Option<T>` instead
2. **Type Safety** - Generics with bounds checking
3. **Explicit Ownership** - `Ptr<T>` for heap allocations
4. **Error Handling** - `Result<T, E>` instead of exceptions
5. **Allocator Aware** - All allocations via `Allocator` trait
6. **Self-Hosted** - Build stdlib in Zen using compiler intrinsics

---

## Known Limitations

1. **Simplified collections** - HashMap uses linear probing
2. **Manual iteration** - No iterator trait yet, use index loops
3. **Limited generic inference** - Complex nested generics may need explicit types
4. **FFI stubs** - Dynamic library loading not yet functional

## Recently Implemented

- ✅ `memcpy`, `memmove`, `memset`, `memcmp` intrinsics
- ✅ `load<T>`, `store<T>` generic memory access
- ✅ `sizeof<T>()`, `alignof<T>()` type introspection
- ✅ Atomic operations (load, store, add, sub, cas, xchg, fence)
- ✅ Overflow-checked arithmetic (add_overflow, sub_overflow, mul_overflow)

---

## Adding New Stdlib Functions

Decision tree:

1. **Can it be pure Zen?** → Add to `stdlib/*.zen`
2. **Needs LLVM IR?** → Add to `src/codegen/llvm/stdlib_codegen/`
3. **Needs type metadata?** → Add to `src/stdlib_metadata/`
4. **Is it a compiler primitive?** → Add to `compiler.rs` in both metadata and codegen
