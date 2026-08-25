use crate::backend::cpp::generator::CodeGenerator;
use crate::frontend::parser::ast::*;

pub(crate) fn type_to_cpp(t: &BaseType) -> String {
    match t {
        BaseType::Int8 => "int8_t".to_string(),
        BaseType::Int16 => "int16_t".to_string(),
        BaseType::Int32 => "int32_t".to_string(),
        BaseType::Int64 => "int64_t".to_string(),
        BaseType::Int128 => "__int128".to_string(),
        BaseType::UInt8 => "uint8_t".to_string(),
        BaseType::UInt16 => "uint16_t".to_string(),
        BaseType::UInt32 => "uint32_t".to_string(),
        BaseType::UInt64 => "uint64_t".to_string(),
        BaseType::UInt128 => "unsigned __int128".to_string(),
        BaseType::USize => "size_t".to_string(),
        BaseType::ISize => "ptrdiff_t".to_string(),
        BaseType::Float32 => "float".to_string(),
        BaseType::Float64 => "double".to_string(),
        BaseType::Float128 => "long double".to_string(),
        BaseType::Char => "char".to_string(),
        BaseType::Bool => "bool".to_string(),
        BaseType::Void => "void".to_string(),
        BaseType::Custom { name, generics, .. } | BaseType::Class { name, generics, .. } | BaseType::Struct { name, generics, .. } => {
            if generics.is_empty() {
                name.clone()
            } else {
                let gen_strs: Vec<String> = generics.iter().map(type_to_cpp).collect();
                format!("{}<{}>", name, gen_strs.join(", "))
            }
        }
        BaseType::Enum { name, generics, .. } | BaseType::Blueprint { name, generics, .. } => {
            if generics.is_empty() {
                name.clone()
            } else {
                let gen_strs: Vec<String> = generics.iter().map(type_to_cpp).collect();
                format!("{}<{}>", name, gen_strs.join(", "))
            }
        }
        BaseType::GenericParam(name) => name.clone(),
        BaseType::Generic(inner_vec) => {
            let strs: Vec<String> = inner_vec.iter().map(type_to_cpp).collect();
            strs.join(", ")
        }
        BaseType::Pointer(inner) => format!("{}*", type_to_cpp(inner)),
        BaseType::Name(inner) => {
            if let BaseType::Generic(ref vec) = &**inner {
                if vec.len() == 1 && matches!(vec[0], BaseType::Fn { .. } | BaseType::Method { .. }) {
                    return format!("fastlang_name<{}>", type_to_cpp(&vec[0]));
                }
            }
            if matches!(&**inner, BaseType::Fn { .. } | BaseType::Method { .. }) {
                format!("fastlang_name<{}>", type_to_cpp(inner))
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
                "fastlang_name<std::function<void()>>".to_string()
            } else {
                format!("fastlang_name<{}>", type_to_cpp(inner))
            }
        }
        BaseType::Array { base_type, .. } => format!("fastlang_slice<{}>", type_to_cpp(base_type)),
        BaseType::Method { return_type, params } | BaseType::Fn { return_type, params } => {
            if return_type.as_ref() == &BaseType::Unknown {
                "auto".to_string()
            } else {
                let ret = type_to_cpp(return_type);
                let p_types: Vec<String> = params.iter().map(type_to_cpp).collect();
                format!("std::function<{}({})>", ret, p_types.join(", "))
            }
        }
        BaseType::Unknown => "auto".to_string(),
        _ => t.as_str(),
    }
}

fn expr_uses_flag(expr: &Expr, flag: &str) -> bool {
    match expr {
        Expr::Identifier(id) => id == flag,
        Expr::PropertyAccess { property, .. } => property == flag,
        Expr::BinaryOp { left, right, .. } => expr_uses_flag(left, flag) || expr_uses_flag(right, flag),
        Expr::UnaryOp { operand, .. } => expr_uses_flag(operand, flag),
        Expr::Call { callee, args } => {
            expr_uses_flag(callee, flag) || args.iter().any(|a| expr_uses_flag(a, flag))
        }
        _ => false,
    }
}

fn stmts_use_flag(stmts: &[Stmt], flag: &str) -> bool {
    for s in stmts {
        match s {
            Stmt::ReassignStmt { target, value, .. } => {
                if expr_uses_flag(target, flag) || expr_uses_flag(value, flag) {
                    return true;
                }
            }
            Stmt::ExpressionStmt(expr) => {
                if expr_uses_flag(expr, flag) {
                    return true;
                }
            }
            Stmt::YieldStmt(_) if flag == "yielded" || flag == "has_yielded" => return true,
            Stmt::LeaveStmt if flag == "leaved" => return true,
            Stmt::IfStmt { condition, then_block, else_block, .. } => {
                if expr_uses_flag(condition, flag) || stmts_use_flag(then_block, flag) {
                    return true;
                }
                if let Some(eb) = else_block {
                    if stmts_use_flag(eb, flag) {
                        return true;
                    }
                }
            }
            Stmt::WhileStmt { condition, body } | Stmt::DoWhileStmt { condition, body } => {
                if expr_uses_flag(condition, flag) {
                    return true;
                }
                if let EitherBlock::Inline(b) = body {
                    if stmts_use_flag(b, flag) {
                        return true;
                    }
                }
            }
            Stmt::ForStmt { body, .. } | Stmt::ForInStmt { body, .. } => {
                if let EitherBlock::Inline(b) = body {
                    if stmts_use_flag(b, flag) {
                        return true;
                    }
                }
            }
            Stmt::SwitchStmt { cases, .. } => {
                if stmts_use_flag(cases, flag) {
                    return true;
                }
            }
            Stmt::CaseStmt { body, .. } => {
                if stmts_use_flag(body, flag) {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

fn decls_use_flag(decls: &[Decl], flag: &str) -> bool {
    for d in decls {
        match d {
            Decl::FnDecl { body, .. } | Decl::MicroDecl { body, .. } => {
                if stmts_use_flag(body, flag) {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
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

                self.emit(&format!("for ({} : fastlang_iterable({})) {{", item_code, iterable_code));
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
            Stmt::DoWhileStmt { body, condition } => {
                self.emit("do {");
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
                let cond_code = self.visit_expression(condition);
                self.emit(&format!("}} while ({});", cond_code));
            }
            Stmt::SwitchStmt { condition, cases, .. } => {
                let cond_code = self.visit_expression(condition);
                self.emit("{");
                self.indent_level += 1;
                self.emit(&format!("auto&& __match_val = {};", cond_code));

                let mut first = true;
                for s in cases {
                    if let Stmt::CaseStmt { option, body, .. } = s {
                        if matches!(option, Expr::Identifier(name) if name == "void") {
                            if first {
                                self.emit("{");
                            } else {
                                self.emit("else {");
                            }
                            self.indent_level += 1;
                            for case_stmt in body {
                                self.visit_statement(case_stmt);
                            }
                            self.indent_level -= 1;
                            self.emit("}");
                        } else {
                            let branch_prefix = if first { "if" } else { "else if" };
                            match option {
                                Expr::Call { callee, args } => {
                                    let callee_code = self.visit_expression(callee);
                                    let variant_name = if let Some(idx) = callee_code.rfind("::") {
                                        &callee_code[idx + 2..]
                                    } else {
                                        &callee_code
                                    };
                                    self.emit(&format!("{} (__match_val.tag == std::decay_t<decltype(__match_val)>::Tag::{}) {{", branch_prefix, variant_name));
                                    self.indent_level += 1;
                                    if !args.is_empty() {
                                        self.emit(&format!("if (std::holds_alternative<typename std::decay_t<decltype(__match_val)>::{}_Payload>(__match_val.data)) {{", variant_name));
                                        self.indent_level += 1;
                                        let var_names: Vec<String> = args.iter().filter_map(|a| {
                                            if let Expr::Identifier(n) = a { Some(n.clone()) } else { None }
                                        }).collect();
                                        if !var_names.is_empty() {
                                            for vn in &var_names {
                                                self.pointer_vars.insert(vn.clone());
                                            }
                                            self.emit(&format!("auto [{}] = std::get<typename std::decay_t<decltype(__match_val)>::{}_Payload>(__match_val.data);", var_names.join(", "), variant_name));
                                        }
                                        for case_stmt in body {
                                            self.visit_statement(case_stmt);
                                        }
                                        self.indent_level -= 1;
                                        self.emit("}");
                                    } else {
                                        for case_stmt in body {
                                            self.visit_statement(case_stmt);
                                        }
                                    }
                                    self.indent_level -= 1;
                                    self.emit("}");
                                }
                                Expr::Instantiate { target, args } => {
                                    let target_code = self.visit_expression(target);
                                    let variant_name = if let Some(idx) = target_code.rfind("::") {
                                        &target_code[idx + 2..]
                                    } else {
                                        &target_code
                                    };
                                    self.emit(&format!("{} (__match_val.tag == std::decay_t<decltype(__match_val)>::Tag::{}) {{", branch_prefix, variant_name));
                                    self.indent_level += 1;
                                    if !args.is_empty() {
                                        self.emit(&format!("if (std::holds_alternative<typename std::decay_t<decltype(__match_val)>::{}_Payload>(__match_val.data)) {{", variant_name));
                                        self.indent_level += 1;
                                        if let Expr::ObjectLiteral(stmts) = &args[0] {
                                            let var_names: Vec<String> = stmts.iter().filter_map(|s| {
                                                if let Stmt::Declaration(Decl::VarDecl { name, .. }) = s {
                                                    Some(name.clone())
                                                } else {
                                                    None
                                                }
                                            }).collect();
                                            if !var_names.is_empty() {
                                                for vn in &var_names {
                                                    self.pointer_vars.insert(vn.clone());
                                                }
                                                self.emit(&format!("auto [{}] = std::get<typename std::decay_t<decltype(__match_val)>::{}_Payload>(__match_val.data);", var_names.join(", "), variant_name));
                                            }
                                        }
                                        for case_stmt in body {
                                            self.visit_statement(case_stmt);
                                        }
                                        self.indent_level -= 1;
                                        self.emit("}");
                                    } else {
                                        for case_stmt in body {
                                            self.visit_statement(case_stmt);
                                        }
                                    }
                                    self.indent_level -= 1;
                                    self.emit("}");
                                }
                                Expr::Identifier(ident) => {
                                    self.emit(&format!("{} (__match_val.tag == std::decay_t<decltype(__match_val)>::Tag::{}) {{", branch_prefix, ident));
                                    self.indent_level += 1;
                                    for case_stmt in body {
                                        self.visit_statement(case_stmt);
                                    }
                                    self.indent_level -= 1;
                                    self.emit("}");
                                }
                                _ => {
                                    let val_code = self.visit_expression(option);
                                    self.emit(&format!("{} (fastlang_match_eq(__match_val, {})) {{", branch_prefix, val_code));
                                    self.indent_level += 1;
                                    for case_stmt in body {
                                        self.visit_statement(case_stmt);
                                    }
                                    self.indent_level -= 1;
                                    self.emit("}");
                                }
                            }
                        }
                        first = false;
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
            Stmt::UsingStmt(_) => {}
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
                let expr_code = self.visit_expression(expr);
                if expr_code.starts_with("new ") {
                    self.emit(&format!("throw *({});", expr_code));
                } else {
                    self.emit(&format!("throw {};", expr_code));
                }
            }
            Stmt::TryCatchStmt { try_block, catch_param, catch_block } => {
                self.emit("try {");
                self.indent_level += 1;
                for s in try_block {
                    self.visit_statement(s);
                }
                self.indent_level -= 1;
                self.emit(&format!("}} catch (const fast_std::Error& {}) {{", catch_param));
                self.indent_level += 1;
                for s in catch_block {
                    self.visit_statement(s);
                }
                self.indent_level -= 1;
                self.emit(&format!("}} catch (const fast_std::Error* __{}_ptr) {{", catch_param));
                self.indent_level += 1;
                self.emit(&format!("const fast_std::Error& {} = *__{}_ptr;", catch_param, catch_param));
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
            Decl::EnumDecl { name, generics, variants, .. } => {
                self.enum_types.insert(name.clone());
                let is_generic = !generics.is_empty();
                let has_payloads = variants.iter().any(|v| !matches!(v.payload, EnumVariantPayload::None));

                if !is_generic && !has_payloads {
                    self.emit(&format!("enum class {} {{", name));
                    self.indent_level += 1;
                    for (i, variant) in variants.iter().enumerate() {
                        let comma = if i < variants.len() - 1 { "," } else { "" };
                        self.emit(&format!("{}{}", variant.name, comma));
                    }
                    self.indent_level -= 1;
                    self.emit("};");

                    for variant in variants.iter() {
                        self.emit(&format!("inline constexpr auto {} = {}::{};", variant.name, name, variant.name));
                    }

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
                } else {
                    for v in variants.iter() {
                        if matches!(v.payload, EnumVariantPayload::None) {
                            self.emit(&format!("struct fastlang_tag_{}_{} {{}};", name, v.name));
                            self.emit(&format!("inline constexpr fastlang_tag_{}_{} {};", name, v.name, v.name));
                        }
                    }

                    let generic_params: Vec<String> = generics.iter().map(|g| format!("typename {}", g.as_str())).collect();
                    let template_prefix = if is_generic {
                        format!("template <{}>\n", generic_params.join(", "))
                    } else {
                        String::new()
                    };

                    self.emit(&format!("{}struct {} {{", template_prefix, name));
                    self.indent_level += 1;

                    self.emit("enum class Tag {");
                    self.indent_level += 1;
                    for (i, v) in variants.iter().enumerate() {
                        let comma = if i < variants.len() - 1 { "," } else { "" };
                        self.emit(&format!("{}{}", v.name, comma));
                    }
                    self.indent_level -= 1;
                    self.emit("} tag;");

                    for v in variants.iter() {
                        match &v.payload {
                            EnumVariantPayload::None => {},
                            EnumVariantPayload::Tuple(types) => {
                                let field_strs: Vec<String> = types.iter().enumerate().map(|(idx, t)| {
                                    format!("{} _{};", type_to_cpp(t), idx)
                                }).collect();
                                self.emit(&format!("struct {}_Payload {{ {} }};", v.name, field_strs.join(" ")));
                            }
                            EnumVariantPayload::Struct(fields) => {
                                let field_strs: Vec<String> = fields.iter().map(|f| {
                                    format!("{} {};", type_to_cpp(&f.type_node), f.name)
                                }).collect();
                                self.emit(&format!("struct {}_Payload {{ {} }};", v.name, field_strs.join(" ")));
                            }
                        }
                    }

                    let mut payload_types = vec!["std::monostate".to_string()];
                    for v in variants.iter() {
                        if !matches!(v.payload, EnumVariantPayload::None) {
                            payload_types.push(format!("{}_Payload", v.name));
                        }
                    }
                    self.emit(&format!("std::variant<{}> data;", payload_types.join(", ")));

                    for v in variants.iter() {
                        match &v.payload {
                            EnumVariantPayload::None => {
                                self.emit(&format!("static {} {}() {{ {} r; r.tag = Tag::{}; return r; }}", name, v.name, name, v.name));
                            }
                            EnumVariantPayload::Tuple(types) => {
                                let param_strs: Vec<String> = types.iter().enumerate().map(|(idx, t)| {
                                    format!("{} _{}", type_to_cpp(t), idx)
                                }).collect();
                                let init_strs: Vec<String> = types.iter().enumerate().map(|(idx, _)| {
                                    format!("_{}", idx)
                                }).collect();
                                self.emit(&format!("static {} {}({}) {{ {} r; r.tag = Tag::{}; r.data = {}_Payload{{{}}}; return r; }}",
                                    name, v.name, param_strs.join(", "), name, v.name, v.name, init_strs.join(", ")));
                            }
                            EnumVariantPayload::Struct(fields) => {
                                let param_strs: Vec<String> = fields.iter().map(|f| {
                                    format!("{} {}", type_to_cpp(&f.type_node), f.name)
                                }).collect();
                                let init_strs: Vec<String> = fields.iter().map(|f| {
                                    f.name.clone()
                                }).collect();
                                self.emit(&format!("static {} {}({}) {{ {} r; r.tag = Tag::{}; r.data = {}_Payload{{{}}}; return r; }}",
                                    name, v.name, param_strs.join(", "), name, v.name, v.name, init_strs.join(", ")));
                            }
                        }
                        self.emit(&format!("bool is_{}() const {{ return tag == Tag::{}; }}", v.name, v.name));
                    }

                    let default_variant = variants.iter().find(|v| matches!(v.payload, EnumVariantPayload::None)).unwrap_or(&variants[0]);
                    self.emit(&format!("{}() : tag(Tag::{}) {{}}", name, default_variant.name));
                    for v in variants.iter() {
                        if matches!(v.payload, EnumVariantPayload::None) {
                            self.emit(&format!("{}(fastlang_tag_{}_{}) : tag(Tag::{}) {{}}", name, name, v.name, v.name));
                            self.emit(&format!("{}& operator=(fastlang_tag_{}_{}) {{ tag = Tag::{}; return *this; }}", name, name, v.name, v.name));
                            self.emit(&format!("bool operator==(fastlang_tag_{}_{}) const {{ return tag == Tag::{}; }}", name, v.name, v.name));
                            self.emit(&format!("bool operator!=(fastlang_tag_{}_{}) const {{ return tag != Tag::{}; }}", name, v.name, v.name));
                        }
                    }
                    if is_generic && variants.iter().any(|v| v.name == "Some") {
                        self.emit(&format!("template <typename... U> {}(const {}<U...>& other) : tag(static_cast<Tag>(other.tag)) {{ if (other.is_Some()) {{ data = Some_Payload{{ T(std::get<1>(other.data)._0) }}; }} }}", name, name));
                        self.emit(&format!("template <typename... U> {}& operator=(const {}<U...>& other) {{ tag = static_cast<Tag>(other.tag); if (other.is_Some()) {{ data = Some_Payload{{ T(std::get<1>(other.data)._0) }}; }} return *this; }}", name, name));
                    }
                    self.emit("bool operator==(Tag t) const { return tag == t; }");
                    self.emit("bool operator!=(Tag t) const { return tag != t; }");
                    self.emit(&format!("bool operator==(const {}& other) const {{ return tag == other.tag; }}", name));
                    self.emit(&format!("bool operator!=(const {}& other) const {{ return tag != other.tag; }}", name));

                    self.indent_level -= 1;
                    self.emit("};");

                    for v in variants.iter() {
                        if let EnumVariantPayload::Tuple(types) = &v.payload {
                            if is_generic {
                                let template_params: Vec<String> = generics.iter().map(|g| format!("typename {}", g.as_str())).collect();
                                let param_strs: Vec<String> = types.iter().enumerate().map(|(idx, t)| format!("{} _{}", type_to_cpp(t), idx)).collect();
                                let arg_strs: Vec<String> = types.iter().enumerate().map(|(idx, _)| format!("_{}", idx)).collect();
                                let gen_args: Vec<String> = generics.iter().map(|g| g.as_str()).collect();
                                self.emit(&format!("template <{}> inline {}<{}> {}({}) {{ return {}<{}>::{}({}); }}",
                                    template_params.join(", "), name, gen_args.join(", "), v.name, param_strs.join(", "), name, gen_args.join(", "), v.name, arg_strs.join(", ")));
                            } else {
                                let param_strs: Vec<String> = types.iter().enumerate().map(|(idx, t)| format!("{} _{}", type_to_cpp(t), idx)).collect();
                                let arg_strs: Vec<String> = types.iter().enumerate().map(|(idx, _)| format!("_{}", idx)).collect();
                                self.emit(&format!("inline {} {}({}) {{ return {}::{}({}); }}",
                                    name, v.name, param_strs.join(", "), name, v.name, arg_strs.join(", ")));
                            }
                        }
                    }

                    let obj_type_str = if is_generic {
                        let generic_args: Vec<String> = generics.iter().map(|g| g.as_str()).collect();
                        format!("{}<{}>", name, generic_args.join(", "))
                    } else {
                        name.clone()
                    };

                    self.emit(&format!("{}inline std::ostream& operator<<(std::ostream& os, const {}& obj) {{", template_prefix, obj_type_str));
                    self.indent_level += 1;
                    self.emit("switch (obj.tag) {");
                    self.indent_level += 1;
                    for v in variants.iter() {
                        self.emit(&format!("case {}::Tag::{}: os << \"{}\"; break;", obj_type_str, v.name, v.name));
                    }
                    self.indent_level -= 1;
                    self.emit("}");
                    self.emit("return os;");
                    self.indent_level -= 1;
                    self.emit("}");
                }
            }
            Decl::VarDecl { name, type_node, value, editability, assign_op, place: _, .. } => {
                if matches!(type_node, BaseType::Custom { .. } | BaseType::Class { .. }) {
                    self.custom_scopes.insert(name.clone());
                }
                if matches!(type_node, BaseType::Pointer(_) | BaseType::Modify(_) | BaseType::Copy(_)) {
                    self.pointer_vars.insert(name.clone());
                } else if let BaseType::Name(inner) = type_node {
                    if !matches!(&**inner, BaseType::Fn { .. } | BaseType::Method { .. }) {
                        self.pointer_vars.insert(name.clone());
                    }
                }
                let is_param = match value {
                    Expr::Identifier(s) if s == "__param__" => true,
                    _ => false,
                };
                let is_const = editability == &Editability::NotEditable;
                let const_prefix = if is_const { "const " } else { "" };
                let cpp_type = type_to_cpp(type_node);

                let is_value_type = !matches!(type_node, BaseType::Pointer(_) | BaseType::Name(_) | BaseType::Modify(_) | BaseType::Copy(_));
                let mut val_code = if is_value_type {
                    if let Expr::New { type_node: inner, target } = value {
                        match &**target {
                            Expr::Instantiate { args, .. } => {
                                let arg_strs: Vec<String> = args.iter().map(|a| self.visit_expression(a)).collect();
                                format!("{}({})", type_to_cpp(inner), arg_strs.join(", "))
                            }
                            Expr::ArrayLiteral(elems) => {
                                let elem_strs: Vec<String> = elems.iter().map(|e| self.visit_expression(e)).collect();
                                format!("{}{{{}}}", type_to_cpp(inner), elem_strs.join(", "))
                            }
                            _ => {
                                let target_code = self.visit_expression(target);
                                if target_code == "__default__" || target_code == "{}" || target_code.is_empty() {
                                    format!("{}()", type_to_cpp(inner))
                                } else {
                                    format!("{}({})", type_to_cpp(inner), target_code)
                                }
                            }
                        }
                    } else {
                        self.visit_expression(value)
                    }
                } else {
                    self.visit_expression(value)
                };

                if cpp_type.contains('<') {
                    let raw_name = cpp_type.split('<').next().unwrap_or(&cpp_type);
                    let prefix = format!("{}::", raw_name);
                    if val_code.starts_with(&prefix) {
                        val_code = format!("{}::{}", cpp_type, &val_code[prefix.len()..]);
                    }
                }

                if is_param || matches!(value, Expr::Default(None)) && assign_op.is_empty() {
                    self.emit(&format!("{}{} {};", const_prefix, cpp_type, name));
                } else if assign_op == "->" {
                    self.emit(&format!("{}{} {};", const_prefix, cpp_type, name));
                    if val_code.starts_with('{') && val_code.ends_with('}') {
                        self.emit(&format!("{}.arrow_assign({});", name, val_code));
                    } else {
                        self.emit(&format!("fastlang_arrow_assign({}, {});", name, val_code));
                    }
                } else if assign_op == "=" || assign_op.is_empty() {
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
                    if matches!(val, Expr::Default(None)) || val_code == "{}" {
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

                let len_str = if len_code == "0" && !matches!(value, Expr::Default(None)) && val_code != "{}" {
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
                let old_in_class = self.in_class_or_scope;
                self.in_class_or_scope = true;
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
                if decls_use_flag(public_block, "broken") || decls_use_flag(private_block, "broken") || decls_use_flag(handle_block, "broken") {
                    self.emit("bool broken = false;");
                }
                if decls_use_flag(public_block, "is_done") || decls_use_flag(private_block, "is_done") || decls_use_flag(handle_block, "is_done") {
                    self.emit("bool is_done = false;");
                }
                if decls_use_flag(public_block, "yielded") || decls_use_flag(private_block, "yielded") || decls_use_flag(handle_block, "yielded") {
                    self.emit("bool yielded = false;");
                }
                if decls_use_flag(public_block, "leaved") || decls_use_flag(private_block, "leaved") || decls_use_flag(handle_block, "leaved") {
                    self.emit("bool leaved = false;");
                }
                if decls_use_flag(public_block, "returned") || decls_use_flag(private_block, "returned") || decls_use_flag(handle_block, "returned") {
                    self.emit("bool returned = false;");
                }
                if decls_use_flag(public_block, "continued") || decls_use_flag(private_block, "continued") || decls_use_flag(handle_block, "continued") {
                    self.emit("bool continued = false;");
                }

                let has_display = handle_block.iter().any(|h| {
                    if let Decl::FnDecl { name: fn_name, .. } = h {
                        fn_name == "display"
                    } else {
                        false
                    }
                });

                if has_display {
                    self.emit(
                        &format!("friend std::ostream& operator<<(std::ostream& os, const {}& obj) {{", name)
                    );
                    self.emit(&format!("    os << const_cast<{}&>(obj).display();", name));
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

                self.emit_operator_overloads(&Some(handle_block.clone()));

                for s in public_block {
                    self.visit_declaration(s);
                }
                for h in handle_block {
                    self.visit_declaration(h);
                }
                for s in static_block {
                    self.visit_declaration(s);
                }

                let has_default_ctor = constructor.as_ref().map(|ctors| {
                    ctors.iter().any(|c| c.params.iter().all(|p| p.type_node.as_str() == "type"))
                }).unwrap_or(false);

                if let Some(ext) = extends {
                    self.emit(&format!("using {}::{};", ext, ext));
                }
                if !has_default_ctor {
                    self.emit(&format!("{}() {{}}", name));
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

                        if c.params.len() == 1 && matches!(&c.params[0].type_node, BaseType::Array { base_type, .. } if matches!(**base_type, BaseType::Char)) {
                            self.emit(&format!("template <size_t N> {}(const char (&arr)[N]) : {}(fastlang_slice<char>(arr)) {{}}", name, name));
                            self.emit(&format!("{}(const char* s) : {}(fastlang_slice<char>(s, s ? std::char_traits<char>::length(s) : 0)) {{}}", name, name));
                        }
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
                self.in_class_or_scope = old_in_class;
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
                let old_in_class = self.in_class_or_scope;
                self.in_class_or_scope = true;
                self.emit(&format!("struct {} {{", name));
                self.indent_level += 1;
                if decls_use_flag(public_block, "broken") || decls_use_flag(private_block, "broken") || decls_use_flag(static_block, "broken") {
                    self.emit("bool broken = false;");
                }
                if decls_use_flag(public_block, "is_done") || decls_use_flag(private_block, "is_done") || decls_use_flag(static_block, "is_done") {
                    self.emit("bool is_done = false;");
                }
                if decls_use_flag(public_block, "yielded") || decls_use_flag(private_block, "yielded") || decls_use_flag(static_block, "yielded") {
                    self.emit("bool yielded = false;");
                }
                if decls_use_flag(public_block, "leaved") || decls_use_flag(private_block, "leaved") || decls_use_flag(static_block, "leaved") {
                    self.emit("bool leaved = false;");
                }
                if decls_use_flag(public_block, "returned") || decls_use_flag(private_block, "returned") || decls_use_flag(static_block, "returned") {
                    self.emit("bool returned = false;");
                }
                if decls_use_flag(public_block, "continued") || decls_use_flag(private_block, "continued") || decls_use_flag(static_block, "continued") {
                    self.emit("bool continued = false;");
                }
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
                self.in_class_or_scope = old_in_class;
            }
            Decl::FnDecl { name, params, return_type, body, is_virtual, is_abstract, is_exported: _ } => {
                let ret_type_str = if name == "main" {
                    "int".to_string()
                } else {
                    type_to_cpp(return_type)
                };

                let mut param_strs = Vec::new();
                for param in params {
                    if matches!(&param.type_node, BaseType::Pointer(_) | BaseType::Modify(_) | BaseType::Copy(_)) {
                        self.pointer_vars.insert(param.name.clone());
                    } else if let BaseType::Name(inner) = &param.type_node {
                        if !matches!(&**inner, BaseType::Fn { .. } | BaseType::Method { .. }) {
                            self.pointer_vars.insert(param.name.clone());
                        }
                    }
                    let param_type = type_to_cpp(&param.type_node);
                    param_strs.push(format!("{} {}", param_type, param.name));
                }

                let safe_name = if name == "throw" { "_throw" } else { name };

                if *is_abstract {
                    self.emit(&format!("virtual {} {}({}) = 0;", ret_type_str, safe_name, param_strs.join(", ")));
                    return;
                }

                let virtual_prefix = if *is_virtual || (self.in_class_or_scope && name != "main") { "virtual " } else { "" };
                self.emit(&format!("{}{} {}({}) {{", virtual_prefix, ret_type_str, safe_name, param_strs.join(", ")));

                self.indent_level += 1;
                if !self.in_class_or_scope {
                    if stmts_use_flag(body, "broken") {
                        self.emit("bool broken = false;");
                    }
                    if stmts_use_flag(body, "is_done") {
                        self.emit("bool is_done = false;");
                    }
                    if stmts_use_flag(body, "yielded") {
                        self.emit("bool yielded = false;");
                    }
                    if stmts_use_flag(body, "leaved") {
                        self.emit("bool leaved = false;");
                    }
                    if stmts_use_flag(body, "returned") {
                        self.emit("bool returned = false;");
                    }
                    if stmts_use_flag(body, "continued") {
                        self.emit("bool continued = false;");
                    }
                }
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
                let old_in_class = self.in_class_or_scope;
                self.in_class_or_scope = true;
                self.custom_scopes.insert(name.clone());
                self.custom_scope_types.insert(name.clone());
                self.emit(&format!("class {} {{", name));
                self.emit("public:");
                self.indent_level += 1;
                self.emit("int __state = 0;");
                self.emit("bool has_yielded = false;");
                self.emit("bool is_done = false;");
                let label_decls: Vec<Decl> = labels.as_ref().map(|l| l.values().cloned().collect()).unwrap_or_default();
                if decls_use_flag(&label_decls, "broken")
                   || decls_use_flag(public_block.as_deref().unwrap_or(&[]), "broken")
                   || decls_use_flag(handle_block.as_deref().unwrap_or(&[]), "broken") {
                    self.emit("bool broken = false;");
                }
                if decls_use_flag(&label_decls, "yielded")
                   || decls_use_flag(public_block.as_deref().unwrap_or(&[]), "yielded")
                   || decls_use_flag(handle_block.as_deref().unwrap_or(&[]), "yielded") {
                    self.emit("bool yielded = false;");
                }
                if decls_use_flag(&label_decls, "leaved")
                   || decls_use_flag(public_block.as_deref().unwrap_or(&[]), "leaved")
                   || decls_use_flag(handle_block.as_deref().unwrap_or(&[]), "leaved") {
                    self.emit("bool leaved = false;");
                }
                if decls_use_flag(&label_decls, "returned")
                   || decls_use_flag(public_block.as_deref().unwrap_or(&[]), "returned")
                   || decls_use_flag(handle_block.as_deref().unwrap_or(&[]), "returned") {
                    self.emit("bool returned = false;");
                }
                if decls_use_flag(&label_decls, "continued")
                   || decls_use_flag(public_block.as_deref().unwrap_or(&[]), "continued")
                   || decls_use_flag(handle_block.as_deref().unwrap_or(&[]), "continued") {
                    self.emit("bool continued = false;");
                }

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
                        &format!("friend std::ostream& operator<<(std::ostream& os, const {}& obj) {{", name)
                    );
                    self.emit(&format!("    os << const_cast<{}&>(obj).display();", name));
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
                        if let Decl::FnDecl { name, return_type, body, .. } = h {
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
                                    self.emit("} catch (const fast_std::Error& __e) {");
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
                self.in_class_or_scope = old_in_class;
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
            Decl::BlockDecl { name, return_type, statements, .. } => {
                let has_yield = statements.iter().any(|s| matches!(s, Stmt::YieldStmt(_)));
                let ret_cpp = match return_type {
                    Some(t) => type_to_cpp(t),
                    None => "void".to_string(),
                };
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
                self.emit(&format!("{} operator()() {{", ret_cpp));
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
                            if !matches!(value, Expr::Default(None)) && val_code != "{}" {
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
                if return_type.as_ref().map_or(false, |t| *t != BaseType::Void) {
                    self.emit("return {};");
                }
                self.indent_level -= 1;
                self.emit("}");
                self.indent_level -= 1;
                self.emit(&format!("}} {};", name));
            }
            Decl::MicroDecl { name, params, return_type, body, .. } => {
                let is_break_micro = body.len() == 1 && matches!(body[0], Stmt::BreakStmt);
                if is_break_micro && params.is_empty() {
                    self.emit(&format!("#define {} break", name));
                    return;
                }
                let ret_cpp = match return_type {
                    Some(t) => type_to_cpp(t),
                    None => "void".to_string(),
                };
                let param_strs: Vec<String> = params
                    .iter()
                    .map(|p| {
                        let t = type_to_cpp(&p.type_node);
                        format!("{} {}", t, p.name)
                    })
                    .collect();

                self.emit(&format!("inline {} {}({}) {{", ret_cpp, name, param_strs.join(", ")));
                self.indent_level += 1;
                for s in body {
                    self.visit_statement(s);
                }
                self.indent_level -= 1;
                self.emit("}");
            }
            Decl::ExternFnDecl { abi, name, params, return_type } => {
                let ret_cpp = type_to_cpp(return_type);
                let param_strs: Vec<String> = params
                    .iter()
                    .map(|p| {
                        let t = type_to_cpp(&p.type_node);
                        format!("{} {}", t, p.name)
                    })
                    .collect();
                if abi == "C" {
                    self.emit(&format!("extern \"C\" {} {}({});", ret_cpp, name, param_strs.join(", ")));
                } else {
                    self.emit(&format!("extern {} {}({});", ret_cpp, name, param_strs.join(", ")));
                }
            }
            Decl::ExternBlockDecl { abi, decls } => {
                if abi == "C" {
                    self.emit("extern \"C\" {");
                } else {
                    self.emit("extern {");
                }
                self.indent_level += 1;
                for d in decls {
                    if let Decl::ExternFnDecl { name, params, return_type, .. } = d {
                        let ret_cpp = type_to_cpp(return_type);
                        let param_strs: Vec<String> = params
                            .iter()
                            .map(|p| {
                                let t = type_to_cpp(&p.type_node);
                                format!("{} {}", t, p.name)
                            })
                            .collect();
                        self.emit(&format!("{} {}({});", ret_cpp, name, param_strs.join(", ")));
                    } else {
                        self.visit_declaration(d);
                    }
                }
                self.indent_level -= 1;
                self.emit("}");
            }
            Decl::Import { module_path, abi, .. } => {
                if let Some(_abi_str) = abi {
                    if let Some(header) = module_path.first() {
                        if header.ends_with(".h") || header.ends_with(".hpp") || !header.contains('/') {
                            if header.starts_with('<') || header.starts_with('"') {
                                self.emit(&format!("#include {}", header));
                            } else {
                                self.emit(&format!("#include <{}>", header));
                            }
                        }
                    }
                } else if let Some(header) = module_path.first() {
                    if header.ends_with(".h") || header.ends_with(".hpp") {
                        if header.starts_with('<') || header.starts_with('"') {
                            self.emit(&format!("#include {}", header));
                        } else {
                            self.emit(&format!("#include <{}>", header));
                        }
                    }
                }
            }
            Decl::MachineDecl { name, labels, handle_block, .. } => {
                let old_in_class = self.in_class_or_scope;
                self.in_class_or_scope = true;
                self.custom_scopes.insert(name.clone());
                self.custom_scope_types.insert(name.clone());

                self.emit(&format!("struct {} {{", name));
                self.emit("public:");
                self.indent_level += 1;
                self.emit("int32_t __state = 0;");
                self.emit("bool has_yielded = false;");
                self.emit("bool is_done = false;");

                // Guard flag for @init label
                let has_init = labels.contains_key("init");
                if has_init {
                    self.emit("bool __initialized = false;");
                }

                // Collect fields: any `this.field` assignment in any label
                // We extract VarDecl statements from labels as persistent fields
                let mut field_names: Vec<String> = Vec::new();
                for (_, label_decl) in labels.iter() {
                    if let Decl::LabelDecl { body, .. } = label_decl {
                        for s in body {
                            if let Stmt::Declaration(Decl::VarDecl { name: f_name, type_node, .. }) = s {
                                if !field_names.contains(f_name) {
                                    let cpp_t = type_to_cpp(type_node);
                                    self.emit(&format!("{} {};", cpp_t, f_name));
                                    field_names.push(f_name.clone());
                                }
                            }
                            // Also detect `this.xxx = ...` ReassignStmt targeting a field
                            if let Stmt::ReassignStmt { target: Expr::PropertyAccess { object, property }, .. } = s {
                                let is_this = match &**object {
                                    Expr::This => true,
                                    Expr::Identifier(obj_name) => obj_name == "this",
                                    _ => false,
                                };
                                if is_this && !field_names.contains(property) {
                                    // Emit as int32_t by default (type inferred or explicit)
                                    field_names.push(property.clone());
                                }
                            }
                        }
                    }
                }

                // Determine return type from `leave` handle
                let leave_ret = handle_block.iter().find_map(|h| {
                    if let Decl::FnDecl { name: fn_name, return_type, .. } = h {
                        if fn_name == "leave" { Some(type_to_cpp(return_type)) } else { None }
                    } else { None }
                }).unwrap_or_else(|| "void".to_string());

                // Determine call return type from `call` handle (or default to leave return type)
                let call_ret = handle_block.iter().find_map(|h| {
                    if let Decl::FnDecl { name: fn_name, return_type, .. } = h {
                        if fn_name == "call" { Some(type_to_cpp(return_type)) } else { None }
                    } else { None }
                }).unwrap_or_else(|| leave_ret.clone());

                // Sort labels so @init comes first
                let mut sorted_labels: Vec<(&String, &Decl)> = labels.iter().collect();
                sorted_labels.sort_by_key(|(k, _)| if *k == "init" { 0usize } else { 1 });

                // Emit operator() that wraps the whole state machine
                self.emit(&format!("{} operator()() {{", call_ret));
                self.indent_level += 1;

                // @init guard
                if has_init {
                    self.emit("if (!this->__initialized) { this->__initialized = true; goto init; }");
                }

                // Use `call` handle body as the entry dispatch
                let call_body: Option<&Vec<Stmt>> = handle_block.iter().find_map(|h| {
                    if let Decl::FnDecl { name: fn_name, body, .. } = h {
                        if fn_name == "call" { Some(body) } else { None }
                    } else { None }
                });
                if let Some(body) = call_body {
                    for s in body {
                        self.visit_statement(s);
                    }
                }

                // Emit each label as a C goto target
                for (lbl_name, lbl_decl) in &sorted_labels {
                    if let Decl::LabelDecl { body, .. } = lbl_decl {
                        let clean = lbl_name.replace("@", "");
                        self.emit(&format!("{}:", clean));
                        self.indent_level += 1;
                        for s in body {
                            // Skip VarDecl (already emitted as fields), emit init assignment as this->field = value
                            match s {
                                Stmt::Declaration(Decl::VarDecl { name: f_name, value, assign_op, .. }) => {
                                    let val_code = self.visit_expression(value);
                                    if assign_op == "->" {
                                        self.emit(&format!("fastlang_arrow_assign(this->{}, {});", f_name, val_code));
                                    } else {
                                        self.emit(&format!("this->{} = {};", f_name, val_code));
                                    }
                                }
                                _ => self.visit_statement(s),
                            }
                        }
                        self.indent_level -= 1;
                    }
                }

                if call_ret != "void" {
                    self.emit(&format!("return {}; // unreachable fallback", "{}"));
                }
                self.indent_level -= 1;
                self.emit("}");

                // Emit `call()` method that invokes operator()()
                self.emit(&format!("{} call() {{ return (*this)(); }}", call_ret));

                // Emit `leave` handle as a separate method
                for h in handle_block.iter() {
                    if let Decl::FnDecl { name: fn_name, return_type, body, .. } = h {
                        if fn_name == "leave" {
                            let ret_str = type_to_cpp(return_type);
                            self.emit(&format!("{} leave() {{", ret_str));
                            self.indent_level += 1;
                            for s in body {
                                self.visit_statement(s);
                            }
                            self.indent_level -= 1;
                            self.emit("}");
                        }
                    }
                }

                // Default constructor
                self.emit(&format!("{}() {{}}", name));

                // friend ostream
                self.emit(&format!("friend std::ostream& operator<<(std::ostream& os, const {}& obj) {{", name));
                self.emit(&format!("    os << \"[machine {}]\";", name));
                self.emit("    return os;");
                self.emit("}");

                self.indent_level -= 1;
                self.emit("};");
                self.in_class_or_scope = old_in_class;
            }

            Decl::ImplDecl { .. } => {
                // Handled during Blueprint / Scope generation
            }
            _ => {
                self.emit(&format!("// TODO: unimplemented declaration {:?}", decl));
            }
        }
    }
}
