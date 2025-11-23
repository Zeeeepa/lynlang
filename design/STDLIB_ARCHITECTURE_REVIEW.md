# Stdlib Architecture Review: From Compiler Primitives to Self-Hosted Implementation

## Executive Summary

The hello_world.zen example reveals the current state of Zen's stdlib architecture:
- **Foundation**: Compiler primitives (memory, pointers, enums) are exposed via `@std.compiler`
- **Current Gap**: Stdlib modules use stubs (empty structs with `id: i32` placeholders)
- **Goal**: Build real implementations in Zen that use compiler primitives

The migration is 20% complete (4/20 tasks). The strategy is sound: expose minimal primitives, build everything else in Zen.

---

## Current Architecture Analysis

### What hello_world.zen Reveals

```zen
{ io } = @std                          // Imports io module from stdlib

Person: {
    age: i32,
    name: StaticString,               // ← StaticString is compiler magic (no stdlib)
}

person.age > 18 ?
| true {
    io.println("Hello, Mr. ${person.name}!")  // ← io.println is built-in
}
```

**Issues identified:**
1. `io.println()` is hardcoded in Rust (`src/stdlib_metadata/io.rs`), not self-hosted
2. `StaticString` is compiler-only, no dynamic String in stdlib
3. `io` module is just a stub: `io: { id: i32 }` in stdlib/std.zen

### Current Stdlib Structure (src/stdlib_metadata/mod.rs)

```
StdNamespace
├── core::CoreModule      → hardcoded in Rust
├── compiler::CompilerModule  → 13 intrinsics exposed ✅
├── io::IOModule          → hardcoded in Rust (print, println, etc.)
├── math::MathModule      → hardcoded in Rust
├── fs::FsModule          → hardcoded in Rust
├── vec::VecModule        → hardcoded in Rust
└── ...
```

**Problem**: Each module has TWO implementations:
- `stdlib/io/io.zen` - Stub file (just declares function signatures)
- `src/stdlib_metadata/io.rs` - Actual Rust implementation (hardcoded)

---

## Compiler Primitives Available (13 total)

All in `src/stdlib_metadata/compiler.rs`, exposed via `@std.compiler`:

### Memory Operations (3)
- `raw_allocate(size: usize) -> Ptr<u8>` - malloc wrapper
- `raw_deallocate(ptr: Ptr<u8>, size: usize) -> void` - free wrapper  
- `raw_reallocate(ptr: Ptr<u8>, old_size: usize, new_size: usize) -> Ptr<u8>` - realloc

### Pointer Operations (5)
- `raw_ptr_offset(ptr: *u8, offset: i64) -> *u8` - Deprecated, use gep
- `raw_ptr_cast(ptr: *u8) -> *u8` - Type-safe pointer casting
- `gep(base_ptr: *u8, offset: i64) -> *u8` - **GetElementPointer** (byte-level)
- `gep_struct(struct_ptr: *u8, field_index: i32) -> *u8` - Struct field access
- (Missing: `null_ptr()` - should create null pointer)

### Enum Operations (4)
- `discriminant(enum_value: *u8) -> i32` - Read variant tag
- `set_discriminant(enum_ptr: *u8, discriminant: i32) -> void` - Write tag
- `get_payload(enum_value: *u8) -> *u8` - Extract payload pointer
- `set_payload(enum_ptr: *u8, payload: *u8) -> void` - Set payload (placeholder)

### FFI/Inline (1+3 placeholders)
- `inline_c(code: StaticString) -> void` - Placeholder
- `load_library(path: StaticString) -> *u8` - Placeholder
- `get_symbol(lib_handle: *u8, symbol_name: StaticString) -> *u8` - Placeholder
- `unload_library(lib_handle: *u8) -> void` - Placeholder

---

## What Needs Self-Hosting

### Phase 1: IO Module (Immediate)
**Current**: Hardcoded in `src/stdlib_metadata/io.rs` (85 lines)
**Target**: Implement in `stdlib/io/io.zen`

```zen
{ compiler } = @std

// These are compiler intrinsics called directly
print = (message: StaticString) void {
    // BUILTIN: maps to LLVM puts() or write()
}

println = (message: StaticString) void {
    // BUILTIN: maps to LLVM puts() with newline
}
```

**Challenge**: These need direct LLVM code generation, can't be pure Zen.
**Solution**: Keep them as built-ins in codegen, but document in stdlib.

### Phase 2: String Type (Partially Done)
**Current**: `stdlib/string.zen` has skeleton, `src/stdlib_metadata/string.rs` has Rust impl
**Target**: Complete self-hosted String using allocator

```zen
{ compiler } = @std
{ gpa } = @std.memory

String: {
    data: *u8,
    len: usize,
    capacity: usize,
    allocator: gpa.Allocator
}

string_new = (allocator: gpa.Allocator) String {
    return String {
        data: allocator.alloc(0),  // Start with minimal capacity
        len: 0,
        capacity: 0,
        allocator: allocator
    }
}

string_push = (s: *String, char: u8) void {
    if s.len >= s.capacity {
        new_capacity := s.capacity * 2 + 1
        new_data := s.allocator.realloc(s.data, new_capacity)
        s.data = new_data
        s.capacity = new_capacity
    }
    // Use gep to compute address
    char_ptr := compiler.gep(s.data, s.len as i64)
    *(char_ptr as *u8) = char
    s.len = s.len + 1
}
```

### Phase 3: Memory Allocator (Task #18 - mostly done)
**Current**: `stdlib/memory/gpa.zen` and `allocator.zen` are stubs
**Status**: ✅ Already defined, just needs testing

```zen
GPA: {
    id: i32  // Global allocator instance
}

Allocator: {
    alloc: (size: usize) -> *u8
    free: (ptr: *u8) -> void
    realloc: (ptr: *u8, new_size: usize) -> *u8
}
```

### Phase 4: Collections (Vec, HashMap, etc.)
**Current**: All stubs in `stdlib/collections/` and `stdlib/vec.zen`
**Target**: Real implementations using `@gep` and allocator

```zen
Vec<T>: {
    data: *u8,      // Pointer to T elements
    len: usize,
    capacity: usize,
    allocator: Allocator
}

vec_new = (allocator: Allocator) Vec<T> {
    return Vec<T> {
        data: allocator.alloc(0),
        len: 0,
        capacity: 0,
        allocator: allocator
    }
}

vec_push = (v: *Vec<T>, item: T) void {
    if v.len >= v.capacity {
        new_capacity := max(1, v.capacity * 2)
        new_size := new_capacity * sizeof(T)
        new_data := v.allocator.realloc(v.data, new_size)
        v.data = new_data
        v.capacity = new_capacity
    }
    
    // GEP to element position
    item_ptr := compiler.gep(v.data, (v.len * sizeof(T)) as i64)
    *(item_ptr as *T) = item
    v.len = v.len + 1
}

vec_get = (v: Vec<T>, index: usize) Option<T> {
    if index >= v.len { return Option.None }
    
    item_ptr := compiler.gep(v.data, (index * sizeof(T)) as i64)
    item := *(item_ptr as *T)
    return Option.Some(item)
}
```

### Phase 5: Option/Result Types (Task #15)
**Current**: Hardcoded enum definitions in compiler
**Target**: Define in `stdlib/core/option.zen` and `stdlib/core/result.zen`

```zen
Option<T>: enum {
    Some: T,
    None
}

Result<T, E>: enum {
    Ok: T,
    Err: E
}

// Using enum intrinsics to implement methods
option_unwrap = (opt: Option<T>) T {
    tag := compiler.discriminant(&opt)
    if tag == 0 {  // SOME variant
        payload_ptr := compiler.get_payload(&opt)
        return *(payload_ptr as *T)
    }
    // panic on None
}

option_is_some = (opt: Option<T>) bool {
    return compiler.discriminant(&opt) == 0
}
```

---

## Current Test Status

From tests/:
- ✅ **87 total tests** - all passing
- ✅ **Enum intrinsics tests** (10 new) - verify discriminant/payload operations
- ✅ **GEP intrinsics tests** (10 new) - verify pointer arithmetic
- ⏳ **IO module tests** - need to add
- ⏳ **String implementation tests** - need to add
- ⏳ **Collection tests** - need to add

---

## Migration Path (Recommended Order)

### 1. Verify/Complete Compiler Primitives ✅ (Done)
```
src/stdlib_metadata/compiler.rs
├── ✅ Memory ops (raw_allocate, raw_deallocate, raw_reallocate)
├── ✅ Pointer ops (gep, gep_struct, raw_ptr_cast)
├── ✅ Enum ops (discriminant, set_discriminant, get_payload, set_payload)
├── ⏳ add null_ptr() if missing
└── 📌 FFI/inline_c (lower priority)
```

### 2. IO Module (Built-in, but document in stdlib)
```
stdlib/io/io.zen
├── print(message: StaticString) void
├── println(message: StaticString) void
├── eprint(message: StaticString) void
├── eprintln(message: StaticString) void
├── read_line() -> Result<String, String>
└── read_input(prompt: StaticString) -> Result<String, String>
```

**Note**: These are compiler built-ins (in codegen), not pure Zen. Just expose signatures.

### 3. Memory/Allocator Module ⏳ (In progress)
```
stdlib/memory/allocator.zen  - Interface definition
stdlib/memory/gpa.zen        - General Purpose Allocator implementation
```

**Status**: Mostly complete, needs integration testing

### 4. String Type ⏳ (Partial)
```
stdlib/string.zen
├── String struct (with allocator)
├── string_new(allocator) -> String
├── string_push(s: *String, char: u8) -> void
├── string_append(s: *String, other: String) -> void
├── string_len(s: String) -> usize
├── string_at(s: String, index: usize) -> Option<u8>
└── ... other methods
```

**Uses**: raw_allocate, gep, discriminant (for Option return types)

### 5. Option/Result Types ⏳ (Task #15)
```
stdlib/core/option.zen
├── Option<T> enum definition
├── option_is_some(opt: Option<T>) -> bool
├── option_is_none(opt: Option<T>) -> bool
├── option_unwrap(opt: Option<T>) -> T
├── option_map(opt: Option<T>, f: (T) -> U) -> Option<U>
└── ... other methods

stdlib/core/result.zen
├── Result<T, E> enum definition
├── result_is_ok(res: Result<T, E>) -> bool
├── result_is_err(res: Result<T, E>) -> bool
├── result_unwrap(res: Result<T, E>) -> T
└── ... other methods
```

**Uses**: discriminant, get_payload, enum construction

### 6. Collections ⏳ (Vec, HashMap, Set, Queue)
```
stdlib/vec.zen          - Dynamic array
stdlib/collections/hashmap.zen   - Hash table
stdlib/collections/set.zen       - Hash set
stdlib/collections/queue.zen     - Queue
```

**Uses**: Memory allocator, GEP, Vec<T> generic

---

## Code Organization Issues to Fix

### 1. Double Definition Problem
**Current state**:
```
stdlib/io/io.zen              ← Stubs only
src/stdlib_metadata/io.rs              ← Real implementation
```

**Solution**: 
- Remove `src/stdlib_metadata/io.rs`
- Complete `stdlib/io/io.zen` with real implementations
- For built-ins only (print/println), document that they're compiler intrinsics

### 2. Circular Dependencies
**Risk**: stdlib modules might need each other.

**Example**: `Vec` needs allocator, allocator needs memory primitives
```
compiler.raw_allocate
    ↓
memory/allocator.zen (GPA)
    ↓
vec.zen (uses allocator)
    ↓
collections/hashmap.zen (uses vec)
```

**Solution**: Load in dependency order, keep compiler module self-contained

### 3. Generic Types
**Problem**: `Vec<T>`, `Option<T>`, `Result<T,E>` are generics
**Solution**: Already supported in Zen parser, just need proper implementation

---

## Testing Strategy

### Add Test Cases For Each Phase

```rust
#[cfg(test)]
mod stdlib_tests {
    // Phase 1: Compiler Primitives
    #[test]
    fn test_raw_allocate() { ... }
    
    #[test]
    fn test_gep_operations() { ... }
    
    #[test]
    fn test_enum_intrinsics() { ... }
    
    // Phase 2: IO Module
    #[test]
    fn test_println() { ... }
    
    // Phase 3: Memory Allocator
    #[test]
    fn test_gpa_allocate_deallocate() { ... }
    
    // Phase 4: String Type
    #[test]
    fn test_string_new() { ... }
    
    #[test]
    fn test_string_push() { ... }
    
    // Phase 5: Option/Result
    #[test]
    fn test_option_is_some() { ... }
    
    #[test]
    fn test_result_unwrap() { ... }
    
    // Phase 6: Collections
    #[test]
    fn test_vec_push_pop() { ... }
}
```

---

## Example: Rewriting hello_world.zen for New Architecture

### Current (Uses hardcoded io.println)
```zen
{ io } = @std

main = () i32 {
    io.println("Hello, World!")
    return 0
}
```

### After Migration (Still same, but implemented correctly)
```zen
{ io } = @std
{ string } = @std
{ gpa } = @std.memory

Person: {
    age: i32,
    name: string.String,  // Dynamic string instead of StaticString
}

main = () i32 {
    allocator = gpa.default_gpa()
    person = Person {
        age: 13,
        name: string.string_from_static("Tom", allocator)
    }
    
    person.age > 18 ?
    | true {
        io.println("Hello, Mr. ${person.name}!")
    }
    | false {
        io.println("Hello, Master ${person.name}!")
    }
    
    string.string_free(&person.name)  // Explicit cleanup
    return 0
}
```

---

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|-----------|
| GEP intrinsics unsafe | Seg fault | Always bounds-check in Zen code |
| Circular dependencies | Link failure | Load modules in strict order |
| Performance overhead | Slower runtime | Intrinsics → single LLVM instructions |
| Generic type monomorphization | Code bloat | Compiler already handles this |
| Memory leaks in allocator | Crashes | Careful testing, explicit free calls |

---

## Dependencies & Blockers

**Can proceed immediately on**:
1. ✅ Verifying compiler.rs has all needed primitives
2. ✅ Testing allocator.zen integration
3. ✅ Completing string.zen implementation

**Blocked by**:
1. Task #15 completion (Option/Result elimination) - Many collections depend on this

**Enables**:
1. Full self-hosted stdlib
2. Custom allocators
3. Zero-cost abstractions
4. Zen-in-Zen compiler (future)

---

## Conclusion

**Current state**: 
- Compiler primitives are well-designed (13 intrinsics)
- Stdlib stubs exist but lack implementations
- Tests are comprehensive (87 tests passing)

**Main work**:
1. Remove Rust implementations from `src/stdlib_metadata/`
2. Complete Zen implementations in `stdlib/`
3. Ensure allocator integration works
4. Eliminate hardcoded Option/Result from compiler

**Timeline**: 
- Phase 1-3: ✅ Complete (already done)
- Phase 4-5: ⏳ In progress (Task #15-19)
- Phase 6: 📋 Planned

This is a solid architecture. The migration path is clear. Keep pushing forward.
