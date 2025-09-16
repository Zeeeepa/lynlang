# Zenlang VS Code Extension

Language support for Zenlang programming language in Visual Studio Code.

## Features

- **Syntax Highlighting**: Full syntax highlighting for Zenlang code
- **Language Server Protocol**: Integration with Zenlang LSP for advanced IDE features
- **Code Completion**: Intelligent code suggestions
- **Hover Information**: Type information and documentation on hover
- **Go to Definition**: Navigate to symbol definitions
- **Diagnostics**: Real-time error and warning highlighting
- **Commands**:
  - Build Zen project (`Zenlang: Build Zen Project`)
  - Run Zen project (`Zenlang: Run Zen Project`)  
  - Run Zen tests (`Zenlang: Run Zen Tests`)
  - Restart Language Server (`Zenlang: Restart Zenlang Language Server`)
  - Show LSP output (`Zenlang: Show Language Server Output`)

## Installation

1. Open VS Code
2. Go to Extensions (Ctrl+Shift+X)
3. Search for "Zenlang"
4. Click Install

Or install manually:
```bash
cd vscode-zenlang
npm install
npm run compile
```

Then package the extension:
```bash
npm install -g vsce
vsce package
```

Install the generated `.vsix` file in VS Code.

## Configuration

The extension can be configured through VS Code settings:

- `zenlang.lsp.enabled`: Enable/disable the Language Server (default: true)
- `zenlang.lsp.path`: Path to the Zenlang LSP executable (auto-detected in tools/zen-lsp.sh)
- `zenlang.lsp.trace.server`: Trace LSP communication for debugging
- `zenlang.compiler.path`: Path to the Zen compiler (default: "zen")
- `zenlang.format.onSave`: Format Zen files on save (default: true)

## Requirements

- VS Code 1.74.0 or higher
- Zenlang compiler and LSP server installed

## Language Features

### Syntax Highlighting

The extension provides comprehensive syntax highlighting for:
- **No traditional keywords** - Zen has no if/else/while/for/match/async/await/impl/trait/class/interface/null
- Types (i32, u64, bool, String, Option, Result, custom types)
- Functions and methods with Uniform Function Call syntax
- Structs and enums
- Comments (line and block)
- Strings and numbers
- Operators including the `?` pattern matching operator
- Special symbols (@std for standard library, @this for current scope)

### Pattern Support

- Variable declarations: `x = 42` (immutable), `y ::= 10` (mutable)
- Type definitions: `Point: {x: f64, y: f64}`
- Function definitions: `main = () i32 { ... }`
- Enum definitions: `Status: Ok | Error`
- Pattern matching with `?` operator:
  ```zen
  value ?
      | true { process() }
      | false { wait() }
  ```
- Option and Result types for null-safety and error handling
- Method chaining with `.raise()` for error propagation
- Loop constructs: `loop()`, `items.loop()`, `(0..10).loop()`

## Development

To contribute to the extension:

1. Clone the repository
2. Run `npm install`
3. Open in VS Code
4. Press F5 to launch a development instance

## License

Same as Zenlang project license.