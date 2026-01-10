# Zen Language Roadmap - January 2026

## Current Status: Late Alpha

The core compiler infrastructure is solid. Lexer, parser, type checker, and LLVM codegen work.
All major collection types now use safe `Ptr<T>` pointers with Zig-style allocator semantics.

---

## Memory Management Philosophy (Zig-Style)

Zen follows Zig's allocator pattern for memory management:

```zen
// Allocators are the memory managers - they own the memory
v = Vec<i32>.new(allocator)  // allocator provides memory
v.mut_ref().push(10)
v.mut_ref().free()           // allocator reclaims memory

// Types are "boxed" by allocators, not self-owned
// This enables:
// - Custom allocators (arena, pool, GPA, etc.)
// - Memory tracking and debugging
// - No hidden allocations
// - Explicit lifetime management
```

### Allocator Behavior
```zen
Allocator: behavior {
    allocate: (self: Self, size: usize) i64
    deallocate: (self: Self, ptr: i64, size: usize) void
    reallocate: (self: Self, ptr: i64, old_size: usize, new_size: usize) i64
}
```

All heap-allocated types (Vec, String, HashMap, Stack, Queue, Set) take an
`Allocator` and delegate memory operations to it. This is explicit, not magical.

---

## Priority 1: Complete Safe Pointer Migration

**Status: COMPLETE ✅**

| Component | Status | Notes |
|-----------|--------|-------|
| `Ptr<T>` definition | Done | `stdlib/core/ptr.zen` |
| `String` | Done | Uses `Ptr<u8>`, allocator-aware |
| `Vec<T>` | Done | Uses `Ptr<T>`, allocator-aware |
| `HashMap<K,V>` | Done | Uses `Vec<Entry<K,V>>` internally |
| `Stack<T>` | Done | Uses `Ptr<T>`, allocator-aware |
| `Queue<T>` | Done | Uses `Ptr<T>` (circular buffer) |
| `Set<T>` | Done | Wraps `HashMap<T, bool>` |

All collection types now use safe pointers with explicit allocator management.

---

## Priority 2: Standard Library Hardening

**Status: Collections Complete, I/O Core Complete ✅**

### Core Types (Done)
- [x] `Option<T>` - fully integrated
- [x] `Result<T,E>` - fully integrated
- [x] `String` - growable, allocator-aware
- [x] `Vec<T>` - growable, allocator-aware
- [x] `HashMap<K,V>` - linear probing, FNV-1a hash

### Collections (Done)
- [x] `Stack<T>` - LIFO with push/pop/peek
- [x] `Queue<T>` - circular buffer with enqueue/dequeue
- [x] `Set<T>` - wraps HashMap<T, bool>
- [x] `LinkedList<T>` - doubly-linked list with iterator

### I/O (Syscall Layer)
- [x] `File` - syscall-based file operations (Linux x86-64)
- [x] `TcpListener` / `TcpStream` - syscall-based TCP (no FFI)
- [x] `UdpSocket` - syscall-based UDP (no FFI)
- [ ] Darwin/Windows syscall support

**Note:** Avoid FFI for I/O. Build native syscall wrappers:
```zen
// Goal: Direct syscalls, not libc wrappers
File.open = (path: String, flags: i32) Result<File, IoError>
File.read = (self: MutPtr<File>, buf: Ptr<u8>, len: usize) Result<usize, IoError>
```

### Known Limitation
Runtime tests for collections require the module import system to be complete.
Currently, complex module imports (e.g., `{ GPA } = @std.memory.gpa`) cause
"Unresolved generic type" errors during monomorphization. Rust tests verify compilation.

---

## Priority 3: Iterator System

**Status: PARTIAL ✅**

Design an iterator trait/behavior system:
```zen
Iter<T>: behavior {
    next: (self: MutPtr<Self>) Option<T>
}

// Enable: vec.iter().map(fn).filter(fn).collect()
```

### Tasks
- [x] Design `Range` iterator with `next()` method
- [x] Implement `VecIterator<T>` for `Vec<T>`
- [x] Add iterator combinators for Range: `sum`, `product`, `min`, `max`, `skip`, `take`
- [x] Add predicate methods: `any_ge`, `all_lt`, `find_ge`
- [x] Implement `HashMapIterator<K,V>` with `iter()`, `keys()`, `values()`
- [ ] Add `map`, `filter`, `fold`, `collect` combinators (needs closures)

**Note:** Full functional iterator chains (`.map(fn).filter(fn).collect()`) require
first-class closures which are not yet implemented. Current iterator methods are
specialized for common operations.

---

## Priority 4: Well-Known Types Refactor

**Status: Partial**

The compiler has hardcoded checks like `if name == "Option"`. Should use the
`WellKnownTypes` registry consistently.

### Files to Audit
- `src/codegen/llvm/expressions/enums.rs`
- `src/typechecker/mod.rs`
- `src/typechecker/inference.rs`

---

## Priority 5: FFI & Interop

**Status: Basic Working**

- [x] `load_library` / `get_symbol` - works
- [ ] `call_external` - stub in codegen
- [ ] `inline_c` - shells to clang (fragile)
- [ ] Struct layout compatibility with C

---

## Priority 6: Module System Improvements

**Status: Needs Work**

The module import system has issues with:
- Generic type resolution across module boundaries
- Monomorphization of imported generic types
- Type inference for behavior implementations

This blocks runtime testing of collections from .zen files.

---

## Priority 7: LSP Improvements

**Status: COMPLETE ✅**

### Full Feature Support
- [x] Hover provider with type info
- [x] Go-to-definition (including nested member access)
- [x] Type definition navigation
- [x] Find all references
- [x] Document highlight
- [x] Code completion with trigger characters (`.`, `:`, `@`, `?`)
- [x] Signature help for function calls
- [x] Document symbols
- [x] Workspace symbols
- [x] Code actions
- [x] Code lens
- [x] Document formatting
- [x] Rename with prepare provider
- [x] Folding ranges
- [x] Inlay hints
- [x] Call hierarchy (incoming/outgoing)
- [x] Semantic tokens (full + delta)
- [x] Incremental text sync

### Fixed Issues
- [x] Nested member access chains (`self.val.data`) now resolve
- [x] Generic type field resolution works (struct fields indexed)
- [x] MutPtr<T>.val recognized as built-in field
- [x] Parameter type inference (e.g., `allocator: Allocator` -> `allocator.allocate`)
- [x] Local variable type inference from constructors

---

## Priority 8: Architecture Cleanup

**Status: IN PROGRESS**

### Completed ✅
- [x] Removed duplicate module declarations from `main.rs`
- [x] Deleted dead FFI module (`src/ffi/` - 1,455 LOC)
- [x] Deleted dead behaviors module (`src/behaviors/` - ~400 LOC)
- [x] Deleted `vec_support.rs` (326 LOC)
- [x] Deleted `stdlib_codegen/collections.rs` (670 LOC)
- [x] Integrated typechecker into main compilation pipeline

### Remaining
- [ ] Remove duplicate type inference from codegen (~1,000 LOC)
- [ ] Add type annotations to AST nodes (enable codegen to trust typechecker)
- [ ] Fix hardcoded generics in GenericTypeTracker
- [ ] Split giant modules (codegen/ 11.6K, lsp/ 12K)

---

## Recent Completions (This Session)

1. **Iterator System Enhanced** - Range now has sum, product, min, max, skip, take, find_ge
2. **VecIterator<T>** - Full iterator for Vec<T> with next() and has_next()
3. **HashMapIterator<K,V>** - Iterates over key-value pairs, plus keys() and values() iterators
4. **StackIterator<T>** - Forward and reverse iterators for Stack
5. **QueueIterator<T>** - Front-to-back iteration
6. **SetIterator<T>** - Element iteration
7. **Syscall-based File I/O** - Linux x86-64 file operations without FFI (474 LOC)
8. **LSP Complete** - All modern LSP features implemented
9. **Architecture Cleanup** - Fixed duplicate module declarations
10. **Dead Code Audit** - Identified legitimate vs false positive dead_code markers
11. **Typechecker Integration** - Now in main compilation pipeline
12. **Pattern Matching Refactor** - Extracted to dedicated patterns.rs module (290 LOC)
13. **Module Size Reduction** - codegen/llvm/mod.rs: 992 → 702 LOC (-29%)
14. **Parser Module Split** - primary.rs: 943 → 802 LOC, control_flow.rs now used
15. **Semaphore** - Counting semaphore with acquire/release/try_acquire
16. **Barrier** - Thread synchronization barrier for phase-based parallel algorithms
17. **Channel<T>** - Bounded MPMC message queue for thread communication
18. **Unix Domain Sockets** - UnixListener, UnixStream, UnixDatagram, SocketPair
19. **Documentation Cleanup** - Fixed dead links, updated STDLIB_DESIGN.md
20. **AsyncAllocator** - Behavior interface for async-capable allocators
21. **AsyncPool** - io_uring-based async allocator with completion callbacks
22. **Task System** - Stackful coroutines for async task management
23. **Scheduler** - Task scheduler with runnable/suspended queues

---

## Testing Commands

```bash
# Build compiler
cargo build --release

# Run all Rust tests
cargo test

# Run allocator-specific tests
cargo test --test allocator_compilation

# Run demo project
./target/release/zen examples/demo_project/main.zen

# Run collections status check
./target/release/zen examples/test_collections.zen
```

---

## Architectural Goals

### Target Pipeline
```
Source
  ↓
Lexer (lexer.rs)
  ↓
Parser (parser/)
  ↓
═══════════════════════════
  SEMA (semantic analysis)
═══════════════════════════
  ├─ process_imports()
  ├─ execute_comptime()
  ├─ resolve_self_types()
  ├─ typecheck() ✅ NOW INTEGRATED
  └─ monomorphize()
═══════════════════════════
  ↓
Codegen (no type decisions!)
  ↓
LLVM IR
```

### Target Metrics
| Metric | Current | Target |
|--------|---------|--------|
| Total LOC | ~41,000 | < 35,000 |
| Dead code | ~0 | 0 |
| Max module LOC | 11,691 | < 2,000 |
| Typechecker integration | ✅ Done | Required |

---

## Self-Hosting Path

To achieve self-hosting, Zen needs:

1. **Intrinsics Only** - Rust compiler provides minimal intrinsics
2. **Stdlib in Zen** - All features built using intrinsics
3. **Parser in Zen** - Rewrite lexer/parser
4. **Typechecker in Zen** - Rewrite semantic analysis
5. **Codegen in Zen** - Either LLVM bindings or custom backend

Current intrinsics are a solid foundation:
- Memory: `raw_allocate`, `raw_deallocate`, `memcpy`, etc.
- Pointers: `gep`, `gep_struct`, `ptr_to_int`, etc.
- Types: `sizeof<T>`, `alignof<T>`
- Syscalls: `syscall0` - `syscall6`
- Atomics: `atomic_load`, `atomic_store`, `atomic_cas`, etc.
