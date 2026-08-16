use crate::parser::ast::*;

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

    pub(crate) fn emit_operator_overloads(&mut self, handle_block: &Option<Vec<Stmt>>) {
        if let Some(handles) = handle_block {
            for h in handles {
                if let Stmt::FnDecl {
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

                    let ret_str = match return_type {
                        TypeNode::Simple(r) => self.map_type(Some(r.base_type.as_str().as_str()), r.size),
                        TypeNode::Generic(g) => self.map_type(Some(g.base_type.as_str().as_str()), None),
                    };

                    if let Some(o) = op {
                        if params.len() == 1 {
                            let param_type = match &params[0].type_node {
                                Some(TypeNode::Simple(r)) => {
                                    self.map_type(Some(r.base_type.as_str().as_str()), r.size)
                                }
                                Some(TypeNode::Generic(g)) => {
                                    self.map_type(Some(g.base_type.as_str().as_str()), None)
                                }
                                None => "auto".to_string(),
                            };
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
                            let param_type = match &params[0].type_node {
                                Some(TypeNode::Simple(r)) => {
                                    self.map_type(Some(r.base_type.as_str().as_str()), r.size)
                                }
                                Some(TypeNode::Generic(g)) => {
                                    self.map_type(Some(g.base_type.as_str().as_str()), None)
                                }
                                None => "auto".to_string(),
                            };
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
        }

        // Pre-pass for Blueprints and Impls
        let mut blueprints = std::collections::HashMap::new();
        let mut impls = std::collections::HashMap::new();

        for stmt in ast {
            if let Stmt::BlueprintDecl {
                name, definition, ..
            } = stmt
            {
                blueprints.insert(name.clone(), definition.clone());
            } else if let Stmt::ImplDecl { target, methods } = stmt {
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
                            TypeNode::Simple(r) => self.map_type(Some(r.base_type.as_str().as_str()), r.size),
                            _ => "auto".to_string(),
                        };
                        self.emit(&format!("{} {};", type_str, field.name));
                    }

                    if let Some(methods) = impls.get(&name) {
                        for m in methods {
                            // Methods are emitted directly inside the struct body
                            self.visit_statement(m);
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
            match stmt {
                Stmt::ClassDecl { .. }
                | Stmt::StructDecl { .. }
                | Stmt::ArrayDecl { .. }
                | Stmt::CustomDecl { .. }
                | Stmt::EnumDecl { .. }
                | Stmt::FnDecl { .. }
                | Stmt::VarDecl { .. } => {
                    self.visit_statement(stmt);
                }
                Stmt::Import {
                    module_path,
                    imports,
                } => {
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
                if let Stmt::FnDecl { name, .. } = s {
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
                        Stmt::ClassDecl { .. }
                        | Stmt::StructDecl { .. }
                        | Stmt::ArrayDecl { .. }
                        | Stmt::VarDecl { .. }
                        | Stmt::BlockDecl { .. }
                        | Stmt::CustomDecl { .. }
                        | Stmt::EnumDecl { .. }
                        | Stmt::BlueprintDecl { .. }
                        | Stmt::ImplDecl { .. }
                        | Stmt::FnDecl { .. }
                        | Stmt::Import { .. } => {}
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

    pub(crate) fn map_type(&self, base_type: Option<&str>, size: Option<i64>) -> String {
        let base = base_type.unwrap_or_default();

        let (type_name, type_size) = if let Some((name, rest)) = base.split_once('(') {
            // But wait, if base is `list<int(32)>`, then split_once('(') would split at `int(32)`
            // We should only split if there are no angle brackets before the parenthesis, or handle it carefully.
            // Let's just look for the LAST '(' that is not inside '<>'
            if !base.contains('<') {
                let inner = rest.strip_suffix(')').unwrap_or(rest);
                (name.trim(), inner.parse::<i64>().ok())
            } else {
                (base, None)
            }
        } else {
            (base, None)
        };
        let resolved_size = type_size.or(size);

        if let Some(inner) = type_name
            .strip_prefix("array<")
            .and_then(|s| s.strip_suffix(">"))
        {
            let mapped_inner = self.map_type(Some(inner), None);
            return match resolved_size {
                Some(len) if len >= 0 => format!("std::array<{}, {}>", mapped_inner, len),
                _ => format!("std::vector<{}>", mapped_inner),
            };
        }

        if type_name.starts_with("name<") {
            let inner = type_name.trim_start_matches("name<").trim_end_matches(">");
            let mapped_inner = self.map_type(Some(inner), None);
            return format!("std::unique_ptr<{}>", mapped_inner);
        }

        if type_name.starts_with("custom<") {
            let inner = type_name.trim_start_matches("custom<").trim_end_matches(">");
            return self.map_type(Some(inner), None);
        }

        if type_name.starts_with("object<") {
            let inner = type_name.trim_start_matches("object<").trim_end_matches(">");
            return self.map_type(Some(inner), None);
        }

        self.map_type_spec(type_name, resolved_size)
    }

    pub(crate) fn map_type_spec(&self, base_type: &str, size: Option<i64>) -> String {
        let trimmed = base_type.trim();
        let (type_name, type_size) = if let Some((name, rest)) = trimmed.split_once('(') {
            let inner = rest.strip_suffix(')').unwrap_or(rest);
            (name.trim(), inner.parse::<i64>().ok())
        } else {
            (trimmed, None)
        };

        let resolved_size = type_size.or(size);

        match type_name {
            "int" => match resolved_size {
                Some(8) => "int8_t".to_string(),
                Some(16) => "int16_t".to_string(),
                Some(32) => "int32_t".to_string(),
                Some(64) => "int64_t".to_string(),
                _ => "int".to_string(),
            },
            "float" => match resolved_size {
                Some(64) => "double".to_string(),
                _ => "float".to_string(),
            },
            "str" => {
                //
                if let Some(len) = resolved_size {
                    format!("std::array<char, {}>", len)
                } else {
                    "std::string".to_string()
                }
            }
            "bool" => "bool".to_string(),
            "name" => "void*".to_string(),
            "blueprint" => "auto".to_string(),
            "length" | "size" => "size_t".to_string(),
            "error" => "std::string".to_string(),
            "array" => {
                if let Some(len) = resolved_size {
                    format!("array<auto, {}>", len) //? error : 'auto' is not allowed hereC/C++(1598)
                } else {
                    "array<auto>".to_string() //todo: error : 'auto' is not allowed hereC/C++(1598)
                                              //? error : too few arguments for class template "std::array"C/C++(442)
                }
            }
            user_type => user_type.to_string(),
        }
    }
}
