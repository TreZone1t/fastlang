pub use crate::backend::cpp::runtime::generate_runtime_header;
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
    pub(crate) variadic_packs: std::collections::HashSet<String>,
    pub(crate) target_impl_methods: std::collections::HashMap<String, Vec<Decl>>,
    pub(crate) target_handle_methods: std::collections::HashMap<String, Vec<Decl>>,
    pub(crate) c_includes: std::collections::HashSet<String>,
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
            variadic_packs: std::collections::HashSet::new(),
            target_impl_methods: std::collections::HashMap::new(),
            target_handle_methods: std::collections::HashMap::new(),
            c_includes: std::collections::HashSet::new(),
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
                    } else if name == "cast" {
                        if params.is_empty() {
                            self.emit(&format!("explicit operator {}() const {{", ret_str));
                            self.indent_level += 1;
                            self.emit("return const_cast<std::decay_t<decltype(*this)>*>(this)->cast();");
                            self.indent_level -= 1;
                            self.emit("}");
                        }
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
        self.output.push_str("#include <clib/fast_runtime.h>\n\n");
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
                if name != "typeof" && name != "sizeof" && !name.starts_with("@compile::") {
                    self.fn_return_types.insert(name.clone(), return_type.get_name());
                }
            } else if let Stmt::Declaration(Decl::ImplDecl { target, target_generics, is_handle_impl, methods, handle_block }) = stmt {
                if target == "type" {
                    continue;
                }
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
                    self.target_handle_methods.entry(target.clone()).or_insert_with(Vec::new).extend(handle_block.clone());
                    if *is_handle_impl {
                        blueprint_handles.entry(target.clone()).or_insert_with(Vec::new).extend(handle_block.clone());
                    } else {
                        self.target_impl_methods.entry(target.clone()).or_insert_with(Vec::new).extend(methods.clone());
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
                        if let Some(ref def_val) = field.default_value {
                            let def_code = self.visit_expression(def_val);
                            self.emit(&format!("{} {} = {};", type_str, field.name, def_code));
                        } else {
                            self.emit(&format!("{} {};", type_str, field.name));
                        }
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
                            } else if cpp_namespace != "fast_std" {
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
                    let mut extra_gen_params = Vec::new();
                    for (idx, p) in params.iter().enumerate() {
                        if matches!(p.type_node, BaseType::Lambda { .. }) {
                            let func_t = format!("__Func{}", idx);
                            extra_gen_params.push(format!("typename {}", func_t));
                            param_cpps.push(format!("{} {}", func_t, p.name));
                        } else {
                            param_cpps.push(format!("{} {}", crate::backend::cpp::stmt::type_to_cpp(&p.type_node), p.name));
                        }
                    }

                    let mut all_gen_params: Vec<String> = if is_generic {
                        target_generics
                            .iter()
                            .map(|g| format!("typename {}", g.as_str()))
                            .collect()
                    } else {
                        Vec::new()
                    };
                    all_gen_params.extend(extra_gen_params);

                    if !all_gen_params.is_empty() {
                        self.emit(&format!("template <{}>", all_gen_params.join(", ")));
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
                        if !params.is_empty() {
                            self.emit(&format!("template <typename T, typename U, size_t N> inline auto fastlang_array_{}(fastlang_slice<T> __this, U (&other)[N]) {{ return fastlang_array_{}(__this, fastlang_slice<T>(other)); }}", name, name));
                        }
                        self.emit(&format!("template <typename T, size_t N, typename... __Args> inline auto fastlang_array_{}(T (&arr)[N], __Args&&... args) {{ return fastlang_array_{}(fastlang_slice<T>(arr), std::forward<__Args>(args)...); }}", name, name));
                    }
                    if target == "array" && params.is_empty() {
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
