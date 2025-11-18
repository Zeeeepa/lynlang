# Zen Language Makefile
# Provides convenient build and test commands

.PHONY: all build test clean install lsp format check help

# Default target
all: build

# Build the compiler
build:
	@echo "Building Zen compiler..."
	@cargo build --release
	@echo "✓ Compiler built successfully"

# Build in debug mode
debug:
	@echo "Building Zen compiler (debug)..."
	@cargo build
	@echo "✓ Debug build complete"

# Run all tests
test:
	@echo "Running Zen test suite..."
	@python3 scripts/run_tests.py

# Run specific test
test-file:
	@if [ -z "$(FILE)" ]; then \
		echo "Usage: make test-file FILE=tests/test_example.zen"; \
	else \
		./target/debug/zen $(FILE); \
	fi

# Build and install the compiler
install: build
	@echo "Installing Zen compiler..."
	@cargo install --path .
	@echo "✓ Zen compiler installed"

# Build the LSP server
lsp:
	@echo "Building Zen LSP server..."
	@cargo build --release --bin zen-lsp
	@echo "✓ LSP server built"

# Format all Rust code
format:
	@echo "Formatting code..."
	@cargo fmt
	@echo "✓ Code formatted"

# Check code without building
check:
	@echo "Checking code..."
	@cargo check --all-targets
	@echo "✓ Code check complete"

# Run comprehensive linter (uses scripts/lint.sh)
lint:
	@echo "Running comprehensive linter..."
	@./scripts/lint.sh
	@echo "✓ Lint complete"

# Clean build artifacts
clean:
	@echo "Cleaning build artifacts..."
	@cargo clean  # cargo clean already removes target/
	@rm -f *.zen.out
	@echo "✓ Clean complete"

# Build documentation
docs:
	@echo "Building documentation..."
	@cargo doc --no-deps --open
	@echo "✓ Documentation built"

# Run benchmarks
bench:
	@echo "Running benchmarks..."
	@cargo bench
	@echo "✓ Benchmarks complete"

# Quick test - run a subset of tests
quick-test:
	@echo "Running quick tests..."
	@./target/debug/zen tests/test_basic.zen
	@./target/debug/zen tests/test_string_basic.zen
	@./target/debug/zen tests/test_nested_generics_simple.zen
	@./target/debug/zen tests/test_no_gc_comprehensive.zen
	@echo "✓ Quick tests complete"

# Show help
help:
	@echo "Zen Language Build System"
	@echo ""
	@echo "Available targets:"
	@echo "  make build       - Build the compiler (release)"
	@echo "  make debug       - Build the compiler (debug)"
	@echo "  make test        - Run all tests"
	@echo "  make test-file FILE=<path> - Run specific test"
	@echo "  make install     - Install the compiler"
	@echo "  make lsp         - Build the LSP server"
	@echo "  make format      - Format all code"
	@echo "  make check       - Check code without building"
	@echo "  make lint        - Run clippy linter"
	@echo "  make clean       - Clean build artifacts"
	@echo "  make docs        - Build documentation"
	@echo "  make bench       - Run benchmarks"
	@echo "  make quick-test  - Run quick test subset"
	@echo "  make help        - Show this help message"

# Development workflow shortcuts
dev: debug quick-test

# Full CI workflow
ci: clean build test lint

# Release workflow
release: clean
	@echo "Building release..."
	@cargo build --release
	@strip target/release/zen
	@echo "✓ Release build complete"
	@ls -lh target/release/zen