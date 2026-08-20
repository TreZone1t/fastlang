use crate::{backend::cpp::stmt, frontend::parser::ast::*};

pub struct CodeGenerator {
    pub(crate) output: String,
    pub(crate) indent_level: usize,
    pub(crate) custom_scopes: std::collections::HashSet<String>,
    pub(crate) yield_counter: usize,
}

impl CodeGenerator {
    pub fn new() -> Self {
        CodeGenerator {
            output: String::new(),
            indent_level: 0,
            custom_scopes: std::collections::HashSet::new(),
            yield_counter: 0,
        }
    }

    pub(crate) fn emit_operator_overloads(&mut self, handle_block: &Option<Vec<Decl>>) {
        if let Some(handles) = handle_block {
            for h in handles {
                if let Decl::FnDecl {
                    name,
                    params,
                    return_type,
                    ..
                } = h
                {
                    let op = match name.as_str() {
                        "add" => Some("+"),
                        "sub" => Some("-"),
                        "mul" => Some("*"),
                        "div" => Some("/"),
                        "mod" => Some("%"),
                        _ => None,
                    };

                    let ret_str = crate::backend::cpp::stmt::type_to_cpp(return_type);

                    if let Some(o) = op {
                        if params.len() == 1 {
                            let param_type = crate::backend::cpp::stmt::type_to_cpp(&params[0].type_node);
                            let param_name = &params[0].name;
                            self.emit(&format!(
                                "{} operator{}({} {}) {{",
                                ret_str, o, param_type, param_name
                            ));
                            self.indent_level += 1;
                            self.emit(&format!("return this->{}({});", name, param_name));
                            self.indent_level -= 1;
                            self.emit("}");
                        }
                    } else if name == "index_access" {
                        if params.len() == 1 {
                            let param_type = crate::backend::cpp::stmt::type_to_cpp(&params[0].type_node);
                            let param_name = &params[0].name;
                            self.emit(&format!(
                                "{} operator[]({} {}) {{",
                                ret_str, param_type, param_name
                            ));
                            self.indent_level += 1;
                            self.emit(&format!("return this->index_access({});", param_name));
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

    pub fn generate(&mut self, ast: &Vec<Stmt>, emit_headers: bool, wrap_in_main: bool) -> String {
        if emit_headers {
            self.output.push_str("#include <iostream>\n");
            self.output.push_str("#include <vector>\n");
            self.output.push_str("#include <array>\n");
            self.output.push_str("#include <memory>\n");
            self.output.push_str("#include <optional>\n");
            self.output.push_str("#include <stdexcept>\n\n");
            self.output.push_str("#include <cstdint>\n");
            self.emit("using namespace std;");
            self.emit("");
            self.emit("std::ostream& operator<<(std::ostream& os, const std::exception& e) {");
            self.emit("    return os << e.what();");
            self.emit("}");
            self.emit("");
            self.emit("template <typename T>");
            self.emit("inline auto __fastlang_ptr(T&& val) {");
            self.emit("using Decayed = std::decay_t<T>;");
            self.emit("if constexpr (std::is_pointer_v<Decayed> || std::is_array_v<std::remove_reference_t<T>>) {");
            self.emit("    return val;");
            self.emit("} else {");
            self.emit("    return &val;");
            self.emit("}");
            self.emit("}");
            self.emit("");
        }

        // Pre-pass for Blueprints and Impls
        let mut blueprints = std::collections::HashMap::new();
        let mut impls = std::collections::HashMap::new();

        for stmt in ast {
            if let Stmt::Declaration(Decl::BlueprintDecl {
                name, definition, ..
            }) = stmt
            {
                blueprints.insert(name.clone(), definition.clone());
            } else if let Stmt::Declaration(Decl::ImplDecl { target, methods }) = stmt {
                impls
                    .entry(target.clone())
                    .or_insert_with(Vec::new)
                    .extend(methods.clone());
            }
        }

        for (name, definition) in blueprints {
            match definition {
                BlueprintDef::Explicit(fields) => {
                    self.emit(&format!("struct {} {{", name));
                    self.indent_level += 1;

                    for field in fields {
                        let type_str = match &field.type_node {
                            t => match t {
                                BaseType::Int8 => "int8_t".to_string(),
                                BaseType::Int16 => "int16_t".to_string(),
                                BaseType::Int32 => "int32_t".to_string(),
                                BaseType::Int64 => "int64_t".to_string(),
                                BaseType::Float32 => "float".to_string(),
                                BaseType::Float64 => "double".to_string(),
                                BaseType::Char => "char".to_string(),
                                BaseType::Bool => "bool".to_string(),
                                BaseType::Array { base_type, .. } => base_type.as_str(),
                                BaseType::Pointer(p) => format!("{}*", p.as_str()),
                                _ => "auto".to_string(),
                            },
                            _ => "auto".to_string(),
                        };
                        self.emit(&format!("{} {};", type_str, field.name));
                    }

                    if let Some(methods) = impls.get(&name) {
                        for m in methods {
                            // Methods are emitted directly inside the struct body
                            self.visit_declaration(m);
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

        // Generate top-level non-main functions, classes, structs, globals first
        for stmt in ast {
            match &stmt {
                Stmt::Declaration(Decl::ClassDecl { .. })
                | Stmt::Declaration(Decl::StructDecl { .. })
                | Stmt::Declaration(Decl::ArrayDecl { .. })
                | Stmt::Declaration(Decl::CustomDecl { .. })
                | Stmt::Declaration(Decl::EnumDecl { .. })
                | Stmt::Declaration(Decl::FnDecl { .. })
                | Stmt::Declaration(Decl::VarDecl { .. }) => {
                    self.visit_statement(stmt);
                }
                Stmt::Declaration(Decl::Import {
                    module_path,
                    imports,
                }) => {
                    let cpp_namespace = module_path.join("_");
                    if let Some(selected) = imports {
                        for sym in selected {
                            self.emit(&format!("using {}::{};", cpp_namespace, sym));
                        }
                    } else {
                        self.emit(&format!("using namespace {};", cpp_namespace));
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
                        Stmt::Declaration(Decl::ClassDecl { .. })
                        | Stmt::Declaration(Decl::StructDecl { .. })
                        | Stmt::Declaration(Decl::ArrayDecl { .. })
                        | Stmt::Declaration(Decl::VarDecl { .. })
                        | Stmt::Declaration(Decl::BlockDecl { .. })
                        | Stmt::Declaration(Decl::CustomDecl { .. })
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
