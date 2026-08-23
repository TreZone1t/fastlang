use crate::backend::cpp::generator::CodeGenerator;
use crate::frontend::parser::ast::*;

pub(crate) fn type_to_cpp(t: &BaseType) -> String {
    match t {
        BaseType::Int8 => "int8_t".to_string(),
        BaseType::Int16 => "int16_t".to_string(),
        BaseType::Int32 => "int32_t".to_string(),
        BaseType::Int64 => "int64_t".to_string(),
        BaseType::Int128 => "__int128".to_string(),
        BaseType::Float32 => "float".to_string(),
        BaseType::Float64 => "double".to_string(),
        BaseType::Char => "char".to_string(),
        BaseType::Bool => "bool".to_string(),
        BaseType::Void => "void".to_string(),
        BaseType::Error => "const std::exception&".to_string(),
        BaseType::Custom { name, generics, .. } | BaseType::Class { name, generics, .. } | BaseType::Struct { name, generics, .. } => {
            if name == "error" {
                "const std::exception&".to_string()
            } else if name == "str" {
                "const char*".to_string()
            } else if generics.is_empty() {
                name.clone()
            } else {
                let gen_strs: Vec<String> = generics.iter().map(type_to_cpp).collect();
                format!("{}<{}>", name, gen_strs.join(", "))
            }
        }
        BaseType::Enum { name, .. } | BaseType::Blueprint { name, .. } => name.clone(),
        BaseType::GenericParam(name) => name.clone(),
        BaseType::Generic(inner_vec) => {
            let strs: Vec<String> = inner_vec.iter().map(type_to_cpp).collect();
            strs.join(", ")
        }
        BaseType::Pointer(inner) => format!("{}*", type_to_cpp(inner)),
        BaseType::Name(inner) => {
            if let BaseType::Generic(ref vec) = &**inner {
                if vec.len() == 1 && matches!(vec[0], BaseType::Fn { .. } | BaseType::Method { .. }) {
                    return type_to_cpp(&vec[0]);
                }
            }
            if matches!(&**inner, BaseType::Fn { .. } | BaseType::Method { .. }) {
                type_to_cpp(inner)
            } else if inner.as_str() == BaseType::Unknown.as_str() {
                "fastlang_name".to_string()
            } else {
                format!("fastlang_name<{}>", type_to_cpp(inner))
            }
        }
        BaseType::Modify(inner) => {
            if inner.as_str() == BaseType::Unknown.as_str() {
                "fastlang_modify".to_string()
            } else {
                let actual_inner = if let BaseType::Name(n) = &**inner { n } else { inner };
                if actual_inner.as_str() == "unknown" {
                    "fastlang_modify".to_string()
                } else {
                    format!("fastlang_modify<{}>", type_to_cpp(actual_inner))
                }
            }
        }
        BaseType::Copy(inner) => {
            if inner.as_str() == BaseType::Unknown.as_str() {
                "fastlang_copy".to_string()
            } else {
                let actual_inner = if let BaseType::Name(n) = &**inner { n } else { inner };
                if actual_inner.as_str() == "unknown" {
                    "fastlang_copy".to_string()
                } else {
                    format!("fastlang_copy<{}>", type_to_cpp(actual_inner))
                }
            }
        }
        BaseType::Flag => "bool".to_string(),
        BaseType::Scope(inner) => {
            if inner.as_str() == BaseType::Unknown.as_str() {
                "fastlang_scope<std::function<void()>>".to_string()
            } else {
                format!("fastlang_scope<{}>", type_to_cpp(inner))
            }
        }
        BaseType::Array { base_type, .. } => format!("fastlang_slice<{}>", type_to_cpp(base_type)),
        BaseType::Method { return_type, params } | BaseType::Fn { return_type, params } => {
            let ret = type_to_cpp(return_type);
            let p_types: Vec<String> = params.iter().map(type_to_cpp).collect();
            format!("std::function<{}({})>", ret, p_types.join(", "))
        }
        BaseType::Unknown => "auto".to_string(),
        _ => t.as_str(),
    }
}

impl CodeGenerator {
    pub(crate) fn visit_statement(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Declaration(decl) => {
                self.visit_declaration(decl);
            }
            Stmt::ExpressionStmt(expr) => {
                let code = self.visit_expression(expr);
                self.emit(&format!("{};", code));
            }
            Stmt::ReassignStmt { target, value, op } => {
                let target_code = self.visit_expression(target);
                let val_code = self.visit_expression(value);
                if op == "->" {
                    if val_code.starts_with('{') && val_code.ends_with('}') {
                        self.emit(&format!("{}.arrow({});", target_code, val_code));
                    } else {
                        self.emit(&format!("fastlang_arrow({}, {});", target_code, val_code));
                    }
                } else if op == "=" {
                    self.emit(&format!("{} = {};", target_code, val_code));
                } else {
                    self.emit(&format!("{} {} {};", target_code, op, val_code));
                }
            }
            Stmt::IfStmt { condition, then_block, else_block } => {
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
            Stmt::ForInStmt { item, iterable, body } => {
                let iterable_code = self.visit_expression(iterable);
                let item_code = if
                    let Stmt::Declaration(Decl::VarDecl { type_node, name, .. }) = &**item
                {
                    let cpp_type = match type_node.clone() {
                        BaseType::Int8 => "int8_t".to_string(),
                        BaseType::Int16 => "int16_t".to_string(),
                        BaseType::Int32 => "int32_t".to_string(),
                        BaseType::Int64 => "int64_t".to_string(),
                        BaseType::Float32 => "float".to_string(),
                        BaseType::Float64 => "double".to_string(),
                        BaseType::Char => "char".to_string(),
                        BaseType::Bool => "bool".to_string(),
                        BaseType::Array { base_type, .. } => base_type.as_str(),
                        _ => "auto".to_string(),
                    };
                    format!("{} {}", cpp_type, name)
                } else if let Stmt::ExpressionStmt(Expr::Identifier(name)) = &**item {
                    format!("auto {}", name)
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
            Stmt::SwitchStmt { condition, cases, .. } => {
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
            Stmt::DelStmt { target, is_array } => {
                let expr_code = self.visit_expression(target);
                if *is_array {
                    self.emit(&format!("_fastlang_del_array({});", expr_code));
                } else {
                    self.emit(&format!("_fastlang_del({});", expr_code));
                }
            }
            Stmt::ForStmt { init, condition, increment, body } => {
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
            Stmt::ForIn { item_decl, iterable, body } => {
                let old_out = std::mem::take(&mut self.output);
                self.visit_statement(item_decl);
                let mut decl_code = std::mem::replace(&mut self.output, old_out);
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
                self.emit("this->has_yielded = true;");
                self.emit(&format!("this->__state = {};", yid));
                if let Some(e) = expr {
                    let expr_code = self.visit_expression(e);
                    self.emit(&format!("return {};", expr_code));
                } else {
                    self.emit("return _fastlang_do_yield(this);");
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
                if let Expr::New { type_node, target } = expr {
                    if let BaseType::Custom { name, .. } = type_node {
                        if name == "error" {
                            let arg_code = self.visit_expression(target);
                            self.emit(&format!("throw std::runtime_error({});", arg_code));
                            return;
                        }
                    }
                }
                if let Expr::Identifier(_) = expr {
                    self.emit("throw;");
                    return;
                }
                let expr_code = self.visit_expression(expr);
                self.emit(&format!("throw std::runtime_error({});", expr_code));
            }
            Stmt::TryCatchStmt { try_block, catch_param, catch_block } => {
                self.emit("try {");
                self.indent_level += 1;
                for s in try_block {
                    self.visit_statement(s);
                }
                self.indent_level -= 1;
                self.emit(&format!("}} catch (const std::exception& {}) {{", catch_param));
                self.indent_level += 1;
                for s in catch_block {
                    self.visit_statement(s);
                }
                self.indent_level -= 1;
                self.emit("}");
            }
            Stmt::EnableStmt(_) => {} //todo : V2 we will remove it as will
            _ => {
                self.emit(
                    &format!("// TODO: unimplemented statement {:?} or something gone wrong", stmt)
                );
            }
        }
    }

    pub(crate) fn visit_declaration(&mut self, decl: &Decl) {
        match decl {
            Decl::EnumDecl { name, variants, .. } => {
                self.emit(&format!("enum class {} {{", name));
                self.indent_level += 1;
                for (i, variant) in variants.iter().enumerate() {
                    let comma = if i < variants.len() - 1 { "," } else { "" };
                    self.emit(&format!("{}{}", variant.name, comma));
                }
                self.indent_level -= 1;
                self.emit("};");

                self.emit(
                    &format!("inline std::ostream& operator<<(std::ostream& os, const {}& obj) {{", name)
                );
                self.indent_level += 1;
                self.emit("switch (obj) {");
                self.indent_level += 1;
                for variant in variants.iter() {
                    self.emit(
                        &format!(
                            "case {}::{}: os << \"{}\"; break;",
                            name,
                            variant.name,
                            variant.name
                        )
                    );
                }
                self.indent_level -= 1;
                self.emit("}");
                self.emit("return os;");
                self.indent_level -= 1;
                self.emit("}");
            }
            Decl::VarDecl { name, type_node, value, editability, assign_op, place: _, .. } => {
                if matches!(type_node, BaseType::Custom { .. } | BaseType::Class { .. }) {
                    self.custom_scopes.insert(name.clone());
                }
                if matches!(type_node, BaseType::Pointer(_) | BaseType::Name(_) | BaseType::Modify(_) | BaseType::Copy(_)) {
                    self.pointer_vars.insert(name.clone());
                }
                let is_param = match value {
                    Expr::Identifier(s) if s == "__param__" => true,
                    _ => false,
                };
                let is_const = editability == &Editability::NotEditable;
                let const_prefix = if is_const { "const " } else { "" };
                let cpp_type = if matches!(type_node, BaseType::Method { .. } | BaseType::Fn { .. }) {
                    "auto".to_string()
                } else if let BaseType::Name(inner) = type_node {
                    if matches!(&**inner, BaseType::Fn { .. } | BaseType::Method { .. }) {
                        "auto".to_string()
                    } else {
                        type_to_cpp(type_node)
                    }
                } else {
                    type_to_cpp(type_node)
                };

                let is_value_type = !matches!(type_node, BaseType::Pointer(_) | BaseType::Name(_) | BaseType::Modify(_) | BaseType::Copy(_));
                let val_code = if is_value_type {
                    if let Expr::New { type_node: inner, target } = value {
                        let target_code = self.visit_expression(target);
                        if target_code == "__default__" || target_code == "{}" || target_code.is_empty() {
                            format!("{}()", type_to_cpp(inner))
                        } else if target_code.starts_with('{') && target_code.ends_with('}') {
                            format!("{}{}", type_to_cpp(inner), target_code)
                        } else {
                            format!("{}({})", type_to_cpp(inner), target_code)
                        }
                    } else {
                        self.visit_expression(value)
                    }
                } else {
                    self.visit_expression(value)
                };

                if is_param {
                    self.emit(&format!("{}{} {};", const_prefix, cpp_type, name));
                } else if assign_op == "->" {
                    self.emit(&format!("{}{} {};", const_prefix, cpp_type, name));
                    if val_code.starts_with('{') && val_code.ends_with('}') {
                        self.emit(&format!("{}.arrow_assign({});", name, val_code));
                    } else {
                        self.emit(&format!("fastlang_arrow_assign({}, {});", name, val_code));
                    }
                } else if assign_op == "=" {
                    self.emit(&format!("{}{} {} = {};", const_prefix, cpp_type, name, val_code));
                } else {
                    self.emit(
                        &format!(
                            "{}{} {} {} {};",
                            const_prefix,
                            cpp_type,
                            name,
                            assign_op,
                            val_code
                        )
                    );
                }
            }
            Decl::DestructureDecl {
                visibility: _,
                editability,
                type_node,
                assignments,
                assign_op,
                ..
            } => {
                let cpp_type = type_to_cpp(type_node);
                let is_const = editability == &Editability::NotEditable;
                let const_prefix = if is_const { "const " } else { "" };

                for (name, val) in assignments {
                    let val_code = self.visit_expression(val);
                    if val_code == "__default__" {
                        self.emit(&format!("{}{} {};", const_prefix, cpp_type, name));
                    } else if assign_op == "->" {
                        self.emit(&format!("{}{} {};", const_prefix, cpp_type, name));
                        self.emit(&format!("fastlang_arrow_assign({}, {});", name, val_code));
                    } else if assign_op == "=" {
                        self.emit(&format!("{}{} {} = {};", const_prefix, cpp_type, name, val_code));
                    } else {
                        self.emit(
                            &format!(
                                "{}{} {} {} {};",
                                const_prefix,
                                cpp_type,
                                name,
                                assign_op,
                                val_code
                            )
                        );
                    }
                }
            }
            Decl::ObjectDestructureDecl { editability, fields, rhs, .. } => {
                let rhs_code = self.visit_expression(rhs);
                let is_const = editability == &Editability::NotEditable;
                let const_prefix = if is_const { "const " } else { "" };
                let names: Vec<String> = fields.iter().map(|(_, name)| name.clone()).collect();
                self.emit(&format!("{}auto [{}] = {};", const_prefix, names.join(", "), rhs_code));
            }
            Decl::ArrayDecl {
                visibility: _,
                editability,
                type_node,
                name,
                length,
                value,
                assign_op: _,
            } => {
                let val_code = self.visit_expression(value);
                let len_code = self.visit_expression(length);
                let cpp_type = type_to_cpp(type_node);

                let is_const = editability == &Editability::NotEditable;
                let const_prefix = if is_const { "const " } else { "" };

                let len_str = if len_code == "0" && val_code != "__default__" {
                    "".to_string()
                } else {
                    len_code
                };

                if val_code == "__param__" {
                    self.emit(&format!("{}{} {}[{}];", const_prefix, cpp_type, name, len_str));
                } else {
                    self.emit(
                        &format!(
                            "{}{} {}[{}] = {};",
                            const_prefix,
                            cpp_type,
                            name,
                            len_str,
                            val_code
                        )
                    );
                }
            }
            Decl::ClassDecl {
                name,
                extends,
                public_block,
                private_block,
                static_block,
                handle_block,
                constructor,
                generics,
                is_exported: _,
                ..
            } => {
                if !generics.is_empty() {
                    let gen_params: Vec<String> = generics
                        .iter()
                        .map(|g| format!("typename {}", g.as_str()))
                        .collect();
                    self.emit(&format!("template <{}>", gen_params.join(", ")));
                }
                let ext_code = if let Some(ext) = extends {
                    format!(": public {}", ext)
                } else {
                    "".to_string()
                };
                self.emit(&format!("class {} {} {{", name, ext_code));
                self.emit("public:");
                self.indent_level += 1;

                let has_display = handle_block.iter().any(|h| {
                    if let Decl::FnDecl { name: fn_name, .. } = h {
                        fn_name == "display"
                    } else {
                        false
                    }
                });

                if has_display {
                    self.emit(
                        &format!("friend std::ostream& operator<<(std::ostream& os, {}& obj) {{", name)
                    );
                    self.emit("    os << obj.display();");
                    self.emit("    return os;");
                    self.emit("}");
                }

                self.emit_operator_overloads(&Some(handle_block.clone()));

                for s in public_block {
                    self.visit_declaration(s);
                }
                for h in handle_block {
                    self.visit_declaration(h);
                }
                for s in static_block {
                    self.emit("static ");
                    self.visit_declaration(s);
                }
                self.emit(&format!("{}() {{}}", name));
                if let Some(constructors) = constructor {
                    for c in constructors {
                        let param_list: Vec<String> = c.params
                            .iter()
                            .filter(|p| p.type_node.as_str() != "type")
                            .map(|p| {
                                let cpp_t = type_to_cpp(&p.type_node);
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
                        self.visit_declaration(s);
                    }
                    self.indent_level -= 1;
                }
                self.emit("};");
            }
            Decl::StructDecl {
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
                    self.visit_declaration(s);
                }
                for s in static_block {
                    self.visit_declaration(s);
                }
                if let Some(constructors) = constructor {
                    for c in constructors {
                        let param_list: Vec<String> = c.params
                            .iter()
                            .filter(|p| p.type_node.as_str() != "type")
                            .map(|p| {
                                let cpp_t = type_to_cpp(&p.type_node);
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
                        self.visit_declaration(s);
                    }
                    self.indent_level -= 1;
                }
                self.emit("};");
            }
            Decl::FnDecl { name, params, return_type, body, is_exported: _ } => {
                let ret_type_str = if name == "main" {
                    "int".to_string()
                } else {
                    type_to_cpp(return_type)
                };

                let mut param_strs = Vec::new();
                for param in params {
                    if matches!(&param.type_node, BaseType::Pointer(_) | BaseType::Name(_) | BaseType::Modify(_) | BaseType::Copy(_)) {
                        self.pointer_vars.insert(param.name.clone());
                    }
                    let param_type = type_to_cpp(&param.type_node);
                    param_strs.push(format!("{} {}", param_type, param.name));
                }

                let safe_name = if name == "throw" { "_throw" } else { name };

                self.emit(&format!("{} {}({}) {{", ret_type_str, safe_name, param_strs.join(", ")));

                self.indent_level += 1;
                for s in body {
                    self.visit_statement(s);
                }
                self.indent_level -= 1;

                self.emit("}");
            }
            Decl::CustomDecl {
                name,
                settings,
                constructor,
                public_block,
                private_block,
                static_block,
                labels,
                data,
                handle_block,
                ..
            } => {
                self.custom_scopes.insert(name.clone());
                self.emit(&format!("class {} {{", name));
                self.emit("public:");
                self.indent_level += 1;
                self.emit("int __state = 0;");
                self.emit("bool has_yielded = false;");
                self.emit("bool is_done = false;");

                let has_custom_has_error = if let Some(handles) = handle_block {
                    handles.iter().any(|h| {
                        if let Decl::FnDecl { name: fn_name, .. } = h {
                            fn_name == "has_error" || fn_name == "_throw"
                        } else {
                            false
                        }
                    })
                } else {
                    false
                };

                if !has_custom_has_error {
                    self.emit("bool has_error = false;");
                }

                let has_display = if let Some(handles) = handle_block {
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
                    self.emit(
                        &format!("friend std::ostream& operator<<(std::ostream& os, {}& obj) {{", name)
                    );
                    self.emit("    os << obj.display();");
                    self.emit("    return os;");
                    self.emit("}");
                } else {
                    self.emit(
                        &format!("friend std::ostream& operator<<(std::ostream& os, const {}& obj) {{", name)
                    );
                    self.emit(&format!("    os << \"[object {}]\";", name));
                    self.emit("    return os;");
                    self.emit("}");
                }

                self.emit_operator_overloads(handle_block);

                let has_data_setting = settings.as_ref().map_or(false, |s| s.iter().any(|setting| matches!(setting, Setting::Data | Setting::All)));

                if let Some(d) = data {
                    let d_code = self.visit_expression(d);
                    let cpp_type = match d {
                        Expr::LiteralInt(_) => "int32_t",
                        Expr::LiteralFloat(_) => "double",
                        Expr::LiteralString(_) => "std::string",
                        Expr::LiteralBool(_) => "bool",
                        Expr::LiteralChar(_) => "char",
                        _ => "int32_t",
                    };
                    self.emit(&format!("{} data = {};", cpp_type, d_code));
                } else if has_data_setting {
                    self.emit("int32_t data[1024] = {0};");
                }

                let mut has_throw_handle = false;
                let mut error_handle_method: Option<String> = None;
                if let Some(handles) = handle_block {
                    for h in handles {
                        if let Decl::FnDecl { name, .. } = h {
                            if name == "throw" || name == "has_error" || name == "error" {
                                has_throw_handle = true;
                                error_handle_method = Some(if name == "throw" { "_throw".to_string() } else { name.clone() });
                                break;
                            }
                        }
                    }
                }

                if let Some(handles) = handle_block {
                    for h in handles {
                        if let Decl::FnDecl { name, params: _, return_type, body, is_exported: _ } = h {
                            if name == "call" {
                                let ret_type_str = type_to_cpp(return_type);
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
                                if let Some(ref lbl_map) = labels {
                                    for (l_name, lb) in lbl_map {
                                        if let Decl::LabelDecl { body: l_body, .. } = lb {
                                            let clean_name = l_name.replace("@", "");
                                            self.emit(&format!("{}:", clean_name));
                                            for stmt in l_body {
                                                self.visit_statement(stmt);
                                            }
                                        }
                                    }
                                }
                                self.indent_level -= 1;
                                self.emit("}");
                                if ret_type_str != "void" {
                                    self.emit("return {};");
                                }
                                if has_throw_handle {
                                    self.indent_level -= 1;
                                    self.emit("} catch (const std::exception& __e) {");
                                    self.indent_level += 1;
                                    let m_name = error_handle_method.as_deref().unwrap_or("_throw");
                                    self.emit(&format!("this->{}(__e);", m_name));
                                    self.indent_level -= 1;
                                    self.emit("}");
                                }
                                self.indent_level -= 1;
                                self.emit("}");
                                continue;
                            }
                        }
                        self.visit_declaration(h);
                    }
                }

                self.emit(&format!("{}() {{}}", name));

                if let Some(const_vec) = constructor {
                    for c in const_vec {
                        let param_list: Vec<String> = c.params
                            .iter()
                            .map(|p| format!("{} {}", type_to_cpp(&p.type_node), p.name))
                            .collect();
                        let arg_list: Vec<String> = c.params
                            .iter()
                            .map(|p| p.name.clone())
                            .collect();
                        self.emit(&format!("{}({}) {{ init({}); }}", name, param_list.join(", "), arg_list.join(", ")));
                        self.emit(&format!("void init({}) {{", param_list.join(", ")));
                        self.indent_level += 1;
                        for s in &c.body {
                            self.visit_statement(s);
                        }
                        self.indent_level -= 1;
                        self.emit("}");
                    }
                } else {
                    self.emit("void init() {}");
                }

                if let Some(pub_stmts) = public_block {
                    for s in pub_stmts {
                        self.visit_declaration(s);
                    }
                }
                if let Some(stat_stmts) = static_block {
                    for s in stat_stmts {
                        self.visit_declaration(s);
                    }
                }

                self.indent_level -= 1;
                self.emit("private:");
                self.indent_level += 1;

                if let Some(priv_stmts) = private_block {
                    for s in priv_stmts {
                        self.visit_declaration(s);
                    }
                }

                self.indent_level -= 1;
                self.emit("};");
            }
            Decl::LabelDecl { name, body } => {
                self.emit(&format!("void {}() {{", name));
                self.indent_level += 1;
                for s in body {
                    self.visit_statement(s);
                }
                self.indent_level -= 1;
                self.emit("}");
            }
            Decl::BlockDecl { name, statements, .. } => {
                let has_yield = statements.iter().any(|s| matches!(s, Stmt::YieldStmt(_)));
                self.emit(&format!("struct __block_type_{} {{", name));
                self.indent_level += 1;
                self.emit("int32_t __state = 0;");
                self.emit("bool has_yielded = false;");
                self.emit("bool is_done = false;");
                self.emit("bool has_error = false;");
                for s in statements {
                    if let Stmt::Declaration(Decl::VarDecl { name: f_name, type_node, .. }) = s {
                        let cpp_t = type_to_cpp(type_node);
                        self.emit(&format!("{} {};", cpp_t, f_name));
                    }
                }
                self.emit("void operator()() {");
                self.indent_level += 1;
                if has_yield {
                    self.emit("switch(this->__state) {");
                    self.emit("case 0:");
                    self.indent_level += 1;
                }
                for s in statements {
                    match s {
                        Stmt::Declaration(Decl::VarDecl { name: f_name, value, assign_op, .. }) => {
                            let val_code = self.visit_expression(value);
                            if val_code != "__default__" {
                                if assign_op == "->" {
                                    self.emit(&format!("fastlang_arrow_assign(this->{}, {});", f_name, val_code));
                                } else {
                                    self.emit(&format!("this->{} = {};", f_name, val_code));
                                }
                            }
                        }
                        _ => {
                            self.visit_statement(s);
                        }
                    }
                }
                if has_yield {
                    self.indent_level -= 1;
                    self.emit("}");
                    self.emit("this->has_yielded = false;");
                    self.emit("this->is_done = true;");
                }
                self.indent_level -= 1;
                self.emit("}");
                self.indent_level -= 1;
                self.emit(&format!("}} {};", name));
            }
            _ => {
                self.emit(&format!("// TODO: unimplemented declaration {:?}", decl));
            }
        }
    }
}
