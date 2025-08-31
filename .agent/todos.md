# Zenlang Development TODOs

## Priority Order (Estimated)

### Immediate (Today)
- [x] Set up .agent directory with meta-information files
- [ ] Review and fix remaining comptime import usage in examples
- [ ] Re-enable import validation in typechecker
- [ ] Create git commit for import system fixes

### Short-term (This Week)
- [ ] Update and run comprehensive import tests
- [ ] Run full test suite and fix any failures
- [ ] Enhance self-hosted compiler components
- [ ] Implement basic LSP import validation

### Medium-term (Next Sprint)
- [ ] Complete self-hosted lexer
- [ ] Complete self-hosted parser
- [ ] Start self-hosted code generator
- [ ] Implement import auto-completion in LSP

### Long-term (Future)
- [ ] Full bootstrap capability
- [ ] Module dependency visualization
- [ ] Import optimization pass
- [ ] Package manager integration

## Testing Checklist
- [ ] All import tests pass
- [ ] No comptime-wrapped imports remain
- [ ] Self-hosted compiler builds
- [ ] LSP validates imports correctly

## Code Quality
- [ ] Follow DRY principle
- [ ] Keep implementations simple (KISS)
- [ ] Document complex logic
- [ ] Add tests for new features