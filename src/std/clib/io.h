#ifndef FASTLANG_CLIB_IO_H
#define FASTLANG_CLIB_IO_H

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifdef __cplusplus
#include <iostream>
#include <string>

template <typename T>
static inline void fast_println(const T& val) {
    std::cout << val << std::endl;
}

static inline void fast_print_str(const char* s) {
    puts(s);
}

template <typename T>
static inline void fast_print_str(const T& s) {
    std::cout << s << std::endl;
}

static inline void fast_print_int(long long val) {
    printf("%lld\n", val);
}

static inline void fast_print_double(double val) {
    printf("%g\n", val);
}

// Variadic print supporting arbitrary arguments of any type (Python-like)
template <typename... Args>
static inline void print(const Args&... args) {
    ((std::cout << args), ...);
    std::cout << std::endl;
}

#else

static inline void fast_println(const char* s) {
    puts(s);
}

static inline void fast_print_int(long long val) {
    printf("%lld\n", val);
}

static inline void fast_print_double(double val) {
    printf("%g\n", val);
}

#endif

#endif // FASTLANG_CLIB_IO_H
