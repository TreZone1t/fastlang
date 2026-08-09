use crate::parser::ast::{Expr, LoopBody, Stmt};

pub struct CodeGenerator {
    output: String,
    indent_level: usize,
    in_class_def: bool,
}

impl CodeGenerator {
    pub fn new() -> Self {
        CodeGenerator {
            output: String::new(),
            indent_level: 0,
            in_class_def: false,
        }
    }

    fn emit(&mut self, s: &str) {
        let indent = "    ".repeat(self.indent_level);
        self.output.push_str(&format!("{}{}\n", indent, s));
    }

    fn emit_raw(&mut self, s: &str) {
        self.output.push_str(s);
    }

    pub fn generate(&mut self, ast: &Vec<Stmt>, emit_headers: bool, wrap_in_main: bool) -> String {
        if emit_headers {
            self.emit("#include <iostream>");
            self.emit("#include <string>");
            self.emit("#include <vector>");
            self.emit("#include <memory>");
            self.emit("#include <stdexcept>");
            self.emit("#include <cstdint>");
            self.emit("");
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
                Stmt::ClassDecl { .. } | Stmt::StructDecl { .. } => {
                    self.visit_statement(stmt);
                }
                Stmt::VarDecl { .. } => {
                    self.visit_statement(stmt);
                }
                Stmt::ScopeDecl { .. } => {
                    // All top-level scopes (fn, block, etc.) become C++ functions
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
                if let Stmt::ScopeDecl { name, .. } = s {
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
                        | Stmt::VarDecl { .. }
                        | Stmt::ScopeDecl { .. }
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

    fn visit_statement(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::ExpressionStmt(expr) => {
                let code = self.visit_expression(expr);
                self.emit(&format!("{};", code));
            }
            Stmt::VarDecl {
                name,
                base_type,
                size,
                value,
                is_exported: _,
                ..
            } => {
                let val_code = self.visit_expression(value);

                // If value is __param__ sentinel, it's a declaration without init (struct field / function param)
                let is_param = val_code == "__param__";

                let cpp_type = match base_type.as_deref() {
                    Some("int") => match size {
                        Some(8) => "int8_t".to_string(),
                        Some(16) => "int16_t".to_string(),
                        Some(32) => "int32_t".to_string(),
                        Some(64) => "int64_t".to_string(),
                        _ => "int".to_string(),
                    },
                    Some("float") => match size {
                        Some(64) => "double".to_string(),
                        _ => "float".to_string(),
                    },
                    Some("string") | Some("str") => "std::string".to_string(),
                    Some("bool") => "bool".to_string(),
                    Some("object") => "auto".to_string(),
                    Some("list") => "auto".to_string(),
                    Some("length") => {
                        let sized_val = format!("{}.size()", val_code);
                        self.emit(&format!("size_t {} = {};", name, sized_val));
                        return;
                    }
                    Some("size") => {
                        let sized_val = format!("sizeof({})", val_code);
                        self.emit(&format!("size_t {} = {};", name, sized_val));
                        return;
                    }
                    Some("init") => {
                        let lambda_val =
                            format!("[]() {{ return std::make_shared<{}>(); }}", val_code);
                        self.emit(&format!("auto {} = {};", name, lambda_val));
                        return;
                    }
                    Some("param") => "auto".to_string(),
                    Some("blueprint") => "auto".to_string(),
                    Some("name") => "auto".to_string(),
                    Some(user_type) => user_type.to_string(),
                    None => "auto".to_string(),
                };

                if is_param {
                    // Declaration without initializer (struct field or function param)
                    self.emit(&format!("{} {};", cpp_type, name));
                } else {
                    self.emit(&format!("{} {} = {};", cpp_type, name, val_code));
                }
            }
            Stmt::ReassignStmt { target, value } => {
                let target_code = self.visit_expression(target);
                let val_code = self.visit_expression(value);
                self.emit(&format!("{} = {};", target_code, val_code));
            }
            Stmt::IfStmt {
                condition,
                then_block,
                else_block,
            } => {
                let cond_code = self.visit_expression(condition);
                self.emit(&format!("if ({}) {{", cond_code));
                self.indent_level += 1;
                for s in then_block {
                    self.visit_statement(s);
                }
                self.indent_level -= 1;
                if let Some(eb) = else_block {
                    self.emit("} else {");
                    self.indent_level += 1;
                    for s in eb {
                        self.visit_statement(s);
                    }
                    self.indent_level -= 1;
                }
                self.emit("}");
            }
            Stmt::WhileStmt { condition, body } => {
                let cond_code = self.visit_expression(condition);
                self.emit(&format!("while ({}) {{", cond_code));
                self.indent_level += 1;
                match body {
                    LoopBody::Inline(stmts) => {
                        for s in stmts {
                            self.visit_statement(s);
                        }
                    }
                    LoopBody::ScopeCall(expr) => {
                        let expr_code = self.visit_expression(expr);
                        self.emit(&format!("{};", expr_code));
                    }
                }
                self.indent_level -= 1;
                self.emit("}");
            }
            Stmt::ForStmt {
                init,
                condition,
                increment,
                body,
            } => {
                self.emit("{");
                self.indent_level += 1;
                if let Some(i) = init {
                    self.visit_statement(i);
                }

                let cond_code = if let Some(c) = condition {
                    self.visit_expression(c)
                } else {
                    "true".to_string()
                };
                self.emit(&format!("while ({}) {{", cond_code));
                self.indent_level += 1;
                match body {
                    LoopBody::Inline(stmts) => {
                        for s in stmts {
                            self.visit_statement(s);
                        }
                    }
                    LoopBody::ScopeCall(expr) => {
                        let expr_code = self.visit_expression(expr);
                        self.emit(&format!("{};", expr_code));
                    }
                }
                if let Some(inc) = increment {
                    let inc_code = self.visit_expression(inc);
                    self.emit(&format!("{};", inc_code));
                }
                self.indent_level -= 1;
                self.emit("}");

                self.indent_level -= 1;
                self.emit("}");
            }
            Stmt::ScopeDecl {
                name,
                scope_type: _,
                is_custom,
                params,
                return_type: _,
                flags: _,
                events,
                handles,
                statements,
                public_block,
                fields,
                private_block,
                return_value,
                is_exported: _,
                settings: _,
                constructor: _,
            } => {
                if *is_custom {
                    self.emit(&format!("struct {} {{", name));
                    self.indent_level += 1;
                    
                    // Fields declared with `add`, followed by constructor parameters.
                    for field in fields {
                        if let Stmt::VarDecl { name: field_name, base_type, size, .. } = field {
                            let cpp_t = self.map_type(base_type.as_deref(), *size);
                            self.emit(&format!("{} {};", cpp_t, field_name));
                        }
                    }

                    // Fields from params
                    for p in params {
                        if let Stmt::VarDecl { name: p_name, base_type, size, .. } = p {
                            let cpp_t = self.map_type(base_type.as_deref(), *size);
                            self.emit(&format!("{} {};", cpp_t, p_name));
                        }
                    }
                    
                    // Constructor
                    if !params.is_empty() {
                        let mut param_list = Vec::new();
                        for p in params {
                            if let Stmt::VarDecl { name: p_name, base_type, size, .. } = p {
                                let cpp_t = self.map_type(base_type.as_deref(), *size);
                                param_list.push(format!("{} _{}", cpp_t, p_name));
                            }
                        }
                        self.emit(&format!("{}({}) {{", name, param_list.join(", ")));
                        self.indent_level += 1;
                        for p in params {
                            if let Stmt::VarDecl { name: p_name, .. } = p {
                                self.emit(&format!("this->{} = _{};", p_name, p_name));
                            }
                        }
                        self.indent_level -= 1;
                        self.emit("}");
                    }
                    
                    // Public Block
                    self.in_class_def = true;
                    self.emit("public:");
                    for s in public_block {
                        self.visit_statement(s);
                    }
                    
                    // Events
                    for e in events {
                        self.emit(&format!("void {}() {{", e.trigger_name));
                        self.indent_level += 1;
                        for s in &e.body {
                            self.visit_statement(s);
                        }
                        self.indent_level -= 1;
                        self.emit("}");
                    }
                    
                    // Handles
                    for h in handles {
                        self.emit(&format!("void {}() {{", h.target_flag));
                        self.indent_level += 1;
                        for s in &h.body {
                            self.visit_statement(s);
                        }
                        self.indent_level -= 1;
                        self.emit("}");
                    }
                    
                    self.indent_level -= 1;
                    if !private_block.is_empty() {
                        self.emit("private:");
                        self.indent_level += 1;
                        for s in private_block {
                            self.visit_statement(s);
                        }
                        self.indent_level -= 1;
                    }
                    self.in_class_def = false;
                    self.emit("};");
                    return;
                }
                // Build param list
                let mut param_list = Vec::new();
                for p in params {
                    if let Stmt::VarDecl {
                        name: p_name,
                        base_type,
                        size,
                        ..
                    } = p
                    {
                        let cpp_t = self.map_type(base_type.as_deref(), *size);
                        param_list.push(format!("{} {}", cpp_t, p_name));
                    }
                }

                if self.indent_level == 0 || self.in_class_def {
                    // Top-level scope or class method → proper C++ function
                    let has_return_stmt =
                        statements.iter().any(|s| matches!(s, Stmt::ReturnStmt(_)));
                    let ret_type = if name == "main" {
                        "int"
                    } else if return_value.is_some() || has_return_stmt {
                        "auto"
                    } else {
                        "void"
                    };
                    self.emit(&format!(
                        "{} {}({}) {{",
                        ret_type,
                        name,
                        param_list.join(", ")
                    ));
                    self.indent_level += 1;
                    for s in statements {
                        self.visit_statement(s);
                    }
                    if let Some(rv) = return_value {
                        let rv_code = self.visit_expression(rv);
                        self.emit(&format!("return {};", rv_code));
                    }
                    self.indent_level -= 1;
                    self.emit("}");
                } else {
                    // Nested scope → lambda bound to variable
                    self.emit(&format!(
                        "auto {} = [&]({}) {{",
                        name,
                        param_list.join(", ")
                    ));
                    self.indent_level += 1;
                    for s in statements {
                        self.visit_statement(s);
                    }
                    if let Some(rv) = return_value {
                        let rv_code = self.visit_expression(rv);
                        self.emit(&format!("return {};", rv_code));
                    }
                    self.indent_level -= 1;
                    self.emit("};");
                }
            }
            Stmt::ClassDecl {
                name,
                extends,
                public_block,
                private_block,
                static_block,
                constructor,
                is_exported: _,
            } => {
                let ext_code = if let Some(ext) = extends {
                    format!(": public {}", ext)
                } else {
                    "".to_string()
                };
                self.emit(&format!("class {} {} {{", name, ext_code));
                self.emit("public:");
                self.indent_level += 1;
                for s in public_block {
                    self.visit_statement(s);
                }
                for s in static_block {
                    self.emit("static ");
                    // hacky way to inject static
                    self.visit_statement(s);
                }
                if let Some(c) = constructor {
                    let param_list: Vec<String> = c
                        .params
                        .iter()
                        .map(|p| {
                            let cpp_t = self.map_type(Some(p.base_type.as_str()), p.size);
                            format!("{} {}", cpp_t, p.name)
                        })
                        .collect();
                    self.emit(&format!("{}({}) {{", name, param_list.join(", ")));
                    self.indent_level += 1;
                    for s in &c.body {
                        self.visit_statement(s);
                    }
                    self.indent_level -= 1;
                    self.emit("}");
                }
                self.indent_level -= 1;
                if !private_block.is_empty() {
                    self.emit("private:");
                    self.indent_level += 1;
                    for s in private_block {
                        self.visit_statement(s);
                    }
                    self.indent_level -= 1;
                }
                self.emit("};");
            }
            Stmt::StructDecl {
                name,
                public_block,
                private_block,
                static_block,
                constructor,
                is_exported: _,
            } => {
                self.emit(&format!("struct {} {{", name));
                self.indent_level += 1;
                for s in public_block {
                    self.visit_statement(s);
                }
                for s in static_block {
                    self.visit_statement(s);
                }
                if let Some(c) = constructor {
                    let param_list: Vec<String> = c
                        .params
                        .iter()
                        .map(|p| {
                            let cpp_t = self.map_type(Some(p.base_type.as_str()), p.size);
                            format!("{} {}", cpp_t, p.name)
                        })
                        .collect();
                    self.emit(&format!("{}({}) {{", name, param_list.join(", ")));
                    self.indent_level += 1;
                    for s in &c.body {
                        self.visit_statement(s);
                    }
                    self.indent_level -= 1;
                    self.emit("}");
                }
                self.indent_level -= 1;
                if !private_block.is_empty() {
                    self.emit("private:");
                    self.indent_level += 1;
                    for s in private_block {
                        self.visit_statement(s);
                    }
                    self.indent_level -= 1;
                }
                self.emit("};");
            }
            Stmt::ReturnStmt(expr) => {
                let expr_code = self.visit_expression(expr);
                self.emit(&format!("return {};", expr_code));
            }
            Stmt::BreakStmt => {
                self.emit("break;");
            }
            Stmt::ContinueStmt => {
                self.emit("continue;");
            }
            Stmt::ThrowStmt(expr) => {
                if let Expr::Call { callee, args } = expr {
                    if let Expr::Identifier(name) = &**callee {
                        if name == "error" && args.len() == 1 {
                            let arg_code = self.visit_expression(&args[0]);
                            self.emit(&format!("throw std::runtime_error({});", arg_code));
                            return;
                        }
                    }
                }
                // If throwing an identifier (caught exception variable), rethrow it
                if let Expr::Identifier(_) = expr {
                    self.emit("throw;");
                    return;
                }
                // Fallback
                let expr_code = self.visit_expression(expr);
                self.emit(&format!("throw std::runtime_error({});", expr_code));
            }
            Stmt::TryCatchStmt {
                try_block,
                catch_param,
                catch_block,
            } => {
                self.emit("try {");
                self.indent_level += 1;
                for s in try_block {
                    self.visit_statement(s);
                }
                self.indent_level -= 1;
                self.emit(&format!(
                    "}} catch (const std::exception& {}) {{",
                    catch_param
                ));
                self.indent_level += 1;
                for s in catch_block {
                    self.visit_statement(s);
                }
                self.indent_level -= 1;
                self.emit("}");
            }
            Stmt::EnableStmt(_) | Stmt::DisableStmt(_) => {
                // these are flag metadata nodes (if any leaked into statement body)
                // they have no C++ runtime representation.
            }
            _ => {
                self.emit(&format!("// TODO: unimplemented statement {:?}", stmt));
            }
        }
    }

    fn visit_expression(&mut self, expr: &Expr) -> String {
        match expr {
            Expr::LiteralInt(val) => val.to_string(),
            Expr::LiteralFloat(val) => val.to_string(),
            Expr::LiteralString(val) => format!("\"{}\"", val),
            Expr::LiteralBool(val) => {
                if *val {
                    "true".to_string()
                } else {
                    "false".to_string()
                }
            }
            Expr::ListLiteral(elements) => {
                let mut elems_code = Vec::new();
                for el in elements {
                    elems_code.push(self.visit_expression(el));
                }
                format!("{{{}}}", elems_code.join(", "))
            }
            Expr::ObjectLiteral(stmts) => {
                let mut struct_code = "([]() { struct __Anon {\n".to_string();
                let mut temp_gen = CodeGenerator::new();
                temp_gen.indent_level = self.indent_level + 1;
                for s in stmts {
                    temp_gen.visit_statement(s);
                }
                struct_code.push_str(&temp_gen.output);
                struct_code.push_str(&format!(
                    "{}}}; return std::make_shared<__Anon>(); }}())",
                    "    ".repeat(self.indent_level)
                ));
                struct_code
            }
            Expr::Identifier(name) => name.clone(),
            Expr::This => "this".to_string(),
            Expr::Super => "super".to_string(), // will be handled in PropertyAccess
            Expr::Global => "::".to_string(),
            Expr::BinaryOp {
                left,
                operator,
                right,
            } => {
                let l = self.visit_expression(left);
                let r = self.visit_expression(right);
                format!("({} {} {})", l, operator, r)
            }
            Expr::PostfixUpdate { left, operator } => {
                let l = self.visit_expression(left);
                format!("{}{}", l, operator)
            }
            Expr::Call { callee, args } => {
                let callee_code = self.visit_expression(callee);

                // Temporary hack: convert fast_lang `log` to `std::cout`
                if callee_code == "log" {
                    let mut cout_expr = "std::cout".to_string();
                    for arg in args {
                        cout_expr.push_str(&format!(" << {}", self.visit_expression(arg)));
                    }
                    cout_expr.push_str(" << std::endl");
                    return cout_expr;
                }

                if callee_code == "input" {
                    // Inline lambda to return user input
                    return "([]() { std::string _s; std::cin >> _s; return _s; }())".to_string();
                }

                let mut args_code = Vec::new();
                for arg in args {
                    args_code.push(self.visit_expression(arg));
                }
                format!("{}({})", callee_code, args_code.join(", "))
            }
            Expr::Instantiate {
                op,
                target,
                args: _,
            } => {
                let target_code = self.visit_expression(target);
                if op == "new" {
                    format!("std::make_shared<{}>()", target_code)
                } else if op == "copy" {
                    format!(
                        "std::make_shared<std::decay_t<decltype(*{})>>(*{})",
                        target_code, target_code
                    ) // basic copy using decltype
                } else {
                    target_code // modify or other
                }
            }
            Expr::PropertyAccess { object, property } => {
                let obj_code = self.visit_expression(object);
                if obj_code == "::" {
                    format!("::{}", property)
                } else if obj_code == "super" {
                    format!("this->{}", property) // Quick map to this-> since C++ derived classes inherit fields directly
                } else if obj_code == "this" {
                    format!("this->{}", property)
                } else {
                    format!("{}.{}", obj_code, property) // we default to . since primitive objects might not be pointers, though shared_ptr requires ->
                }
            }
            _ => "/* unimplemented expr */".to_string(),
        }
    }

    /// Maps a Fast type to a C++ type string.
    fn map_type(&self, base_type: Option<&str>, size: Option<i64>) -> String {
        match base_type {
            Some("int") => match size {
                Some(8) => "int8_t".to_string(),
                Some(16) => "int16_t".to_string(),
                Some(32) => "int32_t".to_string(),
                Some(64) => "int64_t".to_string(),
                _ => "int".to_string(),
            },
            Some("float") => match size {
                Some(64) => "double".to_string(),
                _ => "float".to_string(),
            },
            Some("string") | Some("str") => "std::string".to_string(),
            Some("bool") => "bool".to_string(),
            Some("list") => "auto".to_string(),
            Some("name") | Some("param") | Some("blueprint") | Some("init") => "auto".to_string(),
            Some("length") | Some("size") => "size_t".to_string(),
            Some(user_type) => user_type.to_string(),
            None => "auto".to_string(),
        }
    }
}
