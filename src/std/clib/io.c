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

void fast_print_str(const char* s) {
    if (s) {
        fputs(s, stdout);
    }
}

char* fast_input(const char* prompt) {
    if (prompt && *prompt) {
        fputs(prompt, stdout);
        fflush(stdout);
    }
    char buf[1024];
    if (fgets(buf, sizeof(buf), stdin)) {
        size_t len = 0;
        while (buf[len] != '\0') len++;
        if (len > 0 && buf[len - 1] == '\n') {
            buf[len - 1] = '\0';
            len--;
        }
        char* res = (char*)malloc(len + 1);
        if (res) {
            for (size_t i = 0; i <= len; i++) {
                res[i] = buf[i];
            }
        }
        return res;
    }
    char* empty = (char*)malloc(1);
    if (empty) empty[0] = '\0';
    return empty;
}
