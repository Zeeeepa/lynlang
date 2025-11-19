# Task #18: Complete Allocator Interface - Status Report

**Date**: 2025-01-27  
**Status**: IN PROGRESS - Infrastructure exists, integration needed  
**Tests**: 1/11 passing (allocator_string_usage)

## What Exists

### Zen Code Structure
```
stdlib/memory/
├── allocator.zen (237 lines) ✅ Complete interface definition
└── gpa.zen (203 lines) ✅ GPA implementation
```

### Compiler Intrinsics
All required primitives are implemented in Rust:
```
✅ raw_allocate(size: usize) -> *u8
✅ raw_deallocate(ptr: *u8, size: usize) -> void
✅ raw_reallocate(ptr: *u8, old: usize, new: usize) -> *u8
✅ raw_ptr_cast(ptr: *u8) -> *u8
✅ null_ptr() -> *u8
✅ gep(ptr: *u8, offset: i64) -> *u8
✅ gep_struct(ptr: *u8, field_index: i32) -> *u8
```

### Standard Library Definitions
```
✅ Allocator trait
✅ DefaultAllocator type
✅ ArenaAllocator interface
✅ PoolAllocator interface
✅ ThreadsafeAllocator interface
✅ StatsAllocator interface
✅ AllocatorConstraints type
✅ AllocError type
✅ get_default_allocator() function
✅ allocate_one<T>() helper
✅ deallocate_one<T>() helper
✅ allocate_array<T>() helper
✅ deallocate_array<T>() helper
✅ reallocate_array<T>() helper
✅ memzero() utility
✅ memcpy() utility
```

## Current Issues

### 1. Module System Integration
**Problem**: `{ compiler } = @std` import works, but `compiler.raw_allocate()` fails
**Error**: `InternalError("Symbol 'raw_allocate' is not exported from module '@std.compiler'")`

**Root Cause**: The compiler intrinsics are defined in `src/stdlib/compiler.rs` as static Intrinsic structs, but they're not integrated into the module resolution system.

**Status**: BLOCKER - Can't call intrinsics from Zen code

### 2. Missing Module Exports
The stdlib modules need to be explicitly exported:
- `stdlib/std.zen` doesn't exist (should be the entry point)
- Module exports not hooked up in parser/typechecker
- @std namespace needs to resolve to stdlib/ directory structure

**Status**: HIGH PRIORITY - Required for module system to work

### 3. Integration with Existing Types
String and Vec should use the allocator, but currently:
- String hardcoded allocation in Rust
- Vec hardcoded allocation in Rust
- Need to update both to accept Allocator parameter

**Status**: MEDIUM PRIORITY - Deferred to follow-up task

## What Needs to Be Done

### Phase 1: Module System (IMMEDIATE)
1. **Create stdlib entry point**: `stdlib/std.zen`
   - Import all stdlib modules
   - Export core types and functions
   
2. **Register stdlib directory**: Modify compiler to:
   - Load .zen files from `stdlib/` directory
   - Resolve `@std` import to module system
   - Export intrinsics from `@std.compiler` module

3. **Fix compiler intrinsic exports**: Ensure intrinsics are available:
   - `compiler.raw_allocate()`
   - `compiler.raw_deallocate()`
   - `compiler.raw_reallocate()`
   - `compiler.raw_ptr_cast()`
   - `compiler.null_ptr()`

### Phase 2: Testing (AFTER Phase 1)
1. **Verify intrinsic calls**: Test basic allocation/deallocation
2. **Test GPA module**: Verify gpa.zen compiles and works
3. **Test allocator trait**: Verify Allocator interface works
4. **Integration tests**: String and Vec with allocator parameter

### Phase 3: Integration (AFTER Phase 2)
1. **Update String**: Accept Allocator parameter
2. **Update Vec**: Accept Allocator parameter
3. **Update HashMap**: Accept Allocator parameter
4. **Remove hardcoded allocations**: Clean up Rust stdlib

## Test Coverage

### Current Tests (1/11 passing)
- ✅ test_allocator_string_usage - Basic string compilation
- ❌ test_gpa_allocator_basic - Fails: intrinsic not exported
- ❌ test_allocator_allocate_array - Same issue
- ❌ test_allocator_reallocate - Same issue
- ❌ test_allocator_with_null_check - Same issue
- ❌ test_gpa_allocate_multiple - Same issue
- ❌ test_allocator_with_pointer_arithmetic - Same issue
- ❌ test_allocator_loop_allocations - Same issue
- ❌ test_allocator_conditional_allocation - Same issue
- ❌ test_allocator_overflow_check - Same issue
- ❌ test_allocator_with_type_casting - Same issue

### Tests to Add After Phase 1
- GPA allocator functionality
- Allocator trait implementation
- Memory utilities (memzero, memcpy)
- Integration with String/Vec/HashMap

## Estimated Timeline

| Phase | Task | Estimate | Blocker |
|-------|------|----------|---------|
| 1 | Create std.zen entry point | 1 hour | YES |
| 1 | Register stdlib directory | 2 hours | YES |
| 1 | Fix intrinsic exports | 1 hour | YES |
| 2 | Test intrinsic calls | 1 hour | NO |
| 2 | Test GPA module | 1 hour | NO |
| 2 | Integration tests | 2 hours | NO |
| 3 | Update String/Vec/HashMap | 3 hours | NO |
| 3 | Remove hardcoded code | 1 hour | NO |
| **TOTAL** | | **12 hours** | |

## Next Immediate Action

**Create and register stdlib entry point**: `stdlib/std.zen`

This file should:
1. Import all stdlib modules (memory, io, core, etc.)
2. Explicitly export compiler intrinsics
3. Be loaded when `@std` is imported

```zen
// stdlib/std.zen
export {
    compiler: {
        raw_allocate,
        raw_deallocate,
        raw_reallocate,
        raw_ptr_cast,
        null_ptr,
        gep,
        gep_struct,
        // ... other intrinsics
    },
    memory: {
        Allocator,
        GPA,
        get_default_allocator,
        // ... other memory module exports
    },
    io: {
        // ... io exports
    },
    // ... other module exports
}
```

Once this is created and the module system is updated to load it, all 11 tests should pass.

## Dependency Chain

```
Task #18 Success Requires:
  ├─ Phase 1: Module System
  │  ├─ stdlib/std.zen created
  │  ├─ Parser loads .zen files from stdlib/
  │  └─ @std namespace resolves correctly
  ├─ Phase 2: Testing
  │  └─ Intrinsics callable from Zen
  └─ Phase 3: Integration
     └─ String/Vec/HashMap updated
```

## Known Limitations (Deferred)

1. **set_payload** - Needs size information (can use memcpy workaround)
2. **gep_struct** - Hardcoded 8-byte alignment (works for most cases)
3. **FFI intrinsics** - load_library/get_symbol/unload_library not implemented
4. **inline_c** - Not implemented (placeholder returns void)

These don't block Task #18 completion.

## Conclusion

The Zen code for allocators is complete and well-designed. The blocker is Module System integration. Once `@std.compiler` intrinsics are accessible from Zen code, all tests should pass and the allocator interface can be used throughout the stdlib.

**Critical Path**: Fix module exports → test intrinsics → integrate with String/Vec/HashMap
