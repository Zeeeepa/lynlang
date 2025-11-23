# Stdlib Implementation Roadmap

## Goal
Transform `examples/hello_world.zen` from using hardcoded Rust stdlib to a real self-hosted stdlib built on compiler primitives.

---

## Current Example Analysis

```zen
{ io } = @std

Person: {
    age: i32,
    name: StaticString,  // ← Compiler magic (no allocator)
}

main = () i32 {
    person = Person {
        age: 13,
        name: "Tom"
    }
    io.println("Hello, Master ${person.name}!")  // ← Hardcoded in Rust
    return 0
}
```

**Problems**:
1. `io.println()` is hardcoded in `src/stdlib_metadata/io.rs` (85 lines)
2. `StaticString` forces compile-time strings (no runtime string manipulation)
3. No allocator in use, can't build collections
4. `String` in `stdlib/string.zen` is a stub (skeleton only)

---

## Immediate Action Items (Next 1-2 Days)

### 1. Verify Compiler Intrinsics Are Complete
**File**: `src/stdlib_metadata/compiler.rs`
**Status**: 13 intrinsics defined ✅
**Action**: Quick audit

```bash
# Check current primitives:
grep -A 2 'pub fn\|functions.insert' src/stdlib_metadata/compiler.rs | head -40
```

**Checklist**:
- [ ] raw_allocate(size: usize) -> *u8 ✅
- [ ] raw_deallocate(ptr: *u8, size: usize) ✅
- [ ] raw_reallocate(ptr, old_size, new_size) -> *u8 ✅
- [ ] gep(base_ptr: *u8, offset: i64) -> *u8 ✅
- [ ] gep_struct(ptr: *u8, field_index: i32) -> *u8 ✅
- [ ] raw_ptr_cast(ptr: *u8) -> *u8 ✅
- [ ] discriminant(enum_val: *u8) -> i32 ✅
- [ ] set_discriminant(ptr, tag: i32) ✅
- [ ] get_payload(enum_val: *u8) -> *u8 ✅
- [ ] set_payload(ptr, payload: *u8) ✅
- [ ] inline_c(code: StaticString) ✅
- [ ] load_library, get_symbol, unload_library ✅
- [ ] **MISSING**: null_ptr() - need to add

### 2. Fix Allocator Module
**Files**: 
- `stdlib/memory/allocator.zen` - Define interface
- `stdlib/memory/gpa.zen` - Implement GPA

**Status**: Task #18 completed ✅
**Action**: Integration test

```bash
# Test current allocator
cargo test --test '*allocator*' 2>&1 | head -20
```

**What should work**:
- `gpa_new()` creates allocator
- `gpa_allocate(alloc, size)` calls raw_allocate
- `gpa_deallocate(alloc, ptr, size)` calls raw_deallocate
- `gpa_reallocate(alloc, ptr, old_size, new_size)` works

### 3. Check String Implementation
**File**: `stdlib/string.zen`
**Current**: 26 lines (skeleton)
**Action**: Expand with working methods

**Required methods**:
```zen
string_new(allocator: Allocator) -> String
string_from_static(s: StaticString, allocator: Allocator) -> String
string_len(s: String) -> usize
string_push(s: *String, char: u8) -> void
string_append(s: *String, other: String) -> void
string_free(s: *String) -> void
```

---

## Phase-by-Phase Implementation

### Phase 1: Compiler Primitives Audit (1 day)

**Deliverable**: Ensure all 13 intrinsics are exposed and working

```
src/stdlib_metadata/compiler.rs
├── raw_allocate .................... ✅ (line 30)
├── raw_deallocate .................. ✅ (line 40)
├── raw_reallocate .................. ✅ (line 53)
├── gep ............................. ✅ (line 189)
├── gep_struct ...................... ✅ (line 203)
├── raw_ptr_offset .................. ✅ (deprecated, line 68)
├── raw_ptr_cast .................... ✅ (line 81)
├── discriminant .................... ✅ (line 142)
├── set_discriminant ................ ✅ (line 152)
├── get_payload ..................... ✅ (line 165)
├── set_payload ..................... ✅ (line 175)
├── inline_c ........................ ✅ (line 18)
├── load_library/get_symbol/unload . ✅ (placeholders)
└── **NULL_PTR** .................... ❌ MISSING
```

**Action**: Add `null_ptr()` if missing
```rust
// In src/stdlib_metadata/compiler.rs
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

### Phase 2: Memory & Allocator (1-2 days)

**Goals**:
1. Verify `stdlib/memory/gpa.zen` works
2. Update `stdlib/memory/allocator.zen` interface
3. Create test file `tests/allocator_integration.rs`

**Test code**:
```rust
#[test]
fn test_gpa_allocate_free() {
    // Allocate 1KB
    let code = r#"
    { gpa } = @std.memory
    main = () i32 {
        alloc = gpa.default_gpa()
        ptr = gpa.gpa_allocate(alloc, 1024)
        ptr == null ? | true { return 1 } | false { }
        gpa.gpa_deallocate(alloc, ptr, 1024)
        return 0
    }
    "#;
    // Compile and verify it runs without error
}
```

### Phase 3: String Type (2-3 days)

**Current file**: `stdlib/string.zen` (26 lines, incomplete)
**Action**: Expand to full implementation

```zen
// stdlib/string.zen
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
        data: allocator.gpa_allocate(allocator, 16),  // Start with 16 bytes
        len: 0,
        capacity: 16,
        allocator: allocator
    }
}

string_push = (s: *String, char: u8) void {
    if s.len >= s.capacity {
        new_capacity := s.capacity * 2
        new_data := s.allocator.gpa_reallocate(
            s.allocator,
            s.data,
            s.capacity,
            new_capacity
        )
        s.data = new_data
        s.capacity = new_capacity
    }
    
    // Use GEP to compute byte position
    char_ptr := compiler.gep(s.data, s.len as i64)
    *(char_ptr as *u8) = char
    s.len = s.len + 1
}

string_free = (s: *String) void {
    s.allocator.gpa_deallocate(s.allocator, s.data, s.capacity)
    s.data = compiler.null_ptr()
    s.len = 0
    s.capacity = 0
}

string_len = (s: String) usize {
    return s.len
}
```

**Tests to add** (`tests/string_integration.rs`):
```rust
#[test]
fn test_string_push() { ... }

#[test]
fn test_string_capacity_growth() { ... }

#[test]
fn test_string_free() { ... }
```

### Phase 4: Option/Result Types (Task #15, 3-4 days)

**Current state**: Hardcoded in `src/type_system/ast.rs`
**Goal**: Move to `stdlib/core/option.zen` and `stdlib/core/result.zen`

**File**: `stdlib/core/option.zen`
```zen
{ compiler } = @std

// Define Option as enum
Option<T>: enum {
    Some: T,
    None
}

// Helper: Get discriminant (0 = Some, 1 = None)
option_discriminant = (opt: *Option<T>) i32 {
    return compiler.discriminant(opt)
}

option_is_some = (opt: Option<T>) bool {
    return compiler.discriminant(opt) == 0
}

option_is_none = (opt: Option<T>) bool {
    return compiler.discriminant(opt) == 1
}

option_unwrap = (opt: Option<T>) T {
    tag := compiler.discriminant(opt)
    if tag == 0 {
        payload_ptr := compiler.get_payload(opt)
        return *(payload_ptr as *T)
    }
    // Panic or return default
}
```

**Compiler changes needed**:
- Remove hardcoded Option enum from `src/type_system/ast.rs`
- Update typechecker to recognize Option from stdlib
- Update pattern matching to work with stdlib Option

### Phase 5: Collections (4-5 days)

**Files**:
- `stdlib/vec.zen` - Dynamic array
- `stdlib/collections/hashmap.zen` - Hash table
- `stdlib/collections/set.zen` - Set
- `stdlib/collections/queue.zen` - Queue

**Start with Vec** (most important):
```zen
{ compiler } = @std
{ gpa } = @std.memory

Vec<T>: {
    data: *u8,
    len: usize,
    capacity: usize,
    allocator: gpa.Allocator
}

vec_new = (allocator: gpa.Allocator) Vec<T> {
    return Vec<T> {
        data: allocator.gpa_allocate(allocator, 0),
        len: 0,
        capacity: 0,
        allocator: allocator
    }
}

vec_push = (v: *Vec<T>, item: T) void {
    if v.len >= v.capacity {
        new_capacity := if v.capacity == 0 { 1 } else { v.capacity * 2 }
        item_size := compiler.size_of_type(T)  // Need this intrinsic!
        new_size := new_capacity * item_size
        new_data := v.allocator.gpa_reallocate(
            v.allocator,
            v.data,
            v.capacity * item_size,
            new_size
        )
        v.data = new_data
        v.capacity = new_capacity
    }
    
    item_ptr := compiler.gep(v.data, (v.len * sizeof(T)) as i64)
    *(item_ptr as *T) = item
    v.len = v.len + 1
}

vec_get = (v: Vec<T>, index: usize) Option<T> {
    if index >= v.len {
        return Option.None
    }
    item_ptr := compiler.gep(v.data, (index * sizeof(T)) as i64)
    return Option.Some(*(item_ptr as *T))
}
```

**Problem**: Need `sizeof(T)` intrinsic!

---

## Critical Blockers & Missing Primitives

### 1. sizeof<T>() Intrinsic
**Status**: ❌ Not in compiler.rs
**Impact**: Can't implement Vec<T>, HashMap, etc.
**Solution**: Add to src/stdlib_metadata/compiler.rs
```rust
functions.insert(
    "sizeof".to_string(),
    StdFunction {
        name: "sizeof".to_string(),
        params: vec![],  // Generic type param
        return_type: AstType::Usize,
        is_builtin: true,
    },
);
```

### 2. Type Reflection
**Status**: ❌ Not exposed
**Impact**: Can't dynamically work with arbitrary types
**Solution**: May need additional intrinsics

### 3. null_ptr() Function
**Status**: ❌ Missing from compiler.rs
**Impact**: Can't initialize pointers safely
**Solution**: Add immediately (1 line of code)

---

## Updated Compiler Primitives Checklist

```
MUST HAVE:
├── raw_allocate(size: usize) -> *u8 ................... ✅
├── raw_deallocate(ptr: *u8, size: usize) .............. ✅
├── raw_reallocate(ptr, old, new) -> *u8 ............... ✅
├── gep(ptr: *u8, offset: i64) -> *u8 ................. ✅
├── gep_struct(ptr: *u8, field: i32) -> *u8 ........... ✅
├── discriminant(enum_val: *u8) -> i32 ................. ✅
├── get_payload(enum_val: *u8) -> *u8 ................. ✅
├── null_ptr() -> *u8 ................................. ❌ ADD
├── sizeof<T>() -> usize ............................... ❌ ADD
└── raw_ptr_cast(ptr: *u8) -> *u8 ..................... ✅

NICE TO HAVE:
├── alignof<T>() -> usize .............................. 📌
├── memcpy(dst, src, len) ............................. 📌
├── memset(ptr, val, len) ............................. 📌
└── set_payload(ptr, payload: *u8) .................... ✅ (placeholder)
```

---

## Implementation Timeline

```
Week 1:
├─ Day 1: Audit compiler.rs, add null_ptr() & sizeof()
├─ Day 2: Complete string.zen with push/free/len
├─ Day 3: Allocator integration tests
├─ Day 4: Start Option/Result elimination (Task #15)
└─ Day 5: Tests & fixes

Week 2:
├─ Day 1-2: Complete Option/Result elimination
├─ Day 3-4: Start Vec<T> implementation
└─ Day 5: Tests & documentation
```

---

## Testing Strategy per Phase

### Phase 1: Compiler Primitives
```rust
#[test]
fn test_null_ptr_exists() {
    // Verify null_ptr() can be called
}

#[test]
fn test_sizeof_returns_valid() {
    // Verify sizeof(T) returns correct size
}
```

### Phase 2: Memory/Allocator
```rust
#[test]
fn test_allocate_deallocate_cycle() {
    // Allocate, verify non-null, deallocate
}

#[test]
fn test_reallocate_growth() {
    // Allocate small, reallocate to larger, verify no data loss
}
```

### Phase 3: String
```rust
#[test]
fn test_string_push_grows_capacity() {
    // Push beyond initial capacity, verify growth
}

#[test]
fn test_string_free_nullifies_ptr() {
    // Free string, verify data ptr is null
}
```

### Phase 4: Option/Result
```rust
#[test]
fn test_option_some_is_some() {
    // Create Some(value), test is_some()
}

#[test]
fn test_option_none_is_none() {
    // Create None, test is_none()
}
```

### Phase 5: Vec
```rust
#[test]
fn test_vec_push_pop() {
    // Push 100 items, verify capacity growth
}

#[test]
fn test_vec_get_oob() {
    // Get out-of-bounds, verify returns None
}
```

---

## Refactoring Checklist

- [ ] Add `null_ptr()` intrinsic to compiler.rs
- [ ] Add `sizeof<T>()` intrinsic to compiler.rs
- [ ] Complete `stdlib/string.zen` with all methods
- [ ] Test allocator integration
- [ ] Move Option/Result definitions to stdlib
- [ ] Remove hardcoded Option/Result from src/type_system/
- [ ] Implement Vec<T> in stdlib/vec.zen
- [ ] Add HashMap implementation
- [ ] Update hello_world.zen example to use new stdlib
- [ ] Create comprehensive test suite

---

## Success Criteria

1. ✅ `examples/hello_world.zen` compiles without changes
2. ✅ All 87 tests still pass
3. ❌ → ✅ Add 20+ new stdlib tests
4. ✅ No Rust code in `src/stdlib_metadata/` except compiler module
5. ✅ All stdlib modules are self-hosted (.zen files)
6. ✅ String type works with dynamic allocation
7. ✅ Collections use allocator pattern
8. ✅ Option/Result defined in stdlib, not hardcoded

---

## Commands to Run

```bash
# Check current state
cargo test --lib 2>&1 | grep -E "test result:|FAILED"

# Check stdlib compilation
cargo build --release 2>&1 | grep -E "error|warning"

# Run existing tests
cargo test --test '*' 2>&1 | tail -20

# After changes, verify no regressions
cargo test --all 2>&1 | grep "test result"
```

---

## References

- STDLIB_MIGRATION_PLAN.md - Strategic overview
- STATUS_CURRENT.md - Current progress (4/20 tasks)
- src/stdlib_metadata/compiler.rs - Intrinsic definitions
- stdlib/memory/gpa.zen - Allocator implementation
- examples/hello_world.zen - Example to refactor

---

## Next Action

**START HERE**: Run audit on compiler.rs to identify missing intrinsics
```bash
grep 'functions.insert' src/stdlib_metadata/compiler.rs | wc -l
# Should show 13 + 2 missing (null_ptr, sizeof)
```

Then add the two missing intrinsics and run tests to ensure no breakage.
