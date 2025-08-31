# Zen Language Global Memory

## Project Overview
Zen is a systems programming language with:
- Strong type system
- Pattern matching
- Comptime evaluation for meta-programming
- Self-hosting compiler (in progress)
- Standard library written in Zen

## Key Design Principles
- Simplicity, elegance, practicality, and intelligence
- DRY (Don't Repeat Yourself) & KISS (Keep It Simple, Stupid)
- Comptime is for meta-programming, NOT for imports

## Import System Rules
1. Imports should be at module level (NOT inside comptime blocks)
2. Standard library imports use `@std.module` syntax
3. Build system imports use `@std.build.import("module")` syntax
4. Comptime blocks are for compile-time computation and meta-programming

## Standard Library Modules
- core: Core types and utilities
- io: Input/output operations
- string: String manipulation
- vec: Dynamic arrays
- hashmap: Hash maps
- math: Mathematical functions
- fs: File system operations
- thread: Threading utilities
- json: JSON parsing/serialization
- test: Testing framework
- build: Build system utilities

## Current Tasks
- ✅ Import system fixed (imports at module level work correctly)
- Bootstrap self-hosting compiler (lexer, parser, type checker, codegen complete)
- Comprehensive testing in progress
- LSP implementation for code checking
