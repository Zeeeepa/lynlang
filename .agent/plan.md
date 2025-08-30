# Zen Language Import Syntax Refactor Plan

## Current State
- Imports are currently handled inside comptime blocks
- ModuleImport is already a Declaration variant in AST
- Parser already handles := operator for module imports at top-level

## Goal
Remove comptime requirement for imports - allow direct import statements at module level.

## Implementation Steps

### Phase 1: Parser Changes
1. **Update parser to handle imports without comptime** ✓
   - The parser already supports this via := operator
   - Need to ensure it works correctly outside comptime blocks

### Phase 2: Update Examples and Tests
2. **Update all example files to use new syntax**
   - Remove comptime blocks around imports
   - Convert to direct import statements

### Phase 3: Stdlib Migration
3. **Begin stdlib implementation in Zen**
   - Start with core modules (io, mem, fs)
   - Implement basic functionality first
   - Add more complex features progressively

### Phase 4: Testing Framework
4. **Create basic test framework**
   - Implement assert functions
   - Create test runner
   - Add test discovery

### Phase 5: LSP/Checker
5. **Implement basic checking**
   - Type checking
   - Import resolution
   - Basic error reporting

## Current Focus
Working on removing comptime requirement from imports across the codebase.