# Remove & and * Pointer Syntax - Complete Refactoring Plan

**Status**: Planning  
**Priority**: CRITICAL  
**Scope**: Remove all `&` (address-of) and `*` (dereference/pointer) syntax from language

---

## Why

The `&` and `*` syntax is completely redundant now that we have:
- `Ptr<T>` - safe owned pointers
- `Ref<T>` - safe borrowed references
- Pattern matching for null-safe access

We should NEVER use raw pointers. Everything goes through type-safe wrappers.

---

## What to Remove

### 1. Parser Tokens
**File**: `src/lexer/mod.rs`
```
Remove:
  - Ampersand (&)
  - Asterisk (*)
```

**Impact**: These tokens won't be recognized, so `&x` and `*x` will be syntax errors

### 2. AST Expressions
**File**: `src/type_system/ast.rs`
```
Remove from Expression enum:
  - AddressOf(Box<Expression>)
  - Dereference(Box<Expression>)
  - PointerDereference(Box<Expression>)
```

### 3. Type System
**File**: `src/type_system/ast.rs`
```
Remove from AstType enum:
  - RawPtr(Box<AstType>)       // *u8, *i32, etc
  - MutPtr(Box<AstType>)       // mutable pointers
  - Ptr(Box<AstType>)          // possibly these too?
```

**BUT KEEP**: 
- Generic Ptr<T> type (handled as struct/enum, not special syntax)
- Generic Ref<T> type (handled as struct/enum, not special syntax)

### 4. Parser Rules
**File**: `src/parser/core.rs`
```
Remove parsing of:
  - prefix & (address-of)
  - prefix * (dereference)
  - type syntax *T
```

### 5. Type Checker
**File**: `src/typechecker/mod.rs`
```
Remove handling of:
  - Expression::AddressOf
  - Expression::Dereference
  - Expression::PointerDereference
```

### 6. Code Generator
**Files**: `src/codegen/llvm/`
```
Remove codegen for:
  - AddressOf (no longer exists)
  - Dereference (no longer exists)
  - PointerDereference (no longer exists)
  - RawPtr types
```

---

## What to Keep

### Ptr<T> Wrapper
```zen
Ptr<T>: enum {
    Some: ??? 
    None
}
```

**Note**: The `Some` variant payload needs handling. Currently uses raw address storage.

### Ref<T> Wrapper
```zen
Ref<T>: {
    addr: ???,
    is_valid: bool
}
```

**Note**: The `addr` field needs a type. Currently uses raw i64.

---

## Problem: How to Store Addresses?

Currently we store raw pointers as:
```zen
Some: i64                    // opaque address
addr: i64                    // opaque address
```

**Options**:

**Option A: Keep i64 (simplest)**
- Store addresses as i64 values
- No * or & syntax needed
- No type safety on the address itself
- This works!

```zen
Ptr<T>: enum {
    Some: i64,      // Opaque address, type info implicit
    None
}

Ref<T>: {
    addr: i64,      // Opaque address, type info implicit
    is_valid: bool
}
```

**Option B: Abstract pointer type (future)**
- Create special OpaquePtr or Address type
- Would require new type system feature
- Deferred for later

**Recommendation**: Use Option A (i64) - simplest, works now

---

## Refactoring Steps

### Phase 1: Remove Parser Support
**Files to modify**:
1. `src/lexer/mod.rs` - Remove Ampersand, Asterisk tokens
2. `src/parser/core.rs` - Remove & and * parsing rules

**Expected result**: `&x` and `*x` cause parse errors

### Phase 2: Remove AST Nodes
**Files to modify**:
1. `src/type_system/ast.rs` - Remove AddressOf, Dereference, PointerDereference, RawPtr from enum

**Expected result**: Code that tries to create these nodes won't compile

### Phase 3: Remove Type System Support
**Files to modify**:
1. `src/typechecker/mod.rs` - Remove handling of removed expressions
2. `src/type_system/monomorphization.rs` - Remove mono for removed nodes
3. `src/type_system/instantiation.rs` - Remove instantiation for removed nodes

### Phase 4: Remove Codegen
**Files to modify**:
1. `src/codegen/llvm/expressions/mod.rs` - Remove codegen for removed nodes
2. `src/codegen/llvm/pointers.rs` - Remove pointer operations
3. Other codegen files with pointer handling

### Phase 5: Update Stdlib
**Files to modify**:
```
stdlib/string.zen         - Remove & and * syntax
stdlib/vec.zen            - Remove & and * syntax
stdlib/core/ptr.zen       - Remove & and * syntax
stdlib/memory/gpa.zen     - Remove * from types
stdlib/ffi/ffi.zen        - Remove * pointers
stdlib/memory/allocator.zen
```

**Changes**:
- Replace `(s: *String)` with `(s: String)` or `(s: Ref<String>)`
- Replace `&s.data` with just `s.data`
- Replace `*addr as *u8` casting with proper type handling
- Replace `*(ptr as *u8) = value` with safe accessor functions

### Phase 6: Update Examples
**Files to modify**:
```
examples/hello_world.zen
examples/showcase.zen
examples/compiler_intrinsics.zen
```

### Phase 7: Update Tests
**All test files**: Remove & and * syntax

---

## Detailed Changes Required

### String Type - Before
```zen
string_push = (s: *String, byte: u8) void {
    target = ptr.ptr_offset(s.data, s.len as i64)
    target ?
    | Some(_) {
        byte_addr = compiler.gep(ptr.ptr_unwrap(s.data), s.len as i64)
        *(byte_addr as *u8) = byte
        s.len = s.len + 1
    }
    | None { }
}
```

### String Type - After
```zen
string_push = (s: String, byte: u8) void {
    // Need to handle mutability differently
    // Either: return new String, or use different pattern
    target = ptr.ptr_offset(s.data, s.len as i64)
    target ?
    | Some(_) {
        byte_addr = compiler.gep(ptr.ptr_unwrap(s.data), s.len as i64)
        // No more *(...) = syntax
        // Need helper function: ptr.write_u8(byte_addr, byte)
        ptr.write_byte(byte_addr, byte)
        s.len = s.len + 1
    }
    | None { }
}
```

**But wait**: How do we mutate s.len? We need parameter to be mutable reference!

---

## Critical Issue: Mutability

The code currently uses `*T` to indicate "mutable reference":
```zen
string_push = (s: *String, byte: u8) void { ... }
```

Without `*T` syntax, how do we pass mutable references?

**Options**:

**A: Use Ref<T> with mutability flag**
```zen
MutRef<T>: {
    addr: i64,
    is_valid: bool,
    is_mutable: bool
}

string_push = (s: MutRef<String>, byte: u8) void { ... }
```

**B: Use special wrapper type**
```zen
Mutable<T>: {
    value: T
}

string_push = (s: Mutable<String>, byte: u8) void { ... }
```

**C: Return modified value (functional style)**
```zen
string_push = (s: String, byte: u8) String {
    // Return new string
    new_s = s
    // modify new_s
    return new_s
}
```

**D: Use compiler intrinsics for mutation**
```zen
string_push = (s: String, byte: u8) void {
    // s is immutable, but we can still write through pointers
    // using unsafe operations
    byte_addr = compiler.gep(ptr.ptr_unwrap(s.data), s.len as i64)
    compiler.write_u8(byte_addr, byte)
}
```

---

## Recommendation

**Use Option B: MutRef<T> type**

```zen
// In stdlib/core/ptr.zen
MutRef<T>: {
    addr: i64,
    is_valid: bool
}

// Create mutable reference
mutref_from = (addr: i64) MutRef<T> { ... }

// Mutable access
mutref_write = (r: MutRef<T>, value: T) void { ... }

// Then use it
string_push = (s: MutRef<String>, byte: u8) void {
    // Mutate through s
    // Still no & or * syntax!
}
```

**Benefit**:
- No & or * syntax needed
- Explicit mutability (MutRef<T> vs Ref<T>)
- Type-safe
- Cleaner than functional style

---

## Files to Modify (Complete List)

### Compiler
```
src/lexer/mod.rs                          - Token removal
src/parser/core.rs                        - Parsing rules
src/type_system/ast.rs                    - AST nodes
src/typechecker/mod.rs                    - Type checking
src/type_system/monomorphization.rs       - Monomorphization
src/type_system/instantiation.rs          - Instantiation
src/codegen/llvm/expressions/mod.rs       - Expression codegen
src/codegen/llvm/pointers.rs              - Pointer ops
src/codegen/llvm/structs.rs               - Struct handling
src/codegen/llvm/statements/variables.rs  - Variable handling
src/codegen/llvm/behaviors.rs             - Behavior codegen
```

### Stdlib
```
stdlib/core/ptr.zen
stdlib/string.zen
stdlib/vec.zen
stdlib/memory/gpa.zen
stdlib/memory/allocator.zen
stdlib/ffi/ffi.zen
stdlib/collections/*.zen
```

### Examples & Tests
```
examples/hello_world.zen
examples/showcase.zen
tests/*.rs
```

---

## Testing Strategy

After each phase:
1. Try to compile
2. Fix compiler errors (remove old syntax)
3. Run tests
4. Fix test failures

### Phase Completion Criteria

**Phase 1**: `cargo build` should error on & and * in code
**Phase 2**: AST no longer has pointer nodes
**Phase 3**: Type checker handles all cases
**Phase 4**: No pointer codegen errors
**Phase 5**: Stdlib compiles
**Phase 6**: Examples compile
**Phase 7**: All tests pass

---

## Success Criteria

- ✅ No & or * tokens recognized by lexer
- ✅ No AddressOf, Dereference, PointerDereference in AST
- ✅ No *T pointer types (replaced with Ptr<T>, Ref<T>, MutRef<T>)
- ✅ All operations use type-safe wrappers
- ✅ 87 tests still passing
- ✅ hello_world.zen compiles and runs
- ✅ Zero unsafe pointer operations

---

## Risk Assessment

**High**: This is a major refactoring affecting parser, typechecker, codegen

**Mitigation**:
- Do phase-by-phase
- Commit after each phase
- Run tests frequently
- Have clear rollback plan

---

## Timeline Estimate

- Phase 1-2: 1-2 hours (parser + AST)
- Phase 3: 2-3 hours (type system)
- Phase 4: 2-3 hours (codegen)
- Phase 5-7: 2-3 hours (stdlib + examples + tests)

**Total**: 9-14 hours of focused work

---

## Next Steps

1. Decide on MutRef<T> approach (or alternative)
2. Start Phase 1: Remove tokens from lexer
3. Make & and * syntax errors
4. Proceed phase-by-phase
5. Update documentation as we go

**Start now?** (y/n)
