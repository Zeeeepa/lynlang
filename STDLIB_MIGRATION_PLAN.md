# Self-Hosted Stdlib Migration Plan

## Objective
Move language standard library from Rust (hardcoded in src/stdlib/) to Zen (self-hosted in stdlib/*.zen), with compiler only providing low-level LLVM primitives.

## Architecture: Compiler Primitives vs. Stdlib

### Compiler Primitives (Stay in Rust)
These are **unsafe, low-level LLVM operations** that ONLY the compiler provides:

```
Core Primitives:
├── Type System
│   ├── Enum definitions (discriminant tagging)
│   ├── Struct field access (GEP)
│   └── Generic monomorphization
├── Memory Operations
│   ├── @alloc(size) -> Ptr<u8>
│   ├── @free(ptr)
│   ├── @memcpy(dst, src, size)
│   └── @gep(ptr, indices...) -> Ptr<T>
├── Enum Intrinsics
│   ├── @discriminant(enum_val) -> u64
│   ├── @set_discriminant(enum_ptr, tag)
│   ├── @get_payload(enum_val) -> Ptr<T>
│   └── @set_payload(enum_ptr, value)
├── Pointer Operations
│   ├── @ptr_cast<T>(ptr) -> Ptr<T>
│   ├── @offset(ptr, bytes) -> Ptr<T>
│   └── @deref(ptr) -> T
└── Control Flow
    ├── return, break, continue
    ├── Phi node merging (implicit)
    └── Exception handling (future)
```

### Stdlib (Move to Zen)
Everything else is implemented in Zen using primitives:

```
Stdlib Modules:
├── core/
│   ├── option.zen  - Option<T> type and methods
│   ├── result.zen  - Result<T, E> type and methods
│   ├── propagate.zen - Error propagation (?)
│   └── panic.zen   - Panic handling
├── memory/
│   ├── gpa.zen     - General Purpose Allocator
│   ├── arena.zen   - Arena allocator (future)
│   └── pool.zen    - Object pool (future)
├── collections/
│   ├── vec.zen     - Dynamic array
│   ├── hashmap.zen - Hash table
│   ├── set.zen     - Hash set
│   └── queue.zen   - Queue
├── string.zen      - Dynamic string
├── io/io.zen       - I/O operations
├── fs/fs.zen       - File system
└── ...
```

## Migration Phases

### Phase 1: Define Compiler Primitives (IN PROGRESS)

#### 1.1 Expose Intrinsic Functions
Location: `src/codegen/llvm/intrinsics.rs` (create new)

```rust
pub enum Intrinsic {
    // Memory
    Alloc,      // alloc(size: u64) -> Ptr<u8>
    Free,       // free(ptr: Ptr<u8>)
    Memcpy,     // memcpy(dst, src, len)
    Offset,     // offset(ptr, bytes: i64) -> Ptr<u8>
    
    // Enum
    Discriminant,    // discriminant(val) -> u64
    SetDiscriminant, // set_discriminant(ptr, tag: u64)
    GetPayload,      // get_payload(val) -> Ptr<T>
    SetPayload,      // set_payload(ptr, val)
    
    // Pointer
    PtrCast,    // cast<T>(ptr) -> Ptr<T>
    Deref,      // deref(ptr) -> T
    
    // GEP
    Gep,        // gep(ptr, ...indices) -> Ptr<T>
}
```

#### 1.2 Register in Symbol Table
Parser should recognize `@alloc`, `@free`, etc. as builtin functions.

#### 1.3 Codegen Mapping
Map intrinsic calls to LLVM operations:
- `@alloc(size)` → `malloc(size)`
- `@free(ptr)` → `free(ptr)`
- `@discriminant(val)` → Extract field 0 from enum struct
- `@gep(ptr, idx1, idx2)` → `build_gep()` or `build_struct_gep()`

### Phase 2: Move String to Stdlib (Task #14)

**Current State**: 
- stdlib/string.zen exists but incomplete
- src/stdlib/string.rs has Rust implementation

**Action**:
1. Complete stdlib/string.zen with all methods
2. Verify GPA allocation works
3. Remove src/stdlib/string.rs
4. Update imports: `{ String } = @std.string`

**Definition**:
```zen
String: {
    data: Ptr<u8>
    len: u64
    capacity: u64
    allocator: Allocator
}
```

### Phase 3: Move Option/Result to Stdlib (Task #15)

**Current State**:
- Hardcoded in src/stdlib/result.rs
- Option/Result registered in compiler

**Action**:
1. Complete stdlib/core/option.zen
2. Complete stdlib/core/result.zen
3. Remove hardcoded definitions from src/stdlib/result.rs
4. Compiler only recognizes enum syntax

**Before** (Rust):
```rust
pub fn create_option_type() -> AstType {
    AstType::Enum {
        name: "Option".to_string(),
        variants: vec![
            EnumVariant { name: "Some".to_string(), payload: ... },
            EnumVariant { name: "None".to_string(), payload: None },
        ],
    }
}
```

**After** (Zen):
```zen
Option<T>: enum {
    Some: T
    None
}
```

**Compiler only needs**:
- Parse enum syntax
- Allocate enum struct type
- Implement pattern matching
- NO special hardcoded knowledge of Option/Result

### Phase 4: Expose Enum Intrinsics (Task #16)

**Goal**: Allow Zen code to manipulate enums at low level.

**Implementation**:
```zen
// Compiler intrinsic: read discriminant
get_discriminant = (val: T) u64 {
    @discriminant(val)
}

// Compiler intrinsic: set discriminant
set_discriminant = (ptr: Ptr<T>, tag: u64) void {
    @set_discriminant(ptr, tag)
}

// Compiler intrinsic: extract payload
get_payload = (val: T) Ptr<U> {
    @get_payload(val)
}

// Compiler intrinsic: set payload
set_payload = (ptr: Ptr<T>, value: U) void {
    @set_payload(ptr, value)
}
```

Used in stdlib:
```zen
// option.zen
unwrap = (opt: Option<T>) T {
    @get_payload(opt) as Ptr<T> | * // Dereference payload
}

is_some = (opt: Option<T>) bool {
    @discriminant(opt) == SOME_TAG  // where SOME_TAG = 0
}
```

### Phase 5: Expose GEP Primitive (Task #17)

**Goal**: Allow pointer arithmetic from Zen code.

**Implementation**:
```zen
// Compiler intrinsic: Get Element Pointer
gep = (ptr: Ptr<T>, ...indices: u64) Ptr<T> {
    @gep(ptr, indices...)
}
```

**Used in stdlib for arrays**:
```zen
Array<T>.get = (self: Array<T>, index: u64) Option<T> {
    bounds_check(index, self.len) ?
    | true { 
        elem_ptr := @gep(self.data, index)
        Option.Some(*elem_ptr)
    }
    | false { Option.None }
}
```

### Phase 6: Complete Allocator Interface (Task #18)

**Current**: stdlib/memory/gpa.zen partially implemented

**Complete**:
```zen
Allocator: {
    alloc: (size: u64) Ptr<u8>
    free: (ptr: Ptr<u8>) void
    realloc: (ptr: Ptr<u8>, new_size: u64) Ptr<u8>
}

get_default_allocator = () Allocator {
    // Returns global GPA instance
    @get_default_allocator()
}
```

All memory-using types take allocator parameter:
```zen
String.new = (allocator: Allocator) String { ... }
Vec<T>.new = (allocator: Allocator) Vec<T> { ... }
HashMap<K, V>.new = (allocator: Allocator) HashMap<K, V> { ... }
```

## File Structure After Migration

```
src/
├── codegen/
│   └── llvm/
│       ├── intrinsics.rs      [NEW] Compiler primitives
│       └── ... (GEP, enum, memory ops)
└── ... (no stdlib/*.rs hardcoding)

stdlib/
├── core/
│   ├── option.zen      [COMPLETED] Pure Zen
│   ├── result.zen      [COMPLETED] Pure Zen
│   └── panic.zen       [NEW]
├── memory/
│   ├── gpa.zen         [COMPLETED] General Purpose Allocator
│   └── allocator.zen   [NEW] Interface
├── collections/
│   ├── vec.zen         [UPDATED] Uses @alloc/@gep
│   ├── hashmap.zen     [COMPLETED]
│   └── set.zen         [COMPLETED]
├── string.zen          [COMPLETED] Uses String struct
├── io/
├── fs/
└── ... (all pure Zen)
```

## Deliverables Per Task

### #13: Stdlib Migration Plan
- [x] Define compiler primitives needed
- [x] List what moves to Zen
- [x] Identify dependencies

### #14: Move String to Stdlib
- [ ] Complete stdlib/string.zen
- [ ] Remove src/stdlib/string.rs
- [ ] Test String operations

### #15: Eliminate Hardcoded Option/Result
- [ ] Complete stdlib/core/option.zen
- [ ] Complete stdlib/core/result.zen
- [ ] Remove src/stdlib/result.rs
- [ ] Update compiler to not hardcode Option/Result types

### #16: Expose Enum Intrinsics
- [ ] Add `@discriminant`, `@set_discriminant`, etc.
- [ ] Implement in codegen
- [ ] Document in stdlib

### #17: GEP as Compiler Primitive
- [ ] Expose `@gep` intrinsic
- [ ] Use in stdlib Vec/Array implementations
- [ ] Bounds checking in Zen

### #18: Complete Allocator Interface
- [ ] Finish stdlib/memory/gpa.zen
- [ ] Create stdlib/memory/allocator.zen interface
- [ ] Update all collections to use allocator

## Benefits

1. **Simplicity**: Compiler focuses on core LLVM features
2. **Flexibility**: Users can implement custom allocators, collections
3. **Safety**: Bounds checking in Zen, not compiler
4. **Maintainability**: Zen code easier to modify than Rust
5. **Self-hosting Path**: Compiler can be written in Zen eventually

## Risk Mitigation

- **Risk**: GEP intrinsics unsafe if not bounds-checked
  - **Mitigation**: Bounds checking always in Zen stdlib
  
- **Risk**: Performance overhead of Zen-level code
  - **Mitigation**: Intrinsics map to single LLVM ops, zero cost
  
- **Risk**: Circular dependencies (stdlib uses intrinsics that need stdlib)
  - **Mitigation**: Intrinsics are primitives, no dependencies

## Timeline

- Phase 1 (Intrinsics): 1-2 days
- Phase 2 (String): 1 day
- Phase 3 (Option/Result): 2 days
- Phase 4-6 (Enums/GEP/Allocator): 3-4 days
- Testing & Polish: 2-3 days

**Total**: 1-2 weeks for complete migration

