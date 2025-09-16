#include <stdio.h>
#include <stdlib.h>
#include <stdbool.h>
#include <string.h>

int main(void) {
        const int x = 10;
    const int y = 20;
    const int sum = (x + y);
    const int product = (x * y);
    printf("%s\n", "Sum:");
    printf("%d\n", sum);
    printf("%s\n", "Product:");
    printf("%d\n", product);
    return 0;
}

