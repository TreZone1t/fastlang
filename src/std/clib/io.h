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

template <typename T>
static inline void print_single_arg(const T &val)
{
    if constexpr (std::is_same_v<T, uint8_t> || std::is_same_v<T, int8_t>) {
        std::cout << +val;
    } else {
        std::cout << val;
    }
}

// Variadic print supporting arbitrary arguments of any type (without newline)
template <typename... Args>
static inline void fast_print(const Args &...args)
{
    std::cout << std::boolalpha;
    ((print_single_arg(args)), ...);
}

// Variadic println supporting arbitrary arguments of any type (with newline)
template <typename... Args>
static inline void fast_println(const Args &...args)
{
    std::cout << std::boolalpha;
    ((print_single_arg(args)), ...);
    std::cout << std::endl;
}


static inline fastlang_str fast_input(const fastlang_str &prompt = fastlang_str())
{
    if (prompt._size > 0 && prompt._data) {
        std::cout.write(prompt._data, prompt._size);
        std::cout.flush();
    }
    std::string line;
    if (std::getline(std::cin, line)) {
        size_t len = line.size();
        char* buf = new char[len + 1];
        memcpy(buf, line.data(), len);
        buf[len] = '\0';
        return fastlang_str(buf, len);
    }
    return fastlang_str("", 0);
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

static inline void fast_print_str(const char *s)
{
    if (s) fputs(s, stdout);
}

static inline char* fast_input(const char *prompt)
{
    if (prompt && *prompt) {
        fputs(prompt, stdout);
        fflush(stdout);
    }
    char buf[1024];
    if (fgets(buf, sizeof(buf), stdin)) {
        size_t len = strlen(buf);
        if (len > 0 && buf[len - 1] == '\n') {
            buf[len - 1] = '\0';
            len--;
        }
        char* res = (char*)malloc(len + 1);
        memcpy(res, buf, len + 1);
        return res;
    }
    char* empty = (char*)malloc(1);
    empty[0] = '\0';
    return empty;
}

#endif

#endif // FASTLANG_CLIB_IO_H
