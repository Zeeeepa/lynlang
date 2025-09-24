# Zen Language Development Priority Fixes

## 🚨 CRITICAL ISSUES TO FIX IMMEDIATELY

Based on test results and spec compliance analysis, these are the highest priority fixes needed:

### 1. Range Loop Parser Issue (CRITICAL)
**Problem:** Range loops only execute once instead of iterating properly
- `(0..10).loop((i) { ... })` should iterate 0,1,2,3,4,5,6,7,8,9 but only runs once
- Root cause: Parser not generating correct MethodCall AST for `(range).loop()`

**Fix Required:**
- Update parser to properly handle method calls on range expressions
- Ensure `(0..10).loop()` generates correct AST structure
- Test with various range patterns: `(0..4)`, `(1..5).step(2)`, etc.

### 2. Option Type Implementation (HIGH)
**Problem:** `Option<T>` with `Some(T)` and `None` pattern matching broken
- Values not handled correctly in pattern matching
- Option type construction and destructuring fails

**Fix Required:**
- Complete Option<T> enum implementation in stdlib
- Fix pattern matching for `Some(value)` and `None` variants  
- Ensure `.raise()` works with Option types
- Test: `maybe_value ? | Some(x) { ... } | None { ... }`

### 3. Error Propagation with .raise() (HIGH) 
**Problem:** `.raise()` method for early return error propagation not implemented
- Critical for Result<T,E> error handling
- Needed for idiomatic Zen error handling patterns

**Fix Required:**
- Implement `.raise()` method on Result types
- Add early return semantics in functions returning Result
- Test: `file.read().raise()` should propagate errors up call stack

## 🎯 Implementation Strategy

### Phase 1: Range Loop Fix
1. **Parser Changes:** Update range expression parsing to properly handle method calls
2. **AST Generation:** Ensure `(range).loop()` creates correct MethodCall node
3. **Testing:** Verify all range patterns work: `(0..n)`, `(start..end)`, `(0..n).step(s)`

### Phase 2: Option Type Completion  
1. **Type System:** Complete Option<T> enum with Some/None variants
2. **Pattern Matching:** Fix destructuring in match expressions
3. **Integration:** Ensure works with existing variable system

### Phase 3: Error Propagation
1. **Result Type:** Complete Result<T,E> implementation  
2. **Raise Method:** Add .raise() for early returns
3. **Function Integration:** Update function returns to handle Result propagation

## 🧪 Test Cases to Pass

After fixes, these should work:
```zen
// Range loops
(0..5).loop((i) { 
    io.println("Count: ${i}")  // Should print 0,1,2,3,4
})

// Option handling  
maybe_value: Option<i32> = Some(42)
maybe_value ?
    | Some(x) { io.println("Got: ${x}") }
    | None { io.println("No value") }

// Error propagation
load_file = (path: string) Result<string, Error> {
    contents = File.read(path).raise()  // Early return on error
    return Ok(contents)
}
```

## 📋 Success Criteria

- [ ] Range loops iterate correct number of times
- [ ] Option<T> pattern matching works properly  
- [ ] .raise() propagates errors correctly
- [ ] Test suite passes >75% (currently 50%)
- [ ] Core language spec compliance >90%

## ⚠️ Notes

- Focus ONLY on these three critical issues
- Do not add new features until these core issues are resolved
- All changes must align with LANGUAGE_SPEC.zen
- Test each fix thoroughly before moving to next issue

## 📁 Project Organization Guidelines

### CRITICAL: File Organization Rules
- **NEVER** place test files in the root directory
- **ALL** test files must go in the `/tests/` folder
- **ALWAYS** check existing tests in `/tests/` folder before creating new ones to avoid duplication
- Scripts belong in `/scripts/` folder, not root

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
