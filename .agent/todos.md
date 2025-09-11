# Zenlang Implementation TODOs

## High Priority (Core Language Features)
- [ ] **Pattern Matching Enhancements**
  - [ ] Implement range patterns (0..=12)
  - [ ] Add or-patterns (1 | 2 | 3)
  - [ ] Implement type patterns
  - [ ] Add guard conditions with ->
  
- [ ] **Behaviors System**
  - [ ] Define behavior struct format
  - [ ] Implement behavior registration
  - [ ] Add automatic derivation support
  - [ ] Create standard behaviors (Comparable, Hashable, Serializable)

- [ ] **Memory Management**
  - [ ] Implement Ptr<T> type fully
  - [ ] Add Ref<T> for reference counting
  - [ ] Create GPA (General Purpose Allocator)
  - [ ] Implement ownership rules enforcement

## Medium Priority (Standard Library)
- [ ] **Core Modules**
  - [ ] io module (print, read_line, File operations)
  - [ ] mem module (allocators)
  - [ ] collections (Vec<T>, HashMap<K,V>, List<T>, Set<T>)
  - [ ] math module (trig functions, constants)
  - [ ] string utilities (split, join, format, parse)
  
- [ ] **Async Support**
  - [ ] Implement colorless async via allocators
  - [ ] Create Channel<T> for message passing
  - [ ] Add Actor pattern support
  - [ ] Implement Thread and Atomic types

## Low Priority (Tooling & Polish)
- [ ] **Build System**
  - [ ] Implement build.zen configuration
  - [ ] Add cross-compilation support
  - [ ] Create package manager integration
  
- [ ] **Developer Experience**
  - [ ] Enhance LSP with more features
  - [ ] Add code formatting (zen fmt)
  - [ ] Improve error messages
  - [ ] Create documentation generator

## Completed Today (2025-09-11)
- [x] Implemented FFI builder pattern
- [x] Fixed LSP deprecation warnings  
- [x] Cleaned up unused imports and variables
- [x] Fixed Program struct compilation errors
- [x] Added statements field for backward compatibility
- [x] Updated all test files to use Program::new()
- [x] Fixed delimiter matching errors in tests
- [x] Library tests passing (13 tests)

## Notes
- Language spec v1.1.0 is the authoritative reference
- NO if/else keywords - only ? operator
- All errors must be values (Result/Option)
- No lifetime annotations - use smart pointers
- Tests should be added for each new feature