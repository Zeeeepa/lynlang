# Zen Language: Ptr/Ref Implementation & Next Steps

**Status**: Active Development
**Focus**: Complete Ptr/Ref functionality and integrate with String/Vec

---

## What's Done

### Ptr<T> Implementation (stdlib/core/ptr.zen)
- ✅ Type-safe pointer wrapper (owns memory, must be freed)
- ✅ Enum variant: `Some: i64` (opaque pointer) | `None`
- ✅ Core operations: allocate, free, deref, offset, unwrap
- ✅ Safe access: ptr_value returns Option<T>
- ✅ Pointer arithmetic: ptr_offset with type awareness

### Ref<T> Implementation (stdlib/core/ptr.zen)
- ✅ Borrowed reference wrapper (stack borrow, not owned)
- ✅ Struct variant: addr + is_valid flag
- ✅ Check validity: ref_is_valid
- ✅ Safe read: ref_value returns Option<T>

### Key Compiler Intrinsics in Place
```
Memory: raw_allocate, raw_deallocate, raw_reallocate ✅
Pointers: gep, gep_struct, raw_ptr_cast ✅
Types: sizeof<T>() ✅
Enums: discriminant, set_discriminant, get_payload ✅
```

---

## What Needs Work

### 1. Ptr/Ref Type System Integration
**Status**: Type checking exists but needs verification

**TODO**:
- [ ] Verify Ptr<T> pattern matching works in compiler
- [ ] Verify Ref<T> dereference ops work
- [ ] Add tests for generic Ptr<T> operations
- [ ] Test composition with Option<Ptr<T>>

**Files affected**:
- `src/typechecker/mod.rs` - type checking for Ptr/Ref
- `src/codegen/llvm/` - LLVM codegen for Ptr operations
- `tests/` - new test cases

### 2. String Using Ptr<u8>
**Status**: stdlib/string.zen exists but needs Ptr<u8> integration

**Current**: Uses raw *u8 pointers  
**Target**: Use Ptr<u8> wrapper for type safety

**Changes needed** (stdlib/string.zen):
```zen
String: {
    data: Ptr<u8>,  // ← Change from *u8 to Ptr<u8>
    len: usize,
    capacity: usize,
    allocator: gpa.Allocator
}
```

**Impact**: string_push, string_pop, string_at, string_free need updates

### 3. Vec<T> Using Ptr<T>
**Status**: stdlib/vec.zen stubbed out

**Target**: Full Vec<T> implementation using Ptr<T>

```zen
Vec<T>: {
    data: Ptr<T>,      // ← Type-safe!
    len: usize,
    capacity: usize,
    allocator: gpa.Allocator
}
```

**Methods needed**:
- vec_new, vec_push, vec_pop, vec_get, vec_len
- vec_capacity, vec_free, vec_reserve

### 4. Null Pointer Handling
**Status**: SAFE_POINTERS_DESIGN.md recommends Ptr<T> instead of null_ptr()

**Decision**: Don't add null_ptr() intrinsic. Use Ptr.None instead.

**Migration**:
- Remove any raw null_ptr() calls
- Replace with `ptr_none<T>()`
- Update error handling to use Option/Ptr wrappers

---

## Quick Implementation Checklist

### Phase 1: Verify Ptr/Ref (Today)
```bash
# 1. Build and test current implementation
cargo build
cargo test --all

# 2. Check Ptr<T> types work in code
cargo test ptr 2>&1 | head -20

# 3. Verify Ref<T> types work
cargo test ref 2>&1 | head -20
```

### Phase 2: String with Ptr<u8> (This Week)
```bash
# Edit stdlib/string.zen to use Ptr<u8>
# Update all string functions

# Test:
cargo test string_ --all
```

### Phase 3: Vec<T> with Ptr<T> (Next Week)
```bash
# Edit stdlib/vec.zen with full implementation
# Uses compiler.sizeof(T) for generics

# Test:
cargo test vec_ --all
```

### Phase 4: Integration Tests (After Phase 3)
```bash
# Complex tests: String + Vec together
# Allocator stress tests
# Type safety verification
```

---

## File Organization

**Design Documents** (moved to `design/`):
- `SAFE_POINTERS_DESIGN.md` - Ptr<T> vs Ref<T> rationale
- `SAFE_TYPE_SYSTEM_DESIGN.md` - Type system architecture
- `STDLIB_ARCHITECTURE_REVIEW.md` - Stdlib design overview
- `STDLIB_IMPLEMENTATION_ROADMAP.md` - Phased implementation plan
- `STDLIB_WORK_BREAKDOWN.md` - Detailed code breakdown

**Root Documentation**:
- `README.md` - Project overview
- `INTRINSICS_REFERENCE.md` - Compiler intrinsics reference
- `STDLIB_IMPLEMENTATION.md` - Standard library guide
- `NEXT_STEPS.md` - This file (consolidated view)

---

## Current Code Structure

```
stdlib/
├── core/
│   ├── ptr.zen          ✅ Ptr<T> + Ref<T> implemented
│   ├── option.zen       ✅ Option<T> enum
│   └── propagate.zen
├── memory/
│   ├── allocator.zen    ✅ Allocator trait
│   └── gpa.zen          ✅ GPA allocator implementation
├── string.zen           ⏳ Needs Ptr<u8> integration
├── vec.zen              ⏳ Needs Ptr<T> implementation
├── collections/
│   ├── hashmap.zen
│   ├── queue.zen
│   ├── set.zen
│   └── stack.zen
└── ...
```

---

## Key Insights from Design Docs

### Why Ptr<T> over raw pointers?
1. **Type-safe**: Compiler forces null handling
2. **Familiar**: Same pattern as Option<T>
3. **Composable**: Works with pattern matching
4. **No silent bugs**: Can't accidentally dereference null

### Recommended Compiler Primitives (12 total)
```
Memory (3):       raw_allocate, raw_deallocate, raw_reallocate
Pointers (4):     gep, gep_struct, raw_ptr_cast, (no null_ptr!)
Types (1):        sizeof<T>()
Enums (4):        discriminant, set_discriminant, get_payload, set_payload
```

### String Type Evolution
```zen
// Old (unsafe): raw *u8
String: { data: *u8, len: usize, capacity: usize }

// New (safe): Ptr<u8>
String: { data: Ptr<u8>, len: usize, capacity: usize }
```

---

## Testing Strategy

### Unit Tests
- Ptr<T> operations (allocate, free, offset)
- Ref<T> validity checks
- Pattern matching with Ptr.Some/None

### Integration Tests
- String with Ptr<u8>
- Vec<T> with Ptr<T>
- Collections with Allocator
- Composition: Vec<String>

### Property Tests
- Allocation/deallocation symmetry
- Pointer arithmetic correctness
- Type safety with generics

---

## Success Criteria

✅ **Ptr<T> and Ref<T>** are properly type-checked  
✅ **String uses Ptr<u8>** instead of raw pointers  
✅ **Vec<T> uses Ptr<T>** with generic sizeof  
✅ **All tests pass** (100% coverage)  
✅ **No null pointer bugs** possible in stdlib  
✅ **hello_world.zen** compiles and runs  

---

## Next Action

1. Run: `cargo test --all` to verify current state
2. Read: `design/SAFE_POINTERS_DESIGN.md` for implementation details
3. Edit: `stdlib/string.zen` to use Ptr<u8>
4. Edit: `stdlib/vec.zen` with full Ptr<T> implementation

---

**Focus**: Ptr/Ref type system completion and stdlib integration
