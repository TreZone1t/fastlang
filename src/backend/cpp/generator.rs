use crate::frontend::parser::ast::*;

pub struct CodeGenerator {
    pub(crate) output: String,
    pub(crate) indent_level: usize,
    pub(crate) custom_scopes: std::collections::HashSet<String>,
    pub(crate) custom_scope_types: std::collections::HashSet<String>,
    pub(crate) enum_types: std::collections::HashSet<String>,
    pub(crate) pointer_vars: std::collections::HashSet<String>,
    pub(crate) yield_counter: usize,
    pub(crate) in_class_or_scope: bool,
}

impl CodeGenerator {
    pub fn new() -> Self {
        CodeGenerator {
            output: String::new(),
            indent_level: 0,
            custom_scopes: std::collections::HashSet::new(),
            custom_scope_types: std::collections::HashSet::new(),
            enum_types: std::collections::HashSet::new(),
            pointer_vars: std::collections::HashSet::new(),
            yield_counter: 0,
            in_class_or_scope: false,
        }
    }
    pub(crate) fn emit_operator_overloads(&mut self, handle_block: &Option<Vec<Decl>>) {
        if let Some(handles) = handle_block {
            for h in handles {
                if let Decl::FnDecl { name, params, return_type, .. } = h {
                    let op = match name.as_str() {
                        "add" => Some("+"),
                        "sub" => Some("-"),
                        "mul" => Some("*"),
                        "div" => Some("/"),
                        "mod" => Some("%"),
                        "equal" | "partial_equal" => Some("=="),
                        "not_equal" => Some("!="),
                        "greater_than" => Some(">"),
                        "less_than" => Some("<"),
                        "greater_than_equal" => Some(">="),
                        "less_than_equal" => Some("<="),
                        "index_add" => Some("+="),
                        "index_sub" => Some("-="),
                        "index_mul" => Some("*="),
                        "index_div" => Some("/="),
                        "index_mod" => Some("%="),
                        _ => None,
                    };

                    let ret_str = crate::backend::cpp::stmt::type_to_cpp(return_type);

                    if let Some(o) = op {
                        if params.len() == 1 {
                            let param_type = crate::backend::cpp::stmt::type_to_cpp(
                                &params[0].type_node
                            );
                            let param_name = &params[0].name;
                            self.emit(
                                &format!(
                                    "{} operator{}({} {}) {{",
                                    ret_str,
                                    o,
                                    param_type,
                                    param_name
                                )
                            );
                            self.indent_level += 1;
                            self.emit(&format!("return this->{}({});", name, param_name));
                            self.indent_level -= 1;
                            self.emit("}");
                        }
                    } else if name == "index_access" {
                        if params.len() == 1 {
                            let param_type = crate::backend::cpp::stmt::type_to_cpp(
                                &params[0].type_node
                            );
                            let param_name = &params[0].name;
                            self.emit(
                                &format!("{} operator[]({} {}) {{", ret_str, param_type, param_name)
                            );
                            self.indent_level += 1;
                            self.emit(&format!("return this->index_access({});", param_name));
                            self.indent_level -= 1;
                            self.emit("}");
                        }
                    } else if name == "call" {
                        let param_list: Vec<String> = params
                            .iter()
                            .map(|p| {
                                let p_type = crate::backend::cpp::stmt::type_to_cpp(&p.type_node);
                                format!("{} {}", p_type, p.name)
                            })
                            .collect();
                        let arg_names: Vec<String> = params
                            .iter()
                            .map(|p| p.name.clone())
                            .collect();
                        self.emit(
                            &format!(
                                "{} operator()({}) {{",
                                ret_str,
                                param_list.join(", ")
                            )
                        );
                        self.indent_level += 1;
                        if return_type == &BaseType::Void {
                            self.emit(&format!("this->call({});", arg_names.join(", ")));
                        } else {
                            self.emit(&format!("return this->call({});", arg_names.join(", ")));
                        }
                        self.indent_level -= 1;
                        self.emit("}");
                    }
                }
            }
        }
    }

    pub(crate) fn emit(&mut self, s: &str) {
        let indent = "    ".repeat(self.indent_level);
        self.output.push_str(&format!("{}{}\n", indent, s));
    }

    pub(crate) fn emit_headers(&mut self) {
        self.output.push_str("#include <iostream>\n");
        self.output.push_str("#include <string>\n");
        self.output.push_str("#include <variant>\n");
        self.output.push_str("#include <cstdint>\n");
        self.output.push_str("#include <stdexcept>\n");
        self.output.push_str("#include <type_traits>\n");
        self.output.push_str("#include <functional>\n");
        self.output.push_str("#include <initializer_list>\n\n");
        self.emit("");
        self.emit("std::ostream& operator<<(std::ostream& os, const std::exception& e) {");
        self.emit("    return os << e.what();");
        self.emit("}");
        self.emit("");
        self.emit("template <typename T> class fastlang_name;");
        self.emit("template <typename T> class fastlang_modify;");
        self.emit("template <typename T> class fastlang_copy;");
        self.emit("template <typename T> class fastlang_slice;");
        self.emit("");
        self.emit("template <typename T>");
        self.emit("class fastlang_slice {");
        self.emit("public:");
        self.emit("    const T* _data = nullptr;");
        self.emit("    size_t _size = 0;");
        self.emit("    fastlang_slice() : _data(nullptr), _size(0) {}");
        self.emit("    fastlang_slice(const T* data, size_t size) : _data(data), _size(size) {}");
        self.emit("    template <size_t N>");
        self.emit("    fastlang_slice(const T (&arr)[N]) : _data(arr), _size((N > 0 && std::is_same<T, char>::value && arr[N - 1] == '\\0') ? N - 1 : N) {}");
        self.emit("    fastlang_slice(std::initializer_list<T> list) : _data(list.begin()), _size(list.size()) {}");
        self.emit("    const T& operator[](size_t idx) const { return _data[idx]; }");
        self.emit("    size_t size() const { return _size; }");
        self.emit("    const T* data() const { return _data; }");
        self.emit("    const T* begin() const { return _data; }");
        self.emit("    const T* end() const { return _data + _size; }");
        self.emit("};");
        self.emit("");
        self.emit("template <typename T>");
        self.emit("std::ostream& operator<<(std::ostream& os, const fastlang_slice<T>& s) {");
        self.emit("    if constexpr (std::is_same_v<T, char>) {");
        self.emit("        os.write(s._data, s._size);");
        self.emit("    } else {");
        self.emit("        os << \"[\";");
        self.emit("        for (size_t i = 0; i < s._size; ++i) {");
        self.emit("            if (i > 0) os << \", \";");
        self.emit("            os << s._data[i];");
        self.emit("        }");
        self.emit("        os << \"]\";");
        self.emit("    }");
        self.emit("    return os;");
        self.emit("}");
        self.emit("");
        self.emit("template <typename T>");
        self.emit("auto fastlang_len(const T& val) -> decltype(val.size()) { return val.size(); }");
        self.emit("template <typename T, size_t N>");
        self.emit("size_t fastlang_len(const T (&)[N]) { return N; }");
        self.emit("");
        self.emit("template <typename Target, typename Candidate>");
        self.emit("bool fastlang_match_eq(const Target& target, const Candidate& candidate) {");
        self.emit("    if constexpr (std::is_invocable_r_v<Target, Candidate>) {");
        self.emit("        return target == candidate();");
        self.emit("    } else {");
        self.emit("        return target == candidate;");
        self.emit("    }");
        self.emit("}");
        self.emit("");
        self.emit("template <typename T, typename = void>");
        self.emit("struct has_fastlang_iter : std::false_type {};");
        self.emit("template <typename T>");
        self.emit("struct has_fastlang_iter<T, std::void_t<decltype(std::declval<T>().iterator()), decltype(std::declval<T>().next())>> : std::true_type {};");
        self.emit("");
        self.emit("template <typename T, typename = void>");
        self.emit("struct has_broken_flag : std::false_type {};");
        self.emit("template <typename T>");
        self.emit("struct has_broken_flag<T, std::void_t<decltype(std::declval<T>().broken)>> : std::true_type {};");
        self.emit("");
        self.emit("template <typename T, typename = void>");
        self.emit("struct has_is_done_flag : std::false_type {};");
        self.emit("template <typename T>");
        self.emit("struct has_is_done_flag<T, std::void_t<decltype(std::declval<T>().is_done)>> : std::true_type {};");
        self.emit("");
        self.emit("template <typename T, typename = void>");
        self.emit("struct is_fastlang_option : std::false_type {};");
        self.emit("template <typename T>");
        self.emit("struct is_fastlang_option<T, std::void_t<decltype(std::declval<T>().is_Some()), decltype(std::declval<T>().is_None())>> : std::true_type {};");
        self.emit("");
        self.emit("template <typename Iterable>");
        self.emit("struct fastlang_iter_wrapper {");
        self.emit("    Iterable* _target;");
        self.emit("    bool _done;");
        self.emit("    using RawValType = std::decay_t<decltype(std::declval<Iterable>().next())>;");
        self.emit("    RawValType _current_val;");
        self.emit("    fastlang_iter_wrapper(Iterable* t, bool is_end) : _target(t), _done(is_end), _current_val{} {");
        self.emit("        if (!_done && _target) {");
        self.emit("            _target->iterator();");
        self.emit("            advance();");
        self.emit("        }");
        self.emit("    }");
        self.emit("    void advance() {");
        self.emit("        if (!_target) { _done = true; return; }");
        self.emit("        if constexpr (has_broken_flag<Iterable>::value) {");
        self.emit("            if (_target->broken) { _done = true; return; }");
        self.emit("        }");
        self.emit("        if constexpr (has_is_done_flag<Iterable>::value) {");
        self.emit("            if (_target->is_done) { _done = true; return; }");
        self.emit("        }");
        self.emit("        _current_val = _target->next();");
        self.emit("        if constexpr (is_fastlang_option<RawValType>::value) {");
        self.emit("            if (_current_val.is_None()) {");
        self.emit("                _done = true;");
        self.emit("                return;");
        self.emit("            }");
        self.emit("        }");
        self.emit("        if constexpr (has_broken_flag<Iterable>::value) {");
        self.emit("            if (_target->broken) { _done = true; return; }");
        self.emit("        }");
        self.emit("    }");
        self.emit("    bool operator!=(const fastlang_iter_wrapper& other) const { return _done != other._done; }");
        self.emit("    fastlang_iter_wrapper& operator++() { advance(); return *this; }");
        self.emit("    decltype(auto) operator*() const {");
        self.emit("        if constexpr (is_fastlang_option<RawValType>::value) {");
        self.emit("            return std::get<1>(_current_val.data)._0;");
        self.emit("        } else {");
        self.emit("            return _current_val;");
        self.emit("        }");
        self.emit("    }");
        self.emit("};");
        self.emit("");
        self.emit("template <typename Iterable>");
        self.emit("struct fastlang_iter_range {");
        self.emit("    Iterable* _target;");
        self.emit("    auto begin() const { return fastlang_iter_wrapper<Iterable>(_target, false); }");
        self.emit("    auto end() const { return fastlang_iter_wrapper<Iterable>(_target, true); }");
        self.emit("};");
        self.emit("");
        self.emit("template <typename T>");
        self.emit("auto fastlang_iterable(T& obj) {");
        self.emit("    if constexpr (has_fastlang_iter<T>::value) {");
        self.emit("        return fastlang_iter_range<T>{&obj};");
        self.emit("    } else {");
        self.emit("        return obj;");
        self.emit("    }");
        self.emit("}");
        self.emit("template <typename T, size_t N>");
        self.emit("auto fastlang_iterable(T (&arr)[N]) {");
        self.emit("    return fastlang_slice<T>(arr);");
        self.emit("}");
        self.emit("");
        self.emit("");
        self.emit("template<typename T, typename = void>");
        self.emit("struct has_drop : std::false_type {};");
        self.emit("template<typename T>");
        self.emit("struct has_drop<T, std::void_t<decltype(std::declval<T>().drop())>> : std::true_type {};");
        self.emit("");
        self.emit("template<typename T, typename = void>");
        self.emit("struct has_custom_yield : std::false_type {};");
        self.emit("template<typename T>");
        self.emit("struct has_custom_yield<T, std::void_t<decltype(std::declval<T>().yield())>> : std::true_type {};");
        self.emit("");
        self.emit("template <typename Self>");
        self.emit("inline auto _fastlang_do_yield(Self* self) {");
        self.emit("    if constexpr (has_custom_yield<Self>::value) {");
        self.emit("        return self->yield();");
        self.emit("    } else {");
        self.emit("        return;");
        self.emit("    }");
        self.emit("}");
        self.emit("");
        self.emit("template <typename T>");
        self.emit("inline void _fastlang_del(T* ptr) {");
        self.emit("    if (ptr) {");
        self.emit("        if constexpr (has_drop<T>::value) { ptr->drop(); }");
        self.emit("        delete ptr;");
        self.emit("    }");
        self.emit("}");
        self.emit("");
        self.emit("template <typename T>");
        self.emit("inline void _fastlang_del_array(T* ptr) {");
        self.emit("    if (ptr) { delete[] ptr; }");
        self.emit("}");
        self.emit("");
        self.emit("template <typename T>");
        self.emit("inline void _fastlang_del(fastlang_name<T>& obj) {");
        self.emit("    if (obj.ptr) {");
        self.emit("        if constexpr (has_drop<T>::value) { const_cast<T*>(obj.ptr)->drop(); }");
        self.emit("        delete const_cast<T*>(obj.ptr);");
        self.emit("        obj.ptr = nullptr;");
        self.emit("    }");
        self.emit("}");
        self.emit("");
        self.emit("template <typename T>");
        self.emit("inline void _fastlang_del(fastlang_modify<T>& obj) {");
        self.emit("    if (obj.ptr) {");
        self.emit("        if constexpr (has_drop<T>::value) { obj.ptr->drop(); }");
        self.emit("        delete obj.ptr;");
        self.emit("        obj.ptr = nullptr;");
        self.emit("    }");
        self.emit("}");
        self.emit("");
        self.emit("template <typename T>");
        self.emit("inline void _fastlang_del(fastlang_copy<T>& obj) {");
        self.emit("    if (obj.ptr) {");
        self.emit("        if constexpr (has_drop<T>::value) { obj.ptr->drop(); }");
        self.emit("        delete obj.ptr;");
        self.emit("        obj.ptr = nullptr;");
        self.emit("    }");
        self.emit("}");
        self.emit("");
        self.emit("template <typename T>");
        self.emit("class fastlang_name {");
        self.emit("public:");
        self.emit("const T* ptr;");
        self.emit("const T* target() const { return ptr; }");
        self.emit("const T* get_ptr() const { return ptr; }");
        self.emit("void drop() { ");
        self.emit("delete const_cast<T*>(ptr);");
        self.emit("ptr = nullptr;");
        self.emit("}");
        self.emit("const T& operator[](size_t index) const { return ptr[index]; }");
        self.emit("fastlang_name(const T& ref) : ptr(&ref) {}");
        self.emit("fastlang_name(const T* p = nullptr) : ptr(p) {}");
        self.emit("fastlang_name(const fastlang_name& other) : ptr(other.ptr) {}");
        self.emit("fastlang_name(const fastlang_modify<T>& m);");
        self.emit("fastlang_name(const fastlang_copy<T>& c);");
        self.emit("fastlang_name& operator=(const fastlang_name& other) { ptr = other.ptr; return *this; }");
        self.emit("fastlang_name& operator=(const fastlang_modify<T>& m);");
        self.emit("fastlang_name& operator=(const fastlang_copy<T>& c);");
        self.emit("const T& operator*() const { return *ptr; }");
        self.emit("const T* operator->() const { return ptr; }");
        self.emit("T* operator->() { return const_cast<T*>(ptr); }");
        self.emit("operator const T&() const { return *ptr; }");
        self.emit("fastlang_name& operator=(const T* p) { ptr = p; return *this; }");
        self.emit("fastlang_name& operator=(const T& ref) { ptr = &ref; return *this; }");
        self.emit("template <size_t N> fastlang_name& operator=(const T (&arr)[N]) { ptr = arr; return *this; }");
        self.emit("bool operator==(std::nullptr_t) const { return ptr == nullptr; }");
        self.emit("bool operator!=(std::nullptr_t) const { return ptr != nullptr; }");
        self.emit("explicit operator bool() const { return ptr != nullptr; }");
        self.emit("};");
        self.emit("template <typename T> fastlang_name(const T&) -> fastlang_name<T>;");
        self.emit("template <typename T> fastlang_name(T*) -> fastlang_name<T>;");
        self.emit("");
        self.emit("template <typename Ret, typename... Args>");
        self.emit("class fastlang_name<std::function<Ret(Args...)>> {");
        self.emit("public:");
        self.emit("    std::function<Ret(Args...)> callable{};");
        self.emit("    bool _has_yield = false;");
        self.emit("    bool _has_leave = false;");
        self.emit("    bool _has_return = false;");
        self.emit("    bool _is_done = false;");
        self.emit("    Ret return_value{};");
        self.emit("");
        self.emit("    fastlang_name() : callable{} {}");
        self.emit("    template <typename F, typename = std::enable_if_t<!std::is_same_v<std::decay_t<F>, fastlang_name>>>");
        self.emit("    fastlang_name(F&& fn) : callable(std::forward<F>(fn)) {}");
        self.emit("");
        self.emit("    template <typename F>");
        self.emit("    fastlang_name& operator=(F&& fn) {");
        self.emit("        if constexpr (std::is_same_v<std::decay_t<F>, fastlang_name>) {");
        self.emit("            callable = fn.callable;");
        self.emit("        } else {");
        self.emit("            callable = std::forward<F>(fn);");
        self.emit("        }");
        self.emit("        return *this;");
        self.emit("    }");
        self.emit("");
        self.emit("    bool has_yield() const { return _has_yield; }");
        self.emit("    bool has_leave() const { return _has_leave; }");
        self.emit("    bool has_return() const { return _has_return; }");
        self.emit("    bool is_done() const { return _is_done; }");
        self.emit("");
        self.emit("    Ret operator()(Args... args) {");
        self.emit("        _has_yield = false;");
        self.emit("        _has_leave = false;");
        self.emit("        if constexpr (std::is_void_v<Ret>) {");
        self.emit("            callable(std::forward<Args>(args)...);");
        self.emit("            _is_done = true;");
        self.emit("        } else {");
        self.emit("            auto res = callable(std::forward<Args>(args)...);");
        self.emit("            return_value = res;");
        self.emit("            _has_return = true;");
        self.emit("            _is_done = true;");
        self.emit("            return res;");
        self.emit("        }");
        self.emit("    }");
        self.emit("};");
        self.emit("");
        self.emit("template <typename T>");
        self.emit("class fastlang_modify {");
        self.emit("public:");
        self.emit("T* ptr;");
        self.emit("T* target() const { return ptr; }");
        self.emit("T* get_ptr() const { return ptr; }");
        self.emit("void drop() { ");
        self.emit("delete ptr;");
        self.emit("ptr = nullptr;");
        self.emit("}");
        self.emit("T& operator[](size_t index) { return ptr[index]; }");
        self.emit("fastlang_modify(T& ref) : ptr(&ref) {}");
        self.emit("fastlang_modify(T* p = nullptr) : ptr(p) {}");
        self.emit("fastlang_modify(const fastlang_modify& other) : ptr(other.ptr) {}");
        self.emit("fastlang_modify(const fastlang_name<T>& n);");
        self.emit("fastlang_modify(const fastlang_copy<T>& c);");
        self.emit("fastlang_modify& operator=(const fastlang_modify& other) { ptr = other.ptr; return *this; }");
        self.emit("fastlang_modify& operator=(const fastlang_name<T>& n);");
        self.emit("fastlang_modify& operator=(const fastlang_copy<T>& c);");
        self.emit("T& operator*() const { return *ptr; }");
        self.emit("T* operator->() const { return ptr; }");
        self.emit("operator T&() { return *ptr; }");
        self.emit("operator const T&() const { return *ptr; }");
        self.emit("fastlang_modify& operator=(T* p) { ptr = p; return *this; }");
        self.emit("fastlang_modify& operator=(T& ref) { ptr = &ref; return *this; }");
        self.emit("template <size_t N> fastlang_modify& operator=(T (&arr)[N]) { ptr = arr; return *this; }");
        self.emit("template <size_t N> fastlang_modify& operator=(const T (&arr)[N]) { ptr = const_cast<T*>(arr); return *this; }");
        self.emit("template <typename U> fastlang_modify& operator+=(const U& val) { *ptr += val; return *this; }");
        self.emit("template <typename U> fastlang_modify& operator-=(const U& val) { *ptr -= val; return *this; }");
        self.emit("template <typename U> fastlang_modify& operator*=(const U& val) { *ptr *= val; return *this; }");
        self.emit("template <typename U> fastlang_modify& operator/=(const U& val) { *ptr /= val; return *this; }");
        self.emit("template <typename U> fastlang_modify& operator%=(const U& val) { *ptr %= val; return *this; }");
        self.emit("bool operator==(std::nullptr_t) const { return ptr == nullptr; }");
        self.emit("bool operator!=(std::nullptr_t) const { return ptr != nullptr; }");
        self.emit("explicit operator bool() const { return ptr != nullptr; }");
        self.emit("};");
        self.emit("template <typename T> fastlang_modify(T&) -> fastlang_modify<T>;");
        self.emit("template <typename T> fastlang_modify(T*) -> fastlang_modify<T>;");
        self.emit("");
        self.emit("template <typename T>");
        self.emit("class fastlang_copy {");
        self.emit("public:");
        self.emit("T* ptr;");
        self.emit("T* target() const { return ptr; }");
        self.emit("T* get_ptr() const { return ptr; }");
        self.emit("void drop() { ");
        self.emit("delete ptr;");
        self.emit("ptr = nullptr;");
        self.emit("}");
        self.emit("fastlang_copy(const T& ref) : ptr(new T(ref)) {}");
        self.emit("fastlang_copy(T&& ref) : ptr(new T(std::move(ref))) {}");
        self.emit("fastlang_copy(T* p = nullptr) : ptr(p) {}");
        self.emit("fastlang_copy(const fastlang_copy& other) : ptr(other.ptr ? new T(*other.ptr) : nullptr) {}");
        self.emit("fastlang_copy(const fastlang_name<T>& n);");
        self.emit("fastlang_copy(const fastlang_modify<T>& m);");
        self.emit("fastlang_copy& operator=(const fastlang_copy& other) { ptr = other.ptr; return *this; }");
        self.emit("fastlang_copy& operator=(const fastlang_name<T>& n);");
        self.emit("fastlang_copy& operator=(const fastlang_modify<T>& m);");
        self.emit("T& operator*() const { return *ptr; }");
        self.emit("T* operator->() const { return ptr; }");
        self.emit("operator T&() { return *ptr; }");
        self.emit("operator const T&() const { return *ptr; }");
        self.emit("fastlang_copy& operator=(T* p) { ptr = p; return *this; }");
        self.emit("fastlang_copy& operator=(T& ref) { ptr = &ref; return *this; }");
        self.emit("template <size_t N> fastlang_copy& operator=(T (&arr)[N]) { ptr = arr; return *this; }");
        self.emit("template <typename U> fastlang_copy& operator+=(const U& val) { *ptr += val; return *this; }");
        self.emit("template <typename U> fastlang_copy& operator-=(const U& val) { *ptr -= val; return *this; }");
        self.emit("bool operator==(std::nullptr_t) const { return ptr == nullptr; }");
        self.emit("bool operator!=(std::nullptr_t) const { return ptr != nullptr; }");
        self.emit("explicit operator bool() const { return ptr != nullptr; }");
        self.emit("};");
        self.emit("template <typename T> fastlang_copy(T&) -> fastlang_copy<T>;");
        self.emit("template <typename T> fastlang_copy(T*) -> fastlang_copy<T>;");
        self.emit("");
        self.emit("template <typename T>");
        self.emit(
            "fastlang_name<T>::fastlang_name(const fastlang_modify<T>& m) : ptr(m.ptr) {}"
        );
        self.emit("");
        self.emit("template <typename T>");
        self.emit("fastlang_name<T>::fastlang_name(const fastlang_copy<T>& c) : ptr(c.ptr) {}");
        self.emit("");
        self.emit("template <typename T>");
        self.emit("fastlang_name<T>& fastlang_name<T>::operator=(const fastlang_modify<T>& m) { ptr = m.ptr; return *this; }");
        self.emit("");
        self.emit("template <typename T>");
        self.emit("fastlang_name<T>& fastlang_name<T>::operator=(const fastlang_copy<T>& c) { ptr = c.ptr; return *this; }");
        self.emit("");
        self.emit("template <typename T>");
        self.emit("fastlang_modify<T>::fastlang_modify(const fastlang_name<T>& n) : ptr(const_cast<T*>(n.ptr)) {}");
        self.emit("");
        self.emit("template <typename T>");
        self.emit("fastlang_modify<T>::fastlang_modify(const fastlang_copy<T>& c) : ptr(c.ptr) {}");
        self.emit("");
        self.emit("template <typename T>");
        self.emit("fastlang_modify<T>& fastlang_modify<T>::operator=(const fastlang_name<T>& n) { ptr = const_cast<T*>(n.ptr); return *this; }");
        self.emit("");
        self.emit("template <typename T>");
        self.emit("fastlang_modify<T>& fastlang_modify<T>::operator=(const fastlang_copy<T>& c) { ptr = c.ptr; return *this; }");
        self.emit("");
        self.emit("template <typename T>");
        self.emit("fastlang_copy<T>::fastlang_copy(const fastlang_name<T>& n) : ptr(const_cast<T*>(n.ptr)) {}");
        self.emit("");
        self.emit("template <typename T>");
        self.emit("fastlang_copy<T>::fastlang_copy(const fastlang_modify<T>& m) : ptr(m.ptr) {}");
        self.emit("");
        self.emit("template <typename T>");
        self.emit("fastlang_copy<T>& fastlang_copy<T>::operator=(const fastlang_name<T>& n) { ptr = const_cast<T*>(n.ptr); return *this; }");
        self.emit("");
        self.emit("template <typename T>");
        self.emit("fastlang_copy<T>& fastlang_copy<T>::operator=(const fastlang_modify<T>& m) { ptr = m.ptr; return *this; }");
        self.emit("");
        self.emit("namespace fastlang_detail {");
        self.emit("    template <typename T, typename U>");
        self.emit("    auto arrow_assign_impl(T& target, const U& val, int) -> decltype(target.arrow_assign(val), void()) {");
        self.emit("        target.arrow_assign(val);");
        self.emit("    }");
        self.emit("    template <typename T, typename U>");
        self.emit("    auto arrow_assign_impl(T& target, std::initializer_list<U> val, int) -> decltype(target.arrow_assign(fastlang_slice<U>(val)), void()) {");
        self.emit("        target.arrow_assign(fastlang_slice<U>(val));");
        self.emit("    }");
        self.emit("    template <typename T, typename U>");
        self.emit("    void arrow_assign_impl(T& target, const U& val, ...) {");
        self.emit("        target = val;");
        self.emit("    }");
        self.emit("    template <typename T, typename U>");
        self.emit("    auto arrow_impl(T& target, const U& val, int) -> decltype(target.arrow(val), void()) {");
        self.emit("        target.arrow(val);");
        self.emit("    }");
        self.emit("    template <typename T, typename U>");
        self.emit("    auto arrow_impl(T& target, std::initializer_list<U> val, int) -> decltype(target.arrow(fastlang_slice<U>(val)), void()) {");
        self.emit("        target.arrow(fastlang_slice<U>(val));");
        self.emit("    }");
        self.emit("    template <typename T, typename U>");
        self.emit("    void arrow_impl(T& target, const U& val, ...) {");
        self.emit("        target = val;");
        self.emit("    }");
        self.emit("}");
        self.emit("template <typename T, typename U>");
        self.emit("void fastlang_arrow_assign(T& target, const U& val) {");
        self.emit("    fastlang_detail::arrow_assign_impl(target, val, 0);");
        self.emit("}");
        self.emit("template <typename T, typename U>");
        self.emit("void fastlang_arrow_assign(T& target, std::initializer_list<U> val) {");
        self.emit("    fastlang_detail::arrow_assign_impl(target, val, 0);");
        self.emit("}");
        self.emit("template <typename T, typename U>");
        self.emit("void fastlang_arrow(T& target, const U& val) {");
        self.emit("    fastlang_detail::arrow_impl(target, val, 0);");
        self.emit("}");
        self.emit("template <typename T, typename U>");
        self.emit("void fastlang_arrow(T& target, std::initializer_list<U> val) {");
        self.emit("    fastlang_detail::arrow_impl(target, val, 0);");
        self.emit("}");
        self.emit("");
    }

    pub fn generate(&mut self, ast: &Vec<Stmt>, emit_headers: bool, wrap_in_main: bool) -> String {
        if emit_headers {
            self.emit_headers();
        }

        // Pre-pass for Blueprints and Impls
        let mut blueprints = std::collections::HashMap::new();
        let mut impls: std::collections::HashMap<String, Vec<Decl>> = std::collections::HashMap::new();
        let mut blueprint_handles: std::collections::HashMap<String, Vec<Decl>> = std::collections::HashMap::new();

        for stmt in ast {
            if let Stmt::Declaration(Decl::BlueprintDecl { name, definition, .. }) = stmt {
                blueprints.insert(name.clone(), definition.clone());
            } else if let Stmt::Declaration(Decl::ImplDecl { target, is_handle_impl, methods, handle_block }) = stmt {
                if *is_handle_impl {
                    blueprint_handles.entry(target.clone()).or_insert_with(Vec::new).extend(handle_block.clone());
                } else {
                    impls.entry(target.clone()).or_insert_with(Vec::new).extend(methods.clone());
                    blueprint_handles.entry(target.clone()).or_insert_with(Vec::new).extend(handle_block.clone());
                }
            }
        }

        for (name, definition) in blueprints {
            match definition {
                BlueprintDef::Explicit(fields) => {
                    self.emit(&format!("struct {} {{", name));
                    self.indent_level += 1;

                    for field in fields {
                        let type_str = crate::backend::cpp::stmt::type_to_cpp(&field.type_node);
                        self.emit(&format!("{} {};", type_str, field.name));
                    }

                    if let Some(methods) = impls.get(&name) {
                        for m in methods {
                            self.visit_declaration(m);
                        }
                    }

                    if let Some(handles) = blueprint_handles.get(&name) {
                        for h in handles {
                            self.visit_declaration(h);
                            if let Decl::FnDecl { name: fn_name, params, return_type, .. } = h {
                                let op = match fn_name.as_str() {
                                    "add" => Some("+"),
                                    "sub" => Some("-"),
                                    "mul" => Some("*"),
                                    "div" => Some("/"),
                                    "mod" => Some("%"),
                                    "equal" => Some("=="),
                                    "not_equal" => Some("!="),
                                    "less_than" => Some("<"),
                                    "greater_than" => Some(">"),
                                    "less_than_equal" => Some("<="),
                                    "greater_than_equal" => Some(">="),
                                    "arrow" | "arrow_assign" => Some("="),
                                    _ => None,
                                };
                                let ret_str = crate::backend::cpp::stmt::type_to_cpp(return_type);
                                if let Some(o) = op {
                                    if params.len() == 1 {
                                        let param_type = crate::backend::cpp::stmt::type_to_cpp(&params[0].type_node);
                                        let param_name = &params[0].name;
                                        self.emit(&format!("{} operator{}({} {}) {{", ret_str, o, param_type, param_name));
                                        self.indent_level += 1;
                                        self.emit(&format!("return this->{}({});", fn_name, param_name));
                                        self.indent_level -= 1;
                                        self.emit("}");
                                    }
                                }
                            }
                        }
                    }

                    let has_display = if let Some(handles) = blueprint_handles.get(&name) {
                        handles.iter().any(|h| {
                            if let Decl::FnDecl { name: fn_name, .. } = h {
                                fn_name == "display"
                            } else {
                                false
                            }
                        })
                    } else {
                        false
                    };

                    if has_display {
                        self.emit(&format!("friend std::ostream& operator<<(std::ostream& os, const {}& obj) {{", name));
                        self.emit(&format!("    os << const_cast<{}&>(obj).display();", name));
                        self.emit("    return os;");
                        self.emit("}");
                    } else {
                        self.emit(&format!("friend std::ostream& operator<<(std::ostream& os, const {}& obj) {{", name));
                        self.emit(&format!("    os << \"[blueprint {}]\";", name));
                        self.emit("    return os;");
                        self.emit("}");
                    }

                    self.indent_level -= 1;
                    self.emit("};");
                }
                _ => {
                    self.emit(&format!("// Unsupported BlueprintDef for {}", name));
                }
            }
        }

        // Generate top-level non-main functions, classes, structs, globals first
        for stmt in ast {
            match &stmt {
                | Stmt::Declaration(Decl::ClassDecl { .. })
                | Stmt::Declaration(Decl::StructDecl { .. })
                | Stmt::Declaration(Decl::ArrayDecl { .. })
                | Stmt::Declaration(Decl::CustomDecl { .. })
                | Stmt::Declaration(Decl::MachineDecl { .. })
                | Stmt::Declaration(Decl::EnumDecl { .. })
                | Stmt::Declaration(Decl::FnDecl { .. })
                | Stmt::Declaration(Decl::MicroDecl { .. })
                | Stmt::Declaration(Decl::BlockDecl { .. })
                | Stmt::Declaration(Decl::DestructureDecl { .. })
                | Stmt::Declaration(Decl::VarDecl { .. }) => {
                    self.visit_statement(stmt);
                }
                Stmt::Declaration(Decl::Import { module_path, imports, abi }) => {
                    if abi.is_some() {
                        self.visit_statement(stmt);
                    } else if let Some(first) = module_path.first() {
                        if first.ends_with(".h") || first.ends_with(".hpp") {
                            self.visit_statement(stmt);
                        } else {
                            let cpp_namespace = if first == "std" {
                                "fast_std".to_string()
                            } else {
                                module_path.join("_")
                            };
                            if let Some(selected) = imports {
                                for sym in selected {
                                    self.emit(&format!("using {}::{};", cpp_namespace, sym));
                                }
                            } else {
                                self.emit(&format!("using namespace {};", cpp_namespace));
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        self.emit("");

        if wrap_in_main {
            // Find a ScopeDecl named 'main' or fall back to wrapping everything in main
            let has_main_scope = ast.iter().any(|s| {
                if let Stmt::Declaration(Decl::FnDecl { name, .. }) = s {
                    name == "main"
                } else {
                    false
                }
            });

            if !has_main_scope {
                // Wrap all non-class/struct/var/scope statements in int main()
                self.emit("int main() {");
                self.indent_level += 1;
                for stmt in ast {
                    match stmt {
                        | Stmt::Declaration(Decl::ClassDecl { .. })
                        | Stmt::Declaration(Decl::StructDecl { .. })
                        | Stmt::Declaration(Decl::ArrayDecl { .. })
                        | Stmt::Declaration(Decl::VarDecl { .. })
                        | Stmt::Declaration(Decl::DestructureDecl { .. })
                        | Stmt::Declaration(Decl::BlockDecl { .. })
                        | Stmt::Declaration(Decl::CustomDecl { .. })
                        | Stmt::Declaration(Decl::MachineDecl { .. })
                        | Stmt::Declaration(Decl::EnumDecl { .. })
                        | Stmt::Declaration(Decl::BlueprintDecl { .. })
                        | Stmt::Declaration(Decl::ImplDecl { .. })
                        | Stmt::Declaration(Decl::FnDecl { .. })
                        | Stmt::Declaration(Decl::Import { .. }) => {}
                        _ => {
                            self.visit_statement(stmt);
                        }
                    }
                }
                self.emit("return 0;");
                self.indent_level -= 1;
                self.emit("}");
            }
        }

        self.output.clone()
    }
}
