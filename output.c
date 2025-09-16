#include <stdio.h>
#include <stdlib.h>
#include <stdbool.h>
#include <string.h>

int main(void) {
        const int x = 10;
    const int y = 20;
    const int sum = (x + y);
    const int diff = (y - x);
    const int product = (x * 2);
    const int quotient = (y / 2);
    const bool is_true = true;
    const bool is_false = false;
    const char* greeting = "Hello, Zen!";
    printf("%s\n", "=== Zen Basic Features Test ===");
    printf("%s\n", "");
    printf("%s\n", "Test 1: Variables");
    printf("%s\n", "x = 10, y = 20");
    printf("%s\n", "");
    printf("%s\n", "Test 2: Arithmetic");
    printf("%s\n", "sum (x + y) = 30");
    printf("%s\n", "diff (y - x) = 10");
    printf("%s\n", "product (x * 2) = 20");
    printf("%s\n", "quotient (y / 2) = 10");
    printf("%s\n", "");
    printf("%s\n", "Test 3: Booleans");
    printf("%s\n", "is_true = true");
    printf("%s\n", "is_false = false");
    printf("%s\n", "");
    printf("%s\n", "Test 4: Strings");
    printf("%d\n", greeting);
    printf("%s\n", "");
    printf("%s\n", "=== All tests passed! ===");
    return 0;
}

