# VSCode CodeLens Feature for Zen Language

This feature provides inline buttons in VSCode to run and build Zen functions directly from your editor.

## Overview

When you open a `.zen` file in VSCode, the extension automatically detects function definitions and displays action buttons (CodeLens) above them:

- **▶ Run** - Executes the function using `zen run`
- **🔨 Build** - Compiles the function using `zen build`

## Supported Patterns

### Main Functions (Get both Run and Build buttons)

```zen
// Pattern 1: fn main() syntax
fn main() {
    // code
}

// Pattern 2: main assignment
main = fn() {
    // code
}
```

### Build Functions (Get both Run and Build buttons)

```zen
// Pattern 1: fn build() syntax
fn build() {
    // code
}

// Pattern 2: build assignment
build = fn() {
    // code
}
```

### Regular Functions (Get Run button only)

```zen
fn myFunction() {
    // code
}

fn processData() {
    // code
}
```

## Usage

1. Open any `.zen` file in VSCode
2. Look for the CodeLens buttons above function definitions
3. Click **▶ Run** to execute the function
4. Click **🔨 Build** to compile the function
5. Output appears in the "Zen Run" or "Zen Build" output channel

## Features

- **Automatic Detection**: Scans the file for function definitions when opened
- **Real-time Updates**: CodeLens updates when the file changes
- **Output Capture**: Displays stdout, stderr, and command output
- **Error Handling**: Shows user-friendly error messages on failure
- **Line Number Tracking**: Includes the line number in the output panel

## Implementation Details

The feature is implemented through:

1. **ZenCodeLensProvider** - A VSCode CodeLensProvider that:
   - Scans each line for function patterns
   - Creates CodeLens objects for each function
   - Distinguishes between main, build, and regular functions

2. **Command Handlers** - Register two commands:
   - `zen.run` - Executes `zen run <file>`
   - `zen.build` - Executes `zen build <file>`

3. **Output Channel** - Displays execution results in dedicated output panels

## Requirements

- The `zen` command must be available in your PATH
- `zen run` and `zen build` commands must be properly implemented
- A workspace folder must be open in VSCode

## Configuration

The extension looks for the Zen compiler in the following order:

1. Path specified in `zen.serverPath` configuration
2. Workspace root directory
3. Extension directory (for development)
4. System PATH

You can customize the server path in VSCode settings:

```json
{
    "zen.serverPath": "/path/to/zen-lsp"
}
```

## Troubleshooting

### CodeLens buttons not appearing

1. Ensure your Zen file has valid function syntax
2. Try closing and reopening the file
3. Check that the language is set to "Zen" (bottom right of VSCode)

### "zen command not found" error

1. Ensure the `zen` CLI tool is installed
2. Add it to your system PATH
3. Or configure `zen.serverPath` in settings

### No output displayed

1. Check the "Zen Run" or "Zen Build" output channel
2. Ensure your file is saved
3. Check that your function is valid
