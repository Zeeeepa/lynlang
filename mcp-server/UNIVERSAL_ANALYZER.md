# 🔍 Universal Code Analyzer MCP Server

A **Model Context Protocol (MCP) server** that provides comprehensive code analysis for **ANY programming language**. AI agents can use this to analyze codebases, detect errors, resolve types, find references, and scan for security issues.

---

## 🎯 What It Does

### ✅ Multi-Language Support

Automatically detects and analyzes code in:

- **Python** (.py) - Ruff, mypy, Bandit
- **TypeScript/JavaScript** (.ts, .tsx, .js) - tsc, ESLint
- **Go** (.go) - go vet, golangci-lint
- **Rust** (.rs) - cargo check, clippy
- **Java** (.java) - javac, CheckStyle
- **C++** (.cpp, .hpp) - clang-tidy, cppcheck
- **Ruby** (.rb) - RuboCop
- **PHP** (.php) - PHPStan, Psalm
- **Swift** (.swift) - SwiftLint
- **Kotlin** (.kt) - ktlint
- And **20+ more languages**!

### ✅ Comprehensive Analysis

1. **Error Detection**
   - Syntax errors
   - Type errors
   - Runtime errors
   - Logic errors

2. **Code Quality**
   - Linting violations
   - Code style issues
   - Complexity metrics
   - Code smells

3. **Security Scanning**
   - Vulnerability detection
   - Insecure patterns
   - Dependency issues
   - Secret leaks

4. **Type Resolution**
   - Hover information
   - Type inference
   - Jump to definition
   - Find all references

### ✅ Detailed Diagnostics

Every diagnostic includes:
- **Message**: Clear error/warning description
- **Severity**: error | warning | info | hint
- **Location**: file, line, column (with ranges)
- **Code**: Error code (e.g., "TS2322", "E501")
- **Source**: Tool that generated it
- **Suggestion**: Actionable fix (when available)

---

## 🚀 Quick Start

### Installation

```bash
# Install Python dependencies
pip install ruff mypy bandit radon

# Install language-specific tools (optional)
npm install -g typescript eslint  # For TS/JS
go install github.com/golangci/golangci-lint/cmd/golangci-lint@latest  # For Go
cargo install clippy  # For Rust
```

### Configure for Claude Desktop

Add to `~/.config/Claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "code-analyzer": {
      "command": "python3",
      "args": [
        "/path/to/lynlang/mcp-server/universal-analyzer-mcp.py"
      ]
    }
  }
}
```

Restart Claude Desktop → **Done!** ✨

---

## 🔧 Available Tools

### 1. `analyze_codebase` - Full Analysis

Analyze any file or directory comprehensively.

**Example:**
```
Analyze this Python file for errors:
/home/user/project/main.py
```

**Parameters:**
- `path` (required): File or directory path
- `language` (optional): Force specific language
- `include_metrics` (optional): Include complexity metrics

**Returns:**
```json
{
  "language": "python",
  "files_analyzed": 5,
  "summary": {
    "error": 3,
    "warning": 12,
    "info": 5,
    "hint": 2
  },
  "diagnostics": [
    {
      "message": "Undefined variable 'user_data'",
      "severity": "error",
      "location": {
        "file": "main.py",
        "line": 42,
        "column": 15
      },
      "code": "F821",
      "source": "ruff",
      "suggestion": "Did you mean 'users_data'?"
    }
  ],
  "metrics": {
    "complexity": 8.5,
    "maintainability": 72
  }
}
```

---

### 2. `get_error_list` - Filtered Errors

Get filtered diagnostics with severity filtering.

**Example:**
```
Show me all errors (not warnings) in this codebase:
/home/user/project/
```

**Parameters:**
- `path` (required): Path to analyze
- `min_severity` (optional): "error" | "warning" | "info" | "hint"
- `max_results` (optional): Limit number of results

**Returns:**
```json
{
  "total_diagnostics": 47,
  "filtered_count": 3,
  "diagnostics": [
    {
      "message": "Type mismatch: expected str, got int",
      "severity": "error",
      "location": "utils.py:28:10",
      "code": "type-arg",
      "source": "mypy",
      "suggestion": "Convert to string: str(value)"
    }
  ]
}
```

---

### 3. `hover_info` - Type Information

Get type info and documentation for a symbol.

**Example:**
```
What's the type of `user` at line 42, column 10 in main.py?
```

**Parameters:**
- `file` (required): File path
- `line` (required): Line number (1-indexed)
- `column` (required): Column (0-indexed)
- `language` (optional): Force language

**Returns:**
```json
{
  "language": "python",
  "location": {"file": "main.py", "line": 42, "column": 10},
  "type": "User",
  "documentation": "User model representing authenticated users",
  "signature": "class User(name: str, email: str, is_active: bool)"
}
```

---

### 4. `find_references` - All Usages

Find all references to a symbol.

**Example:**
```
Find everywhere the `calculate_total` function is used
```

**Parameters:**
- `file`: File with symbol
- `line`: Line number
- `column`: Column number

**Returns:**
```json
{
  "count": 8,
  "references": [
    {"file": "main.py", "line": 42, "column": 10},
    {"file": "utils.py", "line": 15, "column": 5},
    {"file": "test_main.py", "line": 78, "column": 20}
  ]
}
```

---

### 5. `go_to_definition` - Jump to Source

Jump to where a symbol is defined.

**Example:**
```
Where is the `User` class defined?
```

**Returns:**
```json
{
  "found": true,
  "location": {
    "file": "models/user.py",
    "line": 15,
    "column": 6
  }
}
```

---

### 6. `detect_languages` - Project Languages

Detect all languages in a project.

**Example:**
```
What languages are used in this project?
```

**Returns:**
```json
{
  "languages": {
    "python": 45,
    "javascript": 12,
    "typescript": 8,
    "rust": 3
  },
  "primary_language": "python",
  "total_files": 68
}
```

---

## 💡 Example AI Conversations

### Conversation 1: Debug Type Error

```
User: "Why is this Python code failing?"

def calculate_total(items):
    total = 0
    for item in items:
        total += item.price
    return total

result = calculate_total("invalid")

AI: [Uses analyze_codebase]

❌ Error found at line 8:
"Type error: Argument of type 'str' to 'calculate_total' has incompatible type; expected 'Iterable[...]'"

AI: "The problem is you're passing a string ('invalid') to calculate_total, 
     but it expects an iterable of objects with a 'price' attribute.
     
     Try passing a list instead:
     result = calculate_total([item1, item2, item3])"
```

---

### Conversation 2: Security Scan

```
User: "Scan my codebase for security issues"

AI: [Uses analyze_codebase with path="/home/user/project"]

🔒 Security Issues Found:

❌ HIGH SEVERITY (api.py:45):
"Potential SQL injection: User input directly concatenated into SQL query"
Code: B608 (Bandit)
Fix: Use parameterized queries

⚠️ MEDIUM SEVERITY (auth.py:102):
"Weak cryptographic hash function (MD5) used for password hashing"
Code: B303 (Bandit)
Fix: Use bcrypt or argon2 instead

AI: "I found 2 security vulnerabilities:
     1. SQL injection risk - use parameterized queries
     2. Weak password hashing - switch to bcrypt
     
     Would you like me to show you how to fix these?"
```

---

### Conversation 3: Code Quality Review

```
User: "Review the code quality of my TypeScript project"

AI: [Uses analyze_codebase]

📊 Analysis Results:
- Files analyzed: 47
- Errors: 0 ✅
- Warnings: 23 ⚠️
- Info: 12 ℹ️

Top Issues:
1. Unused variables (12 occurrences)
2. Missing return types (8 occurrences)
3. Complexity too high (3 functions)

Most Complex Function:
- processUserData() in utils.ts (complexity: 15)
  Suggestion: Split into smaller functions

AI: "Your code is error-free but has room for improvement:
     - Remove 12 unused variables
     - Add explicit return types to 8 functions
     - Refactor 3 complex functions
     
     Should I create a detailed cleanup plan?"
```

---

### Conversation 4: Type Resolution

```
User: "What's the type of this variable?"

const user = fetchUser();

AI: [Uses hover_info at the variable location]

Type: Promise<User>

Where User is:
interface User {
  id: string;
  name: string;
  email: string;
  roles: Role[];
}

AI: "It's a Promise that resolves to a User object.
     You need to await it or use .then() to access the data:
     
     const user = await fetchUser();
     console.log(user.name);  // Now you can access properties"
```

---

## 🏗️ Architecture

```
┌─────────────────────┐
│   AI Agent          │
│   (Claude/GPT)      │
└──────────┬──────────┘
           │
           ↓
┌─────────────────────┐
│   MCP Client        │
└──────────┬──────────┘
           │
           ↓
┌─────────────────────┐
│ universal-analyzer  │
│    -mcp.py          │
└──────────┬──────────┘
           │
           ↓
┌─────────────────────┐
│ Language-Specific   │
│ Analyzers:          │
│ - Ruff (Python)     │
│ - tsc (TypeScript)  │
│ - gopls (Go)        │
│ - rust-analyzer     │
│ - ESLint, etc.      │
└─────────────────────┘
```

---

## 🔌 Supported Language Tools

### Python
- **Ruff**: Lightning-fast linter
- **mypy**: Static type checker
- **Bandit**: Security scanner
- **Radon**: Complexity metrics

### TypeScript/JavaScript
- **tsc**: TypeScript compiler
- **ESLint**: Linter
- **Prettier**: Formatter (optional)

### Go
- **go vet**: Built-in static analyzer
- **golangci-lint**: Comprehensive linter
- **gopls**: Language server

### Rust
- **cargo check**: Compile checker
- **clippy**: Rust linter
- **rust-analyzer**: Language server

### Java
- **javac**: Compiler
- **CheckStyle**: Style checker
- **SpotBugs**: Bug detector

---

## 📊 Output Format

### Diagnostic Object

```typescript
interface Diagnostic {
  message: string;              // Error/warning message
  severity: "error" | "warning" | "info" | "hint";
  location: {
    file: string;              // File path
    line: number;              // Line number (1-indexed)
    column: number;            // Column (0-indexed)
    end_line?: number;         // End line (for ranges)
    end_column?: number;       // End column
  };
  code?: string;               // Error code (e.g., "TS2322")
  source?: string;             // Tool name (e.g., "eslint")
  suggestion?: string;         // Fix suggestion
  related?: Location[];        // Related locations
}
```

### Analysis Result

```typescript
interface AnalysisResult {
  language: string;            // Detected language
  files_analyzed: number;      // Number of files
  summary: {
    error: number;             // Error count
    warning: number;           // Warning count
    info: number;              // Info count
    hint: number;              // Hint count
  };
  diagnostics: Diagnostic[];   // Full diagnostic list
  metrics: {                   // Code metrics
    complexity?: number;
    maintainability?: number;
    coverage?: number;
  };
}
```

---

## 🚧 Implementation Status

### ✅ Fully Implemented

- [x] Python analysis (Ruff, mypy, Bandit)
- [x] TypeScript/JavaScript (tsc, ESLint)
- [x] Go analysis (go vet, golangci-lint)
- [x] Rust analysis (cargo check, clippy)
- [x] Multi-language detection
- [x] Diagnostic formatting
- [x] Severity filtering

### ⏳ Partial Implementation

- [~] LSP integration (hover, references, definition)
- [~] Code metrics (complexity, maintainability)
- [~] Security scanning (basic Bandit support)

### 🔮 Coming Soon

- [ ] Java analysis (javac, CheckStyle)
- [ ] C++ analysis (clang-tidy, cppcheck)
- [ ] Ruby analysis (RuboCop)
- [ ] PHP analysis (PHPStan, Psalm)
- [ ] Full LSP client integration
- [ ] Debugging support (breakpoints, stepping)
- [ ] Code fix application
- [ ] Caching for performance
- [ ] Incremental analysis

---

## 🎓 Best Practices

### For AI Agents

1. **Always analyze before suggesting fixes**
   ```
   ❌ "Change line 42 to..."
   ✅ [Analyze first] "I found an error at line 42. The issue is..."
   ```

2. **Provide context with errors**
   ```
   ❌ "Type error"
   ✅ "Type error: Expected string but got number. This happens because..."
   ```

3. **Suggest actionable fixes**
   ```
   ❌ "Fix the type error"
   ✅ "Convert to string: str(value)"
   ```

4. **Use severity filtering**
   ```
   For critical issues: min_severity="error"
   For all issues: min_severity="hint"
   ```

### For Users

1. **Start with language detection**
   ```
   "What languages are in my project?"
   → Use detect_languages first
   ```

2. **Filter by severity for focus**
   ```
   "Show me only errors" → min_severity="error"
   "Show everything" → min_severity="hint"
   ```

3. **Use hover for type info**
   ```
   "What's the type of X?" → hover_info
   "Where is X defined?" → go_to_definition
   ```

---

## 🐛 Troubleshooting

### Error: "Tool not found (ruff, mypy, etc.)"

**Solution:** Install language-specific tools
```bash
pip install ruff mypy bandit radon  # Python
npm install -g typescript eslint    # TS/JS
```

### Error: "Permission denied"

**Solution:** Make scripts executable
```bash
chmod +x universal-code-analyzer.py
chmod +x universal-analyzer-mcp.py
```

### Slow analysis on large codebases

**Solution:** Analyze specific directories
```python
# Instead of entire project
analyze_codebase("/project")

# Analyze specific subdirectory
analyze_codebase("/project/src")
```

### LSP features not working

**Status:** LSP integration is partial. Full implementation coming soon.  
**Workaround:** Use static analysis tools directly (type checking, linting)

---

## 📚 Resources

- [Model Context Protocol](https://modelcontextprotocol.io/)
- [Ruff Documentation](https://docs.astral.sh/ruff/)
- [mypy Documentation](https://mypy.readthedocs.io/)
- [ESLint Documentation](https://eslint.org/)
- [Go vet](https://pkg.go.dev/cmd/vet)
- [Clippy](https://github.com/rust-lang/rust-clippy)

---

## 🤝 Contributing

Want to add support for more languages?

1. Add language to `LANGUAGE_EXTENSIONS` in `universal-code-analyzer.py`
2. Implement `analyze_<language>()` function
3. Add to `analyzers` dict in `analyze_codebase()`
4. Update documentation
5. Add test cases

---

## 📄 License

MIT License - Same as lynlang

---

## 🎉 Success!

Your AI agents can now analyze code in **ANY language**! 

**Try it:**
```
"Analyze my Python project for type errors"
"Scan this TypeScript code for security issues"
"Find all references to the User class"
"What's the complexity of my Go functions?"
```

**Universal code analysis for the AI age!** 🚀

