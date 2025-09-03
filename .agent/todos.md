# Zen Language - TODOs

## Current Session (2025-09-03)
1. ✅ Consolidate .agent directory files  
2. ✅ Fix GitHub workflows for LLVM 18.1
3. ⏳ Send summary email via SendGrid

## High Priority Tasks
- [ ] Complete self-hosted parser (currently 25%)
- [ ] Fix hanging parser tests
- [ ] Complete integration test suite
- [ ] Implement code generation in Zen
- [ ] Bootstrap compiler process

## Medium Priority  
- [ ] Enhanced error messages and diagnostics
- [ ] Complete remaining stdlib modules
- [ ] Performance optimizations
- [ ] Package manager design
- [ ] Documentation improvements

## Low Priority
- [ ] Tutorial creation
- [ ] Example programs expansion
- [ ] IDE plugin development

## Known Issues to Address
- Missing test scripts (test_integration.sh, test_runner.sh, bootstrap.sh)
- Parser tests hanging
- Self-hosted components incomplete (parser 25%, codegen 0%)

## Development Guidelines
- Use gh CLI for GitHub management
- Frequent git commits (DRY & KISS)
- 80% implementation, 20% testing
- Work at 40% context window
- Simplicity, elegance, practicality