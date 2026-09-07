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

template <typename T> class fastlang_name;
template <typename T> class fastlang_modify;
template <typename T> class fastlang_slice;

template <typename T>
class fastlang_slice {
public:
    using value_type = T;
    T* _data = nullptr;
    size_t _size = 0;
    fastlang_slice() : _data(nullptr), _size(0) {}
    fastlang_slice(const T* data) : _data(const_cast<T*>(data)), _size(data ? (std::is_same<T, char>::value ? strlen((const char*)data) : 0) : 0) {}
    fastlang_slice(const T* data, size_t size) : _data(const_cast<T*>(data)), _size(size) {}
    template <size_t N>
    fastlang_slice(const T (&arr)[N]) : _data(const_cast<T*>(arr)), _size((N > 0 && std::is_same<T, char>::value && arr[N - 1] == '\0') ? N - 1 : N) {}
    fastlang_slice(std::initializer_list<T> list) : _data(const_cast<T*>(list.begin())), _size(list.size()) {}
    T& operator[](size_t idx) { return _data[idx]; }
    const T& operator[](size_t idx) const { return _data[idx]; }
    size_t size() const { return _size; }
    T* data() { return _data; }
    const T* data() const { return _data; }
    T* begin() { return _data; }
    const T* begin() const { return _data; }
    T* end() { return _data + _size; }
    const T* end() const { return _data + _size; }
    operator T*() { return _data; }
    operator const T*() const { return _data; }
};

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

class fastlang_str {
public:
    const char* _data = "";
    size_t _size = 0;
    fastlang_str() : _data(""), _size(0) {}
    fastlang_str(const char* s) : _data(s ? s : ""), _size(s ? strlen(s) : 0) {}
    fastlang_str(const char* data, size_t size) : _data(data ? data : ""), _size(size) {}
    template <size_t N>
    fastlang_str(const char (&arr)[N]) : _data(arr), _size((N > 0 && arr[N - 1] == '\0') ? N - 1 : N) {}
    fastlang_str(const fastlang_slice<char>& s) : _data(s._data ? s._data : ""), _size(s._size) {}

    operator fastlang_slice<char>() const { return fastlang_slice<char>(const_cast<char*>(_data), _size); }
    char operator[](size_t idx) const { return _data[idx]; }
    size_t size() const { return _size; }
    size_t len() const { return _size; }
    bool is_empty() const { return _size == 0; }
    const char* data() const { return _data; }
    const char* c_str() const { return _data; }
    const char* begin() const { return _data; }
    const char* end() const { return _data + _size; }
    operator const char*() const { return _data; }
};

inline bool operator==(const fastlang_str& a, const fastlang_str& b) {
    if (a._size != b._size) return false;
    if (a._size == 0) return true;
    return memcmp(a._data, b._data, a._size) == 0;
}
inline bool operator!=(const fastlang_str& a, const fastlang_str& b) { return !(a == b); }

inline fastlang_str operator+(const fastlang_str& a, const fastlang_str& b) {
    size_t total = a._size + b._size;
    char* buf = new char[total + 1];
    if (a._size > 0) memcpy(buf, a._data, a._size);
    if (b._size > 0) memcpy(buf + a._size, b._data, b._size);
    buf[total] = '\0';
    return fastlang_str(buf, total);
}
inline fastlang_str operator+(const fastlang_str& a, const fastlang_slice<char>& b) {
    size_t total = a._size + b._size;
    char* buf = new char[total + 1];
    if (a._size > 0) memcpy(buf, a._data, a._size);
    if (b._size > 0) memcpy(buf + a._size, b._data, b._size);
    buf[total] = '\0';
    return fastlang_str(buf, total);
}
inline fastlang_str operator+(const fastlang_slice<char>& a, const fastlang_str& b) {
    size_t total = a._size + b._size;
    char* buf = new char[total + 1];
    if (a._size > 0) memcpy(buf, a._data, a._size);
    if (b._size > 0) memcpy(buf + a._size, b._data, b._size);
    buf[total] = '\0';
    return fastlang_str(buf, total);
}
inline fastlang_str operator+(const fastlang_str& a, char c) {
    size_t total = a._size + 1;
    char* buf = new char[total + 1];
    if (a._size > 0) memcpy(buf, a._data, a._size);
    buf[a._size] = c;
    buf[total] = '\0';
    return fastlang_str(buf, total);
}
inline fastlang_str operator+(char c, const fastlang_str& b) {
    size_t total = 1 + b._size;
    char* buf = new char[total + 1];
    buf[0] = c;
    if (b._size > 0) memcpy(buf + 1, b._data, b._size);
    buf[total] = '\0';
    return fastlang_str(buf, total);
}
inline fastlang_str operator+(const fastlang_str& a, bool b) {
    return a + fastlang_str(b ? "true" : "false");
}
inline fastlang_str operator+(bool a, const fastlang_str& b) {
    return fastlang_str(a ? "true" : "false") + b;
}

inline bool operator==(const fastlang_slice<char>& a, const fastlang_slice<char>& b) {
    if (a._size != b._size) return false;
    if (a._size == 0) return true;
    return memcmp(a._data, b._data, a._size) == 0;
}
inline bool operator!=(const fastlang_slice<char>& a, const fastlang_slice<char>& b) { return !(a == b); }

inline fastlang_slice<char> operator+(const fastlang_slice<char>& a, const fastlang_slice<char>& b) {
    size_t total = a._size + b._size;
    char* buf = new char[total + 1];
    if (a._size > 0) memcpy(buf, a._data, a._size);
    if (b._size > 0) memcpy(buf + a._size, b._data, b._size);
    buf[total] = '\0';
    return fastlang_slice<char>(buf, total);
}
inline fastlang_slice<char> operator+(const fastlang_slice<char>& a, char c) {
    size_t total = a._size + 1;
    char* buf = new char[total + 1];
    if (a._size > 0) memcpy(buf, a._data, a._size);
    buf[a._size] = c;
    buf[total] = '\0';
    return fastlang_slice<char>(buf, total);
}
inline fastlang_slice<char> operator+(char c, const fastlang_slice<char>& b) {
    size_t total = 1 + b._size;
    char* buf = new char[total + 1];
    buf[0] = c;
    if (b._size > 0) memcpy(buf + 1, b._data, b._size);
    buf[total] = '\0';
    return fastlang_slice<char>(buf, total);
}
inline fastlang_slice<char> operator+(const fastlang_slice<char>& a, bool b) {
    return a + fastlang_slice<char>(b ? "true" : "false");
}
inline fastlang_slice<char> operator+(bool a, const fastlang_slice<char>& b) {
    return fastlang_slice<char>(a ? "true" : "false") + b;
}

template <typename T, typename = void>
struct fastlang_has_as_str : std::false_type {};
template <typename T>
struct fastlang_has_as_str<T, std::void_t<decltype(std::declval<T>().as_str())>> : std::true_type {};

template <typename T, typename = void>
struct has_display : std::false_type {};
template <typename T>
struct has_display<T, std::void_t<decltype(std::declval<T>().display())>> : std::true_type {};

template <typename U>
struct is_fastlang_slice_type : std::false_type {};
template <typename U>
struct is_fastlang_slice_type<fastlang_slice<U>> : std::true_type {};

template <typename U>
struct is_fastlang_name_type : std::false_type {};
template <typename U>
struct is_fastlang_name_type<fastlang_name<U>> : std::true_type {};

inline fastlang_str fastlang_as_str(bool b) { return fastlang_str(b ? "true" : "false"); }
inline fastlang_str fastlang_as_str(const fastlang_str& s) { return s; }
inline fastlang_str fastlang_as_str(const fastlang_slice<char>& s) { return fastlang_str(s); }
inline fastlang_str fastlang_as_str(const char* s) { return fastlang_str(s ? s : ""); }

template <typename T>
inline auto fastlang_as_str(const T& val) {
    using Decayed = std::decay_t<T>;
    if constexpr (std::is_same_v<Decayed, fastlang_str>) {
        return val;
    } else if constexpr (std::is_same_v<Decayed, fastlang_slice<char>>) {
        return fastlang_str(val);
    } else if constexpr (std::is_same_v<Decayed, const char*> || std::is_same_v<Decayed, char*>) {
        return fastlang_str(val ? val : "");
    } else if constexpr (std::is_same_v<Decayed, char>) {
        char* buf = new char[2];
        buf[0] = val;
        buf[1] = '\0';
        return fastlang_str(buf, 1);
    } else if constexpr (std::is_same_v<Decayed, bool>) {
        return fastlang_str(val ? "true" : "false");
    } else if constexpr (std::is_same_v<Decayed, fastlang_unit_t>) {
        return fastlang_str("()");
    } else if constexpr (std::is_same_v<Decayed, fastlang_undefined_t>) {
        return fastlang_str("undefined");
    } else if constexpr (std::is_same_v<Decayed, uint8_t> || std::is_same_v<Decayed, int8_t>) {
        char buf[64];
        int len = 0;
        if constexpr (std::is_signed_v<Decayed>) {
            len = snprintf(buf, sizeof(buf), "%d", (int)val);
        } else {
            len = snprintf(buf, sizeof(buf), "%u", (unsigned int)val);
        }
        char* res = new char[len + 1];
        memcpy(res, buf, len + 1);
        return fastlang_str(res, len);
    } else if constexpr (std::is_integral_v<Decayed>) {
        char buf[64];
        int len = 0;
        if constexpr (std::is_signed_v<Decayed>) {
            len = snprintf(buf, sizeof(buf), "%lld", (long long)val);
        } else {
            len = snprintf(buf, sizeof(buf), "%llu", (unsigned long long)val);
        }
        char* res = new char[len + 1];
        memcpy(res, buf, len + 1);
        return fastlang_str(res, len);
    } else if constexpr (std::is_floating_point_v<Decayed>) {
        char buf[64];
        int len = snprintf(buf, sizeof(buf), "%g", (double)val);
        char* res = new char[len + 1];
        memcpy(res, buf, len + 1);
        return fastlang_str(res, len);
    } else if constexpr (std::is_pointer_v<Decayed>) {
        if (!val) return fastlang_str("None");
        return fastlang_as_str(*val);
    } else if constexpr (fastlang_has_as_str<Decayed>::value) {
        return fastlang_str(const_cast<Decayed&>(val).as_str());
    } else if constexpr (has_display<Decayed>::value) {
        return fastlang_str(const_cast<Decayed&>(val).display());
    } else if constexpr (is_fastlang_slice_type<Decayed>::value) {
        fastlang_str res = "[";
        for (size_t i = 0; i < val._size; ++i) {
            if (i > 0) res = res + ", ";
            res = res + fastlang_as_str(val._data[i]);
        }
        res = res + "]";
        return res;
    } else {
        return fastlang_str("[object]");
    }
}

template <typename T>
inline auto fastlang_as_str(T* ptr) {
    if (!ptr) return fastlang_str("None");
    return fastlang_as_str(*ptr);
}

template <typename T>
inline auto fastlang_to_str(const T& val) {
    return fastlang_as_str(val);
}

template <typename T>
inline fastlang_str operator+(const fastlang_str& a, const T& b) {
    fastlang_str b_str = fastlang_as_str(b);
    return a + b_str;
}
template <typename T>
inline fastlang_str operator+(const T& a, const fastlang_str& b) {
    fastlang_str a_str = fastlang_as_str(a);
    return a_str + b;
}
template <typename T>
inline fastlang_slice<char> operator+(const fastlang_slice<char>& a, const T& b) {
    fastlang_str b_str = fastlang_as_str(b);
    return a + fastlang_slice<char>(const_cast<char*>(b_str._data), b_str._size);
}
template <typename T>
inline fastlang_slice<char> operator+(const T& a, const fastlang_slice<char>& b) {
    fastlang_str a_str = fastlang_as_str(a);
    return fastlang_slice<char>(const_cast<char*>(a_str._data), a_str._size) + b;
}

template <typename T>
auto fastlang_len(const T& val) -> decltype(val.size()) { return val.size(); }
template <typename T, size_t N>
size_t fastlang_len(const T (&)[N]) { return N; }

template <typename Target, typename Candidate>
bool fastlang_match_eq(const Target& target, const Candidate& candidate) {
    if constexpr (std::is_invocable_r_v<Target, Candidate>) {
        return target == candidate();
    } else {
        return target == candidate;
    }
}

struct fastlang_tag_stop {};
inline constexpr fastlang_tag_stop stop{};

template <typename T, typename = void>
struct is_fastlang_stop : std::false_type {};
template <typename T>
struct is_fastlang_stop<T, std::void_t<decltype(std::declval<T>().is_stop())>> : std::true_type {};

template <typename T, typename = void>
struct is_fastlang_modify_type : std::false_type {};
template <typename T>
struct is_fastlang_modify_type<fastlang_modify<T>> : std::true_type {};

template <typename T, typename = void>
struct has_iter_fn : std::false_type {};
template <typename T>
struct has_iter_fn<T, std::void_t<decltype(std::declval<T>().iter())>> : std::true_type {};

template <typename T, typename = void>
struct has_iterator_fn : std::false_type {};
template <typename T>
struct has_iterator_fn<T, std::void_t<decltype(std::declval<T>().iterator())>> : std::true_type {};

template <typename T, typename = void>
struct has_fastlang_iter : std::false_type {};
template <typename T>
struct has_fastlang_iter<T, std::void_t<decltype(std::declval<T>().next())>> : std::integral_constant<bool, has_iter_fn<T>::value || has_iterator_fn<T>::value> {};

template <typename T, typename = void>
struct has_broken_flag : std::false_type {};
template <typename T>
struct has_broken_flag<T, std::void_t<decltype(std::declval<T>().broken)>> : std::true_type {};

template <typename T, typename = void>
struct has_is_done_flag : std::false_type {};
template <typename T>
struct has_is_done_flag<T, std::void_t<decltype(std::declval<T>().is_done)>> : std::true_type {};

template <typename T, typename = void>
struct is_fastlang_option : std::false_type {};
template <typename T>
struct is_fastlang_option<T, std::void_t<decltype(std::declval<T>().is_Some()), decltype(std::declval<T>().is_None())>> : std::true_type {};

template <typename T>
struct is_fastlang_variant : std::false_type {};
template <typename... Types>
struct is_fastlang_variant<std::variant<Types...>> : std::true_type {};

template <typename Iterable>
struct fastlang_iter_wrapper {
    Iterable* _target;
    bool _done;
    using RawValType = std::decay_t<decltype(std::declval<Iterable>().next())>;
    RawValType _current_val;
    fastlang_iter_wrapper(Iterable* t, bool is_end) : _target(t), _done(is_end), _current_val{} {
        if (!_done && _target) {
            if constexpr (has_iter_fn<Iterable>::value) {
                _target->iter();
            } else if constexpr (has_iterator_fn<Iterable>::value) {
                _target->iterator();
            }
            advance();
        }
    }
    void advance() {
        if (!_target) { _done = true; return; }
        if constexpr (has_broken_flag<Iterable>::value) {
            if (_target->broken) { _done = true; return; }
        }
        if constexpr (has_is_done_flag<Iterable>::value) {
            if (_target->is_done) { _done = true; return; }
        }
        _current_val = _target->next();
        if constexpr (is_fastlang_option<RawValType>::value) {
            if (_current_val.is_None()) {
                _done = true;
                return;
            }
        } else if constexpr (is_fastlang_stop<RawValType>::value) {
            if (_current_val.is_stop()) {
                _done = true;
                return;
            }
        } else if constexpr (is_fastlang_modify_type<RawValType>::value) {
            if (_current_val.ptr == nullptr) {
                _done = true;
                return;
            }
        } else if constexpr (std::is_pointer_v<RawValType>) {
            if (_current_val == nullptr) {
                _done = true;
                return;
            }
        } else if constexpr (is_fastlang_variant<RawValType>::value) {
            if (_current_val.index() != 0) {
                _done = true;
                return;
            }
        }
        if constexpr (has_broken_flag<Iterable>::value) {
            if (_target->broken) { _done = true; return; }
        }
    }
    bool operator!=(const fastlang_iter_wrapper& other) const { return _done != other._done; }
    fastlang_iter_wrapper& operator++() { advance(); return *this; }
    decltype(auto) operator*() const {
        if constexpr (is_fastlang_option<RawValType>::value) {
            return std::get<1>(_current_val.data)._0;
        } else if constexpr (is_fastlang_modify_type<RawValType>::value) {
            return *_current_val;
        } else if constexpr (is_fastlang_variant<RawValType>::value) {
            return std::get<0>(_current_val);
        } else {
            return _current_val;
        }
    }
};

template <typename Iterable>
struct fastlang_iter_range {
    Iterable* _target;
    auto begin() const { return fastlang_iter_wrapper<Iterable>(_target, false); }
    auto end() const { return fastlang_iter_wrapper<Iterable>(_target, true); }
};

template <typename T>
auto fastlang_iterable(T&& obj) {
    if constexpr (has_fastlang_iter<std::remove_reference_t<T>>::value) {
        return fastlang_iter_range<std::remove_reference_t<T>>{&obj};
    } else {
        return std::forward<T>(obj);
    }
}
template <typename T, size_t N>
auto fastlang_iterable(T (&arr)[N]) {
    return fastlang_slice<T>(arr);
}

template <typename T = void>
struct fastlang_spread_acc {
    std::vector<T> items;
    template <typename U>
    void push_one(const U& x) { items.push_back(static_cast<T>(x)); }
    template <typename C>
    void push_spread(const C& c) { for (auto&& x : c) items.push_back(static_cast<T>(x)); }
    fastlang_slice<T> to_slice() {
        if (items.empty()) return fastlang_slice<T>();
        T* buf = new T[items.size()];
        for (size_t i = 0; i < items.size(); ++i) buf[i] = items[i];
        return fastlang_slice<T>(buf, items.size());
    }
};

template<typename T, typename = void>
struct has_copy : std::false_type {};
template<typename T>
struct has_copy<T, std::void_t<decltype(std::declval<T>().copy())>> : std::true_type {};

template<typename T, typename = void>
struct has_drop : std::false_type {};
template<typename T>
struct has_drop<T, std::void_t<decltype(std::declval<T>().drop())>> : std::true_type {};

template<typename T, typename = void>
struct has_custom_yield : std::false_type {};
template<typename T>
struct has_custom_yield<T, std::void_t<decltype(std::declval<T>().yield())>> : std::true_type {};

template<typename T, typename = void>
struct is_throwable : std::false_type {};
template<typename T>
struct is_throwable<T, std::void_t<decltype(std::declval<T>().error())>> : std::true_type {};

template<typename T, typename = void>
struct has_is_done : std::false_type {};
template<typename T>
struct has_is_done<T, std::void_t<decltype(std::declval<T>().is_done)>> : std::true_type {};

template<typename T, typename = void>
struct has_has_yielded : std::false_type {};
template<typename T>
struct has_has_yielded<T, std::void_t<decltype(std::declval<T>().has_yielded)>> : std::true_type {};

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

template <typename Signature>
class fastlang_method;

template <typename Ret, typename... Args>
class fastlang_method<Ret(Args...)> {
public:
    std::function<Ret(Args...)> _fn;
    std::function<bool()> _is_done_fn;
    std::function<bool()> _has_yielded_fn;
    bool is_done = false;
    bool has_yielded = false;

    fastlang_method() : is_done(false), has_yielded(false) {}

    template <typename F, typename = std::enable_if_t<!std::is_same_v<std::decay_t<F>, fastlang_method> && std::is_invocable_r_v<Ret, F, Args...>>>
    fastlang_method(F&& f) {
        using Decayed = std::decay_t<F>;
        if constexpr (has_is_done<Decayed>::value) {
            if constexpr (std::is_lvalue_reference_v<F>) {
                auto ptr = &f;
                _is_done_fn = [ptr]() { return ptr->is_done; };
                if constexpr (has_has_yielded<Decayed>::value) {
                    _has_yielded_fn = [ptr]() { return ptr->has_yielded; };
                }
                _fn = [ptr](Args... args) -> Ret {
                    if constexpr (std::is_void_v<Ret>) {
                        (*ptr)(args...);
                    } else {
                        return (*ptr)(args...);
                    }
                };
            } else {
                auto ptr = std::make_shared<Decayed>(std::move(f));
                _is_done_fn = [ptr]() { return ptr->is_done; };
                if constexpr (has_has_yielded<Decayed>::value) {
                    _has_yielded_fn = [ptr]() { return ptr->has_yielded; };
                }
                _fn = [ptr](Args... args) -> Ret {
                    if constexpr (std::is_void_v<Ret>) {
                        (*ptr)(args...);
                    } else {
                        return (*ptr)(args...);
                    }
                };
            }
        } else {
            _fn = std::forward<F>(f);
        }
    }

    fastlang_method(const fastlang_method& other) = default;
    fastlang_method& operator=(const fastlang_method& other) = default;

    Ret operator()(Args... args) {
        if constexpr (std::is_void_v<Ret>) {
            if (_fn) _fn(args...);
            if (_is_done_fn) is_done = _is_done_fn();
            else is_done = true;
            if (_has_yielded_fn) has_yielded = _has_yielded_fn();
        } else {
            Ret r = _fn ? _fn(args...) : Ret{};
            if (_is_done_fn) is_done = _is_done_fn();
            else is_done = true;
            if (_has_yielded_fn) has_yielded = _has_yielded_fn();
            return r;
        }
    }

    operator std::function<Ret(Args...)>() const {
        return _fn;
    }
};

template <typename Self>
inline auto _fastlang_do_yield(Self* self) {
    if constexpr (has_custom_yield<Self>::value) {
        return self->yield();
    } else {
        return;
    }
}

template <typename T>
inline void _fastlang_del(T* ptr) {
    if (ptr) {
        delete ptr;
    }
}

template <typename T>
inline void _fastlang_del(T& obj) {
    if constexpr (has_drop<T>::value) { obj._fastlang_call_drop(); }
}

template <typename T>
inline void _fastlang_del_array(T* ptr) {
    if (ptr) { delete[] ptr; }
}

template <typename T>
class fastlang_name {
public:
const T* ptr;
const T* target() const { return ptr; }
const T* get_ptr() const { return ptr; }
void drop() { 
delete const_cast<T*>(ptr);
ptr = nullptr;
}
const T& operator[](size_t index) const { return ptr[index]; }
fastlang_name(const T& ref) : ptr(&ref) {}
fastlang_name(const T* p = nullptr) : ptr(p) {}
fastlang_name(const fastlang_name& other) : ptr(other.ptr) {}
fastlang_name(const fastlang_modify<T>& m);
fastlang_name& operator=(const fastlang_name& other) { ptr = other.ptr; return *this; }
fastlang_name& operator=(const fastlang_modify<T>& m);
const T& operator*() const { return *ptr; }
const T* operator->() const { return ptr; }
T* operator->() { return const_cast<T*>(ptr); }
operator const T&() const { return *ptr; }
fastlang_name& operator=(const T* p) { ptr = p; return *this; }
fastlang_name& operator=(const T& ref) { ptr = &ref; return *this; }
template <size_t N> fastlang_name& operator=(const T (&arr)[N]) { ptr = arr; return *this; }
const T* operator+(std::ptrdiff_t offset) const { return ptr + offset; }
const T* operator-(std::ptrdiff_t offset) const { return ptr - offset; }
bool operator==(std::nullptr_t) const { return ptr == nullptr; }
bool operator!=(std::nullptr_t) const { return ptr != nullptr; }
explicit operator bool() const { return ptr != nullptr; }
};
template <typename T> fastlang_name(const T&) -> fastlang_name<T>;
template <typename T> fastlang_name(T*) -> fastlang_name<T>;

template <typename Ret, typename... Args>
class fastlang_name<std::function<Ret(Args...)>> {
public:
    std::function<Ret(Args...)> callable{};
    bool _has_yield = false;
    bool _has_leave = false;
    bool _has_return = false;
    bool _is_done = false;
    Ret return_value{};

    fastlang_name() : callable{} {}
    template <typename F, typename = std::enable_if_t<!std::is_same_v<std::decay_t<F>, fastlang_name>>>
    fastlang_name(F&& fn) {
        if constexpr (std::is_pointer_v<std::decay_t<F>>) {
            callable = [p = std::forward<F>(fn)](Args... args) -> Ret {
                return (*p)(std::forward<Args>(args)...);
            };
        } else {
            callable = std::forward<F>(fn);
        }
    }

    template <typename F>
    fastlang_name& operator=(F&& fn) {
        if constexpr (std::is_same_v<std::decay_t<F>, fastlang_name>) {
            callable = fn.callable;
        } else if constexpr (std::is_pointer_v<std::decay_t<F>>) {
            callable = [p = std::forward<F>(fn)](Args... args) -> Ret {
                return (*p)(std::forward<Args>(args)...);
            };
        } else {
            callable = std::forward<F>(fn);
        }
        return *this;
    }

    bool has_yield() const { return _has_yield; }
    bool has_leave() const { return _has_leave; }
    bool has_return() const { return _has_return; }
    bool is_done() const { return _is_done; }

    Ret operator()(Args... args) {
        _has_yield = false;
        _has_leave = false;
        if constexpr (std::is_void_v<Ret>) {
            callable(std::forward<Args>(args)...);
            _is_done = true;
        } else {
            auto res = callable(std::forward<Args>(args)...);
            return_value = res;
            _has_return = true;
            _is_done = true;
            return res;
        }
    }
};

template <typename T>
class fastlang_modify {
public:
T* ptr;
bool owned = false;
T* target() const { return ptr; }
T* get_ptr() const { return ptr; }
void drop() { 
    if (owned && ptr) { 
        if constexpr (has_drop<T>::value) { ptr->drop(); }
        delete ptr; 
    }
    ptr = nullptr;
    owned = false;
}
static T* make_copy(const T& ref) {
    if constexpr (has_copy<T>::value) {
        return new T(const_cast<T&>(ref).copy());
    } else {
        return new T(ref);
    }
}
T& operator[](size_t index) { return ptr[index]; }
fastlang_modify(T& ref) : ptr(&ref), owned(false) {}
fastlang_modify(const T& ref) : ptr(make_copy(ref)), owned(true) {}
fastlang_modify(T* p, bool is_owned) : ptr(p), owned(is_owned) {}
fastlang_modify(T* p = nullptr) : ptr(p), owned(false) {}
fastlang_modify(const fastlang_modify& other) : ptr(other.owned && other.ptr ? make_copy(*other.ptr) : other.ptr), owned(other.owned) {}
fastlang_modify(fastlang_tag_stop) : ptr(nullptr), owned(false) {}
fastlang_modify(const fastlang_name<T>& n) : ptr(const_cast<T*>(n.ptr)), owned(false) {}
fastlang_modify& operator=(const fastlang_modify& other) { 
    if (owned && ptr && ptr != other.ptr) { delete ptr; }
    ptr = other.owned && other.ptr ? make_copy(*other.ptr) : other.ptr; 
    owned = other.owned; 
    return *this; 
}
fastlang_modify& operator=(const fastlang_name<T>& n) {
    if (owned && ptr) { delete ptr; }
    ptr = const_cast<T*>(n.ptr);
    owned = false;
    return *this;
}
fastlang_modify& operator=(fastlang_tag_stop) {
    drop();
    return *this;
}
bool is_stop() const { return ptr == nullptr; }
T& operator*() const { return *ptr; }
T* operator->() const { return ptr; }
operator T&() { return *ptr; }
operator const T&() const { return *ptr; }
fastlang_modify& operator=(T* p) { if (owned && ptr && ptr != p) { delete ptr; } ptr = p; owned = false; return *this; }
fastlang_modify& operator=(const T& ref) { 
    if (owned && ptr) { 
        *ptr = (has_copy<T>::value ? const_cast<T&>(ref).copy() : ref); 
    } else { 
        ptr = make_copy(ref); 
        owned = true; 
    } 
    return *this; 
}
template <size_t N> fastlang_modify& operator=(T (&arr)[N]) { ptr = arr; owned = false; return *this; }
template <size_t N> fastlang_modify& operator=(const T (&arr)[N]) { ptr = const_cast<T*>(arr); owned = false; return *this; }
template <typename U> fastlang_modify& operator+=(const U& val) { *ptr += val; return *this; }
template <typename U> fastlang_modify& operator-=(const U& val) { *ptr -= val; return *this; }
template <typename U> fastlang_modify& operator*=(const U& val) { *ptr *= val; return *this; }
template <typename U> fastlang_modify& operator/=(const U& val) { *ptr /= val; return *this; }
T* operator+(std::ptrdiff_t offset) const { return ptr + offset; }
T* operator-(std::ptrdiff_t offset) const { return ptr - offset; }
bool operator==(std::nullptr_t) const { return ptr == nullptr; }
bool operator!=(std::nullptr_t) const { return ptr != nullptr; }
explicit operator bool() const { return ptr != nullptr; }
fastlang_modify(fastlang_modify&& other) noexcept : ptr(other.ptr), owned(other.owned) { other.ptr = nullptr; other.owned = false; }
fastlang_modify& operator=(fastlang_modify&& other) noexcept { if (this != &other) { drop(); ptr = other.ptr; owned = other.owned; other.ptr = nullptr; other.owned = false; } return *this; }
};
template <typename T> fastlang_modify(T&) -> fastlang_modify<T>;
template <typename T> fastlang_modify(T*) -> fastlang_modify<T>;
template <typename T> using fastlang_copy = fastlang_modify<T>;
template <typename T>
inline fastlang_modify<T> fastlang_make_copy(const T& val) {
    return fastlang_modify<T>(fastlang_modify<T>::make_copy(val), true);
}
template <typename T>
inline fastlang_modify<T> fastlang_make_copy(const fastlang_modify<T>& m) {
    return fastlang_modify<T>(m.ptr ? fastlang_modify<T>::make_copy(*m.ptr) : nullptr, true);
}
template <typename T>
inline fastlang_modify<T> fastlang_make_copy(const fastlang_name<T>& n) {
    return fastlang_modify<T>(n.ptr ? fastlang_modify<T>::make_copy(*n.ptr) : nullptr, true);
}

template <typename T>
inline void _fastlang_del(fastlang_name<T>& obj) {
    obj.ptr = nullptr;
}
template <typename T>
inline void _fastlang_del(fastlang_modify<T>& obj) {
    obj.drop();
}

template <typename T>
fastlang_name<T>::fastlang_name(const fastlang_modify<T>& m) : ptr(m.ptr) {}

template <typename T>
fastlang_name<T>& fastlang_name<T>::operator=(const fastlang_modify<T>& m) { ptr = m.ptr; return *this; }

template <typename T>
inline fastlang_str fastlang_as_str(const fastlang_name<T>& obj) {
    if (obj.ptr) return fastlang_as_str(*obj.ptr);
    return fastlang_str("None");
}
template <typename T>
inline fastlang_str fastlang_as_str(const fastlang_modify<T>& obj) {
    if (obj.ptr) return fastlang_as_str(*obj.ptr);
    return fastlang_str("None");
}

namespace fastlang_detail {
    template <typename T, typename U>
    auto arrow_assign_impl(T& target, const U& val, int) -> decltype(target.arrow_assign(val), void()) {
        target.arrow_assign(val);
    }
    template <typename T, typename U>
    auto arrow_assign_impl(T& target, std::initializer_list<U> val, int) -> decltype(target.arrow_assign(fastlang_slice<U>(val)), void()) {
        target.arrow_assign(fastlang_slice<U>(val));
    }
    template <typename T, typename U>
    void arrow_assign_impl(T& target, const U& val, ...) {
        target = val;
    }
    template <typename T, typename U>
    auto arrow_impl(T& target, const U& val, int) -> decltype(target.arrow(val), void()) {
        target.arrow(val);
    }
    template <typename T, typename U>
    auto arrow_impl(T& target, std::initializer_list<U> val, int) -> decltype(target.arrow(fastlang_slice<U>(val)), void()) {
        target.arrow(fastlang_slice<U>(val));
    }
    template <typename T, typename U>
    void arrow_impl(T& target, const U& val, ...) {
        target = val;
    }
}
template <typename T, typename U>
void fastlang_arrow_assign(T& target, const U& val) {
    fastlang_detail::arrow_assign_impl(target, val, 0);
}
template <typename T, typename U>
void fastlang_arrow_assign(T& target, std::initializer_list<U> val) {
    fastlang_detail::arrow_assign_impl(target, val, 0);
}
template <typename T, typename U>
void fastlang_arrow(T& target, const U& val) {
    fastlang_detail::arrow_impl(target, val, 0);
}
template <typename T, typename U>
void fastlang_arrow(T& target, std::initializer_list<U> val) {
    fastlang_detail::arrow_impl(target, val, 0);
}

"#.to_string()
}
