# Zenlang Development TODOs

## Priority Order (Estimated)

### Completed Today (2025-09-02)
- [x] Clean up auxiliary files in root directory
- [x] Review and commit clean project state
- [x] Run library tests - all passing (11/11)
- [x] Fix LSP enhanced module compilation errors
- [x] Create zen-check validation tool (bash & Rust)
- [x] Add stdlib collections modules (list, hashmap)
- [x] Add testing framework module (runner)

### Immediate (Now)
- [ ] Integrate enhanced LSP features fully
- [ ] Write integration tests for self-hosted compiler
- [ ] Fix hanging parser tests

### Short-term (This Week)
- [ ] Complete self-hosted compiler components
- [ ] Enhance LSP with goto definition, refactoring
- [ ] Implement module dependency visualization
- [ ] Create comprehensive test framework in Zen

### Medium-term (Next Sprint)
- [ ] Full bootstrap capability
- [ ] Performance optimizations
- [ ] Package manager design
- [ ] Import optimization pass

### Long-term (Future)
- [ ] Module package registry
- [ ] Cross-compilation support
- [ ] IDE integrations
- [ ] Documentation generator

## Testing Checklist
- [x] Library tests pass
- [ ] Parser tests need fixing (hanging)
- [ ] Integration tests
- [ ] Self-hosted compiler tests

## Code Quality
- [x] Follow DRY principle
- [x] Keep implementations simple (KISS)
- [ ] Document complex logic
- [x] Add tests for new features