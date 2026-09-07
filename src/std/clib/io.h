#ifndef FASTLANG_CLIB_IO_H
#define FASTLANG_CLIB_IO_H

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>

#ifdef __cplusplus
#include <type_traits>

template <typename T>
static inline void print_single_arg(const T &val)
{
    using Decayed = std::decay_t<T>;
    if constexpr (std::is_same_v<Decayed, fastlang_str>) {
        if (val._data && val._size > 0) {
            fwrite(val._data, 1, val._size, stdout);
        }
    } else if constexpr (std::is_same_v<Decayed, fastlang_slice<char>>) {
        if (val._data && val._size > 0) {
            fwrite(val._data, 1, val._size, stdout);
        }
    } else if constexpr (std::is_same_v<Decayed, const char*> || std::is_same_v<Decayed, char*>) {
        if (val) fputs(val, stdout);
    } else if constexpr (std::is_same_v<Decayed, char>) {
        putchar(val);
    } else if constexpr (std::is_same_v<Decayed, bool>) {
        fputs(val ? "true" : "false", stdout);
    } else if constexpr (std::is_same_v<Decayed, fastlang_unit_t>) {
        fputs("()", stdout);
    } else if constexpr (std::is_same_v<Decayed, fastlang_undefined_t>) {
        fputs("undefined", stdout);
    } else if constexpr (std::is_same_v<Decayed, uint8_t> || std::is_same_v<Decayed, int8_t>) {
        if constexpr (std::is_signed_v<Decayed>) {
            printf("%d", (int)val);
        } else {
            printf("%u", (unsigned int)val);
        }
    } else if constexpr (std::is_integral_v<Decayed>) {
        if constexpr (std::is_signed_v<Decayed>) {
            printf("%lld", (long long)val);
        } else {
            printf("%llu", (unsigned long long)val);
        }
    } else if constexpr (std::is_floating_point_v<Decayed>) {
        printf("%g", (double)val);
    } else if constexpr (std::is_pointer_v<Decayed>) {
        if (!val) {
            fputs("None", stdout);
        } else {
            print_single_arg(*val);
        }
    } else if constexpr (has_display<Decayed>::value) {
        auto d = const_cast<Decayed&>(val).display();
        print_single_arg(d);
    } else if constexpr (fastlang_has_as_str<Decayed>::value) {
        auto d = const_cast<Decayed&>(val).as_str();
        print_single_arg(d);
    } else if constexpr (is_fastlang_slice_type<Decayed>::value) {
        putchar('[');
        for (size_t i = 0; i < val._size; ++i) {
            if (i > 0) fputs(", ", stdout);
            print_single_arg(val._data[i]);
        }
        putchar(']');
    } else if constexpr (is_fastlang_name_type<Decayed>::value || is_fastlang_modify_type<Decayed>::value) {
        if (val.ptr) {
            print_single_arg(*val.ptr);
        } else {
            fputs("None", stdout);
        }
    } else {
        auto s = fastlang_as_str(val);
        if (s._data && s._size > 0) {
            fwrite(s._data, 1, s._size, stdout);
        }
    }
}

template <typename... Args>
static inline void fast_print(const Args &...args)
{
    ((print_single_arg(args)), ...);
}

template <typename... Args>
static inline void fast_println(const Args &...args)
{
    ((print_single_arg(args)), ...);
    putchar('\n');
}

static inline void fast_print_str(const char *s) { if (s) fputs(s, stdout); }
template <typename T>
static inline void fast_print_str(const T &s) { print_single_arg(s); }
static inline void fast_print_int(long long val) { printf("%lld", val); }
static inline void fast_print_double(double val) { printf("%g", val); }
static inline void fast_print_char(int c) { putchar(c); }
static inline void fast_print_bool(int b) { fputs(b ? "true" : "false", stdout); }

static inline fastlang_str fast_input(const fastlang_str &prompt = fastlang_str())
{
    if (prompt._size > 0 && prompt._data) {
        fwrite(prompt._data, 1, prompt._size, stdout);
        fflush(stdout);
    }
    char buf[4096];
    if (fgets(buf, sizeof(buf), stdin)) {
        size_t len = strlen(buf);
        if (len > 0 && buf[len - 1] == '\n') {
            buf[len - 1] = '\0';
            len--;
        }
        if (len > 0 && buf[len - 1] == '\r') {
            buf[len - 1] = '\0';
            len--;
        }
        char* res = new char[len + 1];
        memcpy(res, buf, len + 1);
        return fastlang_str(res, len);
    }
    return fastlang_str("", 0);
}

#else

static inline void fast_println(const char *s) { if (s) puts(s); else putchar('\n'); }
static inline void fast_print_int(long long val) { printf("%lld\n", val); }
static inline void fast_print_double(double val) { printf("%g\n", val); }
static inline void fast_print_char(int c) { putchar(c); putchar('\n'); }
static inline void fast_print_bool(int b) { puts(b ? "true" : "false"); }
static inline void fast_print_str(const char *s) { if (s) fputs(s, stdout); }

static inline char* fast_input(const char *prompt)
{
    if (prompt && *prompt) {
        fputs(prompt, stdout);
        fflush(stdout);
    }
    char buf[4096];
    if (fgets(buf, sizeof(buf), stdin)) {
        size_t len = strlen(buf);
        if (len > 0 && buf[len - 1] == '\n') {
            buf[len - 1] = '\0';
            len--;
        }
        if (len > 0 && buf[len - 1] == '\r') {
            buf[len - 1] = '\0';
            len--;
        }
        char* res = (char*)malloc(len + 1);
        if (res) memcpy(res, buf, len + 1);
        return res;
    }
    char* empty = (char*)malloc(1);
    if (empty) empty[0] = '\0';
    return empty;
}

#endif

#endif // FASTLANG_CLIB_IO_H
