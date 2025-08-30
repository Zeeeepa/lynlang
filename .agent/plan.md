# Zen Language Import Syntax Reform Plan

## Goal
Remove the comptime requirement for imports, making them first-class declarations at the module level.

## Current State
- Imports already work without comptime in examples (e.g., quickstart.zen)
- Parser supports ModuleImport in both Declaration and Statement enums
- Two import patterns are supported:
  1. Direct stdlib: `core := @std.core`
  2. Build import: `io := build.import("io")`

## Issues Found
1. Import parsing is only done in `parse_program` for top-level declarations
2. The parse_variable_declaration doesn't recognize imports as special
3. Imports inside functions/blocks would be parsed as regular assignments

## Implementation Plan

### Phase 1: Parser Updates ✅
- [x] Analyze current import implementation
- [ ] Ensure imports are only allowed at module level
- [ ] Update parse_variable_declaration to recognize module imports
- [ ] Prevent imports inside functions/blocks

### Phase 2: AST & Validation
- [ ] Ensure ModuleImport is properly handled in typechecker
- [ ] Validate imports are at module scope only
- [ ] Update error messages for misplaced imports

### Phase 3: Code Generation
- [ ] Verify LLVM codegen handles non-comptime imports
- [ ] Ensure proper module resolution
- [ ] Test import symbol visibility

### Phase 4: Testing
- [ ] Write comprehensive tests for new import syntax
- [ ] Test error cases (imports in wrong scope)
- [ ] Update existing tests to use new syntax

### Phase 5: Documentation & Examples
- [ ] Update all example files
- [ ] Document the change
- [ ] Update language specification

## Design Decisions
1. Imports are module-level only (not allowed in functions/blocks)
2. Two syntaxes supported:
   - `name := @std.module` for stdlib
   - `name := build.import("module")` for other modules
3. Imports create immutable bindings
4. Import order doesn't matter (resolved before execution)