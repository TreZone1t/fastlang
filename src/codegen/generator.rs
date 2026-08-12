use crate::parser::ast::{EitherBlock, Expr, Stmt};

pub struct CodeGenerator {
    pub(crate) output: String,
    pub(crate) indent_level: usize,
    pub(crate) in_class_def: bool,
    pub(crate) in_switch: bool,
    pub(crate) current_switch_type: String,
}

impl CodeGenerator {
    pub fn new() -> Self {
        CodeGenerator {
            output: String::new(),
            indent_level: 0,
            in_switch: false,
            current_switch_type: String::new(),
            in_class_def: false,
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

        // Generate top-level non-main functions, classes, structs, globals first
        for stmt in ast {
            match stmt {
                Stmt::ClassDecl { .. }
                | Stmt::StructDecl { .. }
                | Stmt::ArrayDecl { .. }
                | Stmt::StrDecl { .. }
                | Stmt::CustomDecl { .. }
                | Stmt::EnumDecl { .. }
                | Stmt::FnDecl { .. }
                | Stmt::VarDecl { .. } => {
                    self.visit_statement(stmt);
                }
                Stmt::Use {
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
                        | Stmt::StrDecl { .. }
                        | Stmt::VarDecl { .. }
                        | Stmt::BlockDecl { .. }
                        | Stmt::CustomDecl { .. }
                        | Stmt::EnumDecl { .. }
                        | Stmt::FnDecl { .. }
                        | Stmt::Use { .. } => {}
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

        if type_name.starts_with("array<") {
            let inner = type_name.trim_start_matches("array<").trim_end_matches(">");
            let mapped_inner = self.map_type(Some(inner), None);
            if let Some(len) = resolved_size {
                if len == -1 {
                    return format!("std::vector<{}>", mapped_inner);
                }
                return format!("std::array<{}, {}>", mapped_inner, len);
            }
            return format!("std::vector<{}>", mapped_inner);
        }
        if type_name.starts_with("list<") {
            let inner = type_name.trim_start_matches("list<").trim_end_matches(">");
            let mapped_inner = self.map_type(Some(inner), None);
            return format!("std_list::LinkedList<{}>", mapped_inner);
        }
        if type_name.starts_with("Option<") {
            let inner = type_name
                .trim_start_matches("Option<")
                .trim_end_matches(">");
            let mapped_inner = self.map_type(Some(inner), None);
            return format!("std::optional<{}>", mapped_inner);
        }

        if type_name == "Node" {
            // Hardcode map for linked list Node ptr
            return "std_list::Node<T>*".to_string();
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
            "str" | "string" => {
                if let Some(len) = resolved_size {
                    format!("str<{}>", len)
                } else {
                    "str".to_string()
                }
            }
            "bool" => "bool".to_string(),
            "list" => "auto".to_string(),
            "name" => "void*".to_string(),
            "param" | "blueprint" | "init" => "auto".to_string(),
            "Option" => "std::optional".to_string(),
            "length" | "size" => "size_t".to_string(),
            "array" => {
                if let Some(len) = resolved_size {
                    format!("array<auto, {}>", len)
                } else {
                    "array<auto>".to_string()
                }
            }
            user_type => user_type.to_string(),
        }
    }
}
