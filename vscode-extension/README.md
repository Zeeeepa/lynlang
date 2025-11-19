# Zen Language Support for VS Code

This extension provides language support for the Zen programming language in Visual Studio Code.

## Features

- **Syntax Highlighting**: Full syntax highlighting for Zen code
- **CodeLens Actions**: Inline buttons to run and build functions
  - **▶ Run** button for all functions
  - **🔨 Build** button for main and build entry points
  - Automatic detection of `main` and `build` functions
  - Click to execute directly from your editor
- **Language Server Protocol**: Advanced IDE features powered by `zen-lsp`
  - Real-time error checking and diagnostics
  - Code completion for types, functions, and keywords
  - Hover information
  - Go to definition
  - Find references
  - Document symbols
  - Code formatting
- **Code Snippets**: Common Zen code patterns

## Requirements

- The Zen compiler must be installed and available in your PATH
- The `zen-lsp` language server must be built and available

## Installation

### Building the Language Server

1. Build the Zen LSP server:
```bash
cd /path/to/zen
cargo build --release --bin zen-lsp
```

2. Add the binary to your PATH or configure the extension to use the full path.

### Installing the Extension

1. Install dependencies:
```bash
cd vscode-extension
npm install
```

2. Compile the extension:
```bash
npm run compile
```

3. In VS Code, press F5 to launch a new Extension Development Host window with the extension loaded.

## Extension Settings

This extension contributes the following settings:

- `zen.serverPath`: Path to the Zen language server executable (default: `zen-lsp`)
- `zen.trace.server`: Enable tracing of communication between VS Code and the language server

## Usage

1. Open any `.zen` file in VS Code
2. The extension will automatically activate and start the language server
3. You'll see syntax highlighting immediately
4. Error checking and other language features will be available once the language server initializes

### Using CodeLens Actions

1. The extension automatically detects functions in your code
2. Look for **▶ Run** and **🔨 Build** buttons above function definitions
3. Click **▶ Run** to execute a function
4. Click **🔨 Build** to compile a function (available for `main` and `build` functions)
5. Output appears in the "Zen Run" or "Zen Build" output channel at the bottom

See [CODELENS_FEATURE.md](./CODELENS_FEATURE.md) for detailed information about the CodeLens feature.

## Development

To work on this extension:

1. Open the `vscode-extension` folder in VS Code
2. Run `npm install` to install dependencies
3. Press F5 to launch the Extension Development Host
4. Make changes and reload the window to test

## Known Issues

- The language server is in early development and some features may not be fully implemented
- Performance may vary with large files

## Release Notes

### 0.1.0

Initial release with basic language support:
- Syntax highlighting
- Language server integration
- Basic code completion
- Error diagnostics