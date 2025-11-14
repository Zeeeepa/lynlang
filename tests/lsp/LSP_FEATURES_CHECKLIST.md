# Modern LSP Features Checklist

## ✅ Implemented Features

### Core Language Features
- [x] **Hover** - Type information, documentation
- [x] **Completion** - Code completion with context
- [x] **Go to Definition** - Navigate to symbol definition
- [x] **Go to Type Definition** - Navigate to type definition
- [x] **Find References** - Find all references to symbol
- [x] **Document Highlight** - Highlight all occurrences
- [x] **Document Symbols** - List symbols in file
- [x] **Workspace Symbols** - Search across workspace

### Code Intelligence
- [x] **Signature Help** - Function parameter hints
- [x] **Inlay Hints** - Parameter names, type hints
- [x] **Semantic Tokens** - Syntax highlighting
- [x] **Code Lens** - Code actions, references count

### Code Actions
- [x] **Code Actions** - Quick fixes, refactorings
- [x] **Rename Symbol** - Rename across files
- [x] **Formatting** - Document and range formatting

### Advanced Features
- [x] **Call Hierarchy** - Incoming/outgoing calls
- [x] **Folding Range** - Code folding regions
- [x] **Diagnostics** - Errors, warnings, info

### Document Management
- [x] **Text Document Sync** - Incremental sync
- [x] **Did Open** - Document opened
- [x] **Did Change** - Document changed
- [x] **Did Close** - Document closed

## 🔧 Needs Testing/Fixing

### Hover Issues
- [ ] Hover on `${person}` shows struct definition ✅ (fixed)
- [ ] Hover on `person.name` shows field type ✅ (fixed)
- [ ] Hover on struct fields shows correct type
- [ ] Hover on function calls shows signature
- [ ] Hover on imports shows module info
- [ ] Hover on pattern match variables shows type

### Completion Issues
- [ ] Completion after `person.` shows struct fields (not stdlib) ✅ (fixed)
- [ ] Completion for imports (`@std.`)
- [ ] Completion for enum variants
- [ ] Completion for pattern matching
- [ ] Completion resolve provides detailed info
- [ ] Completion filtering works correctly

### Navigation Issues
- [ ] Go to definition works cross-file
- [ ] Go to type definition for structs
- [ ] Find references includes all occurrences
- [ ] Document highlight works correctly
- [ ] Workspace symbol search is accurate

### Code Intelligence Issues
- [ ] Signature help shows active parameter
- [ ] Signature help retriggers on comma
- [ ] Inlay hints show parameter names
- [ ] Inlay hints show type information
- [ ] Semantic tokens cover all constructs
- [ ] Code lens shows reference counts

### Code Actions Issues
- [ ] Code actions for common errors
- [ ] Code actions for missing imports
- [ ] Rename works across files
- [ ] Rename validates new name
- [ ] Formatting preserves comments
- [ ] Formatting handles edge cases

## 🚀 Missing Features (Future)

### LSP 3.17 Features
- [ ] **Document Link** - Links in documentation
- [ ] **Color Provider** - Color information
- [ ] **Linked Editing Range** - Synchronized editing
- [ ] **Moniker** - Symbol identity
- [ ] **Selection Range** - Expand selection
- [ ] **Type Hierarchy** - Type inheritance

### Workspace Features
- [ ] **Will Save** - Before save hook
- [ ] **Will Save Wait Until** - Async save
- [ ] **Did Change Configuration** - Config changes
- [ ] **Did Change Watched Files** - File watchers
- [ ] **Will Create Files** - File creation
- [ ] **Will Rename Files** - File rename
- [ ] **Will Delete Files** - File deletion
- [ ] **Execute Command** - Custom commands
- [ ] **Apply Edit** - Workspace edits

### Advanced Diagnostics
- [ ] **Diagnostic Tags** - Unnecessary, deprecated
- [ ] **Related Information** - Related diagnostics
- [ ] **Code Description** - Diagnostic codes

## 📋 Test Coverage

### Test Files
- `test_unified_lsp.py` - Main unified test suite
- `test_hello_world.py` - Hello world specific tests
- `test_comprehensive_automated.py` - Comprehensive automated tests

### Test Categories
1. **Core Features** - Hover, completion, navigation
2. **Code Intelligence** - Signature help, inlay hints
3. **Code Actions** - Rename, formatting, code actions
4. **Advanced** - Call hierarchy, semantic tokens
5. **Workspace** - Symbols, diagnostics

## 🎯 Priority Fixes

### High Priority
1. ✅ Completion shows struct fields (fixed)
2. ✅ Hover on format strings (fixed)
3. Cross-file navigation
4. Type inference accuracy
5. Workspace symbol search

### Medium Priority
1. Code actions for errors
2. Semantic tokens completeness
3. Inlay hints accuracy
4. Signature help improvements
5. Formatting edge cases

### Low Priority
1. Missing LSP 3.17 features
2. Advanced workspace features
3. Performance optimizations

## 📝 Notes

- All implemented features should be tested systematically
- Focus on fixing existing features before adding new ones
- Use unified test suite for consistency
- Keep test files organized and maintainable

