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
        BaseType::Custom { name, generics, .. } | BaseType::Class { name, generics, .. } => {
            if generics.is_empty() {
                "".to_string()
            } else {
                let gen_strs: Vec<String> = generics.iter().map(type_to_cpp).collect();
                format!("{}<{}>", name, gen_strs.join(", "))
            }
        }
        BaseType::Generic(inner_vec) => {
            let strs: Vec<String> = inner_vec.iter().map(type_to_cpp).collect();
            strs.join(", ")
        }
        BaseType::Pointer(inner) => format!("{}*", type_to_cpp(inner)),
        BaseType::Name(inner) => {
            if inner.as_str() == BaseType::Unknown.as_str() {
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
        BaseType::Array { base_type, .. } => format!("std::vector<{}>", type_to_cpp(base_type)), // TODO: fix this
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
                    self.emit(&format!("{}.arrow_assign({});", target_code, val_code));
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
            Stmt::EnableStmt(_) | Stmt::DisableStmt(_) => {}
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
            Decl::VarDecl { name, type_node, value, editability, assign_op, place, .. } => {
                let val_code = self.visit_expression(value);
                let is_param = val_code == "__param__";
                let cpp_type = type_to_cpp(type_node);

                let is_const = editability == &Editability::NotEditable;
                let const_prefix = if is_const { "const " } else { "" };

                if is_param {
                    self.emit(&format!("{}{} {};", const_prefix, cpp_type, name));
                } else if assign_op == "->" {
                    self.emit(&format!("{}{} {};", const_prefix, cpp_type, name));
                    self.emit(&format!("{}.arrow_assign({});", name, val_code));
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

                if val_code == "__param__" {
                    self.emit(&format!("{}{} {}[{}];", const_prefix, cpp_type, name, len_code));
                } else {
                    self.emit(
                        &format!(
                            "{}{} {}[{}] = {};",
                            const_prefix,
                            cpp_type,
                            name,
                            len_code,
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
                    self.visit_declaration(s);
                }
                for s in static_block {
                    self.emit("static ");
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
                let mut ret_type_str = if name == "main" {
                    "int".to_string()
                } else {
                    type_to_cpp(return_type)
                };

                let mut param_strs = Vec::new();
                for param in params {
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
                params,
                constructor,
                statements,
                public_block,
                private_block,
                flags,
                labels,
                data,
                handle_block,
                ..
            } => {
                if let Some(generics) = params {
                    if !generics.is_empty() {
                        let gen_params: Vec<String> = generics
                            .iter()
                            .map(|g| format!("typename {}", g.name))
                            .collect();
                        self.emit(&format!("template <{}>", gen_params.join(", ")));
                    }
                }
                self.custom_scopes.insert(name.clone());
                self.emit(&format!("class {} {{", name));
                self.emit("public:");
                self.indent_level += 1;
                self.emit("int __state = 0;");

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

                if let Some(d) = data {
                    let d_code = self.visit_expression(d);
                    self.emit(&format!("auto data = {};", d_code));
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
                if let Some(handles) = handle_block {
                    for h in handles {
                        if let Decl::FnDecl { name, .. } = h {
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

                let mut unified_return_type = "void".to_string();
                if let Some(handles) = handle_block {
                    for h in handles {
                        if let Decl::FnDecl { name, return_type: rt, .. } = h {
                            if name == "call" || name == "leave" || name == "yield" {
                                unified_return_type = type_to_cpp(rt);
                                break;
                            }
                        }
                    }
                }

                if let Some(handles) = handle_block {
                    for h in handles {
                        if let Decl::FnDecl { name, params, return_type, body, is_exported } = h {
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
                        self.visit_declaration(h);
                    }
                }

                let has_call_handle = handle_block.as_ref().map_or(false, |handles: &Vec<Decl>| {
                    handles.iter().any(|h| {
                        if let Decl::FnDecl { name, .. } = h { name == "call" } else { false }
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
                        let param_list: Vec<String> = c.params
                            .iter()
                            .map(|p| format!("{} {}", type_to_cpp(&p.type_node), p.name))
                            .collect();
                        self.emit(&format!("void init({}) {{", param_list.join(", ")));
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
            _ => {
                self.emit(&format!("// TODO: unimplemented declaration {:?}", decl));
            }
        }
    }
}
