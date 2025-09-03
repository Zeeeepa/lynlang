# Zen Language - Scratchpad

## Quick Notes
- LLVM 18.1 is critical - DO NOT change (see Cargo.toml comment)
- Import syntax: `module := @std.module` (NO comptime wrapping)
- Use `zen` binary for compilation, `zen-lsp` for IDE support
- Test with: `cargo test` and `./target/release/zen examples/01_hello_world.zen`

## Common Commands
```bash
# Build compiler
cargo build --release

# Run tests
cargo test

# Format code
cargo fmt

# Lint
cargo clippy

# Test example
./target/release/zen examples/01_hello_world.zen

# Git workflow
git add .
git commit -m "feat: Description"
git push
```

## File Locations
- Compiler source: src/
- Standard library: stdlib/
- Examples: examples/
- Tests: tests/
- Self-hosted compiler: stdlib/compiler/
- LSP server: src/lsp/

## Current Focus Areas
1. Self-hosted parser completion (stdlib/compiler/parser.zen)
2. Test infrastructure improvements
3. Error diagnostics enhancement
4. GitHub Actions CI/CD

## Gotchas & Reminders
- Parser tests hang - needs investigation
- Missing test scripts need creation
- Always test imports don't use comptime
- Frequent commits (every significant change)
- Keep context under 140K tokens

## Recent Achievements
- ✅ Updated GitHub workflows to LLVM 18.1
- ✅ Consolidated project documentation
- ✅ Created .agent directory structure
- ✅ 100% test pass rate maintained