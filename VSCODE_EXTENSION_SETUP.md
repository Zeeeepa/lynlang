# VSCode Extension Setup Guide

## What's Been Done

1. ✅ Compiled the Zen compiler (`/home/ubuntu/zenlang/target/release/zen`)
2. ✅ Built the VSCode extension with CodeLens support
3. ✅ Packaged the extension as `zen-language-0.1.0.vsix`

## Installation Steps

### For Remote Development

1. **Download the extension package**:
   - Location: `/home/ubuntu/zenlang/vscode-extension/zen-language-0.1.0.vsix`
   - Download it to your local machine

2. **Install in VSCode**:
   - Open VSCode
   - Go to Extensions (Ctrl+Shift+X / Cmd+Shift+X)
   - Click "..." menu → "Install from VSIX..."
   - Select the downloaded `zen-language-0.1.0.vsix`

3. **Open the remote workspace**:
   - Use VSCode Remote SSH to connect to your server
   - Open `/home/ubuntu/zenlang` as the workspace

4. **Configure the Zen compiler path** (in VSCode settings):
   ```json
   {
       "zen.serverPath": "/home/ubuntu/zenlang/target/release/zen-lsp"
   }
   ```

## Features Available

### CodeLens Actions
When you open `.zen` files, you'll see inline buttons:

- **▶ Run** - Execute any function
- **🔨 Build** - Compile `main()` or `build()` functions

### How to Use
1. Open any `.zen` file
2. Look for the Run/Build buttons above function definitions
3. Click to execute directly in VSCode
4. Output appears in the "Zen Run" or "Zen Build" output panel

### Try It Out
```bash
# Open this demo file
code /home/ubuntu/zenlang/examples/codelens_demo.zen
```

You'll see CodeLens buttons above the functions.

## Manual Installation (Local Testing)

If testing locally on the remote SSH session:

1. **Set PATH to include the Zen compiler**:
   ```bash
   export PATH="/home/ubuntu/zenlang/target/release:$PATH"
   ```

2. **Test the extension in development mode**:
   ```bash
   cd /home/ubuntu/zenlang/vscode-extension
   npm install
   npm run compile
   # Then press F5 in VSCode to launch Extension Development Host
   ```

## Troubleshooting

### "zen command not found"
Ensure the zen compiler is in your PATH:
```bash
which zen
# Should output: /home/ubuntu/zenlang/target/release/zen
```

If not, add it:
```bash
export PATH="/home/ubuntu/zenlang/target/release:$PATH"
```

### CodeLens buttons not showing
1. Ensure the file is saved as `.zen`
2. Verify language is set to "Zen" (bottom right of VSCode)
3. Close and reopen the file
4. Check the extension is activated in Output panel

### Build errors
Check the "Zen Language Server" output channel for details

## Files Reference

- **Extension code**: `/home/ubuntu/zenlang/vscode-extension/src/extension.ts`
- **Package manifest**: `/home/ubuntu/zenlang/vscode-extension/package.json`
- **Feature documentation**: `/home/ubuntu/zenlang/vscode-extension/CODELENS_FEATURE.md`
- **Example file**: `/home/ubuntu/zenlang/examples/codelens_demo.zen`
- **Compiled extension**: `/home/ubuntu/zenlang/vscode-extension/zen-language-0.1.0.vsix`

## What the CodeLens Feature Does

The VSCode extension now detects entry point functions in your code and provides quick action buttons:

```zen
// This function gets both Run and Build buttons
fn main() {
    print("Hello!")
}

// This function gets both Run and Build buttons
fn build() {
    compile()
}

// Regular functions get a Run button only
fn myFunction() {
    process()
}
```

Clicking the buttons will:
- Execute `zen run <file.zen>` for Run
- Execute `zen build <file.zen>` for Build
- Display output in the output panel
- Show errors in a dialog

## Next Steps

1. Download the `.vsix` file
2. Install it in VSCode
3. Connect via Remote SSH
4. Open a `.zen` file and try clicking the buttons!
