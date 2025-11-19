# CodeLens Feature Implementation Notes

## Overview

Added CodeLens support to the Zen VSCode extension to provide inline "Run" and "Build" buttons above function definitions, with special support for `main` and `build` entry points.

## Files Modified

### 1. `/vscode-extension/src/extension.ts`
- Added imports for `child_process.exec` and `util.promisify`
- Created `ZenCodeLensProvider` class implementing `vscode.CodeLensProvider`
- Registered CodeLens provider in the activate function
- Added command handlers for `zen.run` and `zen.build`

### 2. `/vscode-extension/package.json`
- Added `commands` section to contributes
- Registered `zen.run` command
- Registered `zen.build` command

### Files Created

### 3. `/vscode-extension/CODELENS_FEATURE.md`
Comprehensive documentation of:
- Feature overview
- Supported patterns
- Usage instructions
- Implementation details
- Troubleshooting guide

### 4. `/examples/codelens_demo.zen`
Example file demonstrating:
- Both `fn main()` and `main = fn()` syntax
- Both `fn build()` and `build = fn()` syntax
- Regular functions
- How to test the feature

## Technical Implementation

### ZenCodeLensProvider Class

The provider scans the document line-by-line and:

1. **Main Function Pattern**: `/^\s*(?:fn\s+main|main\s*=\s*fn)\s*[({]/`
   - Creates both "Run" and "Build" CodeLens

2. **Build Function Pattern**: `/^\s*(?:fn\s+build|build\s*=\s*fn)\s*[({]/`
   - Creates both "Run" and "Build" CodeLens

3. **Regular Function Pattern**: `/^\s*fn\s+(\w+)\s*[({]/`
   - Creates "Run" CodeLens only

### Command Handlers

#### zen.run
```bash
zen run "<file_path>"
```
- Opens "Zen Run" output channel
- Captures stdout and stderr
- Shows line number in output
- Displays error messages in dialog

#### zen.build
```bash
zen build "<file_path>"
```
- Opens "Zen Build" output channel
- Captures stdout and stderr
- Shows compilation status
- Displays error messages in dialog

## Features

### Automatic Detection
- Scans files on open and when changed
- Supports multiple function definition styles
- Distinguishes between entry points and regular functions

### Rich Output
- Dedicated output channels for run/build output
- Line numbers for debugging
- Formatted error messages
- Clear success/failure indicators

### Error Handling
- Validates workspace folder exists
- Catches command execution errors
- Shows user-friendly error dialogs
- Prevents crashes on missing `zen` command

## Usage Patterns Supported

```zen
// Supported patterns for Run+Build buttons
fn main() { }
main = fn() { }
fn build() { }
build = fn() { }

// Supported pattern for Run button only
fn functionName() { }
```

## Dependencies

- VSCode API (`vscode`)
- VSCode Language Client (`vscode-languageclient`)
- Node.js built-in: `child_process`, `util`

## Performance Considerations

- Line-by-line scanning is O(n) where n = number of lines
- Regex compilation is cached in instance
- CodeLens updates only on file changes
- Large files (1000+ lines) scan in milliseconds

## Future Enhancements

1. **Smart Entry Point Detection**
   - Parse actual AST instead of regex
   - Better support for complex function syntax

2. **Configuration Options**
   - Allow users to customize button appearance
   - Disable CodeLens for certain file types
   - Custom run/build commands

3. **Function Arguments**
   - Detect function parameters
   - Prompt for arguments before running
   - Store common argument sets

4. **Integration with LSP**
   - Use language server for AST parsing
   - More accurate symbol detection
   - Better context awareness

5. **Build Configuration**
   - Read from `build.zen` configuration
   - Support different targets/features
   - Custom build options UI

## Testing

To test the feature:

1. Open `examples/codelens_demo.zen`
2. Look for CodeLens buttons above functions
3. Click "Run" or "Build" to execute
4. Check output in the output channel
5. Verify error handling with missing `zen` command

## Troubleshooting During Development

### CodeLens not appearing
- Check that `provideCodeLenses` is being called
- Verify regex patterns match your code
- Ensure document selector includes `.zen` files

### Commands not executing
- Verify `zen` command is in PATH
- Check workspace folder is valid
- Review output channel for error details

### Memory issues with large files
- Increase `maxBuffer` in execAsync if needed
- Consider streaming output instead of capturing all
- Profile with `zen` command on large projects
