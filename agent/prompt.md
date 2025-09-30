# CURRENT FOCUS

## 🎉 RALPH'S PROGRESS UPDATE 🎉
Ralph is doing EXCELLENT work on the LSP! Already implemented:
- ✅ Enhanced hover with type info and Zen-specific docs
- ✅ Symbol extraction and references functionality
- ✅ Basic goto definition support
- 🔄 Currently enhancing completion and references

## IMMEDIATE PRIORITY FOR RALPH:
0. **FIRST** - Explore the repository structure! Use `find`, `ls`, `tree` to understand:
   - What's in `/src/` directory (compiler components)
   - What's in `/tests/` directory (existing tests)
   - What's in `/stdlib/` directory (standard library)
   - Check `/src/lsp/` for current LSP implementation
1. **FINISH** current string type updates (StaticString/String separation)
2. **THEN START** LSP implementation from Phase 1 below
3. **TEST** each feature as you implement it
4. **COMMIT** working changes frequently

## 🚨 CRITICAL FILE ORGANIZATION RULES 🚨
- **NEVER** create test files in the root directory!
- **ALL** test files MUST go in `/tests/` folder
- **ALL** LSP tests should go in `/tests/lsp/` subfolder
- **NO** temporary test files in root - use `/tests/temp/` if needed
- **CHECK** `/tests/` folder first before creating ANY new test
- **NAME** tests descriptively: `test_[feature].zen` or `zen_test_[feature].zen`

## Priority: Make the LSP the BEST LSP EVER

### String Type System ✅ COMPLETED
- **StaticLiteral** - Internal compiler-known string literals (LLVM use only)
- **StaticString** - User-facing static strings (compile-time, immutable, no allocator)
- **String** - User-facing dynamic strings (runtime, mutable, REQUIRES allocator)

### LSP Requirements for Excellence
The LSP must be world-class with these features:

#### Core Functionality Needed
1. **Type Intelligence**
   - Full nested generic support (`Result<Option<T>,E>`)
   - Real-time type inference display
   - Allocator requirement warnings

2. **Zen-Specific Features**
   - UFC (Universal Function Call) syntax completion
   - Pattern matching with `=>` arrow syntax
   - `.raise()` propagation suggestions
   - Loop construct templates with proper closure syntax

3. **Code Actions**
   - Auto-add missing allocators to collections
   - Convert between StaticString/String
   - Generate match arms for enums
   - Fix type mismatches

4. **Advanced Features**
   - Goto definition/references
   - Rename symbol
   - Semantic highlighting
   - Inline type hints
   - Documentation on hover

### RALPH TODO LIST - MAKE THE LSP AMAZING

#### Phase 1: Core LSP Features (RALPH'S CURRENT WORK)
- [x] ✅ Implement goto definition in `src/lsp/enhanced_server.rs`
  - Track symbol definitions during parsing
  - Handle UFC method resolution
  - Support jumping to stdlib functions
- [x] ✅ Add find references functionality
  - Build reverse index of symbol usage
  - Track type aliases and imports
- [x] ✅ Implement hover with type information
  - Show inferred types for variables
  - Display function signatures with generics
  - Show allocator requirements for collections

#### NEXT: Complete UFC and Allocator Intelligence
- [ ] **PRIORITY**: Implement UFC method resolution in goto definition
  - Parse method calls like `value.method()`
  - Resolve to correct function based on type
- [ ] **PRIORITY**: Add allocator warnings in diagnostics
  - Detect HashMap/DynVec/Array without allocator
  - Provide quick fix suggestions

#### Phase 2: Zen-Specific Intelligence
- [ ] Add UFC syntax completion
  - Suggest available methods for types
  - Chain completion after `.` operator
  - Show return types in completion items
- [ ] Implement pattern matching helpers
  - Auto-complete enum variants in match arms
  - Suggest `=>` syntax after patterns
  - Generate exhaustive match arms
- [ ] Add allocator diagnostics
  - Warn when HashMap/DynVec/Array used without allocator
  - Suggest `get_default_allocator()` quick fix
  - Show allocator flow through functions

#### Phase 3: Code Actions & Quick Fixes
- [ ] Auto-add missing allocators
  - Detect collection instantiation without allocator
  - Insert allocator parameter automatically
- [ ] String type conversions
  - Quick fix to convert StaticString to String
  - Warn about String to StaticString conversions
- [ ] Error handling improvements
  - Add `.raise()` for Result types
  - Convert between Option and Result
  - Generate error handling boilerplate

#### Phase 4: Advanced Features
- [ ] Implement semantic tokens
  - Highlight UFC calls differently
  - Color allocator parameters specially
  - Distinguish StaticString vs String literals
- [ ] Add rename symbol support
  - Handle UFC method renames
  - Update import statements
  - Rename across multiple files
- [ ] Create code lens for tests
  - Show "Run Test" above test functions
  - Display test results inline
  - Quick debug test execution

#### Phase 5: Performance & Polish
- [ ] Implement incremental parsing
  - Cache AST between edits
  - Only reparse changed sections
  - Background type checking
- [ ] Add workspace symbol search
  - Fast fuzzy finding across project
  - Include stdlib symbols
  - Filter by symbol type
- [ ] Create inline type hints
  - Show inferred types for `let` bindings
  - Display closure parameter types
  - Show generic type instantiations

### Testing Requirements
- **LOCATION**: ALL tests go in `/tests/` directory (NEVER in root!)
- Create `/tests/lsp/` subdirectory for LSP-specific tests
- Each LSP feature needs corresponding test in `/tests/lsp/`
- Test with real Zen code examples
- Verify VSCode extension integration
- **BEFORE CREATING**: Always check if test already exists in `/tests/`

### Build Requirements
- Rust compiler for the compiler itself
- Stdlib should be written in Zen (self-hosting)
- Type system must be rock-solid and mature 







# PROJECT STATUS (2025-09-26)

## Test Suite Health
- **340/404 tests passing (84.2% pass rate)** - down from 93.4% due to stricter allocator requirements
- **0 segfaults** - completely eliminated!
- **64 failures** - mostly due to tests not updated for allocator requirements
- **5 disabled tests** - require major features:
  - zen_test_behaviors.zen.disabled - Behavior/trait system
  - zen_test_pointers.zen.disabled - Pointer types
  - zen_lsp_test.zen.disabled - LSP features
  - zen_test_comprehensive_working.zen.disabled - Complex integrations
  - zen_test_structs.zen.disabled - Struct syntax issues

## Critical Issues (UPDATED 2025-09-26 - ALL RESOLVED ✅)
1. ✅ **COMPLETED: NO-GC Allocator Requirements**
   - All collections (HashMap, DynVec, Array<T>) now REQUIRE allocators - compilation fails without them!
   - `get_default_allocator()` function fully implemented in memory_unified.zen
   - String concatenation now checks for allocator availability (returns error if missing)
   - **Status**: NO-GC goal 99% achieved - string ops use allocator check, full virtual dispatch pending
   - test_no_gc_comprehensive.zen validates complete NO-GC implementation
   
2. ✅ **COMPLETED: Nested Generics Support**
   - Triple-nested generics working: `Result<Option<Result<T,E>>,E>`
   - HashMap<K, Option<V>>, HashMap<K, Result<V,E>> fully functional
   - DynVec<Option<T>>, DynVec<Result<T,E>> working with allocators
   - All comprehensive nested generics tests passing
   - test_nested_generics_with_allocators.zen demonstrates complex nested types
   
3. ✅ **CLARIFIED: Collection Types Architecture**
   - **[T; N]** - Built-in fixed array syntax (stack-allocated, no allocator)
   - **Vec<T, N>** - Fixed-size vector wrapper (stack-allocated, methods for [T; N])
   - **DynVec<T>** - Dynamic vector (heap-allocated, REQUIRES allocator)
   - **Array<T>** - Dynamic array type (heap-allocated, REQUIRES allocator, simpler API)
   - **array.zen** - Utility functions for [T; N] built-in arrays
   - **Design rationale**: Each type serves distinct purpose - no redundancy
   
4. **Remaining Issues**:
   - **Type Inference**: Several tests fail on internal compiler errors
   - **Struct Methods**: Not implemented yet


## Recent Major Achievements (Last 24 Hours)
✓ **NO-GC ACHIEVED!** - All dynamic collections now require explicit allocators
✓ **get_default_allocator() Function** - Provides system allocator for collections
✓ **Enforced Allocator Requirements** - HashMap/DynVec/Array fail compilation without allocator 
✓ **Vec<T, size> FULLY IMPLEMENTED** - push/get/set/len/clear/capacity methods working with proper generic tracking
✓ **Nested Generics Working** - Result<Option<T>,E>, Option<Result<T,E>>, even triple-nested Result<Option<Result<T,E>>,E>
✓ **HashMap<K,V> FULLY FUNCTIONAL** - Both string and integer keys, proper collision handling, Option<V> returns
✓ **Pattern Matching Arrow Syntax** - QuestionMatch with => syntax now working correctly
✓ **All Segfaults Eliminated** - Went from multiple segfaults to zero through Option struct layout fixes
✓ **Test Suite Improvements** - From 90.3% to 93.4% pass rate in one day

## Historical Tasks (Archived)
[Note: 240+ completed tasks removed for brevity. Full history in git commits]


## Core Language Features Working
### Type System & Generics
✅ **Nested Generics**: Result<Option<T>,E>, Option<Result<T,E>>, triple-nested types all working
✅ **Generic Collections**: Vec<T, size>, Array<T>, HashMap<K,V> fully implemented
✅ **Pattern Matching**: Arrow syntax, qualified/shorthand enum patterns, Option/Result matching
✅ **Type Coercion**: Automatic int-to-float in operations

### Control Flow & Errors
✅ **Error Propagation**: .raise() correctly extracts Result<T,E> values
✅ **Loop Constructs**: Range loops (0..5).loop(), infinite loops with break
✅ **UFC Syntax**: Universal function call chaining

### String Methods
✅ **Parsing**: to_i32(), to_i64(), to_f64() returning Option types
✅ **Manipulation**: substr(), trim(), split(), to_upper(), to_lower()
✅ **Query**: len(), char_at(), contains(), starts_with(), ends_with(), index_of()

### Collections (ALL WITH ALLOCATORS!)
✅ **HashMap<K,V>**: REQUIRES ALLOCATOR - Insert/get with collision handling, Option<V> returns
✅ **Vec<T,N>**: Fixed-size vector (stack-allocated, no allocator needed)
✅ **DynVec<T>**: REQUIRES ALLOCATOR - Dynamic vector with push/get/set operations  
✅ **Array<T>**: REQUIRES ALLOCATOR - Dynamic arrays with standard operations
⚠️ **HashSet<T>**: REQUIRES ALLOCATOR - Partially implemented (instantiation works, methods limited)

## Implementation Progress
- **Compiler**: ~93% spec compliant (LLVM-based, 0 warnings)
- **Test Suite**: 369/395 passing (93.4%), 0 segfaults

## Not Implemented
- ❌ **Struct Methods**: Method syntax on custom structs
- ❌ **Comptime Evaluation**: Compile-time constants and assertions
- ❌ **Behaviors/Traits**: Structural contracts system
- ❌ **Pointer Types**: Ptr<T> type system
- ❌ **inline.c FFI**: C interop blocks

## Immediate Priorities

1. ✅ **COMPLETED: NO-GC Allocator System** 
   - All collections now require allocators
   - get_default_allocator() function implemented
   - Compilation enforces allocator requirements

2. **Complete Nested Generics**
   - Fix remaining edge cases (e.g., HashMap<K, Option<V>>)
   - Improve type inference for complex nested types
   - Generic function specialization

3. **Implement Struct Methods**
   - Enable method syntax on custom structs
   - Currently causing multiple test failures

4. **Fix Type Inference Issues**
   - Several tests fail with "Internal Compiler Error"
   - Closure return type inference needs improvement
   - UFC type inference failures

---

## 📁 Project Organization Guidelines

### CRITICAL: File Organization Rules
- **NEVER** place test files in the root directory
- **ALL** test files must go in the `/tests/` folder
- **ALWAYS** check existing tests in `/tests/` folder before creating new ones to avoid duplication
- Scripts belong in `/scripts/` folder, not root
- **ALL** analysis reports, progress documents, and thinking artifacts must go in `/.agent/` folder (NEVER in root)

### Pre-Development Checklist
Before making any changes, **ALWAYS**:
1. Check the entire project structure (except `/target/`, `/node_modules/`, `/.git/`)
2. Search for existing implementations in `/tests/` folder
3. Look for duplicate files across folders  
4. Review existing patterns in codebase before implementing new code

### Test File Naming
- Use descriptive names: `zen_test_[feature].zen`
- Group related tests in single files rather than creating many small test files
- Check for existing test coverage before adding new tests

### Analysis and Progress Artifacts
- **ALL** analysis reports (ARCHITECTURAL_CLEANUP_REPORT.md, RAISE_ISSUE_ANALYSIS.md, etc.) → `/.agent/` folder
- **ALL** progress tracking documents → `/.agent/` folder  
- **ALL** thinking and planning artifacts → `/.agent/` folder
- **NEVER** clutter the root directory with temporary analysis files

### Loop Syntax (CRITICAL)
Zen's loop construct manages **internal state** and can pass multiple parameters to closures:
- ✅ `loop() { ... }` - Infinite loop with `break` statement
- ✅ `loop(() { ... })` - Closure-based loop with internal state management
- ✅ `loop((handle) { ... })` - Loop provides control handle (`handle.break()`, `handle.continue()`)
- ✅ `(range).loop((i) { ... })` - Range provides index/value to closure
- ✅ `collection.loop((item) { ... })` - Collection provides each item to closure
- ✅ `collection.loop((item, index) { ... })` - Collection provides item and index
- ✅ `range.loop((value, handle) { ... })` - Multiple parameters: value and control handle
- ❌ `loop(condition) { ... }` - **INCORRECT**: external state condition not supported
- ❌ `loop(i < 3) { ... }` - **INCORRECT**: external variable condition not supported
- **Key principle**: Loop manages internal state and provides context via closure parameters
- **Patterns**: 
  - `loop(() { condition ? { break }; ... })`
  - `loop((handle) { condition ? { handle.break() }; ... })`
  - `(0..10).loop((i) { i == 5 ? { break }; ... })`



<!-- META INFORMATION TO CODING AGENT, DO NOT MODIFY PAST THIS POINT -->

## ENVIRONMENT
- Current directory: ralph
- OOM issue causing system lockups - be careful with builds
- SendGrid API key in env

## .agent/ MEMORY
- `context.md` - what's true right now (current state, what works/fails, key learnings)
- `attempts.md` - things I tried that didn't work (with error messages) - don't repeat these
- `focus.md` - current task, next 3 steps, blockers if any

## TOOLS & WORKFLOW
- gh CLI for github management
- curl for emails (no temp files)
- Git: frequent commits, push often, merge to main when it is smart to
- Don't commit binaries (out, executables)
- Update README to match reality

## CONTACT & NOTIFICATIONS

### Email Configuration
- **Service**: SendGrid curl
- **To**: l.leong1618@gmail.com 
- **From**: agent@lambda.run
- **Subject Format**: `zen-lang-[STATUS]-[CONTEXT]`

### When to Send Email Notifications:

#### 🚨 CRITICAL - Send Immediately
- **Compilation failures** that break the build
- **System crashes** or OOM issues during development
- **Major blockers** that prevent progress for >30 minutes
- **Breaking changes** to core language features
- **Data loss** or file corruption incidents

#### 📈 PROGRESS - Send Every Few Hours
- **Major milestones** completed (e.g., "Range loops now working")
- **Test suite improvements** (>10% pass rate increase)
- **New features** fully implemented and tested
- **Significant bug fixes** that unlock other work

#### 📊 SUMMARY - Send Daily
- **Work session summaries** with tasks completed/remaining
- **Current status** of the 3 critical issues (range loops, Option types, error propagation)
- **Test results** and compliance metrics
- **Next day planning** if working multi-day

#### 🏁 COMPLETION - Send Always  
- **Task completion** when major goals achieved
- **Session termination** with full summary
- **Handoff notes** for next development session

### Email Content Guidelines
- **Subject line** should clearly indicate urgency and context
- **First line** should summarize the key point in one sentence
- **Include relevant** file paths, error messages, or test results
- **End with** clear next steps or actions needed

## PERFORMANCE HINTS
- Best at 40% context (100K-140K tokens)
- 80% coding, 20% testing ratio works well
- Principles: DRY, KISS, simplicity, elegance, practicality, intelligence
- Order todos with time estimates

## META
- Modifying prompt.md triggers new loop (use wisely)
- Can kill process when done with pwd ralph
- ELEGANCE, EFFICENCY AND EXPRESSIVENESS 