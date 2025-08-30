# Fix Plan: Engine-SDK Consistency

## Phase 1: Immediate Critical Fixes (Priority 1)

### 1.1 Fix Missing build.rs Module
- Create `src/stdlib/build.rs` with BuildModule implementation
- Connect to existing `stdlib/build.zen`

### 1.2 Register All Stdlib Modules  
- Add all 30+ stdlib modules to StdNamespace registry
- Create placeholder .rs files for missing modules

### 1.3 Fix Core Module Intrinsics
- Add LLVM implementations for size_of, align_of, type_name
- Connect panic/assert to Zen implementations

## Phase 2: Type System Alignment (Priority 2)

### 2.1 Unify Result/Option Types
- Standardize Result<T,E> definition across engine and SDK
- Update AstType to properly handle enum variants

### 2.2 Fix IO Module Abstraction
- Create high-level IO functions in SDK matching engine interface
- Wrap low-level C FFI with proper error handling

## Phase 3: Comprehensive Testing (Priority 3)

### 3.1 Create Stdlib Tests
- Test each registered function
- Verify type checking
- Ensure codegen works

### 3.2 Integration Tests
- Test module imports
- Test cross-module function calls
- Test error handling

## Estimated Time
- Phase 1: 2-3 hours
- Phase 2: 3-4 hours  
- Phase 3: 2-3 hours
- Total: ~10 hours

## Current Progress
- [x] Analysis complete
- [x] Phase 1 implementation (COMPLETED)
  - [x] Created build.rs module
  - [x] Registered math, string, vec, fs modules
  - [x] Fixed core module intrinsics
- [ ] Phase 2 implementation (TODO)
  - [ ] Unify Result/Option types
  - [ ] Fix IO module abstraction
- [x] Phase 3 implementation (PARTIAL)
  - [x] Created stdlib integration tests
  - [ ] Need execution tests
  - [ ] Need cross-module tests
