#include <stdio.h>
#include <stdlib.h>
#include <stdbool.h>
#include <string.h>

int main(void) {
        printf("%s\n", "=== Zen Compiler Test ===");
    const int x = 42;
    const int y = 7;
    const int sum = (x + y);
    const int diff = (x - y);
    const int prod = (x * y);
    const int quot = (x / y);
    printf("%s\n", "Numbers:");
    printf("%d\n", x);
    printf("%d\n", y);
    printf("%s\n", "Arithmetic:");
    printf("%d\n", sum);
    printf("%d\n", diff);
    printf("%d\n", prod);
    printf("%d\n", quot);
    const bool is_true = true;
    const bool is_false = false;
    printf("%s\n", "Booleans:");
    printf("%d\n", is_true);
    printf("%d\n", is_false);
    const char* greeting = "Hello World";
    printf("%s\n", "String:");
    printf("%d\n", greeting);
    printf("%s\n", "=== Test Complete ===");
    return 0;
}

