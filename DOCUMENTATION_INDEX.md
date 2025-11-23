# Zen Language Documentation Index

**Last Updated**: 2025-11-20  
**Project Status**: Stdlib self-hosting in progress (4/20 tasks complete)

---

## ⭐ START HERE: Understanding the Architecture

**New to the codebase?** Read these in order:
1. **[ARCHITECTURE.md](ARCHITECTURE.md)** - Learn the difference between LLVM primitives and Zen features
2. **[PRIMITIVES_VS_FEATURES.md](PRIMITIVES_VS_FEATURES.md)** - Quick reference: where do I implement something?
3. **[PRIMITIVE_EXAMPLES.md](PRIMITIVE_EXAMPLES.md)** - See concrete examples from actual code

This is essential for understanding where new features belong.

---

## Quick Navigation

### 🚀 Getting Started
- **[NEXT_STEPS.md](NEXT_STEPS.md)** - What's done, what needs work, quick checklist
- **[README.md](README.md)** - Project overview and goals
- **[IMMEDIATE_NEXT_STEPS.md](IMMEDIATE_NEXT_STEPS.md)** - Action items for today

### 📊 Current Status
- **[STATUS_CURRENT.md](STATUS_CURRENT.md)** - Session status, metrics, progress
- **[SESSION_SUMMARY.md](SESSION_SUMMARY.md)** - What was accomplished this session
- **[INTRINSICS_REFERENCE.md](INTRINSICS_REFERENCE.md)** - Compiler primitives documentation

### 🏗️ Architecture & Design
- **[ARCHITECTURE.md](ARCHITECTURE.md)** - LLVM primitives vs Zen-level features (ESSENTIAL READ)
- **[docs/stdlib-architecture.md](docs/stdlib-architecture.md)** - Stdlib folder structure and organization
- **[PRIMITIVES_VS_FEATURES.md](PRIMITIVES_VS_FEATURES.md)** - Quick reference for where to implement features
- **[PRIMITIVE_EXAMPLES.md](PRIMITIVE_EXAMPLES.md)** - Concrete code examples from the codebase
- **[DESIGN_NOTES.md](DESIGN_NOTES.md)** - Historical design decisions

### 📚 Design Documentation (in design/ folder)
- **[design/SAFE_POINTERS_DESIGN.md](design/SAFE_POINTERS_DESIGN.md)** - Ptr<T> vs Ref<T> rationale
- **[design/SAFE_TYPE_SYSTEM_DESIGN.md](design/SAFE_TYPE_SYSTEM_DESIGN.md)** - Type system architecture
- **[design/STDLIB_ARCHITECTURE_REVIEW.md](design/STDLIB_ARCHITECTURE_REVIEW.md)** - Stdlib design
- **[design/STDLIB_IMPLEMENTATION_ROADMAP.md](design/STDLIB_IMPLEMENTATION_ROADMAP.md)** - Phased plan
- **[design/STDLIB_WORK_BREAKDOWN.md](design/STDLIB_WORK_BREAKDOWN.md)** - Detailed code breakdown

### 🔧 How-To Guides
- **[VSCODE_EXTENSION_SETUP.md](VSCODE_EXTENSION_SETUP.md)** - Editor integration

---

## What's Implemented

### Core Language Features ✅
- Parser and typechecker
- LLVM-based code generation
- Generic types (T, Option<T>, etc.)
- Enums and pattern matching
- Allocator abstractions
- GPA (General Purpose Allocator)

### Standard Library - Stdlib ✅
```
core/
  ├─ ptr.zen       → Ptr<T> + Ref<T> (type-safe pointers)
  ├─ option.zen    → Option<T> enum
  └─ propagate.zen → Error propagation

memory/
  ├─ allocator.zen → Allocator trait
  └─ gpa.zen       → GPA allocator

string.zen        → String with Ptr<u8>
vec.zen           → Vec<T> with Ptr<T>

[Other modules in progress...]
```

### Compiler Intrinsics (12 total) ✅
```
Memory:          raw_allocate, raw_deallocate, raw_reallocate
Pointers:        gep, gep_struct, raw_ptr_cast (+ deprecated)
Types:           sizeof<T>()
Enums:           discriminant, set_discriminant, get_payload, set_payload
```

---

## What's In Progress

### Phase 1: Type System Safety ⏳
- Ptr<T> and Ref<T> integration with compiler
- Remove raw * pointers from public API
- Force pattern matching for null handling

### Phase 2: Collections 📋
- Vec<T> iteration support
- HashMap<K,V> implementation
- Set<T>, Queue<T>, Stack<T>

### Phase 3: Advanced Features 📅
- String methods (concat, split, trim, format)
- Iterator trait and implementations
- Specialized allocators (ArenaAllocator, PoolAllocator)

---

## Compiler Intrinsics Reference

### Memory Operations
```zen
{ compiler } = @std

// Allocate raw memory (bytes)
addr = compiler.raw_allocate(1024)  // -> *u8

// Deallocate
compiler.raw_deallocate(addr, 1024)

// Resize
new_addr = compiler.raw_reallocate(addr, 1024, 2048)
```

### Pointer Arithmetic
```zen
// Get element pointer (byte offset)
elem_ptr = compiler.gep(base_addr, 8)  // base + 8 bytes

// Struct field pointer
field_ptr = compiler.gep_struct(struct_ptr, 2)  // field 2

// Type cast
casted = compiler.raw_ptr_cast(ptr)
```

### Type Introspection
```zen
// Size of type (bytes)
size = compiler.sizeof(i32)    // -> 4
size = compiler.sizeof(u64)    // -> 8

// Note: Essential for generic containers
//       Vec<T> uses sizeof(T) to calculate offsets
```

### Enum Operations
```zen
// Get variant tag (discriminant)
tag = compiler.discriminant(&my_enum)

// Set variant tag
compiler.set_discriminant(&my_enum, 1)

// Get variant payload pointer
payload = compiler.get_payload(&my_enum)

// Set payload
compiler.set_payload(&my_enum, new_payload_ptr)
```

---

## Stdlib Type Reference

### Ptr<T> - Owned Pointer

**Usage**: Wraps allocations, owns memory, must be freed

```zen
{ ptr } = @std.core
{ gpa } = @std.memory

allocator = gpa.default_gpa()

// Create
p = ptr.ptr_allocate(allocator, 10)  // Allocate 10 items

// Check
ptr.ptr_is_some(p) ?
| true { io.println("Valid") }
| false { io.println("Null") }

// Read (safe)
ptr.ptr_value(p) ?
| Some(value) { io.println("${value}") }
| None { io.println("No value") }

// Dereference (unsafe)
value = ptr.ptr_unwrap(p)  // Panics if null

// Offset (pointer arithmetic)
p2 = ptr.ptr_offset(p, 5)  // Advance 5 items

// Deallocate (consumed)
ptr.ptr_free(&p, allocator, 10)
```

### Ref<T> - Borrowed Reference

**Usage**: Stack borrow, validates before access, not owned

```zen
{ ptr } = @std.core

// Create
r = ptr.ref_from(some_addr)

// Check validity
ptr.ref_is_valid(r) ?
| true { io.println("Valid reference") }
| false { io.println("Invalid") }

// Read (safe)
ptr.ref_value(r) ?
| Some(value) { io.println("${value}") }
| None { io.println("Invalid reference") }
```

### String - Growable Byte String

**Usage**: Dynamic text storage

```zen
{ string } = @std
{ gpa } = @std.memory

allocator = gpa.default_gpa()

// Create
s = string.string_new(allocator)

// Properties
string.string_len(s)        // Current length
string.string_capacity(s)   // Allocated capacity
string.string_is_empty(s)   // Check if empty

// Read
string.string_at(s, 0) ?
| Some(byte) { io.println("First byte: ${byte}") }
| None { io.println("Empty") }

// Write
string.string_push(&s, 72)      // Append byte (H = 72)
string.string_push(&s, 105)     // Append i

// Modify
string.string_pop(&s) ?
| Some(byte) { io.println("Removed: ${byte}") }
| None { io.println("Already empty") }

string.string_clear(&s)         // Reset but keep capacity

// Clone
s2 = string.string_clone(s, allocator)

// Cleanup
string.string_free(&s)
string.string_free(&s2)
```

### Vec<T> - Growable Generic Array

**Usage**: Dynamic strongly-typed collections

```zen
{ vec } = @std
{ gpa } = @std.memory

allocator = gpa.default_gpa()

// Create
numbers = vec.vec_new(allocator)
numbers = vec.vec_with_capacity(allocator, 100)  // Pre-allocate

// Properties
vec.vec_len(numbers)            // Current length
vec.vec_capacity(numbers)       // Allocated capacity
vec.vec_is_empty(numbers)       // Check if empty

// Read
vec.vec_get(numbers, 0) ?
| Some(n) { io.println("First: ${n}") }
| None { io.println("Empty") }

vec.vec_first(numbers) ?
| Some(n) { io.println("First: ${n}") }
| None {}

vec.vec_last(numbers) ?
| Some(n) { io.println("Last: ${n}") }
| None {}

// Write
vec.vec_push(&numbers, 42)      // Append
vec.vec_push(&numbers, 99)
vec.vec_push(&numbers, 7)

// Insert/Remove
vec.vec_insert(&numbers, 1, 50) // Insert 50 at index 1
vec.vec_remove(&numbers, 1)     // Remove index 1

// Modify
vec.vec_pop(&numbers) ?         // Remove last
| Some(n) { io.println("Popped: ${n}") }
| None {}

vec.vec_clear(&numbers)         // Reset but keep capacity

// Memory management
vec.vec_reserve(&numbers, 50)        // Ensure space
vec.vec_shrink_to_fit(&numbers)      // Trim unused space

// Clone
numbers2 = vec.vec_clone(numbers, allocator)

// Cleanup
vec.vec_free(&numbers)
vec.vec_free(&numbers2)
```

### Option<T> - Maybe Value

**Usage**: Represent presence/absence of value

```zen
{ option } = @std.core

// Constructors
Some(42)
None

// Pattern matching (forced)
maybe_val ?
| Some(val) { io.println("Has: ${val}") }
| None { io.println("No value") }

// Helpers (if needed)
// option_is_some, option_is_none, option_map, etc.
```

---

## Code Organization

### Source Tree
```
src/
├── lexer/              # Tokenization
├── parser/             # Syntax analysis
├── typechecker/        # Type checking & inference
├── codegen/            # Code generation
│   └── llvm/           # LLVM backend
│       ├── functions/
│       │   └── stdlib/ # Standard library implementation
│       └── ...
├── stdlib/             # Intrinsic definitions (Rust)
└── ...

stdlib/                # Standard library (Zen code)
├── core/              # Core types (Ptr, Option, etc.)
├── memory/            # Memory management (Allocator, GPA)
├── collections/       # Collections (HashMap, Vec, etc.)
├── io/                # I/O operations
├── string.zen         # String type
├── vec.zen            # Vector type
└── ...

tests/                 # Integration tests
└── *.rs               # Test files

design/               # Design documentation
└── *.md              # Design docs
```

---

## Testing

### Run All Tests
```bash
cargo test --all
```

### Test Categories (87 total)
- Parser Tests: 10
- Lexer Tests: 2
- Parser Integration: 10
- LSP Text Edit: 11
- Codegen Integration: 8
- Unit Tests: 3
- Enum Intrinsics: 10
- GEP Intrinsics: 10
- Other: 23

### Status
✅ **87/87 passing** (100% pass rate)  
✅ **0 failures** (no regressions)  
✅ **0 skipped**  

---

## Performance Notes

### Compilation
- ~15 seconds for full build (dev profile)
- ~30 seconds for release profile

### Runtime
- Vec<T> operations: O(1) amortized for push
- String operations: O(n) for mutable operations
- Pointer arithmetic: O(1)
- No GC overhead

### Memory
- Ptr<T>: 16 bytes (8 byte pointer + 8 byte enum tag)
- Ref<T>: 16 bytes (8 byte pointer + 8 byte validity)
- String: 32 bytes overhead (3 fields + allocator)
- Vec<T>: 32 bytes overhead (same as String)

---

## Known Limitations

1. **set_payload** - Placeholder, needs size information
2. **gep_struct** - Assumes 8-byte alignment
3. **FFI Intrinsics** - Not yet implemented (load_library, get_symbol, unload_library)
4. **inline_c** - C code embedding not implemented
5. **String methods** - Basic operations only, advanced methods pending

---

## Success Criteria Checklist

### Current Status
- ✅ Ptr<T> and Ref<T> types implemented
- ✅ String uses Ptr<u8>
- ✅ Vec<T> fully implemented with Ptr<T>
- ✅ All 87 tests passing
- ✅ Zero regressions
- ✅ Build clean (warnings pre-existing)
- ✅ Documentation complete
- ✅ Type-safe allocations throughout

### Next Goals
- [ ] Integration tests (String + Vec together)
- [ ] Allocator stress tests
- [ ] Iterator support
- [ ] Collections (HashMap, Set, etc.)
- [ ] Advanced string methods

---

## Document Maintenance

### Active Documents (in root)
Update these regularly:
- NEXT_STEPS.md
- STATUS_CURRENT.md
- SESSION_SUMMARY.md
- INTRINSICS_REFERENCE.md

### Design Documents (in design/)
Reference these during implementation:
- SAFE_POINTERS_DESIGN.md
- STDLIB_ARCHITECTURE_REVIEW.md
- STDLIB_WORK_BREAKDOWN.md

### Archived/Historical
These are for reference only:
- DESIGN_NOTES.md
- REVISED_NEXT_STEPS.md
- ARCHITECTURE_DIAGRAM.md

---

## Quick Links

### Build & Test
```bash
cargo build          # Compile
cargo test --all     # Run all tests
cargo test --lib     # Unit tests only
cargo test pattern   # Specific tests
```

### File Locations
```
Compiler Intrinsics:  src/stdlib_metadata/compiler.rs
Codegen:              src/codegen/llvm/functions/
Standard Library:     stdlib/
Tests:                tests/
Design Docs:          design/
```

### Key Functions to Study
1. Ptr<T> - stdlib/core/ptr.zen
2. String - stdlib/string.zen  
3. Vec<T> - stdlib/vec.zen
4. Allocator - stdlib/memory/allocator.zen
5. GPA - stdlib/memory/gpa.zen

---

## Getting Help

### If You Get Stuck
1. Check **NEXT_STEPS.md** for current priorities
2. Read **design/STDLIB_WORK_BREAKDOWN.md** for detailed code
3. Look at **INTRINSICS_REFERENCE.md** for primitive docs
4. Review **tests/** for usage examples
5. Check **STATUS_CURRENT.md** for known issues

### Common Questions

**Q: How do I use Ptr<T> safely?**  
A: Always pattern match on Some/None, use ptr_value() for safe access

**Q: Why not use raw pointers (*T)?**  
A: They're unsafe - can cause silent crashes. Ptr<T> forces null checking.

**Q: How do I allocate memory?**  
A: Use ptr.ptr_allocate() with an allocator, or gpa.default_gpa()

**Q: What's the difference between Ptr<T> and Ref<T>?**  
A: Ptr<T> owns memory (must free), Ref<T> borrows (stack lifetime)

---

**Last Updated**: 2025-11-19  
**Maintainer**: Amp  
**Status**: 🟢 Active (Stdlib self-hosting in progress)
