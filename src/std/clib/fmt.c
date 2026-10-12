#include <stdio.h>
#include <stdlib.h>
#include <stdbool.h>
#include <string.h>

#ifdef _WIN32
#include <windows.h>
#endif

#ifndef FASTLANG_UTF8_DEFINED
#define FASTLANG_UTF8_DEFINED
static void fastlang_enable_utf8_output(void) {
#ifdef _WIN32
    static int initialized = 0;
    if (!initialized) {
        SetConsoleOutputCP(CP_UTF8);
        initialized = 1;
    }
#endif
}
#endif

#ifndef __cplusplus
void fast_println(const char* s) {
    fastlang_enable_utf8_output();
    if (s && *s) {
        fputs(s, stdout);
    }
    putchar('\n');
    fflush(stdout);
}

void fast_print_int(long long val) {
    fastlang_enable_utf8_output();
    printf("%lld", val);
}

void fast_print_uint(unsigned long long val) {
    fastlang_enable_utf8_output();
    printf("%llu", val);
}

void fast_print_double(double val) {
    fastlang_enable_utf8_output();
    printf("%g", val);
}

void fast_print_char(int c) {
    fastlang_enable_utf8_output();
    putchar(c);
}

void fast_print_bool(int b) {
    fastlang_enable_utf8_output();
    fputs(b ? "true" : "false", stdout);
}

void fast_print_str(const char* s) {
    fastlang_enable_utf8_output();
    if (s) {
        fputs(s, stdout);
    }
}

void fast_print(const char* s) {
    fastlang_enable_utf8_output();
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
#endif
