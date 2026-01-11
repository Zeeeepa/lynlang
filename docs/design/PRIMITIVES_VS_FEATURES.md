# Zen Compiler Architecture

This document covers the separation between LLVM primitives and Zen-level features,
including decision trees for where to implement new functionality and concrete examples.

---

## Overview

The Zen compiler has a clear separation between:

1. **LLVM Primitives** - Low-level operations that generate direct LLVM IR
2. **Zen-Level Features** - High-level language constructs built on top of primitives

---

## Quick Decision: Primitive or Feature?

| Question | Answer | → |
|----------|--------|---|
| Does it call malloc/free/libc? | YES | LLVM Primitive |
| Does it need LLVM GEP/sizeof? | YES | LLVM Primitive |
| Can it be implemented with existing primitives/types? | YES | Zen Feature |
| Is it a data structure (Vec, String, Ptr)? | YES | Zen Feature |
| Is it a wrapper around a primitive? | YES | Zen Feature |
| Is it purely a type definition (enum, struct)? | YES | Zen Feature |

---

## LLVM Primitives (Built-In)

LLVM primitives are **compiler intrinsics** defined in Rust that directly generate LLVM IR. They cannot be implemented in Zen - they must be implemented in the Rust compiler.

### Location
```
src/stdlib_metadata/          - Function declarations/metadata
src/codegen/llvm/stdlib_codegen/  - LLVM IR generation
```

### Current LLVM Primitives (13 total)

#### Memory Operations
```rust
// src/stdlib_metadata/compiler.rs defines these:
raw_allocate(size: usize) -> *u8
raw_deallocate(ptr: *u8, size: usize) -> void
raw_reallocate(ptr: *u8, old_size: usize, new_size: usize) -> *u8
```

**Implemented in**: `src/codegen/llvm/stdlib_codegen/compiler.rs:compile_raw_allocate()`

**Why LLVM primitive**: Must call C `malloc`/`free` which requires external function linking

#### Pointer Operations
```rust
gep(base: *u8, offset: i64) -> *u8          // GetElementPointer
gep_struct(ptr: *u8, field: i32) -> *u8     // GEP for struct fields
raw_ptr_cast(ptr: *u8) -> *u8               // Type reinterpret (0-cost)
```

**Implemented in**: `src/codegen/llvm/stdlib_codegen/compiler.rs:compile_gep()`

**Why LLVM primitive**: Direct LLVM GEP instruction, no Zen equivalent

#### Type Introspection
```rust
sizeof<T>() -> usize    // Size in bytes
```

**Implemented in**: `src/codegen/llvm/stdlib_codegen/compiler.rs:compile_sizeof()`

**Why LLVM primitive**: Compile-time type layout information

#### Enum Operations
```rust
discriminant(enum_ptr: *u8) -> i32
set_discriminant(enum_ptr: *u8, value: i32) -> void
get_payload(enum_ptr: *u8) -> *u8
set_payload(enum_ptr: *u8, data: *u8) -> void
```

**Implemented in**: `src/codegen/llvm/stdlib_codegen/compiler.rs:compile_discriminant()`

**Why LLVM primitive**: Direct access to enum layout (discriminant at offset 0, payload at offset 8)

---

## Zen-Level Features (Can Be Implemented in Zen)

These are built **on top of** LLVM primitives using Zen code. They wrap primitives to provide safe, higher-level abstractions.

### Location
```
stdlib/                     - Zen implementations
src/stdlib_metadata/        - Rust declarations/metadata
src/codegen/llvm/stdlib_codegen/  - Code generation
```

### Current Zen-Level Features

#### 1. **Option<T>** - Maybe value
```zen
// stdlib/core/option.zen
Option<T>: Some: T, None
```

**Uses**: Pattern matching (built-in)  
**Implementation**: Pure enum, no special code needed  
**Can move to Zen**: YES (purely a type definition)

#### 2. **Result<T, E>** - Error handling
```zen
// stdlib/core/result.zen
Result<T, E>: Ok: T, Err: E
```

**Uses**: Pattern matching (built-in)  
**Implementation**: Pure enum  
**Can move to Zen**: YES

#### 3. **Allocator** - Memory management trait
```zen
// stdlib/memory/allocator.zen
Allocator: {
    allocate: (self, size: usize) Option<*u8>,
    deallocate: (self, ptr: *u8, size: usize) void,
}
```

**Uses**: `raw_allocate`, `raw_deallocate` (LLVM primitives)  
**Implementation**: Trait definition  
**Can move to Zen**: PARTIALLY (trait definition yes, GPA implementation needs compiler intrinsics)

#### 4. **GPA (General Purpose Allocator)**
```zen
// stdlib/memory/gpa.zen
GPA: {
    allocate: (self, size: usize) Option<*u8> {
        compiler.raw_allocate(size)
    },
    deallocate: (self, ptr: *u8, size: usize) {
        compiler.raw_deallocate(ptr, size)
    },
}
```

**Uses**: `raw_allocate`, `raw_deallocate` (LLVM primitives)  
**Implementation**: Wrapper around malloc/free  
**Can move to Zen**: YES (it's just calling primitives)

#### 5. **Ptr<T>** - Type-safe owned pointer
```zen
// stdlib/core/ptr.zen
Ptr<T>: Some: i64, None

ptr_allocate = (alloc: Allocator, count: usize) Ptr<T> {
    size = compiler.sizeof(T) * count
    alloc.allocate(size) ?
        | Some(raw_ptr) { Ptr.Some(raw_ptr as i64) }
        | None { Ptr.None }
}

ptr_value = (p: Ptr<T>) Option<T> {
    p ?
        | Some(addr) { 
            loaded = compiler.gep(addr as *u8, 0)
            Some(load loaded)  // Zen pseudocode
        }
        | None { None }
}
```

**Uses**: LLVM primitives (`sizeof`, `gep`)  
**Implementation**: Wrapper with type safety  
**Can move to Zen**: PARTIALLY (core logic yes, but might need some intrinsics)

#### 6. **String** - Dynamic text
```zen
// stdlib/string.zen
String: {
    data: Ptr<u8>,
    len: usize,
    capacity: usize,
    allocator: Allocator,
}

string_push = (str: *String, byte: u8) {
    if str.len >= str.capacity {
        string_grow(str)
    }
    ptr = compiler.gep(str.data.addr as *u8, str.len)
    store(ptr, byte)
    str.len = str.len + 1
}
```

**Uses**: LLVM primitives (`gep`), Ptr<T>, Allocator  
**Implementation**: Growable buffer  
**Can move to Zen**: YES (it's just buffer management)

#### 7. **Vec<T>** - Generic growable array
```zen
// stdlib/vec.zen
Vec<T>: {
    data: Ptr<T>,
    len: usize,
    capacity: usize,
    allocator: Allocator,
}

vec_push = (vec: *Vec<T>, elem: T) {
    if vec.len >= vec.capacity {
        vec_grow(vec)  // Call growth logic
    }
    ptr = compiler.gep(vec.data.addr as *u8, vec.len * compiler.sizeof(T))
    store(ptr, elem)
    vec.len = vec.len + 1
}
```

**Uses**: LLVM primitives (`sizeof`, `gep`), Ptr<T>, Allocator  
**Implementation**: Type-safe growable array  
**Can move to Zen**: YES

#### 8. **IO Functions** - Print/read
```zen
// Built-in (needs special handling)
io.println(message: string) -> void
io.print(message: string) -> void
io.read_line() -> Result<string, string>
```

**Uses**: System calls (write, read)  
**Implementation**: External C library calls  
**Can move to Zen**: NO (requires libc integration)

#### 9. **Math Functions** - Arithmetic
```zen
// stdlib/math/sin, cos, sqrt, etc.
```

**Uses**: libm (C math library)  
**Implementation**: Wraps C functions  
**Can move to Zen**: NO (requires libm)

---

## Decision Tree: Is This an LLVM Primitive?

```
Does it need to...
  ├─ Call external C functions (malloc, libc)?  → YES, LLVM primitive
  ├─ Access LLVM IR directly (GEP, sizeof)?     → YES, LLVM primitive
  ├─ Link external libraries (libm, libc)?      → YES, LLVM primitive
  └─ No to all above?                           → NO, build in Zen
```

---

## Current State: What Can Be Moved to Zen?

### Already Movable (Pure Zen)
- ✅ **Option<T>** - Pure enum type
- ✅ **Result<T, E>** - Pure enum type
- ✅ **GPA** - Thin wrapper around malloc
- ✅ **Vec<T>** - Buffer management using primitives
- ✅ **String** - Buffer management using primitives
- ✅ **Ptr<T>** - Pointer safety wrapper

### Not Movable (Need LLVM)
- ❌ **raw_allocate** - Calls C malloc
- ❌ **raw_deallocate** - Calls C free
- ❌ **gep** - LLVM GEP instruction
- ❌ **sizeof** - Compile-time layout info
- ❌ **io.println** - System call (write)
- ❌ **io.read_line** - System call (read)
- ❌ **Math functions** - Call libm

---

## Architecture Layers

```
┌─────────────────────────────────────┐
│  Zen User Code                      │  Uses: Zen language
├─────────────────────────────────────┤
│  Standard Library (Zen)             │  String, Vec, Ptr, Allocator
│  - stdlib/string.zen                │  Uses: LLVM primitives + types
│  - stdlib/vec.zen                   │
│  - stdlib/core/ptr.zen              │
├─────────────────────────────────────┤
│  LLVM Primitives (Rust)             │  raw_allocate, gep, sizeof, etc.
│  - @std.compiler intrinsics         │  Uses: LLVM IR + C linkage
├─────────────────────────────────────┤
│  LLVM Backend                       │  Code generation
│  - Inkwell (Rust LLVM bindings)     │
├─────────────────────────────────────┤
│  LLVM IR                            │  Intermediate representation
├─────────────────────────────────────┤
│  Machine Code                       │  CPU instructions
└─────────────────────────────────────┘
```

---

## File Organization Reference

### Standard Library (Declarations)
```
src/stdlib_metadata/
├─ compiler.rs      ← LLVM primitives (allocate, gep, sizeof, etc.)
├─ io.rs            ← IO functions (println, read_line)
├─ math.rs          ← Math functions (sin, cos, sqrt)
├─ core.rs          ← Core types (Option, Result)
└─ vec.rs           ← Collection types
```

### Code Generation (Implementation)
```
src/codegen/llvm/stdlib_codegen/
├─ compiler.rs      ← Generates LLVM IR for raw_allocate, gep, sizeof
├─ io.rs            ← Generates calls to libc write/read
├─ math.rs          ← Generates calls to libm
└─ core.rs          ← No generation needed (pure types)
```

### Zen Implementations
```
stdlib/
├─ core/
│  ├─ ptr.zen       ← Ptr<T> wrapper (uses gep, sizeof)
│  ├─ option.zen    ← Option<T> enum
│  └─ ...
├─ memory/
│  ├─ allocator.zen ← Trait definition
│  └─ gpa.zen       ← Wraps raw_allocate/raw_deallocate
├─ string.zen       ← Uses gep, sizeof, Allocator
├─ vec.zen          ← Uses gep, sizeof, Allocator
└─ ...
```

---

## Best Practices

### When Adding a New Feature

**1. Is it built on existing types?**
   → Implement in Zen (`stdlib/*.zen`)

**2. Does it need direct hardware access or C functions?**
   → Implement as LLVM primitive (Rust in `src/stdlib_metadata/` + `src/codegen/llvm/stdlib_codegen/`)

**3. Does it need both?**
   → Create Zen wrapper around LLVM primitive
   ```rust
   // src/stdlib_metadata/my_module.rs - declare primitive
   // src/codegen/llvm/stdlib_codegen/my_module.rs - implement in LLVM
   // stdlib/my_module.zen - wrap with safe API
   ```

### Example: Adding `String.concat()`

**Step 1**: Declare in Zen wrapper (stdlib/string.zen)
```zen
string_concat = (s1: String, s2: String, alloc: Allocator) String {
    result = string_new(alloc)
    // Copy s1 bytes
    (0..string_len(s1)).loop((i) {
        string_push(&result, string_at(s1, i))
    })
    // Copy s2 bytes
    (0..string_len(s2)).loop((i) {
        string_push(&result, string_at(s2, i))
    })
    return result
}
```

This doesn't need an LLVM primitive - it uses existing `string_push`, which already uses `gep`.

### Example: Adding `Float.sin()`

**Step 1**: Declare in Rust (src/stdlib_metadata/math.rs)
```rust
functions.insert(
    "sin".to_string(),
    StdFunction {
        name: "sin".to_string(),
        params: vec![("x".to_string(), AstType::F64)],
        return_type: AstType::F64,
        is_builtin: true,
    },
);
```

**Step 2**: Implement LLVM code generation (src/codegen/llvm/stdlib_codegen/math.rs)
```rust
pub fn compile_sin<'ctx>(
    compiler: &mut LLVMCompiler<'ctx>,
    args: &[ast::Expression],
) -> Result<BasicValueEnum<'ctx>, CompileError> {
    let x = compiler.compile_expression(&args[0])?;
    
    // Get libm sin function
    let sin_fn = compiler.module.get_function("sin").unwrap_or_else(|| {
        let f64_type = compiler.context.f64_type();
        let fn_type = f64_type.fn_type(&[f64_type.into()], false);
        compiler.module.add_function("sin", fn_type, Some(Linkage::External))
    });
    
    let result = compiler.builder.build_call(sin_fn, &[x], "sin_result")?
        .try_as_basic_value().left().unwrap();
    
    Ok(result)
}
```

**Step 3**: Can we wrap this in Zen? NO - it's an external C function.

---

## Self-Hosting Strategy

The long-term goal is to write more of the compiler in Zen itself. This means:

1. ✅ **Phase 1 (Current)**: LLVM primitives in Rust (necessary)
2. ⏳ **Phase 2**: Minimal Rust + maximal Zen standard library
3. 🎯 **Phase 3**: Self-hosted compiler (write compiler in Zen)

For Phase 2-3 to work, we need:
- All data structures (String, Vec, Ptr) in Zen ✅ (mostly done)
- All algorithms in Zen ✅ (mostly done)
- Only bare LLVM primitives in Rust ✅ (good separation)

---

## Testing Implications

### Testing LLVM Primitives
**Location**: `tests/gep_intrinsics.rs`, `tests/enum_intrinsics.rs`  
**Approach**: Test directly, verify LLVM IR generation

**Example**:
```rust
#[test]
fn test_gep_basic() {
    // Allocate, offset, verify
}
```

### Testing Zen-Level Features
**Location**: `tests/*.zen`, `stdlib/*.zen`  
**Approach**: Test through Zen code

**Example**:
```zen
test_vec_push = () void {
    v = vec.vec_new(allocator)
    vec.vec_push(&v, 42)
    // Verify...
}
```

---

## Debugging: Which Layer Has the Bug?

### Symptom: Compilation fails with "Unknown function"
**Likely cause**: Missing LLVM primitive declaration  
**Check**: `src/stdlib_metadata/compiler.rs` - is the function registered?

### Symptom: Wrong LLVM IR generated
**Likely cause**: Bug in code generation  
**Check**: `src/codegen/llvm/stdlib_codegen/compiler.rs` - is IR correct?

### Symptom: Zen code won't compile
**Likely cause**: Missing Zen implementation or type error  
**Check**: `stdlib/*.zen` - does the function exist?

### Symptom: Runtime error but code looks correct
**Likely cause**: Zen logic bug or unsafe primitive usage  
**Check**: `stdlib/*.zen` - is the algorithm correct?

---

## Key Principles

1. **LLVM primitives are minimal** - Only what can't be done in Zen
2. **Zen code is safe** - No unsafe operations, uses primitives correctly
3. **Clear separation** - Rust code gen ≠ Zen library code
4. **Composable** - Features built on features, not just primitives
5. **Testable** - Each layer tested independently

---

## Common Mistakes

❌ **Mistake 1**: Implementing in Zen when it needs libc
```zen
file_read = (path: string) string {
    // ERROR: Can't call C functions from Zen
    fopen(path)  // Won't work
}
```
✅ **Fix**: Make it an LLVM primitive, optionally wrap in Zen

---

❌ **Mistake 2**: Making an LLVM primitive when Zen would work
```rust
// src/stdlib_metadata/string.rs - WRONG
pub fn compile_string_reverse(...) {
    // Complex LLVM IR generation
    // When it could be in Zen!
}
```
✅ **Fix**: Implement in `stdlib/string.zen`

---

❌ **Mistake 3**: Mixing concerns (primitives exposed in Zen)
```zen
main = () {
    // Accessing raw primitives directly
    alloc = compiler.raw_allocate(100)  // Should use GPA instead
    compiler.raw_deallocate(alloc, 100)
}
```
✅ **Fix**: Use safe wrappers
```zen
main = () {
    alloc = gpa.default_gpa()
    ptr = alloc.allocate(100)  // Safe wrapper
}
```

---

## Checklist for Adding a New Primitive

If you determine something MUST be an LLVM primitive:

- [ ] Add declaration in `src/stdlib_metadata/my_module.rs` with `is_builtin: true`
- [ ] Add code generation in `src/codegen/llvm/stdlib_codegen/my_module.rs`
- [ ] Register function call routing in `src/codegen/llvm/stdlib_codegen/mod.rs`
- [ ] Add tests in `tests/`
- [ ] Document in `docs/INTRINSICS_REFERENCE.md`
- [ ] Add example usage in `examples/`
- [ ] Consider: Can this be wrapped in Zen for safety?

---

## Code Examples

### Example: raw_allocate (LLVM Primitive)

**Declaration**: `src/stdlib_metadata/compiler.rs`
```rust
functions.insert(
    "raw_allocate".to_string(),
    StdFunction {
        name: "raw_allocate".to_string(),
        params: vec![("size".to_string(), AstType::Usize)],
        return_type: AstType::Ptr(Box::new(AstType::U8)),
        is_builtin: true,  // ← Marks as LLVM primitive
    },
);
```

**Why LLVM Primitive?** Must call C `malloc()` function, requires external libc linkage.

### Example: GPA.allocate (Zen Feature wrapping Primitive)

**Implementation**: `stdlib/memory/gpa.zen`
```zen
GPA: {
    allocate: (self, size: usize) Option<*u8> {
        compiler.raw_allocate(size)  // Calls LLVM primitive
    },
    deallocate: (self, ptr: *u8, size: usize) void {
        compiler.raw_deallocate(ptr, size)  // Calls LLVM primitive
    },
}
```

**Why Zen Feature?** No special code generation needed, simple wrapper around primitives.

### Example: Vec<T> (Zen Feature with Generics)

```zen
Vec<T>: {
    data: Ptr<T>,
    len: usize,
    capacity: usize,
    allocator: Allocator,
}

vec_push = (vec: *Vec<T>, elem: T) {
    if vec.len >= vec.capacity {
        vec_grow(vec)
    }

    // Calculate offset using sizeof (LLVM primitive)
    offset = vec.len * compiler.sizeof(T)
    addr = compiler.gep(vec.data.addr as *u8, offset)
    store_value(addr, elem)
    vec.len = vec.len + 1
}
```

**Why Zen Feature?** Generic container logic using sizeof for type-aware offsets.

---

## Summary Table

| Feature | Location | LLVM Direct? | Can Be Zen? |
|---------|----------|--------------|-------------|
| raw_allocate | src/stdlib_metadata/compiler.rs | Yes (malloc) | No |
| gep | src/stdlib_metadata/compiler.rs | Yes (GEP inst) | No |
| sizeof | src/stdlib_metadata/compiler.rs | Yes (layout) | No |
| io.println | src/stdlib_metadata/io.rs | Yes (printf) | No |
| GPA | stdlib/memory/gpa.zen | No | Yes |
| Ptr<T> | stdlib/core/ptr.zen | No | Yes |
| String | stdlib/string.zen | No | Yes |
| Vec<T> | stdlib/vec.zen | No | Yes |
| Option<T> | stdlib/core/option.zen | No | Yes |

---

**Remember**: The fewer LLVM primitives, the closer to self-hosting.
