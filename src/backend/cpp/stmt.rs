use crate::backend::cpp::generator::CodeGenerator;
use crate::frontend::parser::ast::*;

pub(crate) fn type_to_cpp(t: &BaseType) -> String {
    match t {
        BaseType::Int(Size::S8) => "int8_t".to_string(),
        BaseType::Int(Size::S16) => "int16_t".to_string(),
        BaseType::Int(Size::S32) => "int32_t".to_string(),
        BaseType::Int(Size::S64) => "int64_t".to_string(),
        BaseType::Int(Size::S128) => "__int128".to_string(),
        BaseType::UInt(Size::S8) => "uint8_t".to_string(),
        BaseType::UInt(Size::S16) => "uint16_t".to_string(),
        BaseType::UInt(Size::S32) => "uint32_t".to_string(),
        BaseType::UInt(Size::S64) => "uint64_t".to_string(),
        BaseType::UInt(Size::S128) => "unsigned __int128".to_string(),
        BaseType::USize => "size_t".to_string(),
        BaseType::ISize => "ptrdiff_t".to_string(),
        BaseType::Float(Size::S32) => "float".to_string(),
        BaseType::Float(Size::S64) => "double".to_string(),
        BaseType::Float(Size::S128) => "long double".to_string(),
        BaseType::Char => "char".to_string(),
        BaseType::Str => "fastlang_str".to_string(),
        BaseType::Bool => "bool".to_string(),
        BaseType::Void => "void".to_string(),
        BaseType::Block { name, .. } => {
            if !name.is_empty() {
                format!("__block_type_{}", name)
            } else {
                "void".to_string()
            }
        }
        BaseType::Machine { name, .. } => {
            if !name.is_empty() {
                format!("__machine_type_{}", name)
            } else {
                "std::function<void()>".to_string()
            }
        }
        BaseType::Class { name, generics, .. } | BaseType::Struct { name, generics, .. } => {
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
        BaseType::GenericParam(name) => name.trim_start_matches('.').to_string(),
        BaseType::Generic(inner_vec) => {
            let strs: Vec<String> = inner_vec.iter().map(type_to_cpp).collect();
            strs.join(", ")
        }
        BaseType::Pointer(inner) => format!("{}*", type_to_cpp(inner)),
        BaseType::Name(inner) => {
            let mut actual_inner = &**inner;
            if let BaseType::Generic(vec) = actual_inner {
                if !vec.is_empty() {
                    actual_inner = &vec[0];
                }
            }
            if matches!(actual_inner, BaseType::Fn { .. } | BaseType::Method { .. }) {
                format!("fastlang_name<{}>", type_to_cpp(actual_inner))
            } else if actual_inner.as_str() == BaseType::Unknown.as_str()
                || actual_inner.as_str() == "unknown"
            {
                "fastlang_name".to_string()
            } else {
                format!("fastlang_name<{}>", type_to_cpp(actual_inner))
            }
        }
        BaseType::Modify(inner) => {
            if inner.as_str() == BaseType::Unknown.as_str() {
                "fastlang_modify".to_string()
            } else {
                let mut actual_inner = if let BaseType::Name(n) = &**inner {
                    &**n
                } else {
                    &**inner
                };
                if let BaseType::Generic(vec) = actual_inner {
                    if !vec.is_empty() {
                        actual_inner = &vec[0];
                    }
                }
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
                let mut actual_inner = if let BaseType::Name(n) = &**inner {
                    &**n
                } else {
                    &**inner
                };
                if let BaseType::Generic(vec) = actual_inner {
                    if !vec.is_empty() {
                        actual_inner = &vec[0];
                    }
                }
                if actual_inner.as_str() == "unknown" {
                    "fastlang_copy".to_string()
                } else {
                    format!("fastlang_copy<{}>", type_to_cpp(actual_inner))
                }
            }
        }
        BaseType::Type(_) => "type".to_string(),
        BaseType::Flag => "bool".to_string(),
        BaseType::Array { base_type, .. } => format!("fastlang_slice<{}>", type_to_cpp(base_type)),
        BaseType::Method {
            return_type,
            params,
            ..
        } => {
            if return_type.as_ref() == &BaseType::Unknown {
                "auto".to_string()
            } else {
                let ret = type_to_cpp(return_type);
                let p_types: Vec<String> = params.iter().map(type_to_cpp).collect();
                format!("fastlang_method<{}({})>", ret, p_types.join(", "))
            }
        }
        BaseType::Fn {
            return_type,
            params,
            ..
        }
        | BaseType::Micro {
            return_type,
            params,
            ..
        }
        | BaseType::Lambda {
            return_type,
            params,
            ..
        } => {
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
        Expr::BinaryOp { left, right, .. } => {
            expr_uses_flag(left, flag) || expr_uses_flag(right, flag)
        }
        Expr::UnaryOp { operand, .. } => expr_uses_flag(operand, flag),
        Expr::Call { callee, args, .. } => {
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
            Stmt::YieldStmt(_) if flag == "yielded" || flag == "has_yielded" => {
                return true;
            }
            Stmt::LeaveStmt if flag == "leaved" => {
                return true;
            }
            Stmt::IfStmt {
                condition,
                then_block,
                else_block,
                ..
            } => {
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
            Stmt::LoopStmt { count, body } => {
                if let Some(c) = count {
                    if expr_uses_flag(c, flag) {
                        return true;
                    }
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

fn collect_block_vars(stmts: &[Stmt], vars: &mut Vec<(String, BaseType)>) {
    for s in stmts {
        match s {
            Stmt::Declaration(Decl::VarDecl {
                name, type_node, ..
            }) => {
                if !vars.iter().any(|(n, _)| n == name) {
                    vars.push((name.clone(), type_node.clone()));
                }
            }
            Stmt::ForStmt { init, body, .. } => {
                if let Some(i) = init {
                    if let Stmt::Declaration(Decl::VarDecl {
                        name, type_node, ..
                    }) = &**i
                    {
                        if !vars.iter().any(|(n, _)| n == name) {
                            vars.push((name.clone(), type_node.clone()));
                        }
                    }
                }
                if let EitherBlock::Inline(inner) = body {
                    collect_block_vars(inner, vars);
                }
            }
            Stmt::WhileStmt { body, .. }
            | Stmt::DoWhileStmt { body, .. }
            | Stmt::LoopStmt { body, .. } => {
                if let EitherBlock::Inline(inner) = body {
                    collect_block_vars(inner, vars);
                }
            }
            Stmt::IfStmt {
                then_block,
                else_block,
                ..
            } => {
                collect_block_vars(then_block, vars);
                if let Some(eb) = else_block {
                    collect_block_vars(eb, vars);
                }
            }
            Stmt::Block(inner) | Stmt::ThisBlock(inner) => {
                collect_block_vars(inner, vars);
            }
            Stmt::SwitchStmt { cases, .. } => {
                for c in cases {
                    if let Stmt::CaseStmt { body, .. } = c {
                        collect_block_vars(body, vars);
                    }
                }
            }
            _ => {}
        }
    }
}

pub(crate) fn cpp_safe_name(name: &str) -> String {
    let clean = if let Some(stripped) = name.strip_prefix('$') {
        format!("_f_{}", stripped)
    } else {
        name.to_string()
    };
    match clean.as_str() {
        "throw" => "_throw".to_string(),
        "default" => "fastlang_handle_default".to_string(),
        "delete" => "_delete".to_string(),
        "new" => "_new".to_string(),
        other => other.to_string(),
    }
}

impl CodeGenerator {
    pub(crate) fn format_param(&mut self, param: &Param) -> String {
        let cpp_t = type_to_cpp(&param.type_node);
        if param.is_variadic {
            let base_pack_type = if cpp_t.is_empty() || cpp_t == "any" || cpp_t == "auto" {
                "__Args".to_string()
            } else {
                cpp_t
                    .trim_start_matches('.')
                    .trim_end_matches('*')
                    .trim()
                    .to_string()
            };
            return format!("{}... {}", base_pack_type, param.name);
        }
        if let Some(ref def_val) = param.default_value {
            let def_code = self.visit_expression(def_val);
            format!("{} {} = {}", cpp_t, param.name, def_code)
        } else {
            format!("{} {}", cpp_t, param.name)
        }
    }

    pub(crate) fn visit_statement(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Block(stmts) | Stmt::ThisBlock(stmts) => {
                for s in stmts {
                    self.visit_statement(s);
                }
            }
            Stmt::Declaration(decl) => {
                self.visit_declaration(decl);
            }
            Stmt::CompileValidation { .. } => return,
            Stmt::ExpressionStmt(expr) => {
                if let Expr::Call { callee, .. } = expr {
                    if let Expr::NamespaceAccess { namespace, .. } = &**callee {
                        if namespace == "@compile" {
                            return;
                        }
                    }
                }
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
            Stmt::IfStmt {
                condition,
                then_block,
                else_block,
            } => {
                if is_comptime_reflection_expr(condition) {
                    for s in then_block {
                        self.visit_statement(s);
                    }
                    return;
                }

                let cond_code = self.visit_expression(condition);
                self.emit(&format!("if ({}) {{", cond_code));
                self.indent_level += 1;
                for s in then_block {
                    self.visit_statement(s);
                }
                self.indent_level -= 1;
                if let Some(eb) = else_block {
                    if !crate::middle_end::semantic::analyzer::has_compile_directive(eb) {
                        self.emit("} else {");
                        self.indent_level += 1;
                        for s in eb {
                            self.visit_statement(s);
                        }
                        self.indent_level -= 1;
                    }
                }
                self.emit("}");
            }
            Stmt::ForInStmt {
                item,
                iterable,
                body,
            } => {
                let iterable_code = self.visit_expression(iterable);
                let item_code = if let Stmt::Declaration(Decl::VarDecl {
                    type_node, name, ..
                }) = &**item
                {
                    let cpp_type = type_to_cpp(type_node);
                    format!("{} {}", cpp_type, name)
                } else if let Stmt::ExpressionStmt(Expr::Identifier(name)) = &**item {
                    format!("auto {}", name)
                } else {
                    "auto item".to_string()
                };

                if self.variadic_packs.contains(&iterable_code) {
                    let item_var = item_code
                        .split_whitespace()
                        .last()
                        .unwrap_or(&item_code)
                        .trim_start_matches('*')
                        .trim_start_matches('&');
                    self.emit(&format!("([&](auto&& {}) {{", item_var));
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
                    self.emit(&format!("}}({}), ...);", iterable_code));
                } else {
                    self.emit(&format!(
                        "for ({} : fastlang_iterable({})) {{",
                        item_code, iterable_code
                    ));
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
            Stmt::LoopStmt { count, body } => {
                if let Some(c) = count {
                    let count_code = self.visit_expression(c);
                    self.emit(&format!(
                        "for (long long _i = 0, _limit = (long long)({}); _i < _limit; ++_i) {{",
                        count_code
                    ));
                } else {
                    self.emit("while (true) {");
                }
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
                                Expr::Call { callee, args, .. } => {
                                    let callee_code = self.visit_expression(callee);
                                    let raw_variant = if let Some(idx) = callee_code.rfind("::") {
                                        &callee_code[idx + 2..]
                                    } else {
                                        &callee_code
                                    };
                                    let variant_name = raw_variant.trim_end_matches("()");
                                    self.emit(
                                        &format!(
                                            "{} (__match_val.tag == std::decay_t<decltype(__match_val)>::Tag::{}) {{",
                                            branch_prefix,
                                            variant_name
                                        )
                                    );
                                    self.indent_level += 1;
                                    if !args.is_empty() {
                                        self.emit(
                                            &format!("if (std::holds_alternative<typename std::decay_t<decltype(__match_val)>::{}_Payload>(__match_val.data)) {{", variant_name)
                                        );
                                        self.indent_level += 1;
                                        let var_names: Vec<String> = args
                                            .iter()
                                            .filter_map(|a| {
                                                if let Expr::Identifier(n) = a {
                                                    Some(n.clone())
                                                } else {
                                                    None
                                                }
                                            })
                                            .collect();
                                        if !var_names.is_empty() {
                                            for vn in &var_names {
                                                self.pointer_vars.insert(vn.clone());
                                            }
                                            self.emit(
                                                &format!(
                                                    "auto [{}] = std::get<typename std::decay_t<decltype(__match_val)>::{}_Payload>(__match_val.data);",
                                                    var_names.join(", "),
                                                    variant_name
                                                )
                                            );
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
                                    let raw_variant = if let Some(idx) = target_code.rfind("::") {
                                        &target_code[idx + 2..]
                                    } else {
                                        &target_code
                                    };
                                    let variant_name = raw_variant.trim_end_matches("()");
                                    self.emit(
                                        &format!(
                                            "{} (__match_val.tag == std::decay_t<decltype(__match_val)>::Tag::{}) {{",
                                            branch_prefix,
                                            variant_name
                                        )
                                    );
                                    self.indent_level += 1;
                                    if !args.is_empty() {
                                        self.emit(
                                            &format!("if (std::holds_alternative<typename std::decay_t<decltype(__match_val)>::{}_Payload>(__match_val.data)) {{", variant_name)
                                        );
                                        self.indent_level += 1;
                                        if let Expr::ObjectLiteral(stmts) = &args[0] {
                                            let var_names: Vec<String> = stmts
                                                .iter()
                                                .filter_map(|s| match s {
                                                    Stmt::Declaration(Decl::VarDecl {
                                                        name,
                                                        ..
                                                    }) => Some(name.clone()),
                                                    Stmt::ReassignStmt { target, .. } => {
                                                        if let Expr::Identifier(n) = target {
                                                            Some(n.clone())
                                                        } else {
                                                            None
                                                        }
                                                    }
                                                    Stmt::ExpressionStmt(Expr::Identifier(n)) => {
                                                        Some(n.clone())
                                                    }
                                                    _ => None,
                                                })
                                                .collect();
                                            if !var_names.is_empty() {
                                                for vn in &var_names {
                                                    self.pointer_vars.insert(vn.clone());
                                                }
                                                self.emit(
                                                    &format!(
                                                        "auto [{}] = std::get<typename std::decay_t<decltype(__match_val)>::{}_Payload>(__match_val.data);",
                                                        var_names.join(", "),
                                                        variant_name
                                                    )
                                                );
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
                                Expr::NamespaceAccess {
                                    namespace,
                                    property,
                                } => {
                                    let prop_code = self.visit_expression(property);
                                    let variant_name = prop_code.trim_end_matches("()");
                                    if self.simple_enum_types.contains(namespace) {
                                        self.emit(&format!(
                                            "{} (__match_val == {}::{}) {{",
                                            branch_prefix, namespace, variant_name
                                        ));
                                    } else {
                                        self.emit(
                                            &format!(
                                                "{} (__match_val.tag == std::decay_t<decltype(__match_val)>::Tag::{}) {{",
                                                branch_prefix,
                                                variant_name
                                            )
                                        );
                                    }
                                    self.indent_level += 1;
                                    for case_stmt in body {
                                        self.visit_statement(case_stmt);
                                    }
                                    self.indent_level -= 1;
                                    self.emit("}");
                                }
                                Expr::PropertyAccess { object, property } => {
                                    let obj_code = self.visit_expression(object);
                                    if self.simple_enum_types.contains(&obj_code) {
                                        self.emit(&format!(
                                            "{} (__match_val == {}::{}) {{",
                                            branch_prefix, obj_code, property
                                        ));
                                    } else {
                                        self.emit(
                                            &format!(
                                                "{} (__match_val.tag == std::decay_t<decltype(__match_val)>::Tag::{}) {{",
                                                branch_prefix,
                                                property
                                            )
                                        );
                                    }
                                    self.indent_level += 1;
                                    for case_stmt in body {
                                        self.visit_statement(case_stmt);
                                    }
                                    self.indent_level -= 1;
                                    self.emit("}");
                                }
                                Expr::Identifier(ident) => {
                                    self.emit(&format!(
                                        "{} (fastlang_match_eq(__match_val, {})) {{",
                                        branch_prefix, ident
                                    ));
                                    self.indent_level += 1;
                                    for case_stmt in body {
                                        self.visit_statement(case_stmt);
                                    }
                                    self.indent_level -= 1;
                                    self.emit("}");
                                }
                                _ => {
                                    let val_code = self.visit_expression(option);
                                    self.emit(&format!(
                                        "{} (fastlang_match_eq(__match_val, {})) {{",
                                        branch_prefix, val_code
                                    ));
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
            Stmt::ForStmt {
                init,
                condition,
                increment,
                body,
            } => {
                let is_in_block_generator = self.current_block_vars.is_some();
                if !is_in_block_generator {
                    self.emit("{");
                    self.indent_level += 1;
                }
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

                if !is_in_block_generator {
                    self.indent_level -= 1;
                    self.emit("}");
                }
            }
            Stmt::ForIn {
                item_decl,
                iterable,
                body,
            } => {
                let old_out = std::mem::take(&mut self.output);
                self.visit_statement(item_decl);
                let mut decl_code = std::mem::replace(&mut self.output, old_out);
                decl_code = decl_code.trim_end().trim_end_matches(';').to_string();

                let iter_code = self.visit_expression(iterable);
                if self.variadic_packs.contains(&iter_code) {
                    let item_var = decl_code
                        .split_whitespace()
                        .last()
                        .unwrap_or(&decl_code)
                        .trim_start_matches('*')
                        .trim_start_matches('&');
                    self.emit(&format!("([&](auto&& {}) {{", item_var));
                    self.indent_level += 1;
                    for s in body {
                        self.visit_statement(s);
                    }
                    self.indent_level -= 1;
                    self.emit(&format!("}}({}), ...);", iter_code));
                } else {
                    self.emit(&format!(
                        "for ({} : fastlang_iterable({})) {{",
                        decl_code, iter_code
                    ));

                    self.indent_level += 1;
                    for s in body {
                        self.visit_statement(s);
                    }
                    self.indent_level -= 1;
                    self.emit("}");
                }
            }
            Stmt::ReturnStmt(expr) => {
                if expr.is_none() || matches!(expr, Some(Expr::LiteralVoid)) {
                    self.emit("return;");
                } else {
                    let expr_code = self.visit_expression(&expr.clone().unwrap());
                    if expr_code == "this" {
                        self.emit("return *this;");
                    } else {
                        self.emit(&format!("return {};", expr_code));
                    }
                }
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
                if let Expr::Call { callee, .. } = expr {
                    if let Expr::NamespaceAccess { namespace, .. } = &**callee {
                        if namespace == "@compile" {
                            return;
                        }
                    }
                }
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
                    "}} catch (const fast_std::Error& {}) {{",
                    catch_param
                ));
                self.indent_level += 1;
                for s in catch_block {
                    self.visit_statement(s);
                }
                self.indent_level -= 1;
                self.emit(&format!(
                    "}} catch (const fast_std::Error* __{}_ptr) {{",
                    catch_param
                ));
                self.indent_level += 1;
                self.emit(&format!(
                    "const fast_std::Error& {} = *__{}_ptr;",
                    catch_param, catch_param
                ));
                for s in catch_block {
                    self.visit_statement(s);
                }
                self.indent_level -= 1;
                self.emit("}");
            }
            _ => {
                self.emit(&format!(
                    "// TODO: unimplemented statement {:?} or something gone wrong",
                    stmt
                ));
            }
        }
    }

    pub(crate) fn visit_declaration(&mut self, decl: &Decl) {
        match decl {
            Decl::EnumDecl {
                name,
                generics,
                variants,
                ..
            } => {
                self.enum_types.insert(name.clone());
                let is_generic = !generics.is_empty();
                let has_payloads = variants
                    .iter()
                    .any(|v| !matches!(v.payload, EnumVariantPayload::None));

                if !is_generic && !has_payloads {
                    self.simple_enum_types.insert(name.clone());
                    self.emit(&format!("enum class {} {{", name));
                    self.indent_level += 1;
                    for (i, variant) in variants.iter().enumerate() {
                        let comma = if i < variants.len() - 1 { "," } else { "" };
                        self.emit(&format!("{}{}", variant.name, comma));
                    }
                    self.indent_level -= 1;
                    self.emit("};");

                    for variant in variants.iter() {
                        self.emit(&format!(
                            "inline constexpr auto {} = {}::{};",
                            variant.name, name, variant.name
                        ));
                    }

                    self.emit(&format!(
                        "inline std::ostream& operator<<(std::ostream& os, const {}& obj) {{",
                        name
                    ));
                    self.indent_level += 1;
                    self.emit("switch (obj) {");
                    self.indent_level += 1;
                    for variant in variants.iter() {
                        self.emit(&format!(
                            "case {}::{}: os << \"{}\"; break;",
                            name, variant.name, variant.name
                        ));
                    }
                    self.indent_level -= 1;
                    self.emit("}");
                    self.emit("return os;");
                    self.indent_level -= 1;
                    self.emit("}");
                } else {
                    self.payload_enum_types.insert(name.clone());
                    for v in variants.iter() {
                        if matches!(v.payload, EnumVariantPayload::None) {
                            self.emit(&format!("struct fastlang_tag_{}_{} {{}};", name, v.name));
                            self.emit(&format!(
                                "inline constexpr fastlang_tag_{}_{} {};",
                                name, v.name, v.name
                            ));
                        }
                    }

                    let generic_params: Vec<String> = generics
                        .iter()
                        .map(|g| format!("typename {}", g.as_str()))
                        .collect();
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
                            EnumVariantPayload::None => {}
                            EnumVariantPayload::Tuple(types) => {
                                let field_strs: Vec<String> = types
                                    .iter()
                                    .enumerate()
                                    .map(|(idx, t)| format!("{} _{};", type_to_cpp(t), idx))
                                    .collect();
                                self.emit(&format!(
                                    "struct {}_Payload {{ {} }};",
                                    v.name,
                                    field_strs.join(" ")
                                ));
                            }
                            EnumVariantPayload::Struct(fields) => {
                                let field_strs: Vec<String> = fields
                                    .iter()
                                    .map(|f| {
                                        let type_str = type_to_cpp(&f.type_node);
                                        if let Some(ref def_val) = f.default_value {
                                            let def_code = self.visit_expression(def_val);
                                            format!("{} {} = {};", type_str, f.name, def_code)
                                        } else {
                                            format!("{} {};", type_str, f.name)
                                        }
                                    })
                                    .collect();
                                self.emit(&format!(
                                    "struct {}_Payload {{ {} }};",
                                    v.name,
                                    field_strs.join(" ")
                                ));
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
                                self.emit(&format!(
                                    "static {} {}() {{ {} r; r.tag = Tag::{}; return r; }}",
                                    name, v.name, name, v.name
                                ));
                            }
                            EnumVariantPayload::Tuple(types) => {
                                let param_strs: Vec<String> = types
                                    .iter()
                                    .enumerate()
                                    .map(|(idx, t)| format!("{} _{}", type_to_cpp(t), idx))
                                    .collect();
                                let init_strs: Vec<String> = types
                                    .iter()
                                    .enumerate()
                                    .map(|(idx, _)| format!("_{}", idx))
                                    .collect();
                                self.emit(
                                    &format!(
                                        "static {} {}({}) {{ {} r; r.tag = Tag::{}; r.data = {}_Payload{{{}}}; return r; }}",
                                        name,
                                        v.name,
                                        param_strs.join(", "),
                                        name,
                                        v.name,
                                        v.name,
                                        init_strs.join(", ")
                                    )
                                );
                            }
                            EnumVariantPayload::Struct(fields) => {
                                let param_strs: Vec<String> = fields
                                    .iter()
                                    .map(|f| format!("{} {}", type_to_cpp(&f.type_node), f.name))
                                    .collect();
                                let init_strs: Vec<String> =
                                    fields.iter().map(|f| f.name.clone()).collect();
                                self.emit(
                                    &format!(
                                        "static {} {}({}) {{ {} r; r.tag = Tag::{}; r.data = {}_Payload{{{}}}; return r; }}",
                                        name,
                                        v.name,
                                        param_strs.join(", "),
                                        name,
                                        v.name,
                                        v.name,
                                        init_strs.join(", ")
                                    )
                                );
                            }
                        }
                        self.emit(&format!(
                            "bool is_{}() const {{ return tag == Tag::{}; }}",
                            v.name, v.name
                        ));
                    }

                    let default_variant = variants
                        .iter()
                        .find(|v| matches!(v.payload, EnumVariantPayload::None))
                        .unwrap_or(&variants[0]);
                    self.emit(&format!(
                        "{}() : tag(Tag::{}) {{}}",
                        name, default_variant.name
                    ));
                    for v in variants.iter() {
                        if matches!(v.payload, EnumVariantPayload::None) {
                            self.emit(&format!(
                                "{}(fastlang_tag_{}_{}) : tag(Tag::{}) {{}}",
                                name, name, v.name, v.name
                            ));
                            self.emit(
                                &format!(
                                    "{}& operator=(fastlang_tag_{}_{}) {{ tag = Tag::{}; return *this; }}",
                                    name,
                                    name,
                                    v.name,
                                    v.name
                                )
                            );
                            self.emit(
                                &format!(
                                    "bool operator==(fastlang_tag_{}_{}) const {{ return tag == Tag::{}; }}",
                                    name,
                                    v.name,
                                    v.name
                                )
                            );
                            self.emit(
                                &format!(
                                    "bool operator!=(fastlang_tag_{}_{}) const {{ return tag != Tag::{}; }}",
                                    name,
                                    v.name,
                                    v.name
                                )
                            );
                        }
                    }
                    if let Some(none_v) = variants.iter().find(|v| v.name == "None") {
                        self.emit(&format!(
                            "{}(fastlang_tag_stop) : tag(Tag::{}) {{}}",
                            name, none_v.name
                        ));
                        self.emit(&format!(
                            "{}& operator=(fastlang_tag_stop) {{ tag = Tag::{}; return *this; }}",
                            name, none_v.name
                        ));
                    }
                    if is_generic && variants.iter().any(|v| v.name == "Some") {
                        self.emit(
                            &format!(
                                "template <typename... U> {}(const {}<U...>& other) : tag(static_cast<Tag>(other.tag)) {{ if (other.is_Some()) {{ data = Some_Payload{{ T(std::get<1>(other.data)._0) }}; }} }}",
                                name,
                                name
                            )
                        );
                        self.emit(
                            &format!(
                                "template <typename... U> {}& operator=(const {}<U...>& other) {{ tag = static_cast<Tag>(other.tag); if (other.is_Some()) {{ data = Some_Payload{{ T(std::get<1>(other.data)._0) }}; }} return *this; }}",
                                name,
                                name
                            )
                        );
                    }
                    self.emit("bool operator==(Tag t) const { return tag == t; }");
                    self.emit("bool operator!=(Tag t) const { return tag != t; }");
                    self.emit(&format!(
                        "bool operator==(const {}& other) const {{ return tag == other.tag; }}",
                        name
                    ));
                    self.emit(&format!(
                        "bool operator!=(const {}& other) const {{ return tag != other.tag; }}",
                        name
                    ));

                    self.indent_level -= 1;
                    self.emit("};");

                    for v in variants.iter() {
                        if let EnumVariantPayload::Tuple(types) = &v.payload {
                            if is_generic {
                                let template_params: Vec<String> = generics
                                    .iter()
                                    .map(|g| format!("typename {}", g.as_str()))
                                    .collect();
                                let param_strs: Vec<String> = types
                                    .iter()
                                    .enumerate()
                                    .map(|(idx, t)| format!("{} _{}", type_to_cpp(t), idx))
                                    .collect();
                                let arg_strs: Vec<String> = types
                                    .iter()
                                    .enumerate()
                                    .map(|(idx, _)| format!("_{}", idx))
                                    .collect();
                                let gen_args: Vec<String> =
                                    generics.iter().map(|g| g.as_str()).collect();
                                self.emit(
                                    &format!(
                                        "template <{}> inline {}<{}> {}({}) {{ return {}<{}>::{}({}); }}",
                                        template_params.join(", "),
                                        name,
                                        gen_args.join(", "),
                                        v.name,
                                        param_strs.join(", "),
                                        name,
                                        gen_args.join(", "),
                                        v.name,
                                        arg_strs.join(", ")
                                    )
                                );
                            } else {
                                let param_strs: Vec<String> = types
                                    .iter()
                                    .enumerate()
                                    .map(|(idx, t)| format!("{} _{}", type_to_cpp(t), idx))
                                    .collect();
                                let arg_strs: Vec<String> = types
                                    .iter()
                                    .enumerate()
                                    .map(|(idx, _)| format!("_{}", idx))
                                    .collect();
                                self.emit(&format!(
                                    "inline {} {}({}) {{ return {}::{}({}); }}",
                                    name,
                                    v.name,
                                    param_strs.join(", "),
                                    name,
                                    v.name,
                                    arg_strs.join(", ")
                                ));
                            }
                        }
                    }

                    let obj_type_str = if is_generic {
                        let generic_args: Vec<String> =
                            generics.iter().map(|g| g.as_str()).collect();
                        format!("{}<{}>", name, generic_args.join(", "))
                    } else {
                        name.clone()
                    };

                    self.emit(&format!(
                        "{}inline std::ostream& operator<<(std::ostream& os, const {}& obj) {{",
                        template_prefix, obj_type_str
                    ));
                    self.indent_level += 1;
                    self.emit("switch (obj.tag) {");
                    self.indent_level += 1;
                    for v in variants.iter() {
                        self.emit(&format!(
                            "case {}::Tag::{}: os << \"{}\"; break;",
                            obj_type_str, v.name, v.name
                        ));
                    }
                    self.indent_level -= 1;
                    self.emit("}");
                    self.emit("return os;");
                    self.indent_level -= 1;
                    self.emit("}");
                }
            }
            Decl::VarDecl {
                name,
                type_node,
                value,
                editability,
                assign_op,
                place: _,
                ..
            } => {
                if matches!(
                    type_node,
                    BaseType::Block { .. } | BaseType::Machine { .. } | BaseType::Class { .. }
                ) {
                    self.custom_scopes.insert(name.clone());
                }
                if matches!(
                    type_node,
                    BaseType::Pointer(_) | BaseType::Modify(_) | BaseType::Copy(_)
                ) {
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
                let cpp_type = if matches!(type_node, BaseType::Unknown) || assign_op == ":=" {
                    "auto".to_string()
                } else {
                    type_to_cpp(type_node)
                };

                let is_value_type = !matches!(
                    type_node,
                    BaseType::Pointer(_)
                        | BaseType::Name(_)
                        | BaseType::Modify(_)
                        | BaseType::Copy(_)
                );
                let mut val_code = if is_value_type {
                    if let Expr::New {
                        type_node: inner,
                        target,
                    } = value
                    {
                        match &**target {
                            Expr::Instantiate { args, .. } => {
                                let arg_strs: Vec<String> =
                                    args.iter().map(|a| self.visit_expression(a)).collect();
                                format!("{}({})", type_to_cpp(inner), arg_strs.join(", "))
                            }
                            Expr::ArrayLiteral(elems) => {
                                let elem_strs: Vec<String> =
                                    elems.iter().map(|e| self.visit_expression(e)).collect();
                                format!("{}{{{}}}", type_to_cpp(inner), elem_strs.join(", "))
                            }
                            _ => {
                                let target_code = self.visit_expression(target);
                                if target_code == "__default__"
                                    || target_code == "{}"
                                    || target_code.is_empty()
                                {
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

                if matches!(type_node, BaseType::Copy(_))
                    && !is_param
                    && !matches!(value, Expr::Default(None))
                    && !val_code.is_empty()
                    && val_code != "{}"
                {
                    val_code = format!("fastlang_make_copy({})", val_code);
                }

                if cpp_type.contains('<') {
                    let raw_name = cpp_type.split('<').next().unwrap_or(&cpp_type);
                    let prefix = format!("{}::", raw_name);
                    if val_code.starts_with(&prefix) {
                        val_code = format!("{}::{}", cpp_type, &val_code[prefix.len()..]);
                    }
                }

                if self
                    .current_block_vars
                    .as_ref()
                    .map_or(false, |bv| bv.contains(name))
                {
                    if assign_op == "->" {
                        self.emit(&format!(
                            "fastlang_arrow_assign(this->{}, {});",
                            name, val_code
                        ));
                    } else if !val_code.is_empty() && val_code != "{}" {
                        self.emit(&format!("this->{} = {};", name, val_code));
                    }
                } else if is_param || (matches!(value, Expr::Default(None)) && assign_op.is_empty())
                {
                    self.emit(&format!("{}{} {};", const_prefix, cpp_type, name));
                } else if assign_op == "->" {
                    self.emit(&format!("{}{} {};", const_prefix, cpp_type, name));
                    if val_code.starts_with('{') && val_code.ends_with('}') {
                        self.emit(&format!("{}.arrow_assign({});", name, val_code));
                    } else {
                        self.emit(&format!("fastlang_arrow_assign({}, {});", name, val_code));
                    }
                } else if assign_op == "=" || assign_op == ":=" || assign_op.is_empty() {
                    self.emit(&format!(
                        "{}{} {} = {};",
                        const_prefix, cpp_type, name, val_code
                    ));
                } else {
                    self.emit(&format!(
                        "{}{} {} {} {};",
                        const_prefix, cpp_type, name, assign_op, val_code
                    ));
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
                        self.emit(&format!(
                            "{}{} {} = {};",
                            const_prefix, cpp_type, name, val_code
                        ));
                    } else {
                        self.emit(&format!(
                            "{}{} {} {} {};",
                            const_prefix, cpp_type, name, assign_op, val_code
                        ));
                    }
                }
            }
            Decl::ObjectDestructureDecl {
                editability,
                type_name,
                fields,
                rhs,
                ..
            } => {
                let rhs_code = self.visit_expression(rhs);
                let is_const = editability == &Editability::NotEditable;
                let const_prefix = if is_const { "const " } else { "" };

                let inferred_type = if let Some(t) = type_name {
                    Some(t.clone())
                } else if let Expr::Call { callee, .. } = rhs {
                    if let Expr::Identifier(fn_name) = &**callee {
                        self.fn_return_types.get(fn_name).cloned()
                    } else {
                        None
                    }
                } else if let Expr::Instantiate { target, .. } = rhs {
                    if let Expr::Identifier(t_name) = &**target {
                        Some(t_name.clone())
                    } else {
                        None
                    }
                } else {
                    None
                };

                let clean_inferred = inferred_type.map(|t| {
                    t.trim_start_matches("blueprint<")
                        .trim_start_matches("struct<")
                        .trim_start_matches("class<")
                        .trim_end_matches('>')
                        .to_string()
                });

                let total_fields = clean_inferred
                    .as_ref()
                    .and_then(|t| self.struct_field_counts.get(t))
                    .copied();

                if let Some(total_count) = total_fields {
                    if total_count > fields.len() {
                        self.yield_counter += 1;
                        let mut dummy_names = Vec::new();
                        for i in 0..total_count {
                            dummy_names.push(format!("__d_{}_{}", self.yield_counter, i));
                        }
                        self.emit(&format!(
                            "auto [{}] = {};",
                            dummy_names.join(", "),
                            rhs_code
                        ));
                        for (idx, (type_node, name)) in fields.iter().enumerate() {
                            let cpp_t = if matches!(type_node, BaseType::Unknown) {
                                "auto".to_string()
                            } else {
                                type_to_cpp(type_node)
                            };
                            self.emit(&format!(
                                "{}{} {} = {};",
                                const_prefix, cpp_t, name, dummy_names[idx]
                            ));
                        }
                        return;
                    }
                }

                let names: Vec<String> = fields.iter().map(|(_, name)| name.clone()).collect();
                self.emit(&format!(
                    "{}auto [{}] = {};",
                    const_prefix,
                    names.join(", "),
                    rhs_code
                ));
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

                let len_str =
                    if len_code == "0" && !matches!(value, Expr::Default(None)) && val_code != "{}"
                    {
                        "".to_string()
                    } else {
                        len_code
                    };

                if val_code == "__param__" {
                    self.emit(&format!(
                        "{}{} {}[{}];",
                        const_prefix, cpp_type, name, len_str
                    ));
                } else if val_code.starts_with("fastlang_slice")
                    || (len_str.is_empty() && !val_code.starts_with("{"))
                {
                    self.emit(&format!(
                        "{}fastlang_slice<{}> {} = {};",
                        const_prefix, cpp_type, name, val_code
                    ));
                } else {
                    self.emit(&format!(
                        "{}{} {}[{}] = {};",
                        const_prefix, cpp_type, name, len_str, val_code
                    ));
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
                visibility: _,
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
                if decls_use_flag(public_block, "broken")
                    || decls_use_flag(private_block, "broken")
                    || decls_use_flag(handle_block, "broken")
                {
                    self.emit("bool broken = false;");
                }
                if decls_use_flag(public_block, "is_done")
                    || decls_use_flag(private_block, "is_done")
                    || decls_use_flag(handle_block, "is_done")
                {
                    self.emit("bool is_done = false;");
                }
                if decls_use_flag(public_block, "yielded")
                    || decls_use_flag(private_block, "yielded")
                    || decls_use_flag(handle_block, "yielded")
                {
                    self.emit("bool yielded = false;");
                }
                if decls_use_flag(public_block, "leaved")
                    || decls_use_flag(private_block, "leaved")
                    || decls_use_flag(handle_block, "leaved")
                {
                    self.emit("bool leaved = false;");
                }
                if decls_use_flag(public_block, "returned")
                    || decls_use_flag(private_block, "returned")
                    || decls_use_flag(handle_block, "returned")
                {
                    self.emit("bool returned = false;");
                }
                if decls_use_flag(public_block, "continued")
                    || decls_use_flag(private_block, "continued")
                    || decls_use_flag(handle_block, "continued")
                {
                    self.emit("bool continued = false;");
                }

                let display_fn = handle_block.iter().find_map(|h| {
                    if let Decl::FnDecl {
                        name: fn_name,
                        return_type,
                        ..
                    } = h
                    {
                        if fn_name == "display" {
                            Some(return_type.clone())
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                });

                if let Some(_ret_type) = display_fn {
                    self.emit(&format!(
                        "friend std::ostream& operator<<(std::ostream& os, const {}& obj) {{",
                        name
                    ));
                    self.emit(&format!("    os << const_cast<{}&>(obj).display();", name));
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

                self.emit_operator_overloads(&Some(handle_block.clone()));

                for s in public_block {
                    self.visit_declaration(s);
                }
                for h in handle_block {
                    self.visit_declaration(h);
                }
                let has_drop_handle = handle_block.iter().any(|h| {
                    if let Decl::FnDecl { name: fn_name, .. } = h {
                        fn_name == "drop"
                    } else {
                        false
                    }
                });
                if has_drop_handle {
                    self.emit("bool _fastlang_dropped = false;");
                    self.emit("void _fastlang_call_drop() {");
                    self.emit("    if (!_fastlang_dropped) {");
                    self.emit("        _fastlang_dropped = true;");
                    self.emit("        this->drop();");
                    self.emit("    }");
                    self.emit("}");
                    self.emit(&format!(
                        "virtual ~{}() {{ this->_fastlang_call_drop(); }}",
                        name
                    ));
                }
                for s in static_block {
                    self.visit_declaration(s);
                }

                let has_default_ctor = constructor
                    .as_ref()
                    .map(|ctors| {
                        ctors
                            .iter()
                            .any(|c| c.params.iter().all(|p| p.type_node.as_str() == "type"))
                    })
                    .unwrap_or(false);

                if let Some(ext) = extends {
                    self.emit(&format!("using {}::{};", ext, ext));
                }
                if !has_default_ctor {
                    self.emit(&format!("{}() {{}}", name));
                }
                if let Some(constructors) = constructor {
                    for c in constructors {
                        let param_list: Vec<String> = c
                            .params
                            .iter()
                            .filter(|p| p.type_node.as_str() != "type")
                            .map(|p| {
                                let cpp_t = type_to_cpp(&p.type_node);
                                if cpp_t == *name
                                    || cpp_t.ends_with(&format!("::{}", name))
                                    || p.type_node.get_name() == *name
                                {
                                    if let Some(ref def_val) = p.default_value {
                                        let def_code = self.visit_expression(def_val);
                                        format!("const {}& {} = {}", cpp_t, p.name, def_code)
                                    } else {
                                        format!("const {}& {}", cpp_t, p.name)
                                    }
                                } else {
                                    self.format_param(p)
                                }
                            })
                            .collect();
                        self.emit(&format!("{}({}) {{", name, param_list.join(", ")));
                        self.indent_level += 1;
                        for s in &c.body {
                            self.visit_statement(s);
                        }
                        self.indent_level -= 1;
                        self.emit("}");

                        if c.params.len() == 1
                            && matches!(&c.params[0].type_node, BaseType::Array { base_type, .. } if matches!(**base_type, BaseType::Char))
                        {
                            self.emit(
                                &format!(
                                    "template <size_t N> {}(const char (&arr)[N]) : {}(fastlang_slice<char>(arr)) {{}}",
                                    name,
                                    name
                                )
                            );
                            self.emit(
                                &format!(
                                    "{}(const char* s) : {}(fastlang_slice<char>(s, s ? std::char_traits<char>::length(s) : 0)) {{}}",
                                    name,
                                    name
                                )
                            );
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
                handle_block,
                visibility: _,
                ..
            } => {
                let old_in_class = self.in_class_or_scope;
                self.in_class_or_scope = true;
                self.emit(&format!("struct {} {{", name));
                self.indent_level += 1;
                if decls_use_flag(public_block, "broken")
                    || decls_use_flag(private_block, "broken")
                    || decls_use_flag(static_block, "broken")
                {
                    self.emit("bool broken = false;");
                }
                if decls_use_flag(public_block, "is_done")
                    || decls_use_flag(private_block, "is_done")
                    || decls_use_flag(static_block, "is_done")
                {
                    self.emit("bool is_done = false;");
                }
                if decls_use_flag(public_block, "yielded")
                    || decls_use_flag(private_block, "yielded")
                    || decls_use_flag(static_block, "yielded")
                {
                    self.emit("bool yielded = false;");
                }
                if decls_use_flag(public_block, "leaved")
                    || decls_use_flag(private_block, "leaved")
                    || decls_use_flag(static_block, "leaved")
                {
                    self.emit("bool leaved = false;");
                }
                if decls_use_flag(public_block, "returned")
                    || decls_use_flag(private_block, "returned")
                    || decls_use_flag(static_block, "returned")
                {
                    self.emit("bool returned = false;");
                }
                if decls_use_flag(public_block, "continued")
                    || decls_use_flag(private_block, "continued")
                    || decls_use_flag(static_block, "continued")
                {
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
                        let param_list: Vec<String> = c
                            .params
                            .iter()
                            .filter(|p| p.type_node.as_str() != "type")
                            .map(|p| {
                                let cpp_t = type_to_cpp(&p.type_node);
                                if cpp_t == *name
                                    || cpp_t.ends_with(&format!("::{}", name))
                                    || p.type_node.get_name() == *name
                                {
                                    format!("const {}& {}", cpp_t, p.name)
                                } else {
                                    format!("{} {}", cpp_t, p.name)
                                }
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
                let display_fn = handle_block.iter().find_map(|h| {
                    if let Decl::FnDecl {
                        name: fn_name,
                        return_type,
                        ..
                    } = h
                    {
                        if fn_name == "display" {
                            Some(return_type.clone())
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                });

                if let Some(ret_type) = display_fn {
                    self.emit(&format!(
                        "friend std::ostream& operator<<(std::ostream& os, const {}& obj) {{",
                        name
                    ));
                    if ret_type == BaseType::Void {
                        self.emit(&format!("    const_cast<{}&>(obj).display();", name));
                    } else {
                        self.emit(&format!("    os << const_cast<{}&>(obj).display();", name));
                    }
                    self.emit("    return os;");
                    self.emit("}");
                } else {
                    self.emit(&format!(
                        "friend std::ostream& operator<<(std::ostream& os, const {}& obj) {{",
                        name
                    ));
                    self.emit(&format!("    os << \"[struct {}]\";", name));
                    self.emit("    return os;");
                    self.emit("}");
                }

                self.emit_operator_overloads(&Some(handle_block.clone()));

                for h in handle_block {
                    self.visit_declaration(h);
                }
                let has_drop_handle = handle_block.iter().any(|h| {
                    if let Decl::FnDecl { name: fn_name, .. } = h {
                        fn_name == "drop"
                    } else {
                        false
                    }
                });
                if has_drop_handle {
                    self.emit("bool _fastlang_dropped = false;");
                    self.emit("void _fastlang_call_drop() {");
                    self.emit("    if (!_fastlang_dropped) {");
                    self.emit("        _fastlang_dropped = true;");
                    self.emit("        this->drop();");
                    self.emit("    }");
                    self.emit("}");
                    self.emit(&format!("~{}() {{ this->_fastlang_call_drop(); }}", name));
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
            Decl::FnDecl {
                name,
                generics,
                params,
                return_type,
                body,
                is_virtual,
                is_abstract,
                visibility: _,
            } => {
                if name.starts_with("@compile::") || crate::middle_end::semantic::analyzer::detect_execution_mode(body) == ExecutionMode::FullyCompilable {
                    return;
                }

                let has_variadic_param = params.iter().any(|p| p.is_variadic);
                let mut gen_names: Vec<String> = generics
                    .iter()
                    .map(|g| {
                        let s = g.as_str();
                        if s.starts_with("...") {
                            format!("typename... {}", s.trim_start_matches('.'))
                        } else {
                            format!("typename {}", s)
                        }
                    })
                    .collect();
                if gen_names.is_empty() && has_variadic_param {
                    gen_names.push("typename... __Args".to_string());
                }
                if !gen_names.is_empty() {
                    self.emit(&format!("template <{}>\n", gen_names.join(", ")));
                }

                let ret_type_str = if name == "main" {
                    "int".to_string()
                } else {
                    type_to_cpp(return_type)
                };

                let mut param_strs = Vec::new();
                let mut added_variadics = Vec::new();
                for param in params {
                    if param.is_variadic {
                        self.variadic_packs.insert(param.name.clone());
                        added_variadics.push(param.name.clone());
                    }
                    if matches!(
                        &param.type_node,
                        BaseType::Pointer(_) | BaseType::Modify(_) | BaseType::Copy(_)
                    ) {
                        self.pointer_vars.insert(param.name.clone());
                    } else if let BaseType::Name(inner) = &param.type_node {
                        if !matches!(&**inner, BaseType::Fn { .. } | BaseType::Method { .. }) {
                            self.pointer_vars.insert(param.name.clone());
                        }
                    }
                    param_strs.push(self.format_param(param));
                }

                let safe_name_str = cpp_safe_name(name);
                let safe_name = safe_name_str.as_str();

                if *is_abstract {
                    self.emit(&format!(
                        "virtual {} {}({}) = 0;",
                        ret_type_str,
                        safe_name,
                        param_strs.join(", ")
                    ));
                    return;
                }

                if body.is_empty() {
                    return;
                }

                let virtual_prefix = if *is_virtual || (self.in_class_or_scope && name != "main") {
                    "virtual "
                } else {
                    ""
                };
                self.emit(&format!(
                    "{}{} {}({}) {{",
                    virtual_prefix,
                    ret_type_str,
                    safe_name,
                    param_strs.join(", ")
                ));

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

                let error_handle = if let Some(handles) = self.function_handles.get(name) {
                    handles
                        .iter()
                        .find(|h| {
                            if let Decl::FnDecl { name: h_name, .. } = h {
                                h_name == "error" || h_name == "throw" || h_name == "has_error"
                            } else {
                                false
                            }
                        })
                        .cloned()
                } else {
                    None
                };

                if let Some(Decl::FnDecl {
                    params: err_params,
                    body: err_body,
                    ..
                }) = error_handle
                {
                    self.emit("try {");
                    self.indent_level += 1;
                    for s in body {
                        self.visit_statement(s);
                    }
                    self.indent_level -= 1;
                    let catch_type = if !err_params.is_empty() {
                        let p_type = type_to_cpp(&err_params[0].type_node);
                        if p_type == "Error" || p_type == "fast_std::Error" {
                            "const fast_std::Error&".to_string()
                        } else {
                            format!("const {}&", p_type)
                        }
                    } else {
                        "const fast_std::Error&".to_string()
                    };
                    let catch_param = if !err_params.is_empty() {
                        err_params[0].name.clone()
                    } else {
                        "__e".to_string()
                    };
                    self.emit(&format!("}} catch ({} {}) {{", catch_type, catch_param));
                    self.indent_level += 1;
                    for s in &err_body {
                        self.visit_statement(s);
                    }
                    self.indent_level -= 1;
                    self.emit("}");
                } else {
                    for s in body {
                        self.visit_statement(s);
                    }
                }
                self.indent_level -= 1;

                self.emit("}");
                for v in added_variadics {
                    self.variadic_packs.remove(&v);
                }
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
            Decl::BlockDecl {
                visibility,
                name,
                return_type,
                statements,
                ..
            } => {
                if *visibility == Visibility::Public {
                    self.emit(&format!("namespace {} {{", name));
                    self.indent_level += 1;
                    for s in statements {
                        self.visit_statement(s);
                    }
                    self.indent_level -= 1;
                    self.emit("}");
                    return;
                }
                let has_yield = stmts_use_flag(statements, "has_yielded");
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
                let mut block_vars: Vec<(String, BaseType)> = Vec::new();
                if has_yield {
                    collect_block_vars(statements, &mut block_vars);
                    for (f_name, type_node) in &block_vars {
                        let cpp_t = type_to_cpp(type_node);
                        self.emit(&format!("{} {};", cpp_t, f_name));
                    }
                } else {
                    for s in statements {
                        if let Stmt::Declaration(Decl::VarDecl {
                            name: f_name,
                            type_node,
                            ..
                        }) = s
                        {
                            let cpp_t = type_to_cpp(type_node);
                            self.emit(&format!("{} {};", cpp_t, f_name));
                        }
                    }
                }
                for s in statements {
                    if matches!(s, Stmt::Declaration(Decl::FnDecl { .. })) {
                        self.visit_statement(s);
                    }
                }
                self.emit(&format!("{} operator()() {{", ret_cpp));
                self.indent_level += 1;
                if has_yield {
                    let var_set: std::collections::HashSet<String> =
                        block_vars.into_iter().map(|(n, _)| n).collect();
                    self.current_block_vars = Some(var_set);
                    self.emit("switch(this->__state) {");
                    self.emit("case 0:");
                    self.indent_level += 1;
                }
                for s in statements {
                    match s {
                        Stmt::Declaration(Decl::FnDecl { .. }) => {
                            // Already emitted outside operator()()
                        }
                        _ => {
                            self.visit_statement(s);
                        }
                    }
                }
                if has_yield {
                    self.current_block_vars = None;
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
            Decl::MicroDecl {
                name,
                params,
                return_type,
                body,
                ..
            } => {
                let is_break_micro = body.len() == 1 && matches!(body[0], Stmt::BreakStmt);
                if is_break_micro && params.is_empty() {
                    self.emit(&format!("#define {}() break", name));
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

                self.emit(&format!(
                    "inline {} {}({}) {{",
                    ret_cpp,
                    name,
                    param_strs.join(", ")
                ));
                self.indent_level += 1;
                for s in body {
                    self.visit_statement(s);
                }
                self.indent_level -= 1;
                self.emit("}");
            }
            Decl::ExternFnDecl { name, alias, .. } => {
                if let Some(alias_name) = alias {
                    self.emit(&format!("#define {} {}", alias_name, name));
                }
            }
            Decl::ExternBlockDecl { decls, .. } => {
                for d in decls {
                    if let Decl::ExternFnDecl {
                        name,
                        alias: Some(alias_name),
                        ..
                    } = d
                    {
                        self.emit(&format!("#define {} {}", alias_name, name));
                    }
                }
            }
            Decl::DefineDecl {
                name,
                type_alias,
                value,
                ..
            } => {
                if let Some(t) = type_alias {
                    let cpp_t = type_to_cpp(t);
                    self.emit(&format!("using {} = {};", name, cpp_t));
                } else if let Some(expr) = value {
                    let val_str = self.visit_expression(expr);
                    self.emit(&format!("constexpr auto {} = {};", name, val_str));
                }
            }
            Decl::Import {
                module_path, abi, ..
            } => {
                if let Some(_abi_str) = abi {
                    if let Some(header) = module_path.first() {
                        if header.ends_with(".h")
                            || header.ends_with(".hpp")
                            || !header.contains('/')
                        {
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
            Decl::MachineDecl {
                name,
                return_type,
                labels,
                handle_block,
                ..
            } => {
                let call_ret = if matches!(return_type, BaseType::Unknown) {
                    // fallback to finding it in call or leave
                    handle_block
                        .iter()
                        .find_map(|h| {
                            if let Decl::FnDecl {
                                name: fn_name,
                                return_type: rt,
                                ..
                            } = h
                            {
                                if fn_name == "call" || fn_name == "leave" {
                                    Some(type_to_cpp(rt))
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        })
                        .unwrap_or_else(|| "void".to_string())
                } else {
                    type_to_cpp(return_type)
                };

                let call_params: Option<&Vec<Param>> = handle_block.iter().find_map(|h| {
                    if let Decl::FnDecl {
                        name: fn_name,
                        params,
                        ..
                    } = h
                    {
                        if fn_name == "call" {
                            Some(params)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                });

                let mut param_strs = Vec::new();
                let mut param_arg_names = Vec::new();
                if let Some(params) = call_params {
                    for p in params {
                        param_strs.push(format!("{} {}", type_to_cpp(&p.type_node), p.name));
                        param_arg_names.push(p.name.clone());
                    }
                }

                let old_in_class = self.in_class_or_scope;
                self.in_class_or_scope = true;
                self.emit(&format!("struct {} {{", name));
                self.indent_level += 1;

                // Machine internal state
                self.emit("bool has_yielded = false;");
                self.emit("int32_t __state = 0;");

                // Hoist `this.var = value` or `this.var := value` or label variables as struct fields
                let mut field_names: Vec<String> = Vec::new();
                let mut fields_out: Vec<String> = Vec::new();

                fn collect_machine_fields(
                    s: &Stmt,
                    field_names: &mut Vec<String>,
                    fields_out: &mut Vec<String>,
                ) {
                    match s {
                        Stmt::Declaration(Decl::VarDecl {
                            name: f_name,
                            type_node,
                            ..
                        }) => {
                            if !field_names.contains(f_name) {
                                let cpp_t = type_to_cpp(type_node);
                                fields_out.push(format!("{} {} = {{}};", cpp_t, f_name));
                                field_names.push(f_name.clone());
                            }
                        }
                        Stmt::Block(inner) | Stmt::ThisBlock(inner) => {
                            for sub in inner {
                                collect_machine_fields(sub, field_names, fields_out);
                            }
                        }
                        Stmt::IfStmt {
                            then_block,
                            else_block,
                            ..
                        } => {
                            for sub in then_block {
                                collect_machine_fields(sub, field_names, fields_out);
                            }
                            if let Some(eb) = else_block {
                                for sub in eb {
                                    collect_machine_fields(sub, field_names, fields_out);
                                }
                            }
                        }
                        Stmt::WhileStmt { body, .. } | Stmt::DoWhileStmt { body, .. } => {
                            if let EitherBlock::Inline(sub) = body {
                                for st in sub {
                                    collect_machine_fields(st, field_names, fields_out);
                                }
                            }
                        }
                        Stmt::ForStmt { init, body, .. } => {
                            if let Some(i) = init {
                                collect_machine_fields(i, field_names, fields_out);
                            }
                            if let EitherBlock::Inline(sub) = body {
                                for st in sub {
                                    collect_machine_fields(st, field_names, fields_out);
                                }
                            }
                        }
                        _ => {}
                    }
                }

                // Collect from handle_block first so typed `this { ... }` declarations take precedence
                for h in handle_block {
                    if let Decl::FnDecl { body, .. } = h {
                        for s in body {
                            collect_machine_fields(s, &mut field_names, &mut fields_out);
                        }
                    }
                }
                for label_decl in labels.iter() {
                    if let Decl::LabelDecl { body, .. } = label_decl {
                        for s in body {
                            collect_machine_fields(s, &mut field_names, &mut fields_out);
                        }
                    }
                }

                for f in fields_out {
                    self.emit(&f);
                }

                // Call handle body logic
                let call_body: Option<&Vec<Stmt>> = handle_block.iter().find_map(|h| {
                    if let Decl::FnDecl {
                        name: fn_name,
                        body,
                        ..
                    } = h
                    {
                        if fn_name == "call" {
                            Some(body)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                });

                let error_handle = handle_block.iter().find(|h| {
                    if let Decl::FnDecl { name: fn_name, .. } = h {
                        fn_name == "error"
                    } else {
                        false
                    }
                });

                // operator() with bounded labels and source-order continue
                self.emit(&format!("inline {} operator()() {{", call_ret));
                self.indent_level += 1;

                if let Some(Decl::FnDecl { .. }) = error_handle {
                    self.emit("try {");
                    self.indent_level += 1;
                }

                if let Some(body) = call_body {
                    for s in body {
                        if !matches!(s, Stmt::ThisBlock(_)) {
                            self.visit_statement(s);
                        }
                    }
                }

                // Labels in exact source order
                for i in 0..labels.len() {
                    let lbl_decl = &labels[i];
                    if let Decl::LabelDecl {
                        name: lbl_name,
                        body,
                    } = lbl_decl
                    {
                        let clean = lbl_name.replace("@", "");
                        self.emit(&format!("{}: {{", clean));
                        self.indent_level += 1;
                        for s in body {
                            match s {
                                Stmt::Declaration(Decl::VarDecl {
                                    name: f_name,
                                    value,
                                    assign_op,
                                    ..
                                }) => {
                                    let val_code = self.visit_expression(value);
                                    if assign_op == "->" {
                                        self.emit(&format!(
                                            "fastlang_arrow_assign(this->{}, {});",
                                            f_name, val_code
                                        ));
                                    } else {
                                        self.emit(&format!("this->{} = {};", f_name, val_code));
                                    }
                                }
                                Stmt::ContinueStmt => {
                                    if i + 1 < labels.len() {
                                        if let Decl::LabelDecl {
                                            name: next_name, ..
                                        } = &labels[i + 1]
                                        {
                                            let next_clean = next_name.replace("@", "");
                                            self.emit(&format!("goto {};", next_clean));
                                        }
                                    } else {
                                        self.emit("return leave();");
                                    }
                                }
                                Stmt::BreakStmt => {
                                    self.emit("return leave();");
                                }
                                Stmt::LeaveStmt => {
                                    self.emit("return leave();");
                                }
                                Stmt::YieldStmt(opt_expr) => {
                                    self.emit("this->has_yielded = true;");
                                    if let Some(e) = opt_expr {
                                        let val = self.visit_expression(e);
                                        self.emit(&format!("return {};", val));
                                    } else {
                                        self.emit("return this->yield();");
                                    }
                                }
                                _ => self.visit_statement(s),
                            }
                        }

                        // implicit leave at end of bounded label block to prevent fallthrough
                        self.emit("return leave();");

                        self.indent_level -= 1;
                        self.emit("}");
                    }
                }

                self.emit("return leave();");

                if let Some(Decl::FnDecl {
                    params: err_params,
                    body: err_body,
                    ..
                }) = error_handle
                {
                    self.indent_level -= 1;
                    let catch_param = if !err_params.is_empty() {
                        err_params[0].name.clone()
                    } else {
                        "__e".to_string()
                    };
                    self.emit(&format!(
                        "}} catch (const fast_std::Error& {}) {{",
                        catch_param
                    ));
                    self.indent_level += 1;
                    for s in err_body {
                        self.visit_statement(s);
                    }
                    if call_ret != "void" {
                        self.emit("return leave();");
                    }
                    self.indent_level -= 1;
                    self.emit(&format!(
                        "}} catch (const fast_std::Error* __{}_ptr) {{",
                        catch_param
                    ));
                    self.indent_level += 1;
                    self.emit(&format!(
                        "const fast_std::Error& {} = *__{}_ptr;",
                        catch_param, catch_param
                    ));
                    for s in err_body {
                        self.visit_statement(s);
                    }
                    if call_ret != "void" {
                        self.emit("return leave();");
                    }
                    self.indent_level -= 1;
                    self.emit("}");
                }

                self.indent_level -= 1;
                self.emit("}");

                // Overloaded operator() with arguments if params exist
                if !param_strs.is_empty() {
                    let mut assigns = Vec::new();
                    if let Some(params) = call_params {
                        for p in params {
                            assigns.push(format!("this->{} = {};", p.name, p.name));
                        }
                    }
                    self.emit(&format!(
                        "inline {} operator()({}) {{ {} return (*this)(); }}",
                        call_ret,
                        param_strs.join(", "),
                        assigns.join(" ")
                    ));
                }

                // Emit `call()` methods
                if !param_strs.is_empty() {
                    let mut assigns = Vec::new();
                    if let Some(params) = call_params {
                        for p in params {
                            assigns.push(format!("this->{} = {};", p.name, p.name));
                        }
                    }
                    self.emit(&format!(
                        "inline {} call({}) {{ {} return (*this)(); }}",
                        call_ret,
                        param_strs.join(", "),
                        assigns.join(" ")
                    ));
                }
                self.emit(&format!(
                    "inline {} call() {{ return (*this)(); }}",
                    call_ret
                ));

                // Emit `leave` handle as a separate method
                let mut found_leave = false;
                for h in handle_block.iter() {
                    if let Decl::FnDecl {
                        name: fn_name,
                        return_type,
                        body,
                        ..
                    } = h
                    {
                        if fn_name == "leave" {
                            found_leave = true;
                            let ret_str = type_to_cpp(return_type);
                            self.emit(&format!("{} leave() {{", ret_str));
                            self.indent_level += 1;
                            for s in body {
                                self.visit_statement(s);
                            }
                            if ret_str != "void" {
                                self.emit("return {};");
                            }
                            self.indent_level -= 1;
                            self.emit("}");
                        }
                    }
                }
                if !found_leave {
                    self.emit(&format!(
                        "{} leave() {{ {} }}",
                        call_ret,
                        if call_ret == "void" { "" } else { "return {};" }
                    ));
                }

                // Emit `yield` handle as a separate method
                let mut found_yield = false;
                for h in handle_block.iter() {
                    if let Decl::FnDecl {
                        name: fn_name,
                        return_type,
                        body,
                        ..
                    } = h
                    {
                        if fn_name == "yield" {
                            found_yield = true;
                            let ret_str = type_to_cpp(return_type);
                            self.emit(&format!("{} yield() {{", ret_str));
                            self.indent_level += 1;
                            for s in body {
                                self.visit_statement(s);
                            }
                            if ret_str != "void" {
                                self.emit("return {};");
                            }
                            self.indent_level -= 1;
                            self.emit("}");
                        }
                    }
                }
                if !found_yield {
                    self.emit(&format!(
                        "{} yield() {{ {} }}",
                        call_ret,
                        if call_ret == "void" { "" } else { "return {};" }
                    ));
                }

                // Default constructor
                self.emit(&format!("{}() {{}}", name));

                // Optional constructor with call_params
                if let Some(params) = call_params {
                    if !params.is_empty() {
                        let mut ctor_init = Vec::new();
                        for p in params {
                            ctor_init.push(format!("this->{} = {};", p.name, p.name));
                        }
                        self.emit(&format!(
                            "{}({}) {{ {} }}",
                            name,
                            param_strs.join(", "),
                            ctor_init.join(" ")
                        ));
                    }
                }

                // friend ostream
                self.emit(&format!(
                    "friend std::ostream& operator<<(std::ostream& os, const {}& obj) {{",
                    name
                ));
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
            Decl::CompileDecl { .. } => {
                // Compile-time only constructs
            }
            Decl::MacroDecl {
                name, params, body, ..
            } => {
                let mut param_strs = Vec::new();
                for param in params {
                    let cpp_t = if matches!(param.type_node, BaseType::Unknown) {
                        "auto".to_string()
                    } else {
                        type_to_cpp(&param.type_node)
                    };
                    param_strs.push(format!("{} {}", cpp_t, param.name));
                }

                self.emit(&format!(
                    "inline void {}({}) {{",
                    name,
                    param_strs.join(", ")
                ));
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

pub(crate) fn is_typeof_expr(expr: &Expr) -> bool {
    match expr {
        Expr::Call { callee, .. } => match &**callee {
            Expr::Identifier(name) => name == "typeof",
            Expr::NamespaceAccess {
                namespace,
                property,
            } => {
                namespace == "@compile"
                    && matches!(&**property, Expr::Identifier(p) if p == "typeof")
            }
            _ => false,
        },
        _ => false,
    }
}

fn is_comptime_reflection_expr(expr: &Expr) -> bool {
    match expr {
        Expr::Call { callee, .. } => {
            if let Expr::PropertyAccess { object, property } = &**callee {
                if matches!(
                    property.as_str(),
                    "printable"
                        | "is_printable"
                        | "is_pointer"
                        | "is_array"
                        | "is_primitive"
                        | "throwable"
                        | "is_throwable"
                        | "castable"
                        | "is_castable"
                        | "castable_to"
                        | "size"
                ) {
                    return is_typeof_expr(object);
                }
            }
            false
        }
        Expr::PropertyAccess { object, property } => {
            if matches!(
                property.as_str(),
                "printable"
                    | "is_printable"
                    | "is_pointer"
                    | "is_array"
                    | "is_primitive"
                    | "throwable"
                    | "is_throwable"
                    | "castable"
                    | "is_castable"
                    | "castable_to"
                    | "size"
            ) {
                return is_typeof_expr(object);
            }
            false
        }
        Expr::UnaryOp { operator, operand } if operator == "!" => {
            is_comptime_reflection_expr(operand)
        }
        _ => false,
    }
}
