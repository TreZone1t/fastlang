#ifndef FASTLANG_CLIB_IO_H
#define FASTLANG_CLIB_IO_H

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifdef __cplusplus
#include <iostream>
#include <string>

template <typename T>
static inline void fast_println(const T &val)
{
    std::cout << val << std::endl;
}

static inline void fast_print_str(const char *s)
{
    fputs(s, stdout);
}

template <typename T>
static inline void fast_print_str(const T &s)
{
    std::cout << s;
}

static inline void fast_print_int(long long val)
{
    printf("%lld", val);
}

static inline void fast_print_double(double val)
{
    printf("%g", val);
}

static inline void fast_print_char(int c)
{
    std::cout << (char)c;
}

static inline void fast_print_bool(int b)
{
    std::cout << (b ? "true" : "false");
}

// Variadic print supporting arbitrary arguments of any type (without newline)
template <typename... Args>
static inline void print(const Args &...args)
{
    std::cout << std::boolalpha;
    ((std::cout << args), ...);
}

// Variadic println supporting arbitrary arguments of any type (with newline)
template <typename... Args>
static inline void println(const Args &...args)
{
    std::cout << std::boolalpha;
    ((std::cout << args), ...);
    std::cout << std::endl;
}

#else

static inline void fast_println(const char *s)
{
    puts(s);
}

static inline void fast_print_int(long long val)
{
    printf("%lld", val);
}

static inline void fast_print_double(double val)
{
    printf("%g", val);
}

static inline void fast_print_char(int c)
{
    putchar(c);
}

static inline void fast_print_bool(int b)
{
    fputs(b ? "true" : "false", stdout);
}

#endif

#endif // FASTLANG_CLIB_IO_H
