# Stdlib Implementation: Detailed Work Breakdown

## Problem Statement

The `examples/hello_world.zen` example works, but relies on hardcoded Rust implementations for the stdlib. The goal is to make the stdlib "real" by:

1. Exposing minimal compiler primitives
2. Building everything else in Zen using those primitives
3. Removing Rust stdlib implementations

---

## Current State Snapshot

### What Works ✅
- Compiler parses Zen code correctly
- Type checking validates programs
- LLVM codegen produces executables
- hello_world.zen compiles and runs
- 87 unit tests all pass

### What's Hardcoded in Rust ❌
```
src/stdlib_metadata/
├── compiler.rs ............ 13 intrinsics ✅ GOOD (stays in Rust)
├── core.rs ............... Option/Result (hardcoded) ❌ MOVE TO STDLIB
├── io.rs ................. println, print, etc (hardcoded) ❌ NEEDS WORK
├── math.rs ............... Math functions ❌ MOVE TO STDLIB
├── fs.rs ................. File operations ❌ MOVE TO STDLIB
├── vec.rs ................ Vector type ❌ MOVE TO STDLIB
└── net.rs ................ Network ❌ MOVE TO STDLIB
```

### What's Stubbed in Zen 📋
```
stdlib/
├── std.zen ............... Module exports (just stubs)
├── io/io.zen ............. Function signatures only
├── string.zen ............ Skeleton (26 lines)
├── memory/allocator.zen .. Interface (stub)
├── memory/gpa.zen ........ GPA impl (partial)
├── vec.zen ............... Skeleton
├── core/option.zen ....... Stub
├── core/result.zen ....... Stub
└── collections/*.zen ..... All stubs
```

---

## Compiler Primitives: Current Inventory

**File**: `src/stdlib_metadata/compiler.rs` (232 lines)

### Memory Operations (3)
```rust
✅ raw_allocate(size: usize) -> *u8
   Line 30: functions.insert("raw_allocate", ...)
   
✅ raw_deallocate(ptr: *u8, size: usize) -> void
   Line 40: functions.insert("raw_deallocate", ...)
   
✅ raw_reallocate(ptr, old_size, new_size) -> *u8
   Line 53: functions.insert("raw_reallocate", ...)
```

### Pointer Operations (5)
```rust
✅ raw_ptr_offset(ptr: *u8, offset: i64) -> *u8
   Line 68: DEPRECATED, use gep instead
   
✅ raw_ptr_cast(ptr: *u8) -> *u8
   Line 81: Type casting
   
✅ gep(base_ptr: *u8, offset: i64) -> *u8
   Line 189: **KEY**: GetElementPointer (byte arithmetic)
   
✅ gep_struct(ptr: *u8, field_index: i32) -> *u8
   Line 203: Struct field access
   
❌ null_ptr() - MISSING
   Should return: *u8 (null pointer constant)
```

### Enum Operations (4)
```rust
✅ discriminant(enum_val: *u8) -> i32
   Line 142: Read variant tag
   
✅ set_discriminant(ptr, tag: i32) -> void
   Line 152: Write variant tag
   
✅ get_payload(enum_val: *u8) -> *u8
   Line 165: Extract payload pointer
   
✅ set_payload(ptr, payload: *u8) -> void
   Line 175: Set payload (placeholder)
```

### Type Introspection (0) ❌
```
❌ sizeof<T>() -> usize - MISSING
   Needed for: generic containers (Vec<T>, HashMap, etc)
   
❌ alignof<T>() -> usize - MISSING (nice to have)
   Needed for: alignment calculations
```

### FFI & Inline (4 + 1)
```rust
✅ inline_c(code: StaticString) -> void
   Line 18: Placeholder for inline C
   
✅ load_library(path: StaticString) -> *u8
   Line 107: Placeholder for dynamic linking
   
✅ get_symbol(lib_handle, symbol) -> *u8
   Line 118: Placeholder
   
✅ unload_library(lib_handle) -> void
   Line 131: Placeholder
```

---

## Priority 1: Add Missing Intrinsics (0.5 days)

### Task 1.1: Add null_ptr()
**File**: `src/stdlib_metadata/compiler.rs`
**Lines**: Add around line 90 (after raw_ptr_cast)

```rust
// In CompilerModule::new()
functions.insert(
    "null_ptr".to_string(),
    StdFunction {
        name: "null_ptr".to_string(),
        params: vec![],
        return_type: AstType::Ptr(Box::new(AstType::U8)),
        is_builtin: true,
    },
);
```

**Usage in Zen**:
```zen
{ compiler } = @std
ptr = compiler.null_ptr()
```

**Why**: Safe way to create null pointers, alternative to casting integers

### Task 1.2: Add sizeof<T>()
**File**: `src/stdlib_metadata/compiler.rs`
**Lines**: Add around line 100

```rust
// In CompilerModule::new()
functions.insert(
    "sizeof".to_string(),
    StdFunction {
        name: "sizeof".to_string(),
        params: vec![],  // Generic type param in type checker
        return_type: AstType::Usize,
        is_builtin: true,
    },
);
```

**Usage in Zen**:
```zen
item_size = compiler.sizeof(T)  // For generic Vec<T>
```

**Why**: Essential for implementing generic containers

---

## Priority 2: Complete String Type (2 days)

### Current File: stdlib/string.zen
**Lines**: 26 (stub)
**Status**: Needs full implementation

### What to Implement

```zen
{ compiler } = @std
{ gpa } = @std.memory

// 1. TYPE DEFINITION
String: {
    data: *u8,
    len: usize,
    capacity: usize,
    allocator: gpa.Allocator
}

// 2. CONSTRUCTORS
string_new = (allocator: gpa.Allocator) String {
    initial_capacity = 16
    return String {
        data: allocator.gpa_allocate(allocator, initial_capacity),
        len: 0,
        capacity: initial_capacity,
        allocator: allocator
    }
}

string_from_static = (s: StaticString, allocator: gpa.Allocator) String {
    // Calculate length of static string
    len = string_strlen(s)  // Need this helper
    
    result = String {
        data: allocator.gpa_allocate(allocator, len + 1),
        len: len,
        capacity: len + 1,
        allocator: allocator
    }
    
    // Copy bytes using memcpy (or loop with GEP)
    for i = 0; i < len; i = i + 1 {
        char_ptr = compiler.gep(result.data, i as i64)
        static_char_ptr = compiler.gep(s as *u8, i as i64)
        *(char_ptr as *u8) = *(static_char_ptr as *u8)
    }
    
    return result
}

// 3. CORE OPERATIONS
string_push = (s: *String, char: u8) void {
    // Grow capacity if needed
    if s.len >= s.capacity {
        new_capacity = s.capacity * 2
        new_data = s.allocator.gpa_reallocate(
            s.allocator,
            s.data,
            s.capacity,
            new_capacity
        )
        s.data = new_data
        s.capacity = new_capacity
    }
    
    // Insert character at s.len
    char_ptr = compiler.gep(s.data, s.len as i64)
    *(char_ptr as *u8) = char
    s.len = s.len + 1
}

string_pop = (s: *String) Option<u8> {
    s.len > 0 ?
    | true {
        s.len = s.len - 1
        char_ptr = compiler.gep(s.data, s.len as i64)
        Option.Some(*(char_ptr as *u8))
    }
    | false { Option.None }
}

// 4. ACCESSORS
string_len = (s: String) usize {
    return s.len
}

string_capacity = (s: String) usize {
    return s.capacity
}

string_at = (s: String, index: usize) Option<u8> {
    index >= s.len ?
    | true { Option.None }
    | false {
        char_ptr = compiler.gep(s.data, index as i64)
        Option.Some(*(char_ptr as *u8))
    }
}

// 5. UTILITY
string_free = (s: *String) void {
    s.allocator.gpa_deallocate(s.allocator, s.data, s.capacity)
    s.len = 0
    s.capacity = 0
    s.data = compiler.null_ptr()
}

string_clone = (s: String, allocator: gpa.Allocator) String {
    result = string_new(allocator)
    
    for i = 0; i < s.len; i = i + 1 {
        char_option = string_at(s, i)
        char_option ?
        | Option.Some(ch) {
            string_push(&result, ch)
        }
        | Option.None {}
    }
    
    return result
}
```

### Tests Needed
```rust
// tests/string_implementation.rs

#[test]
fn test_string_new() { ... }

#[test]
fn test_string_push() { ... }

#[test]
fn test_string_push_capacity_growth() {
    // Create string, push 100 items, verify capacity grew
}

#[test]
fn test_string_at() { ... }

#[test]
fn test_string_free() { ... }

#[test]
fn test_string_from_static() { ... }
```

---

## Priority 3: Memory Allocator Integration (1 day)

### Current Files
- `stdlib/memory/allocator.zen` - Interface (stub)
- `stdlib/memory/gpa.zen` - Implementation (partial)
- Status: **Task #18 COMPLETED** ✅

### What's Already Done ✅
```zen
GPA: { id: i32 }

gpa_allocate(alloc: GPA, size: usize) *u8
gpa_deallocate(alloc: GPA, ptr: *u8, size: usize) void
gpa_reallocate(alloc: GPA, ptr, old_size, new_size) *u8
default_gpa() GPA
```

### What Needs Testing
- Verify allocator works with string.zen
- Test allocation/reallocation cycles
- Verify no memory leaks

### Integration Test
```zen
// tests/stdlib_integration.rs would test:

main = () i32 {
    allocator = gpa.default_gpa()
    
    // Allocate
    ptr1 = allocator.gpa_allocate(allocator, 1024)
    ptr1 == compiler.null_ptr() ? | true { return 1 } | false {}
    
    // Reallocate
    ptr2 = allocator.gpa_reallocate(allocator, ptr1, 1024, 2048)
    ptr2 == compiler.null_ptr() ? | true { return 2 } | false {}
    
    // Deallocate
    allocator.gpa_deallocate(allocator, ptr2, 2048)
    
    return 0
}
```

---

## Priority 4: Remove Option/Result from Compiler (Task #15, 3 days)

### Current State
**Files**:
- `src/type_system/ast.rs` - Hardcoded Option/Result enum definitions
- `src/stdlib_metadata/result.rs` - Hardcoded Option/Result in compiler
- `stdlib/core/option.zen` - Stub
- `stdlib/core/result.zen` - Stub

### What's Hardcoded in Rust
```rust
// In src/type_system/ast.rs (estimate ~100 lines)
pub fn create_option_type() -> AstType { ... }
pub fn create_result_type() -> AstType { ... }
```

### What to Define in Zen
**File**: `stdlib/core/option.zen`
```zen
{ compiler } = @std

// Enum definition
Option<T>: enum {
    Some: T,
    None
}

// Methods
option_is_some = (opt: Option<T>) bool {
    compiler.discriminant(&opt) == 0
}

option_is_none = (opt: Option<T>) bool {
    compiler.discriminant(&opt) == 1
}

option_unwrap = (opt: Option<T>) T {
    is_some := option_is_some(opt)
    is_some ?
    | true {
        payload_ptr = compiler.get_payload(&opt)
        return *(payload_ptr as *T)
    }
    | false {
        // Panic or return default
        // For now: infinite loop (compiler panic)
        loop(() { true ? { break } | false {} })
        return 0 as T  // Unreachable
    }
}

option_map = (opt: Option<T>, f: (T) -> U) Option<U> {
    option_is_some(opt) ?
    | true {
        val = option_unwrap(opt)
        Option.Some(f(val))
    }
    | false { Option.None }
}

option_filter = (opt: Option<T>, predicate: (T) -> bool) Option<T> {
    option_is_some(opt) ?
    | true {
        val = option_unwrap(opt)
        predicate(val) ?
        | true { Option.Some(val) }
        | false { Option.None }
    }
    | false { Option.None }
}
```

**File**: `stdlib/core/result.zen`
```zen
{ compiler } = @std
{ option } = @std.core

Result<T, E>: enum {
    Ok: T,
    Err: E
}

result_is_ok = (res: Result<T, E>) bool {
    compiler.discriminant(&res) == 0
}

result_is_err = (res: Result<T, E>) bool {
    compiler.discriminant(&res) == 1
}

result_unwrap = (res: Result<T, E>) T {
    is_ok := result_is_ok(res)
    is_ok ?
    | true {
        payload_ptr = compiler.get_payload(&res)
        return *(payload_ptr as *T)
    }
    | false {
        // Panic
        loop(() { true ? { break } | false {} })
        return 0 as T
    }
}
```

### Compiler Changes Required
1. Remove `create_option_type()` from `src/type_system/ast.rs`
2. Remove Option/Result handling from `src/stdlib_metadata/result.rs`
3. Update `src/typechecker/` to import Option/Result from stdlib instead
4. Update pattern matching to work with stdlib definitions
5. Ensure generic type checking still works

### Tests to Add
```rust
#[test]
fn test_option_some_is_some() { ... }

#[test]
fn test_option_none_is_none() { ... }

#[test]
fn test_option_unwrap() { ... }

#[test]
fn test_result_ok_is_ok() { ... }

#[test]
fn test_result_err_unwrap_panics() { ... }
```

---

## Priority 5: Implement Vec<T> (3 days)

### File: stdlib/vec.zen (Currently stub)

```zen
{ compiler } = @std
{ gpa } = @std.memory
{ option } = @std.core

Vec<T>: {
    data: *u8,
    len: usize,
    capacity: usize,
    allocator: gpa.Allocator
}

vec_new = (allocator: gpa.Allocator) Vec<T> {
    return Vec<T> {
        data: compiler.null_ptr(),
        len: 0,
        capacity: 0,
        allocator: allocator
    }
}

vec_push = (v: *Vec<T>, item: T) void {
    item_size = compiler.sizeof(T)
    
    if v.len >= v.capacity {
        new_capacity = if v.capacity == 0 { 1 } else { v.capacity * 2 }
        new_total_size = new_capacity * item_size
        old_total_size = v.capacity * item_size
        
        new_data = v.allocator.gpa_reallocate(
            v.allocator,
            v.data,
            old_total_size,
            new_total_size
        )
        
        v.data = new_data
        v.capacity = new_capacity
    }
    
    // Calculate position: data + (len * sizeof(T))
    offset = (v.len * item_size) as i64
    item_ptr = compiler.gep(v.data, offset)
    
    // Write item at position
    *(item_ptr as *T) = item
    v.len = v.len + 1
}

vec_pop = (v: *Vec<T>) Option<T> {
    v.len > 0 ?
    | true {
        v.len = v.len - 1
        item_size = compiler.sizeof(T)
        offset = (v.len * item_size) as i64
        item_ptr = compiler.gep(v.data, offset)
        Option.Some(*(item_ptr as *T))
    }
    | false { Option.None }
}

vec_get = (v: Vec<T>, index: usize) Option<T> {
    index >= v.len ?
    | true { Option.None }
    | false {
        item_size = compiler.sizeof(T)
        offset = (index * item_size) as i64
        item_ptr = compiler.gep(v.data, offset)
        Option.Some(*(item_ptr as *T))
    }
}

vec_len = (v: Vec<T>) usize {
    return v.len
}

vec_capacity = (v: Vec<T>) usize {
    return v.capacity
}

vec_free = (v: *Vec<T>) void {
    old_size = v.capacity * compiler.sizeof(T)
    v.allocator.gpa_deallocate(v.allocator, v.data, old_size)
    v.len = 0
    v.capacity = 0
    v.data = compiler.null_ptr()
}
```

### Tests to Add
```rust
#[test]
fn test_vec_push_pop() { ... }

#[test]
fn test_vec_capacity_growth() {
    // Push 100 items, verify capacity doubles when needed
}

#[test]
fn test_vec_get_oob() {
    // Get out-of-bounds, verify returns None
}

#[test]
fn test_vec_free() { ... }
```

---

## Priority 6: IO Module (2 days)

### Current State
**Files**:
- `src/stdlib_metadata/io.rs` - 102 lines of hardcoded Rust
- `stdlib/io/io.zen` - 13-line stub

### Functions in Rust (needs removal)
```rust
print(message: StaticString) -> void
println(message: StaticString) -> void
eprint(message: StaticString) -> void
eprintln(message: StaticString) -> void
read_line() -> Result<String, String>
read_input(prompt: StaticString) -> Result<String, String>
```

### Implementation Status
- `print`/`println` - **Must be built-in** (need LLVM syscalls)
- `eprint`/`eprintln` - **Must be built-in** (need LLVM syscalls)
- `read_line` - Can be self-hosted if we have libc bindings
- `read_input` - Can be self-hosted

### What to Do
1. Keep print/println/eprint/eprintln as compiler built-ins (in codegen)
2. Document in stdlib/io/io.zen that these are built-in
3. Implement read_line/read_input in Zen (complex, depends on FFI)

```zen
// stdlib/io/io.zen

{ compiler } = @std

// Built-in functions (implemented in LLVM codegen)
print = (message: StaticString) void {
    // BUILTIN: maps to LLVM write() or puts()
}

println = (message: StaticString) void {
    // BUILTIN: maps to LLVM write() with newline
}

eprint = (message: StaticString) void {
    // BUILTIN: maps to LLVM write() to stderr
}

eprintln = (message: StaticString) void {
    // BUILTIN: maps to LLVM write() to stderr with newline
}

// These can be self-hosted (if FFI available)
// read_line() -> Result<String, String>
// read_input(prompt: StaticString) -> Result<String, String>
```

---

## Work Schedule (Estimated)

```
Week 1:
  Mon (0.5d) .... Task 1.1-1.2: Add null_ptr() & sizeof()
  Tue (2d) ...... Task 2: Complete String implementation
  Wed (1d) ...... Task 3: Test Allocator integration
  Thu (3d) ...... Task 4: Move Option/Result to stdlib (START)
  Fri (0.5d) .... Tests, fixes, integration

Week 2:
  Mon-Tue (2d) .. Task 4: Finish Option/Result
  Wed-Fri (3d) .. Task 5: Implement Vec<T>
  
Week 3:
  Mon-Tue (2d) .. Task 6: Complete IO module docs
  Wed-Fri (3d) .. Collections (HashMap, Set, Queue)
```

---

## Success Metrics

### Code Metrics
- [ ] Zero Rust code in `src/stdlib_metadata/` except `compiler.rs`
- [ ] 500+ lines of new Zen code in `stdlib/`
- [ ] 50+ new tests added (100% passing)

### Functionality Metrics
- [ ] String type works with allocation/reallocation
- [ ] Vec<T> can store 1000+ items
- [ ] Option/Result work with pattern matching
- [ ] Collections use allocator pattern
- [ ] hello_world.zen still compiles without changes

### Quality Metrics
- [ ] Zero compiler warnings
- [ ] Zero unsafe code in Zen stdlib
- [ ] All 87 existing tests still pass
- [ ] No performance regressions

---

## Immediate Next Steps

1. **Run audit** (5 mins)
   ```bash
   cargo build 2>&1 | grep -E "error|warning"
   cargo test --lib 2>&1 | tail -5
   ```

2. **Add null_ptr() intrinsic** (10 mins)
   - Edit: `src/stdlib_metadata/compiler.rs`
   - Add function to StdFunction map
   - Run: `cargo build` to verify

3. **Add sizeof() intrinsic** (10 mins)
   - Edit: `src/stdlib_metadata/compiler.rs`
   - Add function to StdFunction map
   - Run: `cargo build` to verify

4. **Complete string.zen** (2 hours)
   - Edit: `stdlib/string.zen`
   - Implement string_new, string_push, string_free
   - Write integration test

5. **Run full test suite** (5 mins)
   - `cargo test --all 2>&1 | grep "test result"`
   - Should still show all passing

---

## References

- STDLIB_ARCHITECTURE_REVIEW.md - Architecture analysis
- STDLIB_MIGRATION_PLAN.md - Strategic plan
- STATUS_CURRENT.md - Current progress
- examples/hello_world.zen - Example to test
- src/stdlib_metadata/compiler.rs - Intrinsics file
- stdlib/string.zen - String skeleton
- stdlib/memory/gpa.zen - Allocator impl
