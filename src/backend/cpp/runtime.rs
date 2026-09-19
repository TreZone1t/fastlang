pub fn generate_runtime_header() -> String {
    r#"#pragma once
#include <variant>
#include <cstdint>
#include <type_traits>
#include <functional>
#include <cstring>
#include <cstdio>
#include <vector>
#include <memory>

using type = const char*;
// ============================================================================
// === [FastLang Array Runtime Implementation] ================================
// === (Temporary C++ Interop - To be migrated to Native Alloc in next release)
// ============================================================================

template <typename T>
inline T* fastlang_array_alloc(size_t len) {
    size_t total_bytes = sizeof(size_t) + (len + 1) * sizeof(T);
    void* mem = std::malloc(total_bytes);
    if (!mem) return nullptr;
    std::memset(mem, 0, total_bytes);
    size_t* raw = reinterpret_cast<size_t*>(mem);
    raw[0] = len;
    T* data = reinterpret_cast<T*>(raw + 1);
    for (size_t i = 0; i < len; ++i) {
        new (&data[i]) T();
    }
    data[len] = T();
    return data;
}

struct fastlang_empty_array_t {
    template <typename T>
    operator T*() const {
        return fastlang_array_alloc<T>(0);
    }
    template <typename T, typename = std::enable_if_t<!std::is_pointer_v<T>>>
    operator T() const {
        return T{};
    }
};

template <typename T>
inline auto fastlang_array_len(const T& arr) {
    using Decayed = std::decay_t<T>;
    if constexpr (std::is_same_v<Decayed, fastlang_empty_array_t>) {
        return (size_t)0;
    } else if constexpr (std::is_pointer_v<Decayed>) {
        if (!arr) return (size_t)0;
        return reinterpret_cast<const size_t*>(arr)[-1];
    } else {
        return arr.len;
    }
}

template <typename T>
inline T* fastlang_array_create(std::initializer_list<T> list) {
    size_t len = list.size();
    size_t total_bytes = sizeof(size_t) + (len + 1) * sizeof(T);
    void* mem = std::malloc(total_bytes);
    if (!mem) return nullptr;
    std::memset(mem, 0, total_bytes);
    size_t* raw = reinterpret_cast<size_t*>(mem);
    raw[0] = len;
    T* data = reinterpret_cast<T*>(raw + 1);
    size_t i = 0;
    for (const auto& item : list) {
        new (&data[i++]) T(item);
    }
    data[len] = T();
    return data;
}

inline char* fastlang_string_create(std::initializer_list<char> list) {
    size_t len = list.size();
    const char* first = list.begin();
    if (len > 0 && *(first + len - 1) == '\0') {
        len -= 1;
    }
    size_t total_bytes = sizeof(size_t) + len + 1;
    void* mem = std::malloc(total_bytes);
    if (!mem) return nullptr;
    size_t* raw = reinterpret_cast<size_t*>(mem);
    raw[0] = len;
    char* data = reinterpret_cast<char*>(raw + 1);
    size_t i = 0;
    for (char c : list) {
        if (i < len) data[i++] = c;
    }
    data[len] = '\0';
    return data;
}

inline char* fastlang_string_create_from_ptr(const char* s) {
    if (!s) s = "";
    size_t len = std::strlen(s);
    size_t total_bytes = sizeof(size_t) + len + 1;
    void* mem = std::malloc(total_bytes);
    if (!mem) return nullptr;
    size_t* raw = reinterpret_cast<size_t*>(mem);
    raw[0] = len;
    char* data = reinterpret_cast<char*>(raw + 1);
    std::memcpy(data, s, len + 1);
    return data;
}

template <typename T>
inline void fastlang_array_free(T* arr) {
    if (!arr) return;
    size_t len = fastlang_array_len(arr);
    for (size_t i = 0; i < len; ++i) {
        arr[i].~T();
    }
    size_t* raw = reinterpret_cast<size_t*>(arr) - 1;
    std::free(raw);
}

namespace fast_std {
template <typename T>
using array = T*;
using str = char*;
}

using fast_std::array;
using fast_std::str;
// ============================================================================

struct fastlang_unit_t {
    bool operator==(const fastlang_unit_t&) const { return true; }
    bool operator!=(const fastlang_unit_t&) const { return false; }
};

struct fastlang_undefined_t {
    template <typename T>
    operator T() const { return T{}; }
    bool operator==(const fastlang_undefined_t&) const { return true; }
    bool operator!=(const fastlang_undefined_t&) const { return false; }
};

template <typename T, typename = void>
struct has_copy : std::false_type {};
template <typename T>
struct has_copy<T, std::void_t<decltype(std::declval<T&>().copy())>> : std::true_type {};

template <typename T>
inline auto fastlang_copy(const T& val) {
    if constexpr (has_copy<T>::value) {
        return const_cast<T&>(val).copy();
    } else {
        return val;
    }
}

namespace fast_std {
template <typename T>
bool fastlang_array_partial_equal(T* __this, T* other);
template <typename T>
bool fastlang_array_not_equal(T* __this, T* other);
template <typename T>
T* fastlang_array_add(T* __this, T* other);
template <typename T>
T* fastlang_array_add(T* __this, T item);
inline char* fastlang_char_add(char __this, char* other);
inline char* fastlang_char_add(char __this, char other);
}

// Forward declarations for string conversion
template <typename T>
inline char* fastlang_as_str(const T& val);
inline char* fastlang_as_str(bool b);
inline char* fastlang_as_str(char* s);
inline char* fastlang_as_str(const char* s);

template <typename A, typename B>
inline auto fastlang_add(A&& a, B&& b) {
    using DecA = std::decay_t<A>;
    using DecB = std::decay_t<B>;
    if constexpr (std::is_same_v<DecA, char> && (std::is_same_v<DecB, char*> || std::is_same_v<DecB, const char*>)) {
        return fast_std::fastlang_char_add(a, const_cast<char*>(b));
    } else if constexpr (std::is_same_v<DecA, char> && std::is_same_v<DecB, char>) {
        return fast_std::fastlang_char_add(a, b);
    } else if constexpr ((std::is_same_v<DecA, char*> || std::is_same_v<DecA, const char*>) && std::is_same_v<DecB, char>) {
        return fast_std::fastlang_array_add(const_cast<char*>(a), b);
    } else if constexpr ((std::is_same_v<DecA, char*> || std::is_same_v<DecA, const char*>) && (std::is_same_v<DecB, char*> || std::is_same_v<DecB, const char*>)) {
        return fast_std::fastlang_array_add(const_cast<char*>(a), const_cast<char*>(b));
    } else if constexpr ((std::is_same_v<DecA, char*> || std::is_same_v<DecA, const char*>)) {
        return fast_std::fastlang_array_add(const_cast<char*>(a), fastlang_as_str(b));
    } else if constexpr ((std::is_same_v<DecB, char*> || std::is_same_v<DecB, const char*>)) {
        return fast_std::fastlang_array_add(fastlang_as_str(a), const_cast<char*>(b));
    } else if constexpr (std::is_pointer_v<DecA> && std::is_integral_v<DecB>) {
        return a + b;
    } else if constexpr (std::is_integral_v<DecA> && std::is_pointer_v<DecB>) {
        return a + b;
    } else if constexpr (std::is_pointer_v<DecA> && std::is_pointer_v<DecB>) {
        return fast_std::fastlang_array_add(a, b);
    } else if constexpr (std::is_pointer_v<DecA>) {
        return fast_std::fastlang_array_add(a, b);
    } else {
        return a + b;
    }
}

template <typename A, typename B>
inline auto& fastlang_add_assign(A& a, B&& b) {
    using DecA = std::decay_t<A>;
    if constexpr (std::is_pointer_v<DecA>) {
        a = fastlang_add(a, std::forward<B>(b));
        return a;
    } else {
        a += std::forward<B>(b);
        return a;
    }
}

template <typename A, typename B>
inline bool fastlang_eq(A&& a, B&& b) {
    using DecA = std::decay_t<A>;
    using DecB = std::decay_t<B>;
    if constexpr ((std::is_same_v<DecA, char*> || std::is_same_v<DecA, const char*>) && (std::is_same_v<DecB, char*> || std::is_same_v<DecB, const char*>)) {
        if (a == b) return true;
        if (!a || !b) return false;
        return fast_std::fastlang_array_partial_equal(const_cast<char*>(a), const_cast<char*>(b));
    } else {
        return a == b;
    }
}

template <typename A, typename B>
inline bool fastlang_ne(A&& a, B&& b) {
    using DecA = std::decay_t<A>;
    using DecB = std::decay_t<B>;
    if constexpr ((std::is_same_v<DecA, char*> || std::is_same_v<DecA, const char*>) && (std::is_same_v<DecB, char*> || std::is_same_v<DecB, const char*>)) {
        if (a == b) return false;
        if (!a || !b) return true;
        return fast_std::fastlang_array_not_equal(const_cast<char*>(a), const_cast<char*>(b));
    } else {
        return a != b;
    }
}

template <typename T, typename = void>
struct fastlang_has_as_str : std::false_type {};
template <typename T>
struct fastlang_has_as_str<T, std::void_t<decltype(std::declval<T>().as_str())>> : std::true_type {};

template <typename T, typename = void>
struct has_display : std::false_type {};
template <typename T>
struct has_display<T, std::void_t<decltype(std::declval<T>().display())>> : std::true_type {};

inline char* fastlang_as_str(bool b) { return fastlang_string_create_from_ptr(b ? "true" : "false"); }
inline char* fastlang_as_str(char* s) { return s ? s : fastlang_string_create_from_ptr(""); }
inline char* fastlang_as_str(const char* s) { return fastlang_string_create_from_ptr(s ? s : ""); }

template <typename T>
inline char* fastlang_as_str(const T& val) {
    using Decayed = std::decay_t<T>;
    if constexpr (std::is_same_v<Decayed, const char*> || std::is_same_v<Decayed, char*>) {
        return fastlang_string_create_from_ptr(val ? val : "");
    } else if constexpr (std::is_same_v<Decayed, char>) {
        char buf[2] = {val, '\0'};
        return fastlang_string_create_from_ptr(buf);
    } else if constexpr (std::is_same_v<Decayed, bool>) {
        return fastlang_string_create_from_ptr(val ? "true" : "false");
    } else if constexpr (std::is_same_v<Decayed, fastlang_unit_t>) {
        return fastlang_string_create_from_ptr("()");
    } else if constexpr (std::is_same_v<Decayed, fastlang_undefined_t>) {
        return fastlang_string_create_from_ptr("undefined");
    } else if constexpr (std::is_signed_v<Decayed> && (std::is_same_v<Decayed, int8_t> || std::is_same_v<Decayed, int16_t> || std::is_same_v<Decayed, int32_t> || std::is_same_v<Decayed, int64_t>)) {
        char buf[64];
        snprintf(buf, sizeof(buf), "%lld", (long long)val);
        return fastlang_string_create_from_ptr(buf);
    } else if constexpr (std::is_unsigned_v<Decayed> && (std::is_same_v<Decayed, uint8_t> || std::is_same_v<Decayed, uint16_t> || std::is_same_v<Decayed, uint32_t> || std::is_same_v<Decayed, uint64_t>)) {
        char buf[64];
        snprintf(buf, sizeof(buf), "%llu", (unsigned long long)val);
        return fastlang_string_create_from_ptr(buf);
    } else if constexpr (std::is_floating_point_v<Decayed>) {
        char buf[64];
        snprintf(buf, sizeof(buf), "%g", (double)val);
        return fastlang_string_create_from_ptr(buf);
    } else if constexpr (has_display<Decayed>::value) {
        return fastlang_string_create_from_ptr(const_cast<Decayed&>(val).display());
    } else if constexpr (fastlang_has_as_str<Decayed>::value) {
        return fastlang_string_create_from_ptr(const_cast<Decayed&>(val).as_str());
    } else if constexpr (std::is_pointer_v<Decayed>) {
        if (!val) return fastlang_string_create_from_ptr("None");
        return fastlang_as_str(*val);
    } else {
        return fastlang_string_create_from_ptr("[object]");
    }
}

template <typename T>
inline char* fastlang_as_str(T* ptr) {
    if (!ptr) return fastlang_string_create_from_ptr("None");
    return fastlang_as_str(*ptr);
}

template <typename T>
inline char* fastlang_to_str(const T& val) {
    return fastlang_as_str(val);
}

template <typename Target, typename Candidate>
bool fastlang_match_eq(const Target& target, const Candidate& candidate) {
    if constexpr (std::is_invocable_r_v<Target, Candidate>) {
        return target == candidate();
    } else {
        return target == candidate;
    }
}

namespace fast_std {
    template <typename T> struct iterator;
    template <typename T> inline iterator<T> fastlang_array_iter(fast_std::array<T> arr);
}

template <typename T, typename = void> struct has_iter_fn : std::false_type {};
template <typename T> struct has_iter_fn<T, std::void_t<decltype(std::declval<T>().iter())>> : std::true_type {};

template <typename T, typename = void> struct has_iterator_fn : std::false_type {};
template <typename T> struct has_iterator_fn<T, std::void_t<decltype(std::declval<T>().iterator())>> : std::true_type {};

template <typename T, typename = void> struct has_ptr_iter_fn : std::false_type {};
template <typename T> struct has_ptr_iter_fn<T, std::void_t<decltype(std::declval<T>()->iter())>> : std::true_type {};

template <typename T>
inline auto fastlang_get_iter(T&& col) {
    using Decayed = std::decay_t<T>;
    if constexpr (std::is_pointer_v<Decayed>) {
        return fast_std::fastlang_array_iter(col);
    } else if constexpr (has_iter_fn<Decayed>::value) {
        return col.iter();
    } else if constexpr (has_iterator_fn<Decayed>::value) {
        return col.iterator();
    } else if constexpr (has_ptr_iter_fn<Decayed>::value) {
        return col->iter();
    } else {
        return col;
    }
}
template <typename T, size_t N>
inline auto fastlang_get_iter(T (&arr)[N]) {
    return fast_std::fastlang_array_iter(static_cast<T*>(arr));
}

template <typename T = void>
struct fastlang_spread_acc {
    std::vector<T> items;
    template <typename U>
    void push_one(const U& x) { items.push_back(static_cast<T>(x)); }
    template <typename C>
    void push_spread(const C& c) {
        if constexpr (std::is_pointer_v<std::decay_t<C>>) {
            if (c) {
                size_t len = fastlang_array_len(c);
                for (size_t i = 0; i < len; ++i) {
                    items.push_back(static_cast<T>(c[i]));
                }
            }
        } else {
            for (auto&& x : c) items.push_back(static_cast<T>(x));
        }
    }
    fast_std::array<T> to_array() {
        if (items.empty()) return fastlang_array_alloc<T>(0);
        T* arr = fastlang_array_alloc<T>(items.size());
        for (size_t i = 0; i < items.size(); ++i) arr[i] = items[i];
        return arr;
    }
    fast_std::array<T> to_slice() {
        return to_array();
    }
};

template<typename T, typename = void>
struct fastlang_type_default_helper {
    static T get() { return T{}; }
};
template<typename T>
struct fastlang_type_default_helper<T, std::void_t<decltype(T::fastlang_handle_default())>> {
    static T get() { return T::fastlang_handle_default(); }
};
template<typename T>
inline T fastlang_type_default() {
    return fastlang_type_default_helper<T>::get();
}

"#.to_string()
}
