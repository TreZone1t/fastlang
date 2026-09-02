#include <stdio.h>
#include <stdlib.h>
#include <stdbool.h>

void fast_println(const char* s) {
    if (s) {
        puts(s);
    }
}

void fast_print_int(long long val) {
    printf("%lld\n", val);
}

void fast_print_double(double val) {
    printf("%g\n", val);
}

void fast_print_char(int c) {
    putchar(c);
    putchar('\n');
}

void fast_print_bool(int b) {
    puts(b ? "true" : "false");
}
