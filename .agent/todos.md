# Zenlang Project Todos

## High Priority
- [x] Implement core types (Option, Result, Vec, HashMap) - DONE
- [x] Fix FFI builder pattern to use core types - DONE
- [x] Create spec-compliant LSP implementation - DONE
- [ ] Fix all if/else/match violations (1,692 remaining)
- [ ] Complete pattern matching codegen (parser done, codegen WIP)
- [ ] Implement comptime execution framework
- [ ] Finish self-hosted lexer (30% complete)
- [ ] Finish self-hosted parser (20% complete)
- [ ] Implement behaviors (traits) system

## Medium Priority  
- [ ] Complete UFCS implementation
- [ ] Add string interpolation to codegen
- [ ] Implement async/await with Task<T>
- [x] Add smart pointers (Ptr<T>, Ref<T>) - DONE
- [ ] Custom allocator interface
- [ ] Implement GPA (General Purpose Allocator)
- [ ] Complete stdlib modules (io, fs, net, etc.)

## Low Priority
- [ ] Package management system
- [ ] Improved C FFI
- [ ] Documentation generation
- [ ] IDE support improvements

## Testing & Quality
- [ ] Reach 100% test pass rate (currently 99.6%)
- [ ] Add integration tests for standard library
- [ ] Performance benchmarks
- [ ] Fuzz testing for parser

## Self-Hosting Milestones
1. ✅ Core language features
2. ✅ Standard library in Zen
3. 🚧 Self-hosted lexer (30%)
4. 🚧 Self-hosted parser (20%)
5. ⏳ Comptime execution
6. ⏳ Bootstrap with Zen stdlib

## Documentation
- [x] Language specification (LANGUAGE_SPEC.md)
- [ ] Tutorial series
- [ ] Standard library API docs
- [ ] Compiler internals guide