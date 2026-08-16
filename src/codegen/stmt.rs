use crate::codegen::generator::CodeGenerator;
use crate::parser::ast::*;

impl CodeGenerator {
    pub(crate) fn visit_statement(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::ExpressionStmt(expr) => {
                let code = self.visit_expression(expr);
                self.emit(&format!("{};", code));
            }
            Stmt::EnumDecl { name, variants, .. } => {
                self.emit(&format!("enum class {} {{", name));
                self.indent_level += 1;
                for (i, variant) in variants.iter().enumerate() {
                    let comma = if i < variants.len() - 1 { "," } else { "" };
                    self.emit(&format!("{}{}", variant.name, comma));
                }
                self.indent_level -= 1;
                self.emit("};");

                self.emit(&format!("inline std::ostream& operator<<(std::ostream& os, const {}& obj) {{", name));
                self.indent_level += 1;
                self.emit("switch (obj) {");
                self.indent_level += 1;
                for variant in variants.iter() {
                    self.emit(&format!("case {}::{}: os << \"{}\"; break;", name, variant.name, variant.name));
                }
                self.indent_level -= 1;
                self.emit("}");
                self.emit("return os;");
                self.indent_level -= 1;
                self.emit("}");
            }
            Stmt::VarDecl {
                name,
                type_node,
                value,
                editability,
                ..
            } => {
                let val_code = self.visit_expression(value);

                // If value is __param__ sentinel, it's a declaration without init (struct field / function param)
                let is_param = val_code == "__param__";

                let base_type = type_node.as_ref().map(|t| match t {
                    TypeNode::Simple(r) => r.base_type.as_str(),
                    TypeNode::Generic(g) => g.base_type.as_str(),
                });
                let size = type_node.as_ref().and_then(|t| match t {
                    TypeNode::Simple(r) => r.size,
                    TypeNode::Generic(_) => None,
                });

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

                let is_const = editability == &Editability::NotEditable;
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
            Stmt::ArrayDecl {
                visibility: _,
                editability,
                type_node,
                name,
                length,
                value,
            } => {
                let mut val_code = self.visit_expression(value);
                let len_code = self.visit_expression(length);

                let base_type = type_node.as_ref().map(|t| match t {
                    TypeNode::Simple(r) => r.base_type.as_str(),
                    TypeNode::Generic(g) => g.base_type.as_str(),
                });
                let size = type_node.as_ref().and_then(|t| match t {
                    TypeNode::Simple(r) => r.size,
                    TypeNode::Generic(_) => None,
                });

                let cpp_type = self.map_type(base_type.as_deref(), size);

                let is_const = editability == &Editability::NotEditable;
                let const_prefix = if is_const { "const " } else { "" };

                // Handle C-style string initialization: char arr[10] = "hello";
                // When FastLang string literal is parsed, val_code is usually `"hello"`.
                // For C-style arrays, using `=` is standard. For array literal `[1, 2, 3]`, val_code is usually `{1, 2, 3}`.
                if val_code == "__param__" {
                    self.emit(&format!("{}{} {}[{}];", const_prefix, cpp_type, name, len_code));
                } else {
                    // If it is an array literal (val_code starts with `{`), we use `= val_code`
                    // If it is a string literal, we use `= val_code`
                    // Both work natively in C++ for C-style arrays
                    self.emit(&format!(
                        "{}{} {}[{}] = {};",
                        const_prefix, cpp_type, name, len_code, val_code
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
            Stmt::ForInStmt {
                item,
                iterable,
                body,
            } => {
                let iterable_code = self.visit_expression(iterable);
                let item_code = if let Stmt::VarDecl { type_node, name, .. } = &**item {
                    let type_str = type_node.as_ref().map(|t| match t {
                        crate::parser::ast::TypeNode::Simple(r) => r.base_type.as_str(),
                        crate::parser::ast::TypeNode::Generic(g) => g.base_type.as_str(),
                    }).unwrap_or("auto".to_string());
                    let cpp_type = self.map_type(Some(&type_str), None);
                    format!("{} {}", cpp_type, name)
                } else if let Stmt::ExpressionStmt(Expr::Identifier(name)) = &**item {
                    format!("auto& {}", name)
                } else {
                    "auto item".to_string()
                };

                self.emit(&format!("for ({} : {}) {{", item_code, iterable_code));
                self.indent_level += 1;
                match body {
                    EitherBlock::Inline(stmts) => {
                        for stmt in stmts {
                            self.visit_statement(stmt);
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
            Stmt::SwitchStmt {
                condition, cases, ..
            } => {
                let cond_code = self.visit_expression(condition);
                self.emit(&format!("switch ({}) {{", cond_code));
                self.indent_level += 1;
                for s in cases {
                    if let Stmt::CaseStmt { option, body, .. } = s {
                        if matches!(option, Expr::Identifier(name) if name == "void") {
                            self.emit("default: {");
                        } else {
                            let val_code = self.visit_expression(option);
                            self.emit(&format!("case {}: {{", val_code));
                        }
                        self.indent_level += 1;
                        for case_stmt in body {
                            self.visit_statement(case_stmt);
                        }
                        // if it doesn't end with return or break, emit break
                        let needs_break = if let Some(last) = body.last() {
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
                    self.visit_statement(inc);
                }
                self.indent_level -= 1;
                self.emit("}");

                self.indent_level -= 1;
                self.emit("}");
            }
            Stmt::ForIn {
                item_decl,
                iterable,
                body,
            } => {
                let old_out = std::mem::take(&mut self.output);
                self.visit_statement(item_decl);
                let mut decl_code = std::mem::replace(&mut self.output, old_out);
                // Decl_code may have a trailing semicolon, remove it
                decl_code = decl_code.trim_end().trim_end_matches(';').to_string();

                let iter_code = self.visit_expression(iterable);
                self.emit(&format!("for ({} : {}) {{", decl_code, iter_code));

                self.indent_level += 1;
                for s in body {
                    self.visit_statement(s);
                }
                self.indent_level -= 1;
                self.emit("}");
            }

            Stmt::ClassDecl {
                name,
                extends,
                public_block,
                private_block,
                static_block,
                constructor,
                is_exported: _,
                ..
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
                if let Some(constructors) = constructor {
                    for c in constructors {
                        let param_list: Vec<String> =
                            c.params
                                .iter()
                                .filter(|p| {
                                    p.type_node
                                        .as_ref()
                                        .map(|t| match t {
                                            TypeNode::Simple(r) => r.base_type.as_str(),
                                            TypeNode::Generic(g) => g.base_type.as_str(),
                                        })
                                        .unwrap_or("unknown".to_string())
                                        != "type"
                                })
                                .map(|p| {
                                    let cpp_t =
                                        p.type_node
                                            .as_ref()
                                            .map(|t| match t {
                                                TypeNode::Simple(r) => self
                                                    .map_type(Some(r.base_type.as_str().as_str()), r.size),
                                                TypeNode::Generic(g) => {
                                                    self.map_type(Some(g.base_type.as_str().as_str()), None)
                                                }
                                            })
                                            .unwrap_or("auto".to_string());
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
                ..
            } => {
                self.emit(&format!("struct {} {{", name));
                self.indent_level += 1;
                for s in public_block {
                    self.visit_statement(s);
                }
                for s in static_block {
                    self.visit_statement(s);
                }
                if let Some(constructors) = constructor {
                    for c in constructors {
                        let param_list: Vec<String> =
                            c.params
                                .iter()
                                .filter(|p| {
                                    p.type_node
                                        .as_ref()
                                        .map(|t| match t {
                                            TypeNode::Simple(r) => r.base_type.as_str(),
                                            TypeNode::Generic(g) => g.base_type.as_str(),
                                        })
                                        .unwrap_or("unknown".to_string())
                                        != "type"
                                })
                                .map(|p| {
                                    let cpp_t =
                                        p.type_node
                                            .as_ref()
                                            .map(|t| match t {
                                                TypeNode::Simple(r) => self
                                                    .map_type(Some(r.base_type.as_str().as_str()), r.size),
                                                TypeNode::Generic(g) => {
                                                    self.map_type(Some(g.base_type.as_str().as_str()), None)
                                                }
                                            })
                                            .unwrap_or("auto".to_string());
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
            Stmt::LeaveStmt => {
                self.emit("this->__state = -1;");
                self.emit("return leave();");
            }
            Stmt::YieldStmt(expr) => {
                self.yield_counter += 1;
                let yid = self.yield_counter;
                self.emit(&format!("this->__state = {};", yid));
                if let Some(e) = expr {
                    let expr_code = self.visit_expression(e);
                    self.emit(&format!("return {};", expr_code));
                } else {
                    self.emit("return yield();");
                }
                self.emit(&format!("case {}:;", yid));
            }
            Stmt::GotoStmt(expr) => {
                let expr_code = self.visit_expression(expr);
                let safe_name = expr_code.replace("@", "");
                self.emit(&format!("goto {};", safe_name));
            }
            Stmt::CallStmt(expr) => {
                let expr_code = self.visit_expression(expr);
                self.emit(&format!("{};", expr_code));
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
            Stmt::FnDecl {
                name,
                params,
                return_type,
                body,
                is_exported: _, // هنتجاهلها دلوقتي لحد ما نحتاجها في الـ Shared Libraries
            } => {
                // 1. تحديد نوع الإرجاع (Return Type)
                let mut ret_type_str = match return_type {
                    TypeNode::Simple(r) => self.map_type(Some(r.base_type.as_str().as_str()), r.size),
                    TypeNode::Generic(g) => self.map_type(Some(g.base_type.as_str().as_str()), None),
                };

                // Edge case for C++ main function
                if name == "main" {
                    ret_type_str = "int".to_string();
                }

                // 2. تجميع المعاملات (Parameters)
                let mut param_strs = Vec::new();
                for param in params {
                    let param_type = match &param.type_node {
                        Some(TypeNode::Simple(r)) => {
                            self.map_type(Some(r.base_type.as_str().as_str()), r.size)
                        }
                        Some(TypeNode::Generic(g)) => {
                            self.map_type(Some(g.base_type.as_str().as_str()), None)
                        }
                        None => "auto".to_string(), // Fallback
                    };
                    param_strs.push(format!("{} {}", param_type, param.name));
                }

                let safe_name = if name == "throw" { "_throw" } else { name };

                // 3. كتابة توقيع الدالة (Function Signature)
                self.emit(&format!(
                    "{} {}({}) {{",
                    ret_type_str,
                    safe_name,
                    param_strs.join(", ")
                ));

                // 4. كتابة محتوى الدالة (Function Body)
                self.indent_level += 1;
                for s in body {
                    self.visit_statement(s);
                }
                self.indent_level -= 1;

                // 5. قفل الدالة
                self.emit("}");
            }
            Stmt::CustomDecl {
                name,
                fields,
                constructor,
                statements,
                public_block,
                private_block,
                flags,
                labels,
                events,
                data,
                length,
                handle_block,
                ..
            } => {
                self.custom_scopes.insert(name.clone());
                self.emit(&format!("class {} {{", name));
                self.emit("public:");
                self.indent_level += 1;
                self.emit("int __state = 0;");

                let has_display = if let Some(handles) = &handle_block {
                    handles.iter().any(|h| {
                        if let Stmt::FnDecl { name: fn_name, .. } = h {
                            fn_name == "display"
                        } else { false }
                    })
                } else { false };

                if has_display {
                    self.emit(&format!(
                        "friend std::ostream& operator<<(std::ostream& os, {}& obj) {{",
                        name
                    ));
                    self.emit("    os << obj.display();");
                    self.emit("    return os;");
                    self.emit("}");
                } else {
                    self.emit(&format!(
                        "friend std::ostream& operator<<(std::ostream& os, const {}& obj) {{",
                        name
                    ));
                    self.emit(&format!("    os << \"[object {}]\";", name));
                    self.emit("    return os;");
                    self.emit("}");
                }

                self.emit(&format!("int length = {};", length));
                if let Some(d) = data {
                    let d_code = self.visit_expression(&d);
                    self.emit(&format!("int data = {};", d_code)); // Simplified to int
                }

                if let Some(fields_vec) = fields {
                    for field in fields_vec {
                        let base_type = match &field.type_node {
                            Some(TypeNode::Simple(r)) => self.map_type(Some(r.base_type.as_str().as_str()), r.size),
                            _ => "auto".to_string(),
                        };
                        self.emit(&format!("{} {} = {{}};", base_type, field.name));
                    }
                }

                let mut default_flags = std::collections::HashSet::from([
                    "has_return",
                    "has_break",
                    "has_throw",
                    "has_switch",
                    "has_exit",
                ]);
                let mut enabled_flags = std::collections::HashSet::new();

                if let Some(flags_vec) = flags {
                    for flag in flags_vec {
                        let flag_name = match flag {
                            Flag::HasReturn => "has_return",
                            Flag::HasBreak => "has_break",
                            Flag::HasThrow => "has_throw",
                            Flag::HasError => "has_error",
                            Flag::HasSwitch => "has_switch",
                            Flag::HasExit => "has_exit",
                            Flag::Custom(s) => s.as_str(),
                        };
                        enabled_flags.insert(flag_name.to_string());
                        default_flags.remove(flag_name);
                    }
                }

                let mut has_throw_handle = false;
                if let Some(handles) = &handle_block {
                    for h in handles {
                        if let Stmt::FnDecl { name, .. } = h {
                            if name == "throw" {
                                has_throw_handle = true;
                                break;
                            }
                        }
                    }
                }

                if has_throw_handle {
                    enabled_flags.insert("has_throw".to_string());
                    default_flags.remove("has_throw");
                }

                for flag in &default_flags {
                    self.emit(&format!("bool {} = false;", flag));
                }
                for flag in &enabled_flags {
                    self.emit(&format!("bool {} = true;", flag));
                }

                if let Some(labels_vec) = labels {
                    // labels are handled as methods, no std::string needed.
                }

                let mut unified_return_type = "void".to_string();
                if let Some(handles) = &handle_block {
                    for h in handles {
                        if let Stmt::FnDecl {
                            name,
                            return_type: rt,
                            ..
                        } = h
                        {
                            if name == "call" || name == "leave" || name == "yield" {
                                unified_return_type = match rt {
                                    TypeNode::Simple(r) => {
                                        self.map_type(Some(r.base_type.as_str().as_str()), r.size)
                                    }
                                    TypeNode::Generic(g) => {
                                        self.map_type(Some(g.base_type.as_str().as_str()), None)
                                    }
                                };
                                break;
                            }
                        }
                    }
                }

                if let Some(handles) = handle_block {
                    for h in handles {
                        if let Stmt::FnDecl {
                            name,
                            params,
                            return_type,
                            body,
                            is_exported,
                        } = h
                        {
                            if name == "call" {
                                let ret_type_str = match return_type {
                                    TypeNode::Simple(r) => {
                                        self.map_type(Some(r.base_type.as_str().as_str()), r.size)
                                    }
                                    TypeNode::Generic(g) => {
                                        self.map_type(Some(g.base_type.as_str().as_str()), None)
                                    }
                                };
                                self.emit(&format!("{} call() {{", ret_type_str));
                                self.indent_level += 1;
                                if has_throw_handle {
                                    self.emit("try {");
                                    self.indent_level += 1;
                                }
                                self.emit("switch(this->__state) {");
                                self.emit("case 0:");
                                self.indent_level += 1;

                                for s in body {
                                    self.visit_statement(s);
                                }

                                if let Some(events_vec) = &events {
                                    for event in events_vec {
                                        let safe_name = event.trigger_name.replace("@", "");
                                        self.indent_level -= 1;
                                        self.emit(&format!("{}:", safe_name));
                                        self.indent_level += 1;
                                        for s in &event.body {
                                            self.visit_statement(s);
                                        }
                                    }
                                }

                                self.indent_level -= 1;
                                self.emit("}");
                                self.indent_level -= 1;
                                self.emit("}");
                                if has_throw_handle {
                                    self.indent_level -= 1;
                                    self.emit("} catch (const std::runtime_error& __e) {");
                                    self.indent_level += 1;
                                    self.emit("this->_throw(__e.what());");
                                    self.indent_level -= 1;
                                    self.emit("}");
                                }
                                continue;
                            }
                        }
                        self.visit_statement(&h);
                    }
                }
                self.emit_operator_overloads(&handle_block);

                // Only generate default call() if it wasn't provided in the handle block
                let has_call_handle = handle_block.as_ref().map_or(false, |handles| {
                    handles.iter().any(|h| {
                        if let Stmt::FnDecl { name, .. } = h {
                            name == "call"
                        } else {
                            false
                        }
                    })
                });

                if !has_call_handle {
                    self.emit("void call() {");
                    self.indent_level += 1;
                    if has_throw_handle {
                        self.emit("try {");
                        self.indent_level += 1;
                    }
                    self.emit("switch(this->__state) {");
                    self.emit("case 0:");
                    self.indent_level += 1;

                    self.emit("init();");

                    if let Some(events_vec) = &events {
                        for event in events_vec {
                            let safe_name = event.trigger_name.replace("@", "");
                            self.indent_level -= 1;
                            self.emit(&format!("{}:", safe_name));
                            self.indent_level += 1;
                            for s in &event.body {
                                self.visit_statement(s);
                            }
                        }
                    }

                    self.indent_level -= 1;
                    self.emit("}");
                    if has_throw_handle {
                        self.indent_level -= 1;
                        self.emit("} catch (const std::runtime_error& __e) {");
                        self.indent_level += 1;
                        self.emit("this->_throw(__e.what());");
                        self.indent_level -= 1;
                        self.emit("}");
                    }
                    self.indent_level -= 1;
                    self.emit("}");
                }

                if let Some(const_vec) = constructor {
                    for c in const_vec {
                        self.emit("void init() {");
                        self.indent_level += 1;
                        for s in &c.body {
                            self.visit_statement(s);
                        }
                        self.indent_level -= 1;
                        self.emit("}");
                    }
                } else {
                    self.emit("void init() {");
                    self.indent_level += 1;
                    if let Some(stmts) = statements {
                        for s in stmts {
                            self.visit_statement(s);
                        }
                    }
                    self.indent_level -= 1;
                    self.emit("}");
                }

                if let Some(pub_stmts) = public_block {
                    for s in pub_stmts {
                        self.visit_statement(s);
                    }
                }

                self.indent_level -= 1;
                self.emit("private:");
                self.indent_level += 1;

                if let Some(priv_stmts) = private_block {
                    for s in priv_stmts {
                        self.visit_statement(s);
                    }
                }

                self.indent_level -= 1;
                self.emit("};");
            }
            Stmt::LabelDecl { name, body } => {
                self.emit(&format!("void {}() {{", name));
                self.indent_level += 1;
                for s in body {
                    self.visit_statement(s);
                }
                self.indent_level -= 1;
                self.emit("}");
            }
            Stmt::GotoStmt(target) => {
                let target_str = self.visit_expression(target);
                self.emit(&format!("goto {};", target_str));
            }
            _ => {
                self.emit(&format!(
                    "// TODO: unimplemented statement {:?} or something gone wrong",
                    stmt,
                ));
            }
        }
    }
}
