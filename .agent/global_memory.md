# Zen Language - Global Memory

## Current State (2025-08-31)

### Completed Features
- ✅ Import syntax fixed - no longer requires comptime wrapper
- ✅ Parser correctly handles top-level imports
- ✅ Test suite validates import functionality
- ✅ Syntax checker wrapper script created
- ✅ Self-hosted lexer test file ready (uses correct imports)
- ✅ Comprehensive test runner (run_tests.sh)
- ✅ Advanced linter with style checking (zen-lint.sh)
- ✅ Self-hosting status documentation
- ✅ Frequent git commits workflow established

### Import Syntax
```zen
// Correct (current):
core := @std.core
build := @std.build
io := build.import("io")

// Incorrect (old):
comptime {
    core := @std.core
    // ...
}
```

### Working Examples
- examples/01_hello_world.zen - Basic hello world with imports
- examples/01_basics_working.zen - Basic arithmetic and variables
- tests/test_self_hosted_lexer.zen - Lexer test suite

### Known Issues
- Memory allocation (malloc) not available for complex types like Vec
- Need to implement external function declarations for malloc/free
- LSP not yet implemented
- Self-hosted parser needs more work

### Next Steps
1. Implement malloc/free external declarations
2. Complete self-hosted parser
3. Build LSP server
4. Create more comprehensive stdlib in Zen

### Test Commands
```bash
# Run comprehensive test suite
./run_tests.sh

# Run individual Rust tests
cargo test

# Check syntax
./zen-check.sh examples/hello.zen

# Compile Zen file
./target/debug/zen examples/01_basics_working.zen
```