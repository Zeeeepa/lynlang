# Zen Language Implementation Plan

## Current Focus: Import System Reform
Remove comptime requirement for imports - make them top-level statements

## Priority Order (80% implementation, 20% testing)
1. Fix import syntax parser changes
2. Update existing code to new syntax  
3. Self-hosting progress
4. Stdlib implementation in Zen
5. Testing infrastructure
6. LSP/syntax checking tools

## Key Principles
- Simplicity, elegance, practicality
- DRY & KISS
- Frequent commits
- Work at ~40% context window (100-140k)
- Merge to main when stable

## Import Syntax Change
From:
```zen
comptime {
    core := @std.core
    build := @std.build
    io := build.import("io")
}
```

To:
```zen
core := @std.core
build := @std.build
io := build.import("io")
```