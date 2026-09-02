use crate::frontend::parser::ast::*;

pub struct CodeGenerator {
    pub(crate) output: String,
    pub(crate) indent_level: usize,
    pub(crate) custom_scopes: std::collections::HashSet<String>,
    pub(crate) custom_scope_types: std::collections::HashSet<String>,
    pub(crate) enum_types: std::collections::HashSet<String>,
    pub(crate) payload_enum_types: std::collections::HashSet<String>,
    pub(crate) simple_enum_types: std::collections::HashSet<String>,
    pub(crate) pointer_vars: std::collections::HashSet<String>,
    pub(crate) function_handles: std::collections::HashMap<String, Vec<Decl>>,
    pub(crate) struct_field_counts: std::collections::HashMap<String, usize>,
    pub(crate) fn_return_types: std::collections::HashMap<String, String>,
    pub(crate) primitive_impl_methods: std::collections::HashMap<String, String>,
    pub(crate) in_primitive_impl: bool,
    pub(crate) yield_counter: usize,
    pub(crate) in_class_or_scope: bool,
    pub(crate) in_machine: bool,
    pub(crate) current_block_vars: Option<std::collections::HashSet<String>>,
}

impl CodeGenerator {
    pub fn new() -> Self {
        CodeGenerator {
            output: String::new(),
            indent_level: 0,
            custom_scopes: std::collections::HashSet::new(),
            custom_scope_types: std::collections::HashSet::new(),
            enum_types: std::collections::HashSet::new(),
            payload_enum_types: std::collections::HashSet::new(),
            simple_enum_types: std::collections::HashSet::new(),
            pointer_vars: std::collections::HashSet::new(),
            function_handles: std::collections::HashMap::new(),
            struct_field_counts: std::collections::HashMap::new(),
            fn_return_types: std::collections::HashMap::new(),
            primitive_impl_methods: std::collections::HashMap::new(),
            in_primitive_impl: false,
            yield_counter: 0,
            in_class_or_scope: false,
            in_machine: false,
            current_block_vars: None,
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
        self.output.push_str("#include <cstring>\n");
        self.output.push_str("#include <memory>\n");
        self.emit("");
        self.emit("using type = uint32_t;");
        self.emit("");
        self.emit("template <typename T>");
        self.emit("constexpr uint32_t fastlang_typeof_val(const T&) {");
        self.emit("    if constexpr (std::is_same_v<T, bool>) return 1;");
        self.emit("    else if constexpr (std::is_same_v<T, int8_t>) return 2;");
        self.emit("    else if constexpr (std::is_same_v<T, int16_t>) return 3;");
        self.emit("    else if constexpr (std::is_same_v<T, int32_t>) return 4;");
        self.emit("    else if constexpr (std::is_same_v<T, int64_t>) return 5;");
        self.emit("    else if constexpr (std::is_same_v<T, uint8_t>) return 7;");
        self.emit("    else if constexpr (std::is_same_v<T, uint16_t>) return 8;");
        self.emit("    else if constexpr (std::is_same_v<T, uint32_t>) return 9;");
        self.emit("    else if constexpr (std::is_same_v<T, uint64_t>) return 10;");
        self.emit("    else if constexpr (std::is_same_v<T, float>) return 12;");
        self.emit("    else if constexpr (std::is_same_v<T, double>) return 13;");
        self.emit("    else if constexpr (std::is_same_v<T, char>) return 15;");
        self.emit("    else if constexpr (std::is_same_v<T, std::string>) return 19;");
        self.emit("    else return 100;");
        self.emit("}");
        self.emit("#define typeof fastlang_typeof_val");
        self.emit("#define sizeof(x) ((int32_t)sizeof(x))");
        self.emit("");
        self.emit("std::ostream& operator<<(std::ostream& os, const std::exception& e) {");
        self.emit("    return os << e.what();");
        self.emit("}");
        self.emit("");
        self.emit("template <typename T> class fastlang_name;");
        self.emit("template <typename T> class fastlang_modify;");
        self.emit("template <typename T> class fastlang_slice;");
        self.emit("");
        self.emit("template <typename T>");
        self.emit("class fastlang_slice {");
        self.emit("public:");
        self.emit("    T* _data = nullptr;");
        self.emit("    size_t _size = 0;");
        self.emit("    fastlang_slice() : _data(nullptr), _size(0) {}");
        self.emit("    fastlang_slice(const T* data) : _data(const_cast<T*>(data)), _size(data ? (std::is_same<T, char>::value ? strlen((const char*)data) : 0) : 0) {}");
        self.emit("    fastlang_slice(const T* data, size_t size) : _data(const_cast<T*>(data)), _size(size) {}");
        self.emit("    template <size_t N>");
        self.emit("    fastlang_slice(const T (&arr)[N]) : _data(const_cast<T*>(arr)), _size((N > 0 && std::is_same<T, char>::value && arr[N - 1] == '\\0') ? N - 1 : N) {}");
        self.emit("    fastlang_slice(std::initializer_list<T> list) : _data(const_cast<T*>(list.begin())), _size(list.size()) {}");
        self.emit("    T& operator[](size_t idx) { return _data[idx]; }");
        self.emit("    const T& operator[](size_t idx) const { return _data[idx]; }");
        self.emit("    size_t size() const { return _size; }");
        self.emit("    T* data() { return _data; }");
        self.emit("    const T* data() const { return _data; }");
        self.emit("    T* begin() { return _data; }");
        self.emit("    const T* begin() const { return _data; }");
        self.emit("    T* end() { return _data + _size; }");
        self.emit("    const T* end() const { return _data + _size; }");
        self.emit("    operator T*() { return _data; }");
        self.emit("    operator const T*() const { return _data; }");
        self.emit("};");
        self.emit("");
        self.emit("struct fastlang_unit_t {");
        self.emit("    bool operator==(const fastlang_unit_t&) const { return true; }");
        self.emit("    bool operator!=(const fastlang_unit_t&) const { return false; }");
        self.emit("};");
        self.emit("inline std::ostream& operator<<(std::ostream& os, const fastlang_unit_t&) {");
        self.emit("    os << \"()\";");
        self.emit("    return os;");
        self.emit("}");
        self.emit("");
        self.emit("struct fastlang_undefined_t {");
        self.emit("    template <typename T>");
        self.emit("    operator T() const { return T{}; }");
        self.emit("    bool operator==(const fastlang_undefined_t&) const { return true; }");
        self.emit("    bool operator!=(const fastlang_undefined_t&) const { return false; }");
        self.emit("};");
        self.emit("inline std::ostream& operator<<(std::ostream& os, const fastlang_undefined_t&) {");
        self.emit("    os << \"undefined\";");
        self.emit("    return os;");
        self.emit("}");
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
        self.emit("class fastlang_str {");
        self.emit("public:");
        self.emit("    const char* _data = \"\";");
        self.emit("    size_t _size = 0;");
        self.emit("    fastlang_str() : _data(\"\"), _size(0) {}");
        self.emit("    fastlang_str(const char* s) : _data(s ? s : \"\"), _size(s ? strlen(s) : 0) {}");
        self.emit("    fastlang_str(const char* data, size_t size) : _data(data ? data : \"\"), _size(size) {}");
        self.emit("    template <size_t N>");
        self.emit("    fastlang_str(const char (&arr)[N]) : _data(arr), _size((N > 0 && arr[N - 1] == '\\0') ? N - 1 : N) {}");
        self.emit("    fastlang_str(const fastlang_slice<char>& s) : _data(s._data ? s._data : \"\"), _size(s._size) {}");
        self.emit("");
        self.emit("    operator fastlang_slice<char>() const { return fastlang_slice<char>(const_cast<char*>(_data), _size); }");
        self.emit("    char operator[](size_t idx) const { return _data[idx]; }");
        self.emit("    size_t size() const { return _size; }");
        self.emit("    size_t len() const { return _size; }");
        self.emit("    bool is_empty() const { return _size == 0; }");
        self.emit("    const char* data() const { return _data; }");
        self.emit("    const char* c_str() const { return _data; }");
        self.emit("    const char* begin() const { return _data; }");
        self.emit("    const char* end() const { return _data + _size; }");
        self.emit("    operator const char*() const { return _data; }");
        self.emit("};");
        self.emit("");
        self.emit("inline std::ostream& operator<<(std::ostream& os, const fastlang_str& s) {");
        self.emit("    if (s._data && s._size > 0) os.write(s._data, s._size);");
        self.emit("    return os;");
        self.emit("}");
        self.emit("inline bool operator==(const fastlang_str& a, const fastlang_str& b) {");
        self.emit("    if (a._size != b._size) return false;");
        self.emit("    if (a._size == 0) return true;");
        self.emit("    return memcmp(a._data, b._data, a._size) == 0;");
        self.emit("}");
        self.emit("inline bool operator!=(const fastlang_str& a, const fastlang_str& b) { return !(a == b); }");
        self.emit("");
        self.emit("inline fastlang_str operator+(const fastlang_str& a, const fastlang_str& b) {");
        self.emit("    size_t total = a._size + b._size;");
        self.emit("    char* buf = new char[total + 1];");
        self.emit("    if (a._size > 0) memcpy(buf, a._data, a._size);");
        self.emit("    if (b._size > 0) memcpy(buf + a._size, b._data, b._size);");
        self.emit("    buf[total] = '\\0';");
        self.emit("    return fastlang_str(buf, total);");
        self.emit("}");
        self.emit("inline fastlang_str operator+(const fastlang_str& a, char c) {");
        self.emit("    size_t total = a._size + 1;");
        self.emit("    char* buf = new char[total + 1];");
        self.emit("    if (a._size > 0) memcpy(buf, a._data, a._size);");
        self.emit("    buf[a._size] = c;");
        self.emit("    buf[total] = '\\0';");
        self.emit("    return fastlang_str(buf, total);");
        self.emit("}");
        self.emit("inline fastlang_str operator+(char c, const fastlang_str& b) {");
        self.emit("    size_t total = 1 + b._size;");
        self.emit("    char* buf = new char[total + 1];");
        self.emit("    buf[0] = c;");
        self.emit("    if (b._size > 0) memcpy(buf + 1, b._data, b._size);");
        self.emit("    buf[total] = '\\0';");
        self.emit("    return fastlang_str(buf, total);");
        self.emit("}");
        self.emit("");
        self.emit("inline bool operator==(const fastlang_slice<char>& a, const fastlang_slice<char>& b) {");
        self.emit("    if (a._size != b._size) return false;");
        self.emit("    if (a._size == 0) return true;");
        self.emit("    return memcmp(a._data, b._data, a._size) == 0;");
        self.emit("}");
        self.emit("inline bool operator!=(const fastlang_slice<char>& a, const fastlang_slice<char>& b) { return !(a == b); }");
        self.emit("");
        self.emit("inline fastlang_slice<char> operator+(const fastlang_slice<char>& a, const fastlang_slice<char>& b) {");
        self.emit("    size_t total = a._size + b._size;");
        self.emit("    char* buf = new char[total + 1];");
        self.emit("    if (a._size > 0) memcpy(buf, a._data, a._size);");
        self.emit("    if (b._size > 0) memcpy(buf + a._size, b._data, b._size);");
        self.emit("    buf[total] = '\\0';");
        self.emit("    return fastlang_slice<char>(buf, total);");
        self.emit("}");
        self.emit("inline fastlang_slice<char> operator+(const fastlang_slice<char>& a, char c) {");
        self.emit("    size_t total = a._size + 1;");
        self.emit("    char* buf = new char[total + 1];");
        self.emit("    if (a._size > 0) memcpy(buf, a._data, a._size);");
        self.emit("    buf[a._size] = c;");
        self.emit("    buf[total] = '\\0';");
        self.emit("    return fastlang_slice<char>(buf, total);");
        self.emit("}");
        self.emit("inline fastlang_slice<char> operator+(char c, const fastlang_slice<char>& b) {");
        self.emit("    size_t total = 1 + b._size;");
        self.emit("    char* buf = new char[total + 1];");
        self.emit("    buf[0] = c;");
        self.emit("    if (b._size > 0) memcpy(buf + 1, b._data, b._size);");
        self.emit("    buf[total] = '\\0';");
        self.emit("    return fastlang_slice<char>(buf, total);");
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
        self.emit("struct fastlang_tag_stop {};");
        self.emit("inline constexpr fastlang_tag_stop stop{};");
        self.emit("");
        self.emit("template <typename T, typename = void>");
        self.emit("struct is_fastlang_stop : std::false_type {};");
        self.emit("template <typename T>");
        self.emit("struct is_fastlang_stop<T, std::void_t<decltype(std::declval<T>().is_stop())>> : std::true_type {};");
        self.emit("");
        self.emit("template <typename T, typename = void>");
        self.emit("struct is_fastlang_modify_type : std::false_type {};");
        self.emit("template <typename T>");
        self.emit("struct is_fastlang_modify_type<fastlang_modify<T>> : std::true_type {};");
        self.emit("");
        self.emit("template <typename T, typename = void>");
        self.emit("struct has_iter_fn : std::false_type {};");
        self.emit("template <typename T>");
        self.emit("struct has_iter_fn<T, std::void_t<decltype(std::declval<T>().iter())>> : std::true_type {};");
        self.emit("");
        self.emit("template <typename T, typename = void>");
        self.emit("struct has_iterator_fn : std::false_type {};");
        self.emit("template <typename T>");
        self.emit("struct has_iterator_fn<T, std::void_t<decltype(std::declval<T>().iterator())>> : std::true_type {};");
        self.emit("");
        self.emit("template <typename T, typename = void>");
        self.emit("struct has_fastlang_iter : std::false_type {};");
        self.emit("template <typename T>");
        self.emit("struct has_fastlang_iter<T, std::void_t<decltype(std::declval<T>().next())>> : std::integral_constant<bool, has_iter_fn<T>::value || has_iterator_fn<T>::value> {};");
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
        self.emit("template <typename T>");
        self.emit("struct is_fastlang_variant : std::false_type {};");
        self.emit("template <typename... Types>");
        self.emit("struct is_fastlang_variant<std::variant<Types...>> : std::true_type {};");
        self.emit("");
        self.emit("template <typename Iterable>");
        self.emit("struct fastlang_iter_wrapper {");
        self.emit("    Iterable* _target;");
        self.emit("    bool _done;");
        self.emit("    using RawValType = std::decay_t<decltype(std::declval<Iterable>().next())>;");
        self.emit("    RawValType _current_val;");
        self.emit("    fastlang_iter_wrapper(Iterable* t, bool is_end) : _target(t), _done(is_end), _current_val{} {");
        self.emit("        if (!_done && _target) {");
        self.emit("            if constexpr (has_iter_fn<Iterable>::value) {");
        self.emit("                _target->iter();");
        self.emit("            } else if constexpr (has_iterator_fn<Iterable>::value) {");
        self.emit("                _target->iterator();");
        self.emit("            }");
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
        self.emit("        } else if constexpr (is_fastlang_stop<RawValType>::value) {");
        self.emit("            if (_current_val.is_stop()) {");
        self.emit("                _done = true;");
        self.emit("                return;");
        self.emit("            }");
        self.emit("        } else if constexpr (is_fastlang_modify_type<RawValType>::value) {");
        self.emit("            if (_current_val.ptr == nullptr) {");
        self.emit("                _done = true;");
        self.emit("                return;");
        self.emit("            }");
        self.emit("        } else if constexpr (std::is_pointer_v<RawValType>) {");
        self.emit("            if (_current_val == nullptr) {");
        self.emit("                _done = true;");
        self.emit("                return;");
        self.emit("            }");
        self.emit("        } else if constexpr (is_fastlang_variant<RawValType>::value) {");
        self.emit("            if (_current_val.index() != 0) {");
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
        self.emit("        } else if constexpr (is_fastlang_modify_type<RawValType>::value) {");
        self.emit("            return *_current_val;");
        self.emit("        } else if constexpr (is_fastlang_variant<RawValType>::value) {");
        self.emit("            return std::get<0>(_current_val);");
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
        self.emit("auto fastlang_iterable(T&& obj) {");
        self.emit("    if constexpr (has_fastlang_iter<std::remove_reference_t<T>>::value) {");
        self.emit("        return fastlang_iter_range<std::remove_reference_t<T>>{&obj};");
        self.emit("    } else {");
        self.emit("        return std::forward<T>(obj);");
        self.emit("    }");
        self.emit("}");
        self.emit("template <typename T, size_t N>");
        self.emit("auto fastlang_iterable(T (&arr)[N]) {");
        self.emit("    return fastlang_slice<T>(arr);");
        self.emit("}");
        self.emit("");
        self.emit("template <typename T = void>");
        self.emit("struct fastlang_spread_acc {");
        self.emit("    std::vector<T> items;");
        self.emit("    template <typename U>");
        self.emit("    void push_one(const U& x) { items.push_back(static_cast<T>(x)); }");
        self.emit("    template <typename C>");
        self.emit("    void push_spread(const C& c) { for (auto&& x : c) items.push_back(static_cast<T>(x)); }");
        self.emit("    fastlang_slice<T> to_slice() {");
        self.emit("        if (items.empty()) return fastlang_slice<T>();");
        self.emit("        T* buf = new T[items.size()];");
        self.emit("        for (size_t i = 0; i < items.size(); ++i) buf[i] = items[i];");
        self.emit("        return fastlang_slice<T>(buf, items.size());");
        self.emit("    }");
        self.emit("};");
        self.emit("");
        self.emit("template<typename T, typename = void>");
        self.emit("struct has_custom_copy : std::false_type {};");
        self.emit("template<typename T>");
        self.emit("struct has_custom_copy<T, std::void_t<decltype(std::declval<T>().copy())>> : std::true_type {};");
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
        self.emit("template<typename T, typename = void>");
        self.emit("struct has_display : std::false_type {};");
        self.emit("template<typename T>");
        self.emit("struct has_display<T, std::void_t<decltype(std::declval<T>().display())>> : std::true_type {};");
        self.emit("");
        self.emit("template<typename T, typename = void>");
        self.emit("struct is_throwable : std::false_type {};");
        self.emit("template<typename T>");
        self.emit("struct is_throwable<T, std::void_t<decltype(std::declval<T>().error())>> : std::true_type {};");
        self.emit("");
        self.emit("template<typename T, typename = void>");
        self.emit("struct has_is_done : std::false_type {};");
        self.emit("template<typename T>");
        self.emit("struct has_is_done<T, std::void_t<decltype(std::declval<T>().is_done)>> : std::true_type {};");
        self.emit("");
        self.emit("template<typename T, typename = void>");
        self.emit("struct has_has_yielded : std::false_type {};");
        self.emit("template<typename T>");
        self.emit("struct has_has_yielded<T, std::void_t<decltype(std::declval<T>().has_yielded)>> : std::true_type {};");
        self.emit("");
        self.emit("template <typename Signature>");
        self.emit("class fastlang_method;");
        self.emit("");
        self.emit("template <typename Ret, typename... Args>");
        self.emit("class fastlang_method<Ret(Args...)> {");
        self.emit("public:");
        self.emit("    std::function<Ret(Args...)> _fn;");
        self.emit("    std::function<bool()> _is_done_fn;");
        self.emit("    std::function<bool()> _has_yielded_fn;");
        self.emit("    bool is_done = false;");
        self.emit("    bool has_yielded = false;");
        self.emit("");
        self.emit("    fastlang_method() : is_done(false), has_yielded(false) {}");
        self.emit("");
        self.emit("    template <typename F, typename = std::enable_if_t<!std::is_same_v<std::decay_t<F>, fastlang_method> && std::is_invocable_r_v<Ret, F, Args...>>>");
        self.emit("    fastlang_method(F&& f) {");
        self.emit("        using Decayed = std::decay_t<F>;");
        self.emit("        if constexpr (has_is_done<Decayed>::value) {");
        self.emit("            if constexpr (std::is_lvalue_reference_v<F>) {");
        self.emit("                auto ptr = &f;");
        self.emit("                _is_done_fn = [ptr]() { return ptr->is_done; };");
        self.emit("                if constexpr (has_has_yielded<Decayed>::value) {");
        self.emit("                    _has_yielded_fn = [ptr]() { return ptr->has_yielded; };");
        self.emit("                }");
        self.emit("                _fn = [ptr](Args... args) -> Ret {");
        self.emit("                    if constexpr (std::is_void_v<Ret>) {");
        self.emit("                        (*ptr)(args...);");
        self.emit("                    } else {");
        self.emit("                        return (*ptr)(args...);");
        self.emit("                    }");
        self.emit("                };");
        self.emit("            } else {");
        self.emit("                auto ptr = std::make_shared<Decayed>(std::move(f));");
        self.emit("                _is_done_fn = [ptr]() { return ptr->is_done; };");
        self.emit("                if constexpr (has_has_yielded<Decayed>::value) {");
        self.emit("                    _has_yielded_fn = [ptr]() { return ptr->has_yielded; };");
        self.emit("                }");
        self.emit("                _fn = [ptr](Args... args) -> Ret {");
        self.emit("                    if constexpr (std::is_void_v<Ret>) {");
        self.emit("                        (*ptr)(args...);");
        self.emit("                    } else {");
        self.emit("                        return (*ptr)(args...);");
        self.emit("                    }");
        self.emit("                };");
        self.emit("            }");
        self.emit("        } else {");
        self.emit("            _fn = std::forward<F>(f);");
        self.emit("        }");
        self.emit("    }");
        self.emit("");
        self.emit("    fastlang_method(const fastlang_method& other) = default;");
        self.emit("    fastlang_method& operator=(const fastlang_method& other) = default;");
        self.emit("");
        self.emit("    Ret operator()(Args... args) {");
        self.emit("        if constexpr (std::is_void_v<Ret>) {");
        self.emit("            if (_fn) _fn(args...);");
        self.emit("            if (_is_done_fn) is_done = _is_done_fn();");
        self.emit("            else is_done = true;");
        self.emit("            if (_has_yielded_fn) has_yielded = _has_yielded_fn();");
        self.emit("        } else {");
        self.emit("            Ret r = _fn ? _fn(args...) : Ret{};");
        self.emit("            if (_is_done_fn) is_done = _is_done_fn();");
        self.emit("            else is_done = true;");
        self.emit("            if (_has_yielded_fn) has_yielded = _has_yielded_fn();");
        self.emit("            return r;");
        self.emit("        }");
        self.emit("    }");
        self.emit("");
        self.emit("    operator std::function<Ret(Args...)>() const {");
        self.emit("        return _fn;");
        self.emit("    }");
        self.emit("};");
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
        self.emit("        delete ptr;");
        self.emit("    }");
        self.emit("}");
        self.emit("");
        self.emit("template <typename T>");
        self.emit("inline void _fastlang_del(T& obj) {");
        self.emit("    if constexpr (has_drop<T>::value) { obj._fastlang_call_drop(); }");
        self.emit("}");
        self.emit("");
        self.emit("template <typename T>");
        self.emit("inline void _fastlang_del_array(T* ptr) {");
        self.emit("    if (ptr) { delete[] ptr; }");
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
        self.emit("fastlang_name& operator=(const fastlang_name& other) { ptr = other.ptr; return *this; }");
        self.emit("fastlang_name& operator=(const fastlang_modify<T>& m);");
        self.emit("const T& operator*() const { return *ptr; }");
        self.emit("const T* operator->() const { return ptr; }");
        self.emit("T* operator->() { return const_cast<T*>(ptr); }");
        self.emit("operator const T&() const { return *ptr; }");
        self.emit("fastlang_name& operator=(const T* p) { ptr = p; return *this; }");
        self.emit("fastlang_name& operator=(const T& ref) { ptr = &ref; return *this; }");
        self.emit("template <size_t N> fastlang_name& operator=(const T (&arr)[N]) { ptr = arr; return *this; }");
        self.emit("const T* operator+(std::ptrdiff_t offset) const { return ptr + offset; }");
        self.emit("const T* operator-(std::ptrdiff_t offset) const { return ptr - offset; }");
        self.emit("bool operator==(std::nullptr_t) const { return ptr == nullptr; }");
        self.emit("bool operator!=(std::nullptr_t) const { return ptr != nullptr; }");
        self.emit("explicit operator bool() const { return ptr != nullptr; }");
        self.emit("friend std::ostream& operator<<(std::ostream& os, const fastlang_name<T>& obj) { if (obj.ptr) os << *obj.ptr; return os; }");
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
        self.emit("    fastlang_name(F&& fn) {");
        self.emit("        if constexpr (std::is_pointer_v<std::decay_t<F>>) {");
        self.emit("            callable = [p = std::forward<F>(fn)](Args... args) -> Ret {");
        self.emit("                return (*p)(std::forward<Args>(args)...);");
        self.emit("            };");
        self.emit("        } else {");
        self.emit("            callable = std::forward<F>(fn);");
        self.emit("        }");
        self.emit("    }");
        self.emit("");
        self.emit("    template <typename F>");
        self.emit("    fastlang_name& operator=(F&& fn) {");
        self.emit("        if constexpr (std::is_same_v<std::decay_t<F>, fastlang_name>) {");
        self.emit("            callable = fn.callable;");
        self.emit("        } else if constexpr (std::is_pointer_v<std::decay_t<F>>) {");
        self.emit("            callable = [p = std::forward<F>(fn)](Args... args) -> Ret {");
        self.emit("                return (*p)(std::forward<Args>(args)...);");
        self.emit("            };");
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
        self.emit("bool owned = false;");
        self.emit("T* target() const { return ptr; }");
        self.emit("T* get_ptr() const { return ptr; }");
        self.emit("void drop() { ");
        self.emit("    if (owned && ptr) { ");
        self.emit("        if constexpr (has_drop<T>::value) { ptr->drop(); }");
        self.emit("        delete ptr; ");
        self.emit("    }");
        self.emit("    ptr = nullptr;");
        self.emit("    owned = false;");
        self.emit("}");
        self.emit("static T* make_copy(const T& ref) {");
        self.emit("    if constexpr (has_custom_copy<T>::value) {");
        self.emit("        return new T(const_cast<T&>(ref).copy());");
        self.emit("    } else {");
        self.emit("        return new T(ref);");
        self.emit("    }");
        self.emit("}");
        self.emit("T& operator[](size_t index) { return ptr[index]; }");
        self.emit("fastlang_modify(T& ref) : ptr(&ref), owned(false) {}");
        self.emit("fastlang_modify(const T& ref) : ptr(make_copy(ref)), owned(true) {}");
        self.emit("fastlang_modify(T* p = nullptr) : ptr(p), owned(false) {}");
        self.emit("fastlang_modify(const fastlang_modify& other) : ptr(other.owned && other.ptr ? make_copy(*other.ptr) : other.ptr), owned(other.owned) {}");
        self.emit("fastlang_modify(fastlang_tag_stop) : ptr(nullptr), owned(false) {}");
        self.emit("fastlang_modify(const fastlang_name<T>& n) : ptr(const_cast<T*>(n.ptr)), owned(false) {}");
        self.emit("fastlang_modify& operator=(const fastlang_modify& other) { ");
        self.emit("    if (owned && ptr && ptr != other.ptr) { delete ptr; }");
        self.emit("    ptr = other.owned && other.ptr ? make_copy(*other.ptr) : other.ptr; ");
        self.emit("    owned = other.owned; ");
        self.emit("    return *this; ");
        self.emit("}");
        self.emit("fastlang_modify& operator=(const fastlang_name<T>& n) {");
        self.emit("    if (owned && ptr) { delete ptr; }");
        self.emit("    ptr = const_cast<T*>(n.ptr);");
        self.emit("    owned = false;");
        self.emit("    return *this;");
        self.emit("}");
        self.emit("fastlang_modify& operator=(fastlang_tag_stop) {");
        self.emit("    drop();");
        self.emit("    return *this;");
        self.emit("}");
        self.emit("bool is_stop() const { return ptr == nullptr; }");
        self.emit("T& operator*() const { return *ptr; }");
        self.emit("T* operator->() const { return ptr; }");
        self.emit("operator T&() { return *ptr; }");
        self.emit("operator const T&() const { return *ptr; }");
        self.emit("fastlang_modify& operator=(T* p) { if (owned && ptr && ptr != p) { delete ptr; } ptr = p; owned = false; return *this; }");
        self.emit("fastlang_modify& operator=(const T& ref) { ");
        self.emit("    if (owned && ptr) { ");
        self.emit("        *ptr = (has_custom_copy<T>::value ? const_cast<T&>(ref).copy() : ref); ");
        self.emit("    } else { ");
        self.emit("        ptr = make_copy(ref); ");
        self.emit("        owned = true; ");
        self.emit("    } ");
        self.emit("    return *this; ");
        self.emit("}");
        self.emit("template <size_t N> fastlang_modify& operator=(T (&arr)[N]) { ptr = arr; owned = false; return *this; }");
        self.emit("template <size_t N> fastlang_modify& operator=(const T (&arr)[N]) { ptr = const_cast<T*>(arr); owned = false; return *this; }");
        self.emit("template <typename U> fastlang_modify& operator+=(const U& val) { *ptr += val; return *this; }");
        self.emit("template <typename U> fastlang_modify& operator-=(const U& val) { *ptr -= val; return *this; }");
        self.emit("template <typename U> fastlang_modify& operator*=(const U& val) { *ptr *= val; return *this; }");
        self.emit("template <typename U> fastlang_modify& operator/=(const U& val) { *ptr /= val; return *this; }");
        self.emit("T* operator+(std::ptrdiff_t offset) const { return ptr + offset; }");
        self.emit("T* operator-(std::ptrdiff_t offset) const { return ptr - offset; }");
        self.emit("bool operator==(std::nullptr_t) const { return ptr == nullptr; }");
        self.emit("bool operator!=(std::nullptr_t) const { return ptr != nullptr; }");
        self.emit("explicit operator bool() const { return ptr != nullptr; }");
        self.emit("friend std::ostream& operator<<(std::ostream& os, const fastlang_modify<T>& obj) { if (obj.ptr) os << *obj.ptr; return os; }");
        self.emit("};");
        self.emit("template <typename T> fastlang_modify(T&) -> fastlang_modify<T>;");
        self.emit("template <typename T> fastlang_modify(T*) -> fastlang_modify<T>;");
        self.emit("template <typename T> using fastlang_copy = fastlang_modify<T>;");
        self.emit("");
        self.emit("template <typename T>");
        self.emit("inline void _fastlang_del(fastlang_name<T>& obj) {");
        self.emit("    obj.ptr = nullptr;");
        self.emit("}");
        self.emit("template <typename T>");
        self.emit("inline void _fastlang_del(fastlang_modify<T>& obj) {");
        self.emit("    obj.drop();");
        self.emit("}");
        self.emit("");
        self.emit("template <typename T>");
        self.emit("fastlang_name<T>::fastlang_name(const fastlang_modify<T>& m) : ptr(m.ptr) {}");
        self.emit("");
        self.emit("template <typename T>");
        self.emit("fastlang_name<T>& fastlang_name<T>::operator=(const fastlang_modify<T>& m) { ptr = m.ptr; return *this; }");

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
        let mut primitive_impls: Vec<(String, Vec<BaseType>, Vec<Decl>, Vec<Decl>)> = Vec::new();

        for stmt in ast {
            if let Stmt::Declaration(Decl::BlueprintDecl { name, definition, .. }) = stmt {
                blueprints.insert(name.clone(), definition.clone());
                if let BlueprintDef::Explicit(fields) = definition {
                    self.struct_field_counts.insert(name.clone(), fields.len());
                }
            } else if let Stmt::Declaration(Decl::StructDecl { name, public_block, private_block, .. }) = stmt {
                let cnt = public_block.iter().chain(private_block.iter()).filter(|d| matches!(d, Decl::VarDecl { .. })).count();
                self.struct_field_counts.insert(name.clone(), cnt);
            } else if let Stmt::Declaration(Decl::ClassDecl { name, public_block, private_block, .. }) = stmt {
                let cnt = public_block.iter().chain(private_block.iter()).filter(|d| matches!(d, Decl::VarDecl { .. })).count();
                self.struct_field_counts.insert(name.clone(), cnt);
            } else if let Stmt::Declaration(Decl::FnDecl { name, return_type, .. }) = stmt {
                self.fn_return_types.insert(name.clone(), return_type.get_name());
            } else if let Stmt::Declaration(Decl::ImplDecl { target, target_generics, is_handle_impl, methods, handle_block }) = stmt {
                let is_primitive = matches!(target.as_str(), "str" | "char" | "bool" | "flag" | "string" | "array" | "byte" | "usize" | "isize") || target.starts_with("int") || target.starts_with("uint") || target.starts_with("float");
                if is_primitive {
                    for m in methods {
                        if let Decl::FnDecl { name, .. } = m {
                            self.primitive_impl_methods.insert(name.clone(), target.clone());
                        }
                    }
                    for h in handle_block {
                        if let Decl::FnDecl { name, .. } = h {
                            self.primitive_impl_methods.insert(name.clone(), target.clone());
                        }
                    }
                    primitive_impls.push((target.clone(), target_generics.clone(), methods.clone(), handle_block.clone()));
                } else {
                    self.function_handles.entry(target.clone()).or_insert_with(Vec::new).extend(handle_block.clone());
                    if *is_handle_impl {
                        blueprint_handles.entry(target.clone()).or_insert_with(Vec::new).extend(handle_block.clone());
                    } else {
                        impls.entry(target.clone()).or_insert_with(Vec::new).extend(methods.clone());
                        blueprint_handles.entry(target.clone()).or_insert_with(Vec::new).extend(handle_block.clone());
                    }
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

                    if let Some(handles) = blueprint_handles.get(&name) {
                        let has_drop_handle = handles.iter().any(|h| {
                            if let Decl::FnDecl { name: fn_name, .. } = h {
                                fn_name == "drop"
                            } else {
                                false
                            }
                        });
                        if has_drop_handle {
                            self.emit("bool _fastlang_dropped = false;");
                            self.emit("void _fastlang_call_drop() {");
                            self.emit("    if (!_fastlang_dropped) {");
                            self.emit("        _fastlang_dropped = true;");
                            self.emit("        this->drop();");
                            self.emit("    }");
                            self.emit("}");
                            self.emit(&format!("~{}() {{ this->_fastlang_call_drop(); }}", name));
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
                        self.emit(&format!("    const_cast<{}&>(obj).display();", name));
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

        for stmt in ast {
            if let Stmt::Declaration(Decl::MachineDecl { name, .. }) = stmt {
                self.custom_scope_types.insert(name.clone());
            }
        }

        // Generate top-level non-main functions, classes, structs, globals first
        for stmt in ast {
            match &stmt {
                | Stmt::Declaration(Decl::ClassDecl { .. })
                | Stmt::Declaration(Decl::StructDecl { .. })
                | Stmt::Declaration(Decl::ArrayDecl { .. })
                | Stmt::Declaration(Decl::MachineDecl { .. })
                | Stmt::Declaration(Decl::EnumDecl { .. })
                | Stmt::Declaration(Decl::FnDecl { .. })
                | Stmt::Declaration(Decl::ExternFnDecl { .. })
                | Stmt::Declaration(Decl::ExternBlockDecl { .. })
                | Stmt::Declaration(Decl::MicroDecl { .. })
                | Stmt::Declaration(Decl::MacroDecl { .. })
                | Stmt::Declaration(Decl::BlockDecl { .. })
                | Stmt::Declaration(Decl::DefineDecl { .. })
                | Stmt::Declaration(Decl::DestructureDecl { .. })
                | Stmt::Declaration(Decl::VarDecl { .. }) => {
                    self.visit_statement(stmt);
                }
                Stmt::Declaration(Decl::Import { module_path, imports, abi, alias }) => {
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
                            if let Some(alias_name) = alias {
                                self.emit(&format!("namespace {} = {};", alias_name, cpp_namespace));
                            } else if let Some(selected) = imports {
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

        for (target, target_generics, methods, handle_block) in primitive_impls {
            let target_base_type = if target == "array" {
                BaseType::Array {
                    base_type: Box::new(if target_generics.is_empty() {
                        BaseType::Unknown
                    } else {
                        target_generics[0].clone()
                    }),
                    size: Box::new(None),
                }
            } else {
                BaseType::from_str(&target)
            };
            let target_cpp = crate::backend::cpp::stmt::type_to_cpp(&target_base_type);

            let is_generic = !target_generics.is_empty() && target_generics.iter().any(|g| matches!(g, BaseType::GenericParam(_)));

            for m in methods.iter().chain(handle_block.iter()) {
                if let Decl::FnDecl { name, params, return_type, body, .. } = m {
                    let ret_cpp = crate::backend::cpp::stmt::type_to_cpp(return_type);
                    let mut param_cpps = Vec::new();
                    param_cpps.push(format!("{} __this", target_cpp));
                    for p in params {
                        param_cpps.push(format!("{} {}", crate::backend::cpp::stmt::type_to_cpp(&p.type_node), p.name));
                    }

                    if is_generic {
                        let gen_params: Vec<String> = target_generics
                            .iter()
                            .map(|g| format!("typename {}", g.as_str()))
                            .collect();
                        self.emit(&format!("template <{}>", gen_params.join(", ")));
                    }
                    self.emit(&format!("inline {} fastlang_{}_{}({}) {{", ret_cpp, target, name, param_cpps.join(", ")));
                    self.indent_level += 1;
                    let prev_prim = self.in_primitive_impl;
                    self.in_primitive_impl = true;
                    for s in body {
                        self.visit_statement(s);
                    }
                    self.in_primitive_impl = prev_prim;
                    self.indent_level -= 1;
                    self.emit("}");
                    if target == "array" {
                        self.emit(&format!("inline auto fastlang_array_{}(fastlang_str __this) {{ return fastlang_str_{}(__this); }}", name, name));
                    }
                }
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
                        | Stmt::Declaration(Decl::MachineDecl { .. })
                        | Stmt::Declaration(Decl::EnumDecl { .. })
                        | Stmt::Declaration(Decl::BlueprintDecl { .. })
                        | Stmt::Declaration(Decl::ImplDecl { .. })
                        | Stmt::Declaration(Decl::FnDecl { .. })
                        | Stmt::Declaration(Decl::MicroDecl { .. })
                        | Stmt::Declaration(Decl::MacroDecl { .. })
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
