# ZenLang Scratchpad

## Current Work Session Notes

### GitHub Workflows Issues
- LLVM version mismatch (using 18.1, workflows may have wrong config)
- Environment variable LLVM_SYS_181_PREFIX needs proper setting
- APT repository for LLVM 18 might need updating

### Quick Commands
```bash
# Build debug
export LLVM_SYS_181_PREFIX=/usr/lib/llvm-18 && cargo build

# Build release
export LLVM_SYS_181_PREFIX=/usr/lib/llvm-18 && cargo build --release

# Run tests
export LLVM_SYS_181_PREFIX=/usr/lib/llvm-18 && cargo test

# Format code
cargo fmt

# Lint
cargo clippy
```

### Git Workflow
```bash
# Frequent commits
git add -A && git commit -m "feat: description"
git push origin main

# Check status
git status
git diff

# Branch management
gh pr create --title "Title" --body "Description"
```

### Known Issues
1. Self-hosted compiler components don't exist yet
2. Some examples may be missing or broken
3. Stdlib is incomplete

### Ideas & Notes
- Consider using WASM target for browser-based playground
- Look into incremental compilation for faster builds
- Add property-based testing with proptest
- Consider TreeSitter grammar for better IDE support

### Resources
- LLVM 18.1 docs: https://llvm.org/docs/
- Inkwell examples: https://github.com/TheDan64/inkwell
- Language design inspiration: Zig, Rust, Odin