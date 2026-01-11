# Codegen Architecture Guidelines

## DO NOT ADD HARDCODED TYPE NAMES HERE

This file contains type inference code that should eventually be removed.
The typechecker should provide all type information via `TypeContext`.

### Three-Layer Architecture (see docs/design/SEPARATION_OF_CONCERNS.md)

**Layer 1 - Compiler Primitives (OK to hardcode):**
- `i32`, `i64`, `f32`, `f64`, `bool`, `void`
- Intrinsics: `raw_allocate`, `sizeof`, `gep`, syscalls

**Layer 2 - Well-Known Types (use WellKnownTypes registry):**
- `Option`, `Result` - pattern matching, `.raise()`
- `Ptr`, `MutPtr`, `RawPtr` - pointer codegen

**Layer 3 - Regular Stdlib (NO special handling):**
- `Vec`, `DynVec`, `Array`, `HashMap`, `HashSet`, `Range`, `String`
- These should work through generic struct/method resolution
- Type info should come from TypeContext, not hardcoded here

### Current Technical Debt

The following hardcoded patterns exist and should be removed:
- Constructor return type inference (`HashMap.new()` → `HashMap<String, i32>`)
- Method return type inference (`vec.len()` → `i64`)
- Struct field definitions (`Range { current, end, step }`)

### Migration Path

1. Ensure typechecker populates TypeContext with all type info
2. Make codegen query TypeContext for ALL type lookups
3. Remove hardcoded fallbacks one by one
4. Eventually delete most of inference.rs

### Adding New Features

If you need type info for a stdlib type:
1. ❌ DO NOT add it to inference.rs
2. ✅ Add it to TypeContext population in typechecker
3. ✅ Query TypeContext from codegen
