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

Defined in `src/stdlib_metadata/compiler.rs`, used via `@std.compiler`:

### Memory (3)
```
raw_allocate(size: usize) -> *u8
raw_deallocate(ptr: *u8, size: usize) -> void
raw_reallocate(ptr: *u8, old_size: usize, new_size: usize) -> *u8
```

### Pointer (5)
```
gep(base: *u8, offset: i64) -> *u8           # GetElementPointer
gep_struct(ptr: *u8, field: i32) -> *u8      # Struct field access
raw_ptr_cast(ptr: *u8) -> *u8                # Type reinterpret
raw_ptr_offset(ptr: *u8, offset: i64) -> *u8 # Deprecated, use gep
```

### Enum (4)
```
discriminant(enum_val: *u8) -> i32           # Read variant tag
set_discriminant(ptr: *u8, tag: i32) -> void # Write variant tag
get_payload(enum_val: *u8) -> *u8            # Extract payload pointer
set_payload(ptr: *u8, payload: *u8) -> void  # Set payload
```

### FFI (4)
```
inline_c(code: StaticString) -> void
load_library(path: StaticString) -> *u8
get_symbol(lib: *u8, symbol: StaticString) -> *u8
unload_library(lib: *u8) -> void
```

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
    allocate: (self, size: usize) *u8,
    deallocate: (self, ptr: *u8, size: usize) void,
    reallocate: (self, ptr: *u8, old_size: usize, new_size: usize) *u8
}
```

### GPA (General Purpose Allocator)
```zen
GPA: { id: i32 }

gpa_new() GPA
gpa_allocate(alloc: GPA, size: usize) *u8
gpa_deallocate(alloc: GPA, ptr: *u8, size: usize)
gpa_reallocate(alloc: GPA, ptr: *u8, old_size: usize, new_size: usize) *u8
default_allocator() GPA
```

---

## Collections

### Vec<T>
```zen
Vec<T>: {
    data: *u8,
    len: usize,
    capacity: usize
}

vec_new() Vec<T>
vec_push(v: *Vec<T>, elem_size: usize, elem_addr: *u8)
vec_pop(v: *Vec<T>)
vec_len(v: Vec<T>) usize
vec_free(v: *Vec<T>, elem_size: usize)
```

### String
```zen
String: {
    data: *u8,
    len: usize,
    capacity: usize
}

string_new() String
string_push(s: *String, byte: u8)
string_len(s: String) usize
string_free(s: *String)
```

---

## Implementation Status

| Module | Zen Source | Metadata | Codegen | Status |
|--------|------------|----------|---------|--------|
| core/option | Done | Done | N/A | Working |
| core/result | Done | Done | N/A | Working |
| core/ptr | Done | Done | N/A | Working |
| memory/gpa | Done | Done | Done | Working |
| io | Stub | Done | Partial | print/println work |
| math | Stub | Done | Done | Working |
| string | Partial | Done | N/A | Needs completion |
| vec | Partial | Done | N/A | Needs completion |
| collections/* | Stub | - | - | Needs work |
| fs | Stub | Done | Done | Working |
| net | Stub | Done | - | Not implemented |

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

1. **No load/store primitives** - Manual byte copying required
2. **No memcpy intrinsic** - Inefficient bulk operations
3. **Limited generics** - No sizeof<T> yet
4. **Simplified collections** - HashMap uses linear probing

---

## Adding New Stdlib Functions

Decision tree:

1. **Can it be pure Zen?** → Add to `stdlib/*.zen`
2. **Needs LLVM IR?** → Add to `src/codegen/llvm/stdlib_codegen/`
3. **Needs type metadata?** → Add to `src/stdlib_metadata/`
4. **Is it a compiler primitive?** → Add to `compiler.rs` in both metadata and codegen
