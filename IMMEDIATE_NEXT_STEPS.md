# Immediate Next Steps: Make Stdlib Real

## Quick Summary
You have a well-designed stdlib architecture with 13 compiler primitives. The problem: everything else is hardcoded in Rust. Goal: move 500+ lines of stdlib to Zen, remove Rust implementations.

**4 documents were created to guide you**:
1. STDLIB_ARCHITECTURE_REVIEW.md - Architecture analysis
2. STDLIB_IMPLEMENTATION_ROADMAP.md - Phased implementation plan  
3. STDLIB_WORK_BREAKDOWN.md - Exact code to write
4. This file - Quick checklist

---

## TODAY (Next 2 hours)

### Step 1: Add null_ptr() Intrinsic (10 mins)

**Edit**: `src/stdlib/compiler.rs`

**Find**: Line 90 (after `raw_ptr_cast` function definition)

**Add this code**:
```rust
// Null pointer constant
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

**Verify**: Run `cargo build` - should compile without errors

### Step 2: Add sizeof<T>() Intrinsic (10 mins)

**Edit**: `src/stdlib/compiler.rs`

**Find**: Line 100 (after null_ptr)

**Add this code**:
```rust
// Size of type (essential for generic containers)
functions.insert(
    "sizeof".to_string(),
    StdFunction {
        name: "sizeof".to_string(),
        params: vec![],
        return_type: AstType::Usize,
        is_builtin: true,
    },
);
```

**Verify**: Run `cargo build` - should compile without errors

### Step 3: Test Both Intrinsics Work (10 mins)

**Create test**: `tests/new_intrinsics.rs`

```rust
#[test]
fn test_null_ptr_compiles() {
    let code = r#"
    { compiler } = @std
    main = () i32 {
        ptr = compiler.null_ptr()
        return 0
    }
    "#;
    // Compile code (you have existing test infrastructure)
    // If it compiles, intrinsic works
}

#[test]
fn test_sizeof_compiles() {
    let code = r#"
    { compiler } = @std
    main = () i32 {
        size = compiler.sizeof(i32)
        return 0
    }
    "#;
    // Compile code
    // If it compiles, intrinsic works
}
```

**Run**: `cargo test new_intrinsics`

### Step 4: Run Full Test Suite (5 mins)

```bash
cargo test --all 2>&1 | tail -20
```

**Expected**: `test result: ok. X passed; 0 failed`

---

## THIS WEEK

### Task 1: Expand String Type (4-6 hours)

**File**: `stdlib/string.zen`

**Replace the entire file** with:

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
        data: allocator.gpa_allocate(allocator, 16),
        len: 0,
        capacity: 16,
        allocator: allocator
    }
}

string_push = (s: *String, char: u8) void {
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

string_free = (s: *String) void {
    s.allocator.gpa_deallocate(s.allocator, s.data, s.capacity)
    s.len = 0
    s.capacity = 0
    s.data = compiler.null_ptr()
}
```

**Test**: Create `tests/string_implementation.rs` with 5-10 basic tests

**Run**: `cargo test string_` to verify

### Task 2: Allocator Integration Testing (2-3 hours)

**File**: `tests/allocator_integration.rs`

```rust
#[test]
fn test_gpa_basic_allocation() {
    // Test that gpa.default_gpa() works
    // Test that allocate returns non-null
    // Test that deallocate works
}

#[test]
fn test_gpa_reallocation() {
    // Test allocation -> reallocation -> deallocation
}

#[test]
fn test_string_uses_allocator() {
    // Test String works with allocator
    // Create string, push chars, verify length
}
```

**Run**: `cargo test allocator_`

---

## NEXT 2 WEEKS

### Task 3: Move Option/Result to Stdlib (3-4 days)

**Files**: 
- `stdlib/core/option.zen`
- `stdlib/core/result.zen`

Use code examples from STDLIB_WORK_BREAKDOWN.md

**After**: Update compiler to remove hardcoded Option/Result from `src/type_system/ast.rs`

### Task 4: Implement Vec<T> (3-4 days)

**File**: `stdlib/vec.zen`

Use code examples from STDLIB_WORK_BREAKDOWN.md

**Key method**: `vec_push` with capacity growth using `compiler.sizeof(T)`

### Task 5: Complete Collections (4-5 days)

- HashMap
- Set
- Queue
- Stack

---

## REFACTORING CHECKLIST

After each task, check these:

- [ ] Code compiles: `cargo build`
- [ ] All tests pass: `cargo test --all`
- [ ] No warnings: `cargo build 2>&1 | grep warning`
- [ ] hello_world.zen still works: (test manually)
- [ ] Documentation updated

---

## Files to Remove After Completion

```
src/stdlib/
├── core.rs ................ Remove after Task 3
├── io.rs .................. Remove after documenting
├── math.rs ................ Remove after Task 4+
├── fs.rs .................. Remove after Task 5+
├── vec.rs ................. Remove after Task 4
└── net.rs ................. Remove after Task 5+

src/type_system/
├── ast.rs ................. Remove hardcoded Option/Result (Task 3)
```

---

## Success Metrics

After completion, you should have:

1. ✅ 15+ new compiler primitive tests
2. ✅ 500+ lines of Zen stdlib code
3. ✅ 50+ new integration tests
4. ✅ All 87 original tests still passing
5. ✅ Zero hardcoded stdlib in src/stdlib/*.rs (except compiler.rs)
6. ✅ hello_world.zen still compiles and runs

---

## Quick Reference: Key Functions to Implement

### String (50 lines)
```
string_new
string_push
string_pop
string_len
string_capacity
string_at
string_free
```

### Option (100 lines)
```
Option<T> enum
option_is_some
option_is_none
option_unwrap
option_map
option_filter
option_or
option_and
```

### Result (100 lines)
```
Result<T,E> enum
result_is_ok
result_is_err
result_unwrap
result_map
result_and_then
result_or_else
```

### Vec (150 lines)
```
Vec<T> struct
vec_new
vec_push
vec_pop
vec_get
vec_len
vec_capacity
vec_free
vec_reserve
vec_shrink_to_fit
```

---

## Run This Now

```bash
cd /home/ubuntu/zenlang

# 1. Check current state
cargo test --lib 2>&1 | tail -5

# 2. Add null_ptr() to compiler.rs (copy from above)
# 3. Add sizeof() to compiler.rs (copy from above)

# 4. Build and verify
cargo build 2>&1 | head -10

# 5. Run tests
cargo test --all 2>&1 | tail -10
```

**Expected output**: `test result: ok. 87 passed; 0 failed`

---

## Questions to Answer Before Starting

1. **Where is the codegen for built-in functions?**
   - Answer: `src/codegen/llvm/functions/stdlib/`
   - Need to check if null_ptr/sizeof need codegen mapping

2. **How are compiler intrinsics called?**
   - Answer: Direct LLVM instruction generation
   - null_ptr() → `i8* null`
   - sizeof(T) → `@llvm.objectsize(i32 0)` or similar

3. **Do we have generic type handling?**
   - Answer: Yes, already in parser and type checker
   - `sizeof<T>()` should work with existing machinery

---

## Contact Points

If you get stuck:

1. Check STDLIB_WORK_BREAKDOWN.md for exact code
2. Look at existing string.zen for structure
3. Review memory/gpa.zen for allocator pattern
4. Look at examples/hello_world.zen for usage patterns

---

## Your Goal

Transform Zen from having a hardcoded stdlib in Rust to a real self-hosted stdlib built on 13 compiler primitives. The architecture is sound. The tools are ready. Just implement the functions in Zen instead of Rust.

**Start now**: Add null_ptr() and sizeof() intrinsics. Takes 20 minutes. Compile it. Run tests. Everything should still work.
