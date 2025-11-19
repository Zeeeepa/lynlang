# Zen Stdlib Architecture: Current vs Target

## Current Architecture (What's Wrong)

```
┌─────────────────────────────────────────────────────────────┐
│                    Zen Source Code                          │
│                  (hello_world.zen, etc)                     │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│                    Zen Compiler                             │
│  (Parser, Type Checker, LLVM Codegen)                       │
└─────────────────────────────────────────────────────────────┘
                            ↓
                  ┌─────────┴─────────┐
                  ↓                   ↓
        ┌─────────────────┐  ┌──────────────────────┐
        │ Compiler        │  │ Stdlib (Rust Code)   │
        │ Primitives      │  │ ❌ HARDCODED         │
        │ (13)            │  │                      │
        │                 │  │ src/stdlib/          │
        │ ✅ Exposed      │  │ ├─ core.rs           │
        │                 │  │ ├─ io.rs             │
        │ - allocate      │  │ ├─ math.rs           │
        │ - gep           │  │ ├─ fs.rs             │
        │ - sizeof ❌     │  │ ├─ vec.rs            │
        │ - null_ptr ❌   │  │ └─ net.rs            │
        │                 │  │                      │
        └─────────────────┘  └──────────────────────┘
                  ↓
        ┌─────────────────┐
        │  LLVM IR        │
        │  (Executable)   │
        └─────────────────┘
```

**Problem**: 
- stdlib functions hardcoded in Rust (102 lines just for io.rs)
- Can't modify stdlib without recompiling compiler
- Duplicated: stdlib/io/io.zen (stub) + src/stdlib/io.rs (real)
- Missing: sizeof<T>(), null_ptr()

---

## Target Architecture (What We're Building)

```
┌─────────────────────────────────────────────────────────────┐
│                    Zen Source Code                          │
│                  (hello_world.zen, etc)                     │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│                    Zen Compiler                             │
│  (Parser, Type Checker, LLVM Codegen)                       │
└─────────────────────────────────────────────────────────────┘
                            ↓
         ┌──────────────────┴──────────────────┐
         ↓                                     ↓
    ┌─────────────┐               ┌───────────────────────┐
    │ Compiler    │               │ Stdlib (Zen Code)     │
    │ Primitives  │               │ ✅ SELF-HOSTED        │
    │ (15 total)  │               │                       │
    │             │               │ stdlib/               │
    │ ✅ Exposed  │               │ ├─ core/              │
    │             │               │ │  ├─ option.zen      │
    │ Memory (3)  │               │ │  └─ result.zen      │
    │ - allocate  │               │ ├─ string.zen         │
    │ - free      │               │ ├─ vec.zen            │
    │ - realloc   │               │ ├─ memory/            │
    │             │               │ │  ├─ allocator.zen   │
    │ Pointers (5)│               │ │  └─ gpa.zen         │
    │ - gep       │               │ ├─ collections/       │
    │ - gep_struct│               │ │  ├─ hashmap.zen     │
    │ - cast      │               │ │  ├─ set.zen         │
    │ - null_ptr  │ ✅ ADD        │ │  └─ queue.zen       │
    │ - offset    │               │ ├─ io/                │
    │             │               │ │  └─ io.zen          │
    │ Enums (4)   │               │ ├─ math/              │
    │ - discrim   │               │ │  └─ math.zen        │
    │ - set_disc  │               │ ├─ fs/                │
    │ - get_load  │               │ │  └─ fs.zen          │
    │ - set_load  │               │ └─ net/               │
    │             │               │    └─ net.zen         │
    │ Types (1)   │               │                       │
    │ - sizeof    │ ✅ ADD        │ All use:              │
    │             │               │ - compiler.raw_alloc  │
    │ FFI (4)     │               │ - compiler.gep        │
    │ - inline_c  │               │ - compiler.sizeof     │
    │ - load_lib  │               │ - compiler.null_ptr   │
    │ - get_sym   │               │                       │
    │ - unload    │               │                       │
    └─────────────┘               └───────────────────────┘
         ↓
        LLVM IR
         ↓
     Executable
```

**Benefits**:
- All stdlib in Zen (self-hosted)
- Can modify stdlib without recompiling compiler
- Smaller compiler surface area
- Clear separation: compiler (13 primitives) vs stdlib (built on primitives)
- Users can write custom allocators, collections, etc.

---

## File Changes Summary

### Remove (Hardcoded in Rust)
```
src/stdlib/
├── core.rs ...................... DELETE (move to stdlib)
├── io.rs ........................ REFACTOR (keep builtins, document)
├── math.rs ...................... DELETE (move to stdlib)
├── fs.rs ........................ DELETE (move to stdlib)
├── vec.rs ....................... DELETE (move to stdlib)
└── net.rs ....................... DELETE (move to stdlib)

src/type_system/
├── ast.rs ....................... MODIFY (remove hardcoded Option/Result)
└── ... other files
```

### Add (Self-Hosted in Zen)
```
stdlib/
├── string.zen ................... EXPAND (from 26 to 80+ lines)
├── core/
│   ├── option.zen .............. ADD (100+ lines)
│   └── result.zen .............. ADD (100+ lines)
├── memory/
│   ├── allocator.zen ........... COMPLETE (interface)
│   └── gpa.zen ................. COMPLETE (implementation)
├── vec.zen ...................... REWRITE (from stub to real, 150+ lines)
├── collections/
│   ├── hashmap.zen ............. ADD (200+ lines)
│   ├── set.zen ................. ADD (150+ lines)
│   └── queue.zen ............... ADD (100+ lines)
├── math/
│   └── math.zen ................ ADD (100+ lines)
├── fs/
│   └── fs.zen .................. ADD (FFI to libc)
└── net/
    └── net.zen ................. ADD (FFI to libc)
```

### Modify (Add Missing Intrinsics)
```
src/stdlib/
└── compiler.rs ................. EXTEND (add null_ptr, sizeof)
```

---

## Migration Steps

### Step 1: Add Missing Intrinsics (0.5 day)
```
src/stdlib/compiler.rs
  + null_ptr() → *u8
  + sizeof<T>() → usize
  
Verify: cargo test --all
Expected: 87 tests pass
```

### Step 2: String (2 days)
```
stdlib/string.zen
  + string_new(allocator) → String
  + string_push(s, char) → void
  + string_pop(s) → Option<u8>
  + string_free(s) → void
  + string_len, string_at, etc.
  
Uses: compiler.{gep, raw_allocate, null_ptr}
Verify: cargo test string_*
```

### Step 3: Allocator Testing (1 day)
```
stdlib/memory/gpa.zen - already done
Test: allocator integration
Verify: cargo test allocator_*
```

### Step 4: Option/Result (3 days) [Task #15]
```
stdlib/core/option.zen + result.zen
  + enum definitions
  + is_some, is_none, unwrap, map, filter
  
src/type_system/ast.rs
  - remove hardcoded Option/Result
  
Verify: cargo test option_* result_*
```

### Step 5: Vec<T> (3 days)
```
stdlib/vec.zen
  + vec_new(allocator) → Vec<T>
  + vec_push(v, item) → void
  + vec_pop(v) → Option<T>
  + vec_get(v, index) → Option<T>
  + vec_free(v) → void
  
Uses: compiler.{sizeof, gep, raw_allocate}
Uses: Option<T> from step 4
Verify: cargo test vec_*
```

### Step 6: Collections (4 days)
```
stdlib/collections/
  + hashmap.zen
  + set.zen
  + queue.zen
  + stack.zen
  
All use: Vec<T>, allocator, Option<T>
```

### Step 7: Remove Rust Stubs (2 days)
```
Delete: src/stdlib/{core,io,math,fs,vec,net}.rs
Update: Imports in compiler
Verify: cargo build, cargo test
```

---

## Compilation Flow: Before vs After

### Before (Current - Hardcoded)
```
hello_world.zen
    ↓
Parse & Typecheck
    ↓
    ├─ Find io.println
    ├─ Look in @std
    ├─ Find src/stdlib/io.rs definition ← HARDCODED
    ├─ Generate LLVM for println call
    │
Codegen to LLVM
    ↓
    Link with libc
    ↓
Execute
```

### After (Target - Self-Hosted)
```
hello_world.zen
    ↓
Parse & Typecheck
    ↓
    ├─ Find io.println
    ├─ Look in @std
    ├─ Find stdlib/io/io.zen definition ← SELF-HOSTED
    ├─ io.zen calls compiler.raw_allocate, compiler.gep, etc.
    ├─ Generate LLVM for compiler primitives
    │
Codegen to LLVM (compiler primitives only)
    ↓
    Link with libc
    ↓
Execute
```

---

## Dependency Graph

```
compiler primitives (15)
  ↓
  ├─→ memory/allocator.zen ← GPA impl
  │
  ├─→ string.zen (uses allocator)
  │
  ├─→ core/option.zen
  │   │
  │   └─→ core/result.zen
  │
  ├─→ vec.zen (uses Option, allocator, sizeof)
  │   │
  │   └─→ collections/
  │       ├─ hashmap.zen (uses Vec)
  │       ├─ set.zen (uses Vec)
  │       ├─ queue.zen (uses Vec)
  │       └─ stack.zen (uses Vec)
  │
  ├─→ io.zen (uses Option for read operations)
  │
  ├─→ fs.zen (FFI + types)
  │
  ├─→ net.zen (FFI + types)
  │
  └─→ math.zen (pure math, minimal deps)
```

**Critical Path**: Intrinsics → Allocator → Option/Result → Vec → Collections

---

## Success Criteria

| Metric | Before | After | Status |
|--------|--------|-------|--------|
| Hardcoded stdlib code | 500+ lines | 0 lines | Goal |
| Self-hosted stdlib code | 50 lines | 500+ lines | Goal |
| Compiler primitives | 13 | 15 | Add 2 |
| Tests | 87 ✅ | 130+ ✅ | Goal |
| Type safety | High | Higher | Goal |
| Extensibility | Low | High | Goal |

---

## Key Insight

The architecture is **already correct**. The implementation is **incomplete**.

You have:
- ✅ Good compiler foundation (13 intrinsics)
- ✅ Clear separation of concerns
- ✅ Solid test infrastructure
- ✅ Type system that supports generics

You need:
- ❌ 2 missing intrinsics (null_ptr, sizeof)
- ❌ Self-hosted implementations in Zen
- ❌ Remove Rust stdlib code

This is a **straightforward migration**, not a redesign.

Start with the intrinsics. Then implement String. Then Option/Result. Then Vec. Everything else follows naturally.
