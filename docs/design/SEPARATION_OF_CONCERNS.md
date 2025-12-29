# Separation of Concerns: Compiler Primitives vs Zen-Defined Types

## Executive Summary

The Zen compiler should contain the **minimum necessary** to bootstrap the language.
Everything else belongs in Zen stdlib files, discovered at compile time.

**Current Problem:** 100+ hardcoded string references like `"Option"`, `"Some"`, `"Ptr"` scattered across the compiler.

**Solution:** A `WellKnownTypes` registry that:
1. Hardcodes **names** (not definitions)
2. Discovers **definitions** from stdlib at startup
3. Provides type-safe checks: `well_known.is_option(name)` instead of `name == "Option"`

---

## The Three Layers

```
┌─────────────────────────────────────────────────────────────────┐
│                     User Zen Code                                │
│  main = () { x = Some(42); x ? | Some(v) { io.println(v) } }   │
├─────────────────────────────────────────────────────────────────┤
│                  Layer 3: Regular Stdlib                         │
│  Vec<T>, String, HashMap<K,V>, GPA, HashSet<T>, Queue<T>        │
│  Location: stdlib/*.zen                                          │
│  Special handling: NONE - just regular Zen types                 │
├─────────────────────────────────────────────────────────────────┤
│                  Layer 2: Well-Known Types                       │
│  Option<T>, Result<T,E>, Ptr<T>, MutPtr<T>, RawPtr<T>           │
│  Location: stdlib/core/*.zen                                     │
│  Special handling: Pattern matching, .raise(), pointer codegen   │
│  Discovery: Parsed at compiler startup, registered in WellKnown  │
├─────────────────────────────────────────────────────────────────┤
│                  Layer 1: True Primitives                        │
│  i32, f64, bool, void, raw_allocate, gep, sizeof, etc.          │
│  Location: Hardcoded in Rust compiler                            │
│  Special handling: LLVM IR generation, cannot be in Zen          │
└─────────────────────────────────────────────────────────────────┘
```

---

## Layer 1: True Compiler Primitives

These MUST be implemented in Rust/LLVM. They cannot be written in Zen.

### Primitive Types
| Type | Size | Description |
|------|------|-------------|
| `i8`, `i16`, `i32`, `i64` | 1-8 bytes | Signed integers |
| `u8`, `u16`, `u32`, `u64` | 1-8 bytes | Unsigned integers |
| `usize` | platform | Pointer-sized unsigned |
| `f32`, `f64` | 4-8 bytes | Floating point |
| `bool` | 1 byte | Boolean |
| `void` | 0 bytes | No value |

### Memory Intrinsics
```zen
// These call libc malloc/free - must be LLVM primitives
@std.compiler.raw_allocate(size: usize) RawPtr<u8>
@std.compiler.raw_deallocate(ptr: RawPtr<u8>, size: usize) void
@std.compiler.raw_reallocate(ptr: RawPtr<u8>, old_size: usize, new_size: usize) RawPtr<u8>
```

### Pointer Intrinsics
```zen
// These generate LLVM GEP instructions - must be LLVM primitives
@std.compiler.gep(ptr: RawPtr<u8>, offset: i64) RawPtr<u8>
@std.compiler.gep_struct(ptr: RawPtr<u8>, field_index: i32) RawPtr<u8>
@std.compiler.sizeof<T>() usize
```

### Enum Intrinsics
```zen
// These know enum memory layout - must be LLVM primitives
@std.compiler.discriminant(ptr: RawPtr<u8>) i32
@std.compiler.set_discriminant(ptr: RawPtr<u8>, value: i32) void
@std.compiler.get_payload(ptr: RawPtr<u8>) RawPtr<u8>
@std.compiler.set_payload(ptr: RawPtr<u8>, data: RawPtr<u8>) void
```

### IO Intrinsics
```zen
// These make syscalls - must be LLVM primitives
@std.io.print(msg: string) void
@std.io.read_line() string
```

---

## Layer 2: Well-Known Types

These are **defined in Zen** but the compiler needs special knowledge of them.

### Why Are They Special?

| Type | Why Special |
|------|-------------|
| `Option<T>` | Pattern exhaustiveness, `.raise()` propagation, `?` operator |
| `Result<T,E>` | Pattern exhaustiveness, `.raise()` propagation, `?` operator |
| `Ptr<T>` | Pointer dereference codegen, null checks |
| `MutPtr<T>` | Mutable pointer codegen |
| `RawPtr<T>` | Unsafe pointer codegen, FFI |

### Where They're Defined

```
stdlib/
├── core/
│   ├── option.zen    # Option<T>: Some: T, None
│   ├── result.zen    # Result<T, E>: Ok: T, Err: E
│   └── ptr.zen       # Ptr<T>, MutPtr<T>, RawPtr<T>
```

### How The Compiler Discovers Them

At startup:
1. Parse `stdlib/core/option.zen`
2. Find enum `Option<T>` with variants `Some`, `None`
3. Register in `WellKnownTypes` registry
4. Repeat for `Result`, `Ptr`, etc.

### The Registry

```rust
pub struct WellKnownTypes {
    types: HashMap<String, WellKnownType>,
    variants: HashMap<String, (WellKnownType, WellKnownVariant)>,
}

impl WellKnownTypes {
    pub fn is_option(&self, name: &str) -> bool;
    pub fn is_result(&self, name: &str) -> bool;
    pub fn is_ptr(&self, name: &str) -> bool;
    pub fn is_some(&self, name: &str) -> bool;
    pub fn is_none(&self, name: &str) -> bool;
    pub fn is_ok(&self, name: &str) -> bool;
    pub fn is_err(&self, name: &str) -> bool;
}
```

### Usage Pattern

**Before (hardcoded):**
```rust
if name == "Option" && type_args.len() == 1 {
    if variant == "Some" { ... }
    else if variant == "None" { ... }
}
```

**After (registry):**
```rust
if self.well_known.is_option(name) && type_args.len() == 1 {
    if self.well_known.is_some(variant) { ... }
    else if self.well_known.is_none(variant) { ... }
}
```

---

## Layer 3: Regular Stdlib Types

These have NO special compiler handling. They're just Zen code using Layer 1 primitives.

| Type | Uses | Location |
|------|------|----------|
| `Vec<T>` | `Ptr<T>`, `gep`, `sizeof` | `stdlib/vec.zen` |
| `String` | `Ptr<u8>`, `gep` | `stdlib/string.zen` |
| `HashMap<K,V>` | `Ptr<T>`, `gep` | `stdlib/collections/hashmap.zen` |
| `GPA` | `raw_allocate`, `raw_deallocate` | `stdlib/memory/gpa.zen` |

---

## Self-Hosting Implications

This architecture enables self-hosting:

1. **Bootstrap Compiler (Rust)**
   - Has `WellKnownTypes` registry with hardcoded names
   - Parses stdlib to discover type definitions
   - Compiles Zen code

2. **Self-Hosted Compiler (Zen)**
   - Has equivalent `WellKnownTypes` registry
   - Uses same stdlib files
   - Same discovery mechanism

The key insight: **hardcode names, discover definitions**.

---

## Migration Path

See `.opencode/command/remove-hardcoding.md` for step-by-step implementation.

### Progress Metrics

```bash
# Count hardcoded references (should decrease to 0)
rg '"Option"|"Result"|"Some"|"None"|"Ok"|"Err"|"Ptr"' src/ -c

# Target: Only references in well_known.rs
```

### Phases

1. **Create Registry** - `src/well_known.rs`
2. **Integrate** - Add to Compiler, TypeChecker, LLVMCompiler
3. **Refactor Typechecker** - Use registry methods
4. **Refactor Codegen** - Use registry methods
5. **Refactor Parser** - Use registry for validation
6. **Refactor LSP** - Use registry for completions
7. **(Optional) Simplify AST** - Remove `AstType::Ptr`, use `AstType::Generic`

---

## Future: Stdlib Discovery

Currently we hardcode the list of well-known types. Future enhancement:

```zen
// stdlib/core/option.zen
#[well_known]  // Attribute marks this as discoverable
Option<T>:
    Some: T,
    None
```

Compiler scans stdlib for `#[well_known]` attributes and auto-registers.

---

## Summary

| Layer | What | Where | Special Handling |
|-------|------|-------|------------------|
| 1 | Primitives | Rust compiler | LLVM IR generation |
| 2 | Well-Known | Zen stdlib (discovered) | Pattern matching, codegen |
| 3 | Regular | Zen stdlib | None |

**Principle:** Move as much as possible from Layer 1 → Layer 2 → Layer 3.

The compiler should be a thin layer that bootstraps Zen, not a repository of type definitions.
