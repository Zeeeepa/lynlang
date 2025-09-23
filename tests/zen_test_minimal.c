// Generated from Zen source
#include <stdio.h>
#include <stdlib.h>
#include <stdbool.h>
#include <string.h>
#include <stdarg.h>

// Zen runtime
char* zen_str(int v) {
    static char b[32];
    sprintf(b, "%d", v);
    return b;
}

char* zen_sprintf(const char* fmt, ...) {
    static char b[1024];
    va_list a;
    va_start(a, fmt);
    vsnprintf(b, 1024, fmt, a);
    va_end(a);
    return b;
}

int main() {
    auto x = 10;
    auto y = 20;
    return 0;
}
