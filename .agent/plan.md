# Zen Language Import System Fix Plan

## Current State
- Import system currently uses both old comptime syntax and new direct syntax
- Parser supports module-level imports but needs cleanup
- Some examples use correct syntax, others still use comptime blocks

## Goal
Transform imports from comptime blocks to direct module-level imports:
```zen
// From:
comptime {
    core := @std.core
    build := @std.build
    io := build.import("io")
}

// To:
core := @std.core
build := @std.build
io := build.import("io")
```

## Implementation Steps

1. **Parser Updates** ✅
   - Already supports module-level imports
   - Need to ensure comptime blocks are for meta-programming only

2. **Update All Zen Files**
   - Fix all .zen files using old comptime import syntax
   - Update stdlib files
   - Update examples
   - Update tests

3. **Semantic Analysis**
   - Ensure imports are resolved correctly
   - Update type checker for import handling

4. **LLVM Codegen**
   - Verify codegen handles imports properly
   - Fix any issues with module resolution

5. **Self-Hosting Components**
   - Update compiler/*.zen files
   - Update tools/*.zen files
   - Ensure bootstrap works

6. **Testing**
   - Add comprehensive import tests
   - Run existing test suite
   - Verify self-hosting works

## Priority Order
1. Fix core parser/semantic issues
2. Update stdlib (foundation for everything)
3. Update compiler components (self-hosting)
4. Update examples and tests
5. Documentation updates
