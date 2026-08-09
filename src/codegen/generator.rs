use crate::parser::ast::{Expr, EitherBlock, Stmt};

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

    pub fn generate(&mut self, ast: &Vec<Stmt>, emit_headers: bool, wrap_in_main: bool) -> String {
        if emit_headers {
            self.output.push_str("#include <iostream>\n");
            self.output.push_str("#include <vector>\n");
            self.output.push_str("#include <array>\n");
            self.output.push_str("#include <memory>\n");
            self.output.push_str("#include <optional>\n");
            self.output.push_str("#include <stdexcept>\n\n");
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
                type_sized,
                value,
                editability,
                ..
            } => {
                let val_code = self.visit_expression(value);

                // If value is __param__ sentinel, it's a declaration without init (struct field / function param)
                let is_param = val_code == "__param__";

                let base_type = type_sized.as_ref().map(|t| t.base_type.clone());
                let size = type_sized.as_ref().and_then(|t| t.size);

                let cpp_type = match base_type.as_deref() {
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
                    _ => self.map_type(base_type.as_deref(), size),
                };

                let is_const = editability == &crate::parser::ast::Editability::NotEditable;
                let const_prefix = if is_const { "const " } else { "" };

                if is_param {
                    // Declaration without initializer (struct field or function param)
                    self.emit(&format!("{}{} {};", const_prefix, cpp_type, name));
                } else {
                    self.emit(&format!(
                        "{}{} {} = {};",
                        const_prefix, cpp_type, name, val_code
                    ));
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
                    EitherBlock::Inline(stmts) => {
                        for s in stmts {
                            self.visit_statement(s);
                        }
                    }
                    EitherBlock::External(expr) => {
                        let expr_code = self.visit_expression(expr);
                        self.emit(&format!("{};", expr_code));
                    }
                }
                self.indent_level -= 1;
                self.emit("}");
            }
            Stmt::SwitchStmt { condition, cases } => {
                let cond_code = self.visit_expression(condition);
                self.emit(&format!("switch ({}) {{", cond_code));
                self.indent_level += 1;
                match cases {
                    crate::parser::ast::EitherBlock::Inline(stmts) => {
                        for s in stmts {
                            if let Stmt::ScopeDecl {
                                scope_type,
                                return_value,
                                statements,
                                ..
                            } = s
                            {
                                if *scope_type == crate::parser::ast::ScopeType::Case {
                                    if let Some(val) = return_value {
                                        let val_code = self.visit_expression(val);
                                        self.emit(&format!("case {}: {{", val_code));
                                    } else {
                                        self.emit("default: {");
                                    }
                                    self.indent_level += 1;
                                    for case_stmt in statements {
                                        self.visit_statement(case_stmt);
                                    }
                                    // if it doesn't end with return or break, emit break
                                    let needs_break = if let Some(last) = statements.last() {
                                        !matches!(last, Stmt::ReturnStmt(_) | Stmt::BreakStmt)
                                    } else {
                                        true
                                    };
                                    if needs_break {
                                        self.emit("break;");
                                    }
                                    self.indent_level -= 1;
                                    self.emit("}");
                                }
                            }
                        }
                    }
                    crate::parser::ast::EitherBlock::External(name_expr) => {
                        let name_code = self.visit_expression(name_expr);
                        self.emit(&format!("// TODO: inject cases from {}", name_code));
                    }
                }
                self.indent_level -= 1;
                self.emit("}");
            }
            Stmt::DelStmt(expr) => {
                let expr_code = self.visit_expression(expr);
                self.emit(&format!("delete {};", expr_code)); // Assuming 'del temp' maps to 'delete temp' for C++ memory management (if pointers) or we can implement it as a destructor call. Since it's C++, delete is fine if it's a pointer. But wait, FastLang variables are stack allocated unless `new`. We'll just emit delete for now.
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
                    EitherBlock::Inline(stmts) => {
                        for s in stmts {
                            self.visit_statement(s);
                        }
                    }
                    EitherBlock::External(expr) => {
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
                is_exported: _,
                is_const,
                name,
                scope_type,
                params,
                return_type,
                flags: _,
                settings: _,
                events,
                handles,
                statements,
                public_block,
                fields,
                private_block,
                return_value,
                constructor,
            } => {
                if *scope_type == crate::parser::ast::ScopeType::Custom {
                    let type_params: Vec<_> = params
                        .iter()
                        .filter(|p| p.base_type == "type" || p.base_type.starts_with("type<"))
                        .collect();
                    if !type_params.is_empty() {
                        let template_args: Vec<_> = type_params
                            .iter()
                            .map(|p| format!("typename {}", p.name))
                            .collect();
                        self.emit(&format!("template <{}>", template_args.join(", ")));
                    }
                    self.emit(&format!("struct {} {{", name));
                    self.indent_level += 1;

                    for field in fields {
                        if field.type_sized.as_ref().map(|t| t.base_type.as_str()) != Some("type")
                            && !field
                                .type_sized
                                .as_ref()
                                .map_or(false, |t| t.base_type.starts_with("type<"))
                        {
                            let base_type = field.type_sized.as_ref().map(|t| t.base_type.as_str());
                            let size = field.type_sized.as_ref().and_then(|t| t.size);
                            let cpp_type = self.map_type(base_type, size);
                            self.emit(&format!("{} {};", cpp_type, field.name));
                        }
                    }

                    // Fields from params
                    for p in params {
                        if p.base_type != "type" && !p.base_type.starts_with("type<") {
                            let cpp_t = self.map_type(Some(p.base_type.as_str()), p.size);
                            self.emit(&format!("{} {};", cpp_t, p.name));
                        }
                    }

                    // Constructor
                    if let Some(c) = constructor {
                        let param_list: Vec<String> = c
                            .params
                            .iter()
                            .filter(|p| p.base_type != "type" && !p.base_type.starts_with("type<"))
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
                    } else {
                        // Fallback constructor
                        let mut param_list = Vec::new();
                        let mut has_constructor_params = false;
                        for p in params {
                            if p.base_type != "type" && !p.base_type.starts_with("type<") {
                                let cpp_t = self.map_type(Some(p.base_type.as_str()), p.size);
                                param_list.push(format!("{} _{}", cpp_t, p.name));
                                has_constructor_params = true;
                            }
                        }
                        if has_constructor_params {
                            self.emit(&format!("{}({}) {{", name, param_list.join(", ")));
                            self.indent_level += 1;
                            for p in params {
                                if p.base_type == "type" || p.base_type.starts_with("type<") {
                                    continue;
                                }
                                self.emit(&format!("this->{} = _{};", p.name, p.name));
                            }
                            self.indent_level -= 1;
                            self.emit("}");
                        }
                    }

                    self.in_class_def = true;
                    // General statements
                    for s in statements {
                        self.visit_statement(s);
                    }

                    // Public Block
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
                    let cpp_t = self.map_type(Some(p.base_type.as_str()), p.size);
                    param_list.push(format!("{} {}", cpp_t, p.name));
                }

                let constexpr_prefix = if *is_const { "constexpr " } else { "" };

                if self.indent_level == 0 || self.in_class_def {
                    // Top-level scope or class method → proper C++ function
                    let has_return_stmt =
                        statements.iter().any(|s| matches!(s, Stmt::ReturnStmt(_)));
                    let cpp_ret = if name == "main" {
                        "int".to_string()
                    } else if *scope_type == crate::parser::ast::ScopeType::Custom {
                        "".to_string() // Custom scope (struct/class) doesn't have a return type like this
                    } else if let Some(rt) = return_type {
                        self.map_type(Some(&rt.base_type), rt.size)
                    } else if has_return_stmt || return_value.is_some() {
                        "auto".to_string()
                    } else {
                        "void".to_string()
                    };

                    self.emit(&format!(
                        "{}{} {}({}) {{",
                        constexpr_prefix,
                        cpp_ret,
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
                        .filter(|p| p.base_type != "type" && !p.base_type.starts_with("type<"))
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
                        .filter(|p| p.base_type != "type" && !p.base_type.starts_with("type<"))
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
        //debug
        println!("Visiting expression: {:?}", expr);
        match expr {
            Expr::LiteralInt(val) => val.to_string(),
            Expr::LiteralFloat(f) => format!("{:?}", f),
            Expr::LiteralString(s) => format!("\"{}\"", s),
            Expr::LiteralChar(c) => format!("'{}'", c),
            Expr::LiteralBool(val) => {
                if *val {
                    "true".to_string()
                } else {
                    "false".to_string()
                }
            }
            Expr::ArrayLiteral(elements) => {
                let mut elems_code = Vec::new();
                for el in elements {
                    elems_code.push(self.visit_expression(el));
                }
                format!("std::vector{{{}}}", elems_code.join(", "))
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
            Expr::Identifier(name) => {
                if name == "null" {
                    "nullptr".to_string()
                } else if name == "None" {
                    "std::nullopt".to_string()
                } else {
                    name.clone()
                }
            }
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

                if callee_code == "Some" {
                    format!("std::optional{{{}}}", args_code.join(", "))
                } else {
                    format!("{}({})", callee_code, args_code.join(", "))
                }
            }
            Expr::Instantiate { op, target, args } => {
                let target_code = self.visit_expression(target);
                let mut args_code = Vec::new();
                for arg in args {
                    args_code.push(self.visit_expression(arg));
                }
                if op == "new" {
                    let mut tc = target_code.clone();
                    if tc == "Node" {
                        tc = "Node<T>".to_string();
                    }
                    format!("(void*)new {}({})", tc, args_code.join(", "))
                } else if op == "copy" {
                    format!(
                        "new std::decay_t<decltype(*{})>(*{})",
                        target_code, target_code
                    ) // basic copy using decltype
                } else {
                    target_code // modify or other
                }
            }
            Expr::PropertyAccess { object, property } => {
                let obj_code = self.visit_expression(object);
                if property == "set_next"
                    || property == "get_next"
                    || property == "get_value"
                    || property == "set_value"
                {
                    return format!("((std_list::Node<T>*){})->{}", obj_code, property);
                }
                if obj_code == "::" {
                    format!("::{}", property)
                } else if obj_code == "super" {
                    format!("this->{}", property) // Quick map to this-> since C++ derived classes inherit fields directly
                } else if obj_code == "this" {
                    format!("this->{}", property)
                } else if property == "size" {
                    format!("{}.size()", obj_code)
                } else {
                    format!("{}.{}", obj_code, property) // we default to . since primitive objects might not be pointers, though shared_ptr requires ->
                }
            }
            Expr::IndexAccess { object, index } => {
                let obj_code = self.visit_expression(object);
                let idx_code = self.visit_expression(index);
                format!("{}[{}]", obj_code, idx_code)
            }
            _ => "/* unimplemented expr */".to_string(),
        }
    }

    fn map_type(&self, base_type: Option<&str>, size: Option<i64>) -> String {
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

    fn map_type_spec(&self, base_type: &str, size: Option<i64>) -> String {
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
