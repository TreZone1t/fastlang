#include <stdio.h>
#include <stdlib.h>
#include <stdbool.h>

#ifdef _WIN32
#include <windows.h>
#endif

static void fastlang_enable_utf8_output(void) {
#ifdef _WIN32
    static int initialized = 0;
    if (!initialized) {
        SetConsoleOutputCP(CP_UTF8);
        initialized = 1;
    }
#endif
}

void fast_println(const char* s) {
    fastlang_enable_utf8_output();
    if (s) {
        puts(s);
    }
}

void fast_print_int(long long val) {
    fastlang_enable_utf8_output();
    printf("%lld\n", val);
}

void fast_print_double(double val) {
    fastlang_enable_utf8_output();
    printf("%g\n", val);
}

void fast_print_char(int c) {
    fastlang_enable_utf8_output();
    putchar(c);
    putchar('\n');
}

void fast_print_bool(int b) {
    fastlang_enable_utf8_output();
    puts(b ? "true" : "false");
}

void fast_print_str(const char* s) {
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
