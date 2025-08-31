# Global Memory - Zen Language Project

## Project Overview
Zen is a systems programming language being developed with a focus on simplicity and self-hosting.

## Key Architectural Decisions
- **Import Syntax**: Direct module-level imports without comptime wrapper
- **Self-Hosting**: Progressive migration from Rust to Zen
- **Stdlib**: Pure Zen implementations prioritized

## Current Working Directory
`/home/ubuntu/zenlang`

## Important Files
- Parser: `src/parser/statements.rs` (handles import parsing)
- Examples: `examples/02_imports_example.zen` (correct import syntax)
- Self-hosted components: `compiler/` directory
- Stdlib: `stdlib/` directory

## Testing Commands
- Run tests: `cargo test`
- Zen-specific tests: `./test_zen.sh`
- Integration tests: `./test_integration.sh`

## Git Workflow
- Frequent commits with descriptive messages
- Main branch for stable code
- Feature branches for experimental work