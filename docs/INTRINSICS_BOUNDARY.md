# Intrinsics vs Features Boundary

**Purpose:** Define what belongs in the compiler vs stdlib

---

## Principle

> If it can be written in Zen using intrinsics, it SHOULD be written in Zen.

The compiler provides primitive building blocks. Everything else is stdlib.

---

## What Are Intrinsics?

Intrinsics are operations that CANNOT be expressed in Zen itself. They map directly to:
- LLVM IR instructions
- System calls
- Hardware operations

### Current Intrinsics (`src/intrinsics.rs`)

| Category | Examples | Why Intrinsic? |
|----------|----------|----------------|
| **Memory** | `raw_allocate`, `raw_deallocate`, `memcpy` | Direct syscall/libc |
| **Pointers** | `gep`, `gep_struct`, `ptr_to_int` | LLVM GEP instruction |
| **Type Info** | `sizeof<T>`, `alignof<T>` | Compile-time only |
| **Atomics** | `atomic_load`, `atomic_cas` | CPU atomic instructions |
| **Syscalls** | `syscall0`-`syscall6` | Raw kernel interface |
| **Enums** | `discriminant`, `get_payload` | Memory layout knowledge |

These are correct - they MUST be in the compiler.

---

## What Are Features?

Features are things built ON TOP of intrinsics. They should be in stdlib.

### Correct (in stdlib)

| Type | Location | Uses Intrinsics |
|------|----------|-----------------|
| `Vec<T>` | `stdlib/vec.zen` | `raw_allocate`, `sizeof<T>`, `memcpy` |
| `String` | `stdlib/string.zen` | `raw_allocate`, `memcpy` |
| `HashMap<K,V>` | `stdlib/collections/hashmap.zen` | `raw_allocate`, `sizeof<T>` |
| `Option<T>` | `stdlib/core/option.zen` | enum primitives |
| `Result<T,E>` | `stdlib/core/result.zen` | enum primitives |

### Problematic (hardcoded in codegen)

| Issue | Location | LOC | Should Be |
|-------|----------|-----|-----------|
| Fixed-size array methods | `vec_support.rs` | 326 | `stdlib/core/array.zen` |
| HashMap codegen | `stdlib_codegen/collections.rs` | 670 | Normal method calls |
| Type inference | `expressions/inference.rs` | 1023 | Typechecker result |

---

## The Two Vec Problem

There are TWO different types both using "Vec" naming:

### 1. `Vec<T, N>` - Fixed-Size Array (Primitive)
```zen
// Stack-allocated, size known at compile time
arr: Vec<i32, 10> = Vec { ... }
arr.push(42)  // Compiled by vec_support.rs
```
- Defined in: `AstType::Vec { element_type, size }`
- Methods in: `src/codegen/llvm/vec_support.rs`
- Like Rust's `[T; N]`

### 2. `Vec<T>` - Growable Array (Stdlib)
```zen
// Heap-allocated, growable
v = Vec<i32>.new(allocator)
v.mut_ref().push(42)  // Calls stdlib method
```
- Defined in: `stdlib/vec.zen`
- Uses intrinsics: `raw_allocate`, `sizeof<T>`, etc.
- Like Rust's `Vec<T>`

**Naming Confusion:** The fixed-size array should be `Array<T, N>`, not `Vec<T, N>`.

---

## Type Inference Duplication

### Current State (BAD)
```
typechecker/inference.rs (1008 LOC) ─┐
                                     ├── BOTH infer types!
codegen/expressions/inference.rs (1023 LOC) ─┘
```

### Why It Exists
- AST expressions don't carry type annotations
- Typechecker checks but doesn't annotate
- Codegen must re-infer types to generate correct LLVM IR

### Proper Fix (FUTURE)
```
Parser → AST
           ↓
Typechecker → Typed AST (with type annotations)
           ↓
Codegen → reads types, doesn't infer
```

This requires AST changes - tracked separately.

---

## Action Items

### Immediate
1. ✅ Document the boundary (this file)
2. Rename `Vec<T, N>` to `Array<T, N>` for clarity

### Short-term
3. Move fixed-array methods to `stdlib/core/array.zen`
4. Remove `vec_support.rs` hardcoding

### Long-term
5. Add type annotations to AST
6. Remove `codegen/expressions/inference.rs`
7. Codegen trusts typechecker completely

---

## Decision Tree: Where Does It Go?

```
Is it a raw operation on memory/hardware?
├─ YES → Intrinsic (src/intrinsics.rs)
└─ NO
   │
   Can it be written using existing intrinsics?
   ├─ YES → Stdlib (stdlib/*.zen)
   └─ NO
      │
      Is it a language primitive (array, tuple)?
      ├─ YES → AST type + minimal codegen
      └─ NO → You're doing it wrong
```

---

## Files Reference

| File | Purpose | Status |
|------|---------|--------|
| `src/intrinsics.rs` | Intrinsic definitions | ✅ Correct |
| `stdlib/vec.zen` | Growable Vec<T> | ✅ Correct |
| `stdlib/collections/*` | HashMap, Set, etc. | ✅ Correct |
| `src/codegen/llvm/vec_support.rs` | Fixed array methods | ✅ DELETED (326 LOC) |
| `src/codegen/llvm/stdlib_codegen/collections.rs` | Hardcoded collections | ✅ DELETED (670 LOC) |
| `src/codegen/llvm/stdlib_codegen/compiler.rs` | Compiler intrinsics | ✅ Correct (true intrinsics) |
| `src/codegen/llvm/expressions/inference.rs` | Type inference | ⚠️ Duplicate of typechecker |
