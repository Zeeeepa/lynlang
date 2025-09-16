# Zen Language Tests

## Organization

- `working/` - Current working tests that compile and run
- Outputs go to `target/debug/` and `target/release/`

## Running Tests

```bash
# Build the Rust compiler
cargo build

# Compile and run a Zen program
./target/debug/zen tests/working/zen_test_simple.zen -o target/debug/test_simple
./target/debug/test_simple

# All binaries go to target/ (like Rust convention)
```

## Working Tests (Rust compiler)

- `zen_test_simple.zen` - Basic variables and io.println ✅
- `zen_test_arithmetic.zen` - Math operations
- `zen_test_struct_access.zen` - Struct definitions and access
- Others need updating for strict Rust compiler
