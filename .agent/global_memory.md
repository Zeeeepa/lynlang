# Global Memory - Zen Language Project

## Project Overview
Zen is a systems programming language being developed with a focus on simplicity and self-hosting.

## Key Architectural Decisions
- **Import Syntax**: Direct module-level imports without comptime wrapper ✅
- **Self-Hosting**: Progressive migration from Rust to Zen (parser, lexer, type checker in progress)
- **Stdlib**: Pure Zen implementations prioritized (comprehensive stdlib with 40+ modules)
- **Testing**: Comprehensive test framework and test suites for all stdlib modules
- **LSP**: Enhanced LSP server with hover, go-to-definition, and symbol tracking

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
- Stdlib tests: Located in `tests/stdlib/` directory

## Recent Achievements
- Import syntax fixed - no comptime wrapper needed for imports
- Self-hosted parser handles module-level imports correctly
- Comprehensive stdlib with 40+ modules implemented
- Test framework and comprehensive test suites added
- LSP server with advanced features implemented
- Math module fully integrated and working
- Struct field mutability syntax fixed (field:: type)
- Module import resolution working for stdlib modules

## Git Workflow
- Frequent commits with descriptive messages
- Main branch for stable code
- Feature branches for experimental work