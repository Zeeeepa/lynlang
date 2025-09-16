// Generated C code from Zen compiler v4
#include <stdio.h>
#include <stdlib.h>
#include <stdbool.h>
#include <string.h>

typedef struct Option {
    bool is_some;
    void* value;
} Option;

/* Import: io from @std */

typedef struct Point {
    double x;
    double y /* default: 0 */;
} Point;

typedef struct Circle {
    Point center;
    double radius;
} Circle;

int main(void) {
    printf("=== Zen Language Demo ===\n");
    printf("\n");
    printf("1. Variables:\n");
    const int x = 10;
    int v = 30;
    int u = 40;
    printf("   Immutable x = \n");
    printf("%d\n", x);
    printf("   Mutable v = \n");
    printf("%d\n", v);
    v = (v + 5);
    printf("   After v = v + 5: \n");
    printf("%d\n", v);
    printf("\n");
    printf("2. Pattern Matching:\n");
    const bool is_ready = true;
    /* Pattern match */
    if (is_ready) {
        printf("   System is ready!\n");
    };
    const bool has_data = false;
    /* Pattern match */
    if (has_data == true) {
        printf("   Processing data...\n");
    } else if (has_data == false) {
        printf("   Waiting for data...\n");
    };
    printf("\n");
    printf("3. Structs:\n");
    const Circle circle = (struct Circle){.center = (struct Point){.x = 100, .y = 100}, .radius = 50};
    printf("   Circle created with radius 50\n");
    printf("\n");
    printf("4. Comparisons:\n");
    const int a = 10;
    const int b = 20;
    const int is_less = (a < b);
    /* Pattern match */
    if (is_less) {
        printf("   10 < 20 is true\n");
    };
    printf("\n");
    printf("5. Arithmetic:\n");
    const int sum = (a + b);
    const int diff = (b - a);
    const int product = (a * 2);
    const int quotient = (b / 2);
    printf("   10 + 20 = \n");
    printf("%d\n", sum);
    printf("   20 - 10 = \n");
    printf("%d\n", diff);
    printf("   10 * 2 = \n");
    printf("%d\n", product);
    printf("   20 / 2 = \n");
    printf("%d\n", quotient);
    printf("\n");
    printf("=== Demo Complete ===\n");
}
