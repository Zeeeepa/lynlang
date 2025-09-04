# Zenlang Scratchpad

## Quick Notes
- Test failure at 285/286 - investigate remaining test
- Pattern matching parser complete (src/parser/expressions.rs:373-429)
- LLVM 18.1 configured with inkwell
- VSCode extension exists but needs work

## Code Patterns to Remember

### Variable Declaration
```zen
// Immutable
value := 42
PI: f64 = 3.14159

// Mutable  
counter ::= 0
buffer:: [1024]u8
```

### Pattern Matching (NO if/else!)
```zen
score ? | 90..=100 => "A"
        | 80..=89  => "B"
        | _ => "F"

result ? | .Ok -> val => process(val)
         | .Err -> err => handle(err)
```

### Functions
```zen
add = (a: i32, b: i32) i32 { a + b }
greet = (name: string = "World") void {
    print("Hello, $(name)!")
}
```

## Important Paths
- Compiler source: `/src/`
- Standard library: `/std/`
- Examples: `/examples/`
- Tests: `/tests/`
- Self-hosted code: `/self_hosted/`

## Compiler Commands
```bash
# Build compiler
cargo build --release

# Run tests
cargo test

# Compile zen file
cargo run --bin zen file.zen

# With verbose output
RUST_LOG=debug cargo run --bin zen file.zen
```

## Git Workflow
- Frequent commits (every significant change)
- Descriptive messages
- Push to remote regularly
- Use gh CLI for PRs

## Performance Notes
- LLVM optimization passes enabled
- Inline small functions
- Const propagation working
- Dead code elimination active

## Known Issues
- String interpolation parser done, codegen incomplete
- Pattern matching codegen needs work
- One test failing (investigate)
- LSP diagnostics need fixing