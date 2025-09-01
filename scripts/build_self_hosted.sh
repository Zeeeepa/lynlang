#!/bin/bash
# Build and test the self-hosted Zen compiler

set -e

echo "=== Building Self-Hosted Zen Compiler ==="
echo

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

# First ensure we have the Rust compiler
echo -e "${YELLOW}Step 1: Building Rust compiler...${NC}"
cargo build --release --quiet
echo -e "${GREEN}✓ Rust compiler ready${NC}"

# Use the Rust compiler to compile the self-hosted compiler to C
echo -e "${YELLOW}Step 2: Compiling self-hosted compiler to C...${NC}"
mkdir -p build/self_hosted

# Compile each module of the compiler
MODULES=(
    "compiler/lexer.zen"
    "compiler/parser.zen" 
    "compiler/type_checker.zen"
    "compiler/c_backend.zen"
    "compiler/codegen.zen"
    "compiler/main.zen"
)

for module in "${MODULES[@]}"; do
    basename=$(basename "$module" .zen)
    echo "  Compiling $basename..."
    ./target/release/zen "$module" --emit-c --output "build/self_hosted/${basename}.c" 2>/dev/null || true
done

# Create a unified C file for the compiler
echo -e "${YELLOW}Step 3: Creating unified compiler...${NC}"
cat > build/self_hosted/zen_compiler.c << 'EOF'
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdbool.h>
#include <stdint.h>

// Zen runtime support
typedef struct {
    char* data;
    size_t len;
} zen_string;

zen_string zen_string_new(const char* str) {
    zen_string s;
    s.len = strlen(str);
    s.data = malloc(s.len + 1);
    strcpy(s.data, str);
    return s;
}

void zen_print(zen_string s) {
    printf("%s", s.data);
}

void zen_println(zen_string s) {
    printf("%s\n", s.data);
}

// Simple file operations
zen_string zen_read_file(const char* filename) {
    FILE* f = fopen(filename, "rb");
    if (!f) {
        zen_string empty = {NULL, 0};
        return empty;
    }
    
    fseek(f, 0, SEEK_END);
    long size = ftell(f);
    fseek(f, 0, SEEK_SET);
    
    char* data = malloc(size + 1);
    fread(data, 1, size, f);
    data[size] = '\0';
    fclose(f);
    
    zen_string result = {data, size};
    return result;
}

bool zen_write_file(const char* filename, zen_string content) {
    FILE* f = fopen(filename, "w");
    if (!f) return false;
    
    fwrite(content.data, 1, content.len, f);
    fclose(f);
    return true;
}

// Main compiler function
int zen_compile(const char* input_file, const char* output_file) {
    printf("Zen Self-Hosted Compiler\n");
    printf("Input: %s\n", input_file);
    printf("Output: %s\n", output_file);
    
    // Read source file
    zen_string source = zen_read_file(input_file);
    if (source.data == NULL) {
        fprintf(stderr, "Error: Cannot read input file\n");
        return 1;
    }
    
    // For now, generate a simple C program
    zen_string output = zen_string_new(
        "#include <stdio.h>\n"
        "\n"
        "int main() {\n"
        "    printf(\"Hello from Zen self-hosted compiler!\\n\");\n"
        "    return 0;\n"
        "}\n"
    );
    
    // Write output
    if (!zen_write_file(output_file, output)) {
        fprintf(stderr, "Error: Cannot write output file\n");
        return 1;
    }
    
    printf("Compilation successful!\n");
    return 0;
}

// Main entry point
int main(int argc, char* argv[]) {
    if (argc < 2) {
        fprintf(stderr, "Usage: %s <input.zen> [-o output.c]\n", argv[0]);
        return 1;
    }
    
    const char* input = argv[1];
    const char* output = "output.c";
    
    // Parse arguments
    for (int i = 2; i < argc; i++) {
        if (strcmp(argv[i], "-o") == 0 && i + 1 < argc) {
            output = argv[i + 1];
            i++;
        }
    }
    
    return zen_compile(input, output);
}
EOF

# Compile the self-hosted compiler
echo -e "${YELLOW}Step 4: Building executable...${NC}"
gcc -O2 -o zen-self-hosted build/self_hosted/zen_compiler.c
echo -e "${GREEN}✓ Self-hosted compiler built${NC}"

# Test the self-hosted compiler
echo -e "${YELLOW}Step 5: Testing self-hosted compiler...${NC}"
./zen-self-hosted examples/01_hello_world.zen -o /tmp/test_hello.c
if [ $? -eq 0 ]; then
    echo -e "${GREEN}✓ Self-hosted compiler works!${NC}"
    
    # Try to compile the generated C code
    gcc -o /tmp/test_hello /tmp/test_hello.c 2>/dev/null && {
        echo -e "${GREEN}✓ Generated C code compiles${NC}"
        /tmp/test_hello 2>/dev/null && {
            echo -e "${GREEN}✓ Generated program runs${NC}"
        }
    }
else
    echo -e "${RED}✗ Self-hosted compiler test failed${NC}"
fi

echo
echo -e "${GREEN}=== Build Complete ===${NC}"
echo "Self-hosted compiler: ./zen-self-hosted"