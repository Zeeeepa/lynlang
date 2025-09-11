# Zenlang Compliance Implementation Plan

## Current State Analysis
- Found 147 files with if/else/match violations
- Found 73 files with non-@std @ references  
- Need to implement FFI builder pattern
- Need to fix LSP implementation

## Implementation Strategy

### Phase 1: Pattern Replacement (Priority: HIGH)
1. Replace all if/else/match with ? operator pattern
2. Update control flow to use ? | pattern => expression syntax
3. Fix boolean pattern usage

### Phase 2: Import System (Priority: HIGH)
1. Remove all non-@std @ references
2. Update imports to use @std.build.import() pattern
3. Fix module resolution

### Phase 3: FFI Implementation (Priority: MEDIUM)
1. Implement FFI builder pattern per spec
2. Create LibBuilder structure
3. Add safe C interop methods

### Phase 4: LSP Enhancement (Priority: MEDIUM)
1. Fix LSP server implementation
2. Add proper error diagnostics
3. Implement hover, completion, and goto definition

### Phase 5: Testing (Priority: HIGH)
1. Create comprehensive test suite
2. Ensure all examples compile
3. Validate language spec compliance

## Files to Update (Critical Path)
- compiler/parser.zen - Core parsing logic
- compiler/lexer.zen - Tokenization
- stdlib/*.zen - Standard library modules
- lsp/*.zen - Language server
- tests/*.zen - Test suite

## Success Criteria
- All .zen files compile without errors
- No if/else/match keywords in codebase
- Only @std namespace references
- FFI builder pattern implemented
- LSP server functional
- Test suite passing
