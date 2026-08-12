use crate::codegen::generator::CodeGenerator;
use crate::parser::ast::*;

impl CodeGenerator {
    pub(crate) fn visit_statement(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::ExpressionStmt(expr) => {
                let code = self.visit_expression(expr);
                self.emit(&format!("{};", code));
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
                    crate::parser::ast::TypeNode::Simple(r) => r.base_type.clone(),
                    crate::parser::ast::TypeNode::Generic(g) => g.base_type.clone(),
                });
                let size = type_node.as_ref().and_then(|t| match t {
                    crate::parser::ast::TypeNode::Simple(r) => r.size,
                    crate::parser::ast::TypeNode::Generic(_) => None,
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
                    let inc_code = self.visit_expression(inc);
                    self.emit(&format!("{};", inc_code));
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
                if let Some(c) = constructor {
                    let param_list: Vec<String> = c
                        .params
                        .iter()
                        .filter(|p| {
                            p.type_node
                                .as_ref()
                                .map(|t| match t {
                                    crate::parser::ast::TypeNode::Simple(r) => r.base_type.clone(),
                                    crate::parser::ast::TypeNode::Generic(g) => g.base_type.clone(),
                                })
                                .unwrap_or("unknown".to_string())
                                != "type"
                        })
                        .map(|p| {
                            let cpp_t = p
                                .type_node
                                .as_ref()
                                .map(|t| match t {
                                    crate::parser::ast::TypeNode::Simple(r) => {
                                        self.map_type(Some(r.base_type.as_str()), r.size)
                                    }
                                    crate::parser::ast::TypeNode::Generic(g) => {
                                        self.map_type(Some(g.base_type.as_str()), None)
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
                if let Some(c) = constructor {
                    let param_list: Vec<String> = c
                        .params
                        .iter()
                        .filter(|p| {
                            p.type_node
                                .as_ref()
                                .map(|t| match t {
                                    crate::parser::ast::TypeNode::Simple(r) => r.base_type.clone(),
                                    crate::parser::ast::TypeNode::Generic(g) => g.base_type.clone(),
                                })
                                .unwrap_or("unknown".to_string())
                                != "type"
                        })
                        .map(|p| {
                            let cpp_t = p
                                .type_node
                                .as_ref()
                                .map(|t| match t {
                                    crate::parser::ast::TypeNode::Simple(r) => {
                                        self.map_type(Some(r.base_type.as_str()), r.size)
                                    }
                                    crate::parser::ast::TypeNode::Generic(g) => {
                                        self.map_type(Some(g.base_type.as_str()), None)
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
            Stmt::FnDecl {
                name,
                params,
                return_type,
                body,
                is_exported: _, // هنتجاهلها دلوقتي لحد ما نحتاجها في الـ Shared Libraries
            } => {
                // 1. تحديد نوع الإرجاع (Return Type)
                let mut ret_type_str = match return_type {
                    crate::parser::ast::TypeNode::Simple(r) => {
                        self.map_type(Some(r.base_type.as_str()), r.size)
                    }
                    crate::parser::ast::TypeNode::Generic(g) => {
                        self.map_type(Some(g.base_type.as_str()), None)
                    }
                };

                // Edge case for C++ main function
                if name == "main" {
                    ret_type_str = "int".to_string();
                }

                // 2. تجميع المعاملات (Parameters)
                let mut param_strs = Vec::new();
                for param in params {
                    let param_type = match &param.type_node {
                        Some(crate::parser::ast::TypeNode::Simple(r)) => {
                            self.map_type(Some(r.base_type.as_str()), r.size)
                        }
                        Some(crate::parser::ast::TypeNode::Generic(g)) => {
                            self.map_type(Some(g.base_type.as_str()), None)
                        }
                        None => "auto".to_string(), // Fallback
                    };
                    param_strs.push(format!("{} {}", param_type, param.name));
                }

                // 3. كتابة توقيع الدالة (Function Signature)
                self.emit(&format!(
                    "{} {}({}) {{",
                    ret_type_str,
                    name,
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
            _ => {
                self.emit(&format!("// TODO: unimplemented statement {:?}", stmt));
            }
        }
    }
}
