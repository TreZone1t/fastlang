use crate::backend::cpp::generator::CodeGenerator;
use crate::backend::cpp::stmt::type_to_cpp;
use crate::frontend::parser::ast::*;

impl CodeGenerator {
    pub(crate) fn visit_expression(&mut self, expr: &Expr) -> String {
        match expr {
            Expr::LiteralInt(val) => val.to_string(),
            Expr::LiteralUInt(val) => {
                if *val > i64::MAX as u128 {
                    format!("{}ULL", val)
                } else {
                    val.to_string()
                }
            }
            Expr::LiteralFloat(f) => format!("{:?}", f),
            Expr::LiteralString(s) => {
                let mut chars_code = Vec::new();
                for &b in s.as_bytes() {
                    let b_str = match b {
                        b'\n' => "'\\n'".to_string(),
                        b'\t' => "'\\t'".to_string(),
                        b'\r' => "'\\r'".to_string(),
                        b'\\' => "'\\\\'".to_string(),
                        b'\'' => "'\\''".to_string(),
                        b'\0' => "'\\0'".to_string(),
                        32..=126 => format!("'{}'", b as char),
                        _ => format!("(char)0x{:02x}", b),
                    };
                    chars_code.push(b_str);
                }
                chars_code.push("'\\0'".to_string());
                format!("fastlang_string_create({{{}}})", chars_code.join(", "))
            }
            Expr::LiteralChar(c) => match c {
                '\n' => "'\\n'".to_string(),
                '\t' => "'\\t'".to_string(),
                '\r' => "'\\r'".to_string(),
                '\\' => "'\\\\'".to_string(),
                '\'' => "'\\''".to_string(),
                '\0' => "'\\0'".to_string(),
                _ => format!("'{}'", c),
            },
            Expr::LiteralUChar(c) => format!("static_cast<char32_t>({}U)", c),
            Expr::LiteralBool(val) => {
                if *val {
                    "true".to_string()
                } else {
                    "false".to_string()
                }
            }
            Expr::LiteralVoid => "fastlang_unit_t{}".to_string(),
            Expr::LiteralUndefined => "fastlang_undefined_t{}".to_string(),
            Expr::ArrayLiteral(elements) => {
                let has_spread = elements.iter().any(|e| matches!(e, Expr::Spread(_)));
                if has_spread {
                    let first = elements
                        .iter()
                        .find(|e| !matches!(e, Expr::Spread(_)))
                        .or_else(|| elements.first());
                    let mut stmts = Vec::new();
                    let first_type_decl = if let Some(f) = first {
                        match f {
                            Expr::Spread(inner) => {
                                let c = self.visit_expression(inner);
                                format!("using __elem_t = std::decay_t<decltype(*({}))>;", c)
                            }
                            _ => {
                                let c = self.visit_expression(f);
                                format!("using __elem_t = std::decay_t<decltype({})>;", c)
                            }
                        }
                    } else {
                        "using __elem_t = int32_t;".to_string()
                    };
                    stmts.push(first_type_decl);
                    stmts.push("fastlang_spread_acc<__elem_t> __acc;".to_string());
                    for el in elements {
                        match el {
                            Expr::Spread(inner) => {
                                let inner_code = self.visit_expression(inner);
                                stmts.push(format!("__acc.push_spread({});", inner_code));
                            }
                            _ => {
                                let el_code = self.visit_expression(el);
                                stmts.push(format!("__acc.push_one({});", el_code));
                            }
                        }
                    }
                    stmts.push("return __acc.to_array();".to_string());
                    format!("([&]() {{\n    {}\n}}())", stmts.join("\n    "))
                } else if elements.is_empty() {
                    "fastlang_empty_array_t{}".to_string()
                } else {
                    let mut elems_code = Vec::new();
                    for el in elements {
                        elems_code.push(self.visit_expression(el));
                    }
                    let first_elem = &elems_code[0];
                    format!(
                        "fastlang_array_create<std::decay_t<decltype({})>>({{{}}})",
                        first_elem,
                        elems_code.join(", ")
                    )
                }
            }
            Expr::Spread(inner) => {
                let inner_code = self.visit_expression(inner);
                if self.variadic_packs.contains(&inner_code) {
                    format!("{}...", inner_code)
                } else {
                    format!("fastlang_spread({})", inner_code)
                }
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
            Expr::Default(type_arg) => {
                if let Some(t) = type_arg {
                    format!("{}()", type_to_cpp(t))
                } else {
                    "{}".to_string()
                }
            }
            Expr::Identifier(name) => crate::backend::cpp::stmt::cpp_safe_name(name),
            Expr::This => {
                if self.in_primitive_impl || self.in_machine {
                    "__this".to_string()
                } else {
                    "this".to_string()
                }
            }
            Expr::Super => "super".to_string(), // will be handled in PropertyAccess
            Expr::Global => "::".to_string(),
            Expr::BinaryOp {
                left,
                operator,
                right,
            } => {
                let l = self.visit_expression(left);
                let r = self.visit_expression(right);
                if operator == "->" {
                    let l_type = if let Expr::Identifier(var_name) = &**left {
                        self.vars.get(var_name).cloned()
                    } else if let Expr::PropertyAccess { object, property } = &**left {
                        if matches!(&**object, Expr::This) {
                            self.current_class_name.as_ref().and_then(|cls| {
                                self.struct_field_types
                                    .get(cls)
                                    .and_then(|fields| fields.get(property).cloned())
                            })
                        } else if let Expr::Identifier(obj_name) = &**object {
                            self.vars.get(obj_name.as_str()).and_then(|obj_ty| {
                                let base = Self::clean_type_name(obj_ty);
                                self.struct_field_types
                                    .get(&base)
                                    .and_then(|fields| fields.get(property).cloned())
                            })
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                    let handle = l_type
                        .as_deref()
                        .and_then(|t| self.find_assign_handle(t, &["arrow", "arrow_assign"]));
                    if let Some(h) = handle {
                        format!("{}.{}({})", l, h, r)
                    } else if self.custom_scopes.contains(&l) {
                        format!("([&]() {{ {}.arrow({}); return {}; }}())", l, r, l)
                    } else {
                        format!("{}.arrow({})", l, r)
                    }
                } else if operator == "=>" {
                    let l_type = if let Expr::Identifier(var_name) = &**left {
                        self.vars.get(var_name).cloned()
                    } else if let Expr::PropertyAccess { object, property } = &**left {
                        if matches!(&**object, Expr::This) {
                            self.current_class_name.as_ref().and_then(|cls| {
                                self.struct_field_types
                                    .get(cls)
                                    .and_then(|fields| fields.get(property).cloned())
                            })
                        } else if let Expr::Identifier(obj_name) = &**object {
                            self.vars.get(obj_name.as_str()).and_then(|obj_ty| {
                                let base = Self::clean_type_name(obj_ty);
                                self.struct_field_types
                                    .get(&base)
                                    .and_then(|fields| fields.get(property).cloned())
                            })
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                    let handle = l_type
                        .as_deref()
                        .and_then(|t| self.find_assign_handle(t, &["fat_arrow"]));
                    if let Some(h) = handle {
                        format!("{}.{}({})", l, h, r)
                    } else {
                        format!("{}.fat_arrow({})", l, r)
                    }
                } else if operator == "+" {
                    format!("fastlang_add({}, {})", l, r)
                } else if operator == "==" {
                    format!("fastlang_eq({}, {})", l, r)
                } else if operator == "!=" {
                    format!("fastlang_ne({}, {})", l, r)
                } else {
                    format!("({} {} {})", l, operator, r)
                }
            }
            Expr::PostfixUpdate { left, operator } => {
                let l = self.visit_expression(left);
                format!("{}{}", l, operator)
            }
            Expr::PrefixUpdate { right, operator } => {
                let r = self.visit_expression(right);
                format!("{}{}", operator, r)
            }
            Expr::IndexAccess { object, indices } => {
                let obj_code = self.visit_expression(object);
                let indices_code: Vec<String> =
                    indices.iter().map(|i| self.visit_expression(i)).collect();
                if self.custom_scopes.contains(&obj_code) || indices_code.len() > 1 {
                    format!("{}.index_access({})", obj_code, indices_code.join(", "))
                } else if indices_code.len() == 1 {
                    format!("{}[{}]", obj_code, indices_code[0])
                } else {
                    let chained = indices_code
                        .iter()
                        .map(|i| format!("[{}]", i))
                        .collect::<Vec<_>>()
                        .join("");
                    format!("{}{}", obj_code, chained)
                }
            }
            Expr::UnaryOp { operator, operand } => {
                let op_code = self.visit_expression(operand);
                if operator == "copy" {
                    format!("fastlang_copy(&{})", op_code)
                } else if operator == "*" && self.address_vars.contains(&op_code) {
                    op_code
                } else {
                    format!("{}{}", operator, op_code)
                }
            }
            Expr::MacroCall { callee, args } => {
                let callee_code = self.visit_expression(callee);
                let mut args_code = Vec::new();
                for arg in args {
                    args_code.push(self.visit_expression(arg));
                }
                format!("{}({})", callee_code, args_code.join(", "))
            }

            Expr::Call {
                callee,
                generics,
                args,
            } => {
                if let Expr::PropertyAccess { object, property } = &**callee {
                    if property == "default" {
                        if let Some(cpp_t) = self.extract_cpp_type_from_expr(object) {
                            return format!("fastlang_type_default<{}>()", cpp_t);
                        }
                    }
                    if property == "as_str" && args.is_empty() {
                        if matches!(**object, Expr::This) {
                            if self.in_primitive_impl {
                                let target =
                                    self.current_primitive_target.as_deref().unwrap_or("array");
                                return format!("fastlang_{}_as_str(__this)", target);
                            }
                            return "this->as_str()".to_string();
                        }
                        let obj_code = self.visit_expression(object);
                        return format!("fastlang_as_str({})", obj_code);
                    }
                    if property == "copy" && args.is_empty() {
                        let obj_code = self.visit_expression(object);
                        return format!("fastlang_copy({})", obj_code);
                    }

                    if let Some(targets) = self.primitive_impl_methods.get(property).cloned() {
                        let obj_code = self.visit_expression(object);
                        let cpp_type = self.extract_cpp_type_from_expr(object).unwrap_or_default();
                        let is_this = matches!(**object, Expr::This);

                        let matched_target = if is_this && self.in_primitive_impl {
                            self.current_primitive_target.clone()
                        } else if targets.iter().any(|t| t == &cpp_type) {
                            Some(cpp_type.clone())
                        } else if targets.contains(&"array".to_string())
                            && (cpp_type == "array"
                                || cpp_type == "str"
                                || cpp_type == "fast_std::str"
                                || cpp_type.starts_with("fast_std::array")
                                || cpp_type.starts_with("array<")
                                || cpp_type.ends_with("[]"))
                        {
                            Some("array".to_string())
                        } else if targets.contains(&"char".to_string()) && cpp_type == "char" {
                            Some("char".to_string())
                        } else if targets.contains(&"bool".to_string())
                            && (cpp_type == "bool" || cpp_type == "flag")
                        {
                            Some("bool".to_string())
                        } else {
                            None
                        };

                        if let Some(target) = matched_target {
                            let mut args_code = Vec::new();
                            for arg in args {
                                args_code.push(self.visit_expression(arg));
                            }
                            let mut all_args = vec![obj_code];
                            all_args.extend(args_code);
                            return format!(
                                "fastlang_{}_{}({})",
                                target,
                                property,
                                all_args.join(", ")
                            );
                        }
                    }
                }

                let callee_code = match &**callee {
                    Expr::PropertyAccess { object, property } => {
                        let obj_code = self.visit_expression(object);
                        let safe_prop = crate::backend::cpp::stmt::cpp_safe_name(property);
                        if obj_code == "::" {
                            format!("::{}", safe_prop)
                        } else if self.enum_types.contains(&obj_code) {
                            format!("{}::{}", obj_code, safe_prop)
                        } else if obj_code == "super" || obj_code == "this" {
                            format!("this->{}", safe_prop)
                        } else if self.raw_pointer_vars.contains(&obj_code) {
                            format!("{}->{}", obj_code, safe_prop)
                        } else if let Some(t) = self.extract_cpp_type_from_expr(object) {
                            if t.ends_with('*') {
                                format!("{}->{}", obj_code, safe_prop)
                            } else {
                                format!("{}.{}", obj_code, safe_prop)
                            }
                        } else {
                            format!("{}.{}", obj_code, safe_prop)
                        }
                    }
                    Expr::NamespaceAccess {
                        namespace,
                        property,
                    } => {
                        let prop_code = self.visit_expression(property);
                        let clean_prop = prop_code.trim_end_matches("()");
                        if namespace == "@compile" {
                            clean_prop.to_string()
                        } else {
                            format!("{}::{}", namespace, clean_prop)
                        }
                    }
                    _ => self.visit_expression(callee),
                };

                let mut args_code = Vec::new();
                for arg in args {
                    args_code.push(self.visit_expression(arg));
                }

                let gen_code = if !generics.is_empty() {
                    let gen_strs: Vec<String> = generics.iter().map(type_to_cpp).collect();
                    format!("<{}>", gen_strs.join(", "))
                } else {
                    "".to_string()
                };

                let safe_callee = match &**callee {
                    Expr::UnaryOp { .. } => format!("({})", callee_code),
                    _ => callee_code,
                };

                if self.custom_scope_types.contains(&safe_callee) {
                    format!("{}{}()({})", safe_callee, gen_code, args_code.join(", "))
                } else if self.pointer_vars.contains(&safe_callee) {
                    format!("(*{}){}({})", safe_callee, gen_code, args_code.join(", "))
                } else {
                    format!("{}{}({})", safe_callee, gen_code, args_code.join(", "))
                }
            }
            Expr::Instantiate { target, args } => {
                let target_code = self.visit_expression(target);
                if args.len() == 1 && matches!(args[0], Expr::ObjectLiteral(_)) {
                    if let Expr::ObjectLiteral(ref stmts) = args[0] {
                        if target_code.contains("::") {
                            let parts: Vec<&str> = target_code.split("::").collect();
                            if parts.len() == 2 && self.enum_types.contains(parts[0]) {
                                let enum_name = parts[0];
                                let variant_name = parts[1].trim_end_matches("()");
                                let mut struct_code = format!("([&]() {{\n    {} __obj;\n    __obj.tag = {}::Tag::{};\n    typename {}::{}_Payload __payload;\n", enum_name, enum_name, variant_name, enum_name, variant_name);
                                let mut temp_gen = CodeGenerator::new();
                                temp_gen.indent_level = self.indent_level + 1;
                                for s in stmts {
                                    match s {
                                        Stmt::Declaration(Decl::VarDecl {
                                            name,
                                            value,
                                            assign_op,
                                            ..
                                        }) => {
                                            let v = temp_gen.visit_expression(value);
                                            if !matches!(value, Expr::Default(None)) && v != "{}" {
                                                let op = if assign_op.is_empty() {
                                                    "="
                                                } else {
                                                    assign_op.as_str()
                                                };
                                                temp_gen.emit(&format!(
                                                    "__payload.{} {} {};",
                                                    name, op, v
                                                ));
                                            }
                                        }
                                        Stmt::ReassignStmt { target, value, op } => {
                                            let t = temp_gen.visit_expression(target);
                                            let v = temp_gen.visit_expression(value);
                                            temp_gen
                                                .emit(&format!("__payload.{} {} {};", t, op, v));
                                        }
                                        _ => {
                                            temp_gen.visit_statement(s);
                                        }
                                    }
                                }
                                struct_code.push_str(&temp_gen.output);
                                struct_code.push_str(&format!(
                                    "{}__obj.data = __payload;\n{}return __obj;\n{}}}())",
                                    "    ".repeat(self.indent_level + 1),
                                    "    ".repeat(self.indent_level + 1),
                                    "    ".repeat(self.indent_level)
                                ));
                                return struct_code;
                            }
                        }
                        let mut struct_code = format!("([&]() {{\n    {} __obj;\n", target_code);
                        let mut temp_gen = CodeGenerator::new();
                        temp_gen.indent_level = self.indent_level + 1;
                        for s in stmts {
                            match s {
                                Stmt::Declaration(Decl::VarDecl {
                                    name,
                                    value,
                                    assign_op,
                                    ..
                                }) => {
                                    let v = temp_gen.visit_expression(value);
                                    if !matches!(value, Expr::Default(None)) && v != "{}" {
                                        let op = if assign_op.is_empty() {
                                            "="
                                        } else {
                                            assign_op.as_str()
                                        };
                                        temp_gen.emit(&format!("__obj.{} {} {};", name, op, v));
                                    }
                                }
                                Stmt::ReassignStmt { target, value, op } => {
                                    let t = temp_gen.visit_expression(target);
                                    let v = temp_gen.visit_expression(value);
                                    temp_gen.emit(&format!("__obj.{} {} {};", t, op, v));
                                }
                                _ => {
                                    temp_gen.visit_statement(s);
                                }
                            }
                        }
                        struct_code.push_str(&temp_gen.output);
                        struct_code.push_str(&format!(
                            "{}return __obj;\n{}}}())",
                            "    ".repeat(self.indent_level + 1),
                            "    ".repeat(self.indent_level)
                        ));
                        return struct_code;
                    }
                }
                let mut args_code = Vec::new();
                for arg in args {
                    args_code.push(self.visit_expression(arg));
                }
                // Value instantiation in C++ (stack allocation)
                format!("{}({})", target_code, args_code.join(", "))
            }
            Expr::PropertyAccess { object, property } => {
                if property == "default" {
                    if let Some(cpp_t) = self.extract_cpp_type_from_expr(object) {
                        return format!("fastlang_type_default<{}>()", cpp_t);
                    }
                }
                if property == "len" {
                    let obj_code = self.visit_expression(object);
                    return format!("fastlang_array_len({})", obj_code);
                }
                let obj_code = self.visit_expression(object);
                let safe_prop = crate::backend::cpp::stmt::cpp_safe_name(property);
                if obj_code == "::" {
                    format!("::{}", safe_prop)
                } else if self.enum_types.contains(&obj_code) {
                    format!("{}::{}", obj_code, safe_prop)
                } else if obj_code == "super" || obj_code == "this" {
                    format!("this->{}", safe_prop)
                } else if self.raw_pointer_vars.contains(&obj_code) {
                    format!("{}->{}", obj_code, safe_prop)
                } else if let Some(t) = self.extract_cpp_type_from_expr(object) {
                    if t.ends_with('*') {
                        format!("{}->{}", obj_code, safe_prop)
                    } else {
                        format!("{}.{}", obj_code, safe_prop)
                    }
                } else {
                    format!("{}.{}", obj_code, safe_prop)
                }
            }
            Expr::NamespaceAccess {
                namespace,
                property,
            } => {
                let prop_code = self.visit_expression(property);
                if namespace.starts_with('@') {
                    format!("this->{}", prop_code)
                } else if self.payload_enum_types.contains(namespace) {
                    format!("{}::{}()", namespace, prop_code)
                } else {
                    let clean_prop = prop_code.trim_end_matches("()");
                    format!("{}::{}", namespace, clean_prop)
                }
            }
            Expr::ArrayAllocate {
                type_node,
                size,
                length,
            } => {
                let cpp_type: String = type_to_cpp(type_node);
                if let Some(init) = length {
                    let init_code = self.visit_expression(init);
                    format!("fastlang_array_create<{}>({})", cpp_type, init_code)
                } else {
                    let size_code = self.visit_expression(size);
                    format!(
                        "fastlang_array_alloc<{}>((size_t)({}))",
                        cpp_type, size_code
                    )
                }
            }
            Expr::New { type_node, target } => {
                if let BaseType::Array { base_type, size } = type_node {
                    let cpp_elem = type_to_cpp(base_type);
                    if let Some(s) = size.as_ref() {
                        let size_code = self.visit_expression(s);
                        return format!(
                            "fastlang_array_alloc<{}>((size_t)({}))",
                            cpp_elem, size_code
                        );
                    } else {
                        return format!("fastlang_array_alloc<{}>(0)", cpp_elem);
                    }
                }
                let cpp_type = type_to_cpp(type_node);
                match &**target {
                    Expr::Instantiate { args, .. } => {
                        let arg_strs: Vec<String> =
                            args.iter().map(|a| self.visit_expression(a)).collect();
                        format!("new {}({})", cpp_type, arg_strs.join(", "))
                    }
                    Expr::ArrayLiteral(elems) => {
                        let elem_strs: Vec<String> =
                            elems.iter().map(|e| self.visit_expression(e)).collect();
                        format!("new {}[]{{{}}}", cpp_type, elem_strs.join(", "))
                    }
                    _ => {
                        let target_code = self.visit_expression(target);
                        if target_code == "__default__"
                            || target_code == "{}"
                            || target_code.is_empty()
                        {
                            format!("new {}()", cpp_type)
                        } else {
                            format!("new {}({})", cpp_type, target_code)
                        }
                    }
                }
            }
            Expr::Lambda {
                params,
                return_type,
                body,
                ..
            } => {
                let p_list: Vec<String> = params
                    .iter()
                    .map(|p| {
                        if p.type_node == BaseType::Unknown
                            || matches!(p.type_node, BaseType::GenericParam(_))
                        {
                            format!("auto {}", p.name)
                        } else {
                            format!("{} {}", type_to_cpp(&p.type_node), p.name)
                        }
                    })
                    .collect();
                let ret_str = if let Some(rt) = return_type {
                    format!(" -> {}", type_to_cpp(rt))
                } else {
                    "".to_string()
                };

                let mut temp_gen = CodeGenerator::new();
                temp_gen.indent_level = self.indent_level + 1;
                for s in body {
                    temp_gen.visit_statement(s);
                }

                format!(
                    "([&]({}){} {{\n{}{}}})",
                    p_list.join(", "),
                    ret_str,
                    temp_gen.output,
                    "    ".repeat(self.indent_level)
                )
            }
            Expr::Cast { expr, target_type } => {
                let expr_code = self.visit_expression(expr);
                let cpp_type = type_to_cpp(target_type);
                format!("static_cast<{}>({})", cpp_type, expr_code)
            }
            Expr::HandleCall {
                object,
                handle_name,
                args,
            } => {
                let obj_code = self.visit_expression(object);
                let args_code: Vec<String> =
                    args.iter().map(|a| self.visit_expression(a)).collect();
                let safe_handle = crate::backend::cpp::stmt::cpp_safe_name(handle_name);

                if handle_name == "as_str" && args.is_empty() {
                    return format!("fastlang_as_str({})", obj_code);
                }
                if handle_name == "copy" && args.is_empty() {
                    return format!("fastlang_copy({})", obj_code);
                }

                if let Some(targets) = self.primitive_impl_methods.get(handle_name).cloned() {
                    let cpp_type = self.extract_cpp_type_from_expr(object).unwrap_or_default();
                    let is_this = matches!(**object, Expr::This);

                    let matched_target = if is_this && self.in_primitive_impl {
                        self.current_primitive_target.clone()
                    } else if targets.iter().any(|t| t == &cpp_type) {
                        Some(cpp_type.clone())
                    } else if targets.contains(&"array".to_string())
                        && (cpp_type == "array"
                            || cpp_type == "str"
                            || cpp_type == "fast_std::str"
                            || cpp_type.starts_with("fast_std::array")
                            || cpp_type.starts_with("array<")
                            || cpp_type.ends_with("[]"))
                    {
                        Some("array".to_string())
                    } else if targets.contains(&"char".to_string()) && cpp_type == "char" {
                        Some("char".to_string())
                    } else if targets.contains(&"bool".to_string())
                        && (cpp_type == "bool" || cpp_type == "flag")
                    {
                        Some("bool".to_string())
                    } else {
                        None
                    };

                    if let Some(target) = matched_target {
                        let mut all_args = vec![obj_code];
                        all_args.extend(args_code);
                        return format!(
                            "fastlang_{}_{}({})",
                            target,
                            handle_name,
                            all_args.join(", ")
                        );
                    }
                }

                if obj_code == "super" || obj_code == "this" {
                    format!("this->{}({})", safe_handle, args_code.join(", "))
                } else if self.raw_pointer_vars.contains(&obj_code) {
                    format!("{}->{}({})", obj_code, safe_handle, args_code.join(", "))
                } else {
                    format!("{}.{}({})", obj_code, safe_handle, args_code.join(", "))
                }
            }
            Expr::IfExpr {
                condition,
                then_branch,
                else_branch,
            } => {
                let has_block = matches!(**then_branch, Expr::BlockExpr { .. })
                    || matches!(**else_branch, Expr::BlockExpr { .. });
                if !has_block {
                    let cond_code = self.visit_expression(condition);
                    let then_code = self.visit_expression(then_branch);
                    let else_code = self.visit_expression(else_branch);
                    format!("(({}) ? ({}) : ({}))", cond_code, then_code, else_code)
                } else {
                    let cond_code = self.visit_expression(condition);
                    let old_out = std::mem::take(&mut self.output);
                    let old_indent = self.indent_level;
                    self.indent_level += 2;

                    let then_code = match &**then_branch {
                        Expr::BlockExpr {
                            statements,
                            final_expr,
                        } => {
                            for s in statements {
                                self.visit_statement(s);
                            }
                            if let Some(f) = final_expr {
                                let fc = self.visit_expression(f);
                                self.emit(&format!("return {};", fc));
                            }
                            std::mem::take(&mut self.output)
                        }
                        _ => {
                            let tc = self.visit_expression(then_branch);
                            self.emit(&format!("return {};", tc));
                            std::mem::take(&mut self.output)
                        }
                    };

                    let else_code = match &**else_branch {
                        Expr::BlockExpr {
                            statements,
                            final_expr,
                        } => {
                            for s in statements {
                                self.visit_statement(s);
                            }
                            if let Some(f) = final_expr {
                                let fc = self.visit_expression(f);
                                self.emit(&format!("return {};", fc));
                            }
                            std::mem::take(&mut self.output)
                        }
                        _ => {
                            let ec = self.visit_expression(else_branch);
                            self.emit(&format!("return {};", ec));
                            std::mem::take(&mut self.output)
                        }
                    };

                    self.indent_level = old_indent;
                    self.output = old_out;

                    let base_indent = "    ".repeat(self.indent_level);
                    let branch_indent = "    ".repeat(self.indent_level + 1);

                    format!(
                        "([&]() {{\n{}if ({}) {{\n{}{}}} else {{\n{}{}}}\n{}}}())",
                        branch_indent,
                        cond_code,
                        then_code,
                        branch_indent,
                        else_code,
                        branch_indent,
                        base_indent
                    )
                }
            }
            Expr::BlockExpr {
                statements,
                final_expr,
            } => {
                let old_out = std::mem::take(&mut self.output);
                let old_indent = self.indent_level;
                self.indent_level += 1;

                for s in statements {
                    self.visit_statement(s);
                }
                if let Some(f) = final_expr {
                    let fc = self.visit_expression(f);
                    self.emit(&format!("return {};", fc));
                }

                let body_code = std::mem::take(&mut self.output);
                self.indent_level = old_indent;
                self.output = old_out;

                let base_indent = "    ".repeat(self.indent_level);
                format!("([&]() {{\n{}{}}}())", body_code, base_indent)
            }
            Expr::QuestionMark(inner) => {
                let inner_code = self.visit_expression(inner);
                let base_indent = "    ".repeat(self.indent_level);
                format!(
                    "([&]() {{\n{}    try {{\n{}        return ({});\n{}    }} catch (const fast_std::Error& __e) {{\n{}        throw __e;\n{}    }}\n{}}}())",
                    base_indent, base_indent, inner_code, base_indent, base_indent, base_indent, base_indent
                )
            }
        }
    }

    pub(crate) fn extract_cpp_type_from_expr(&mut self, expr: &Expr) -> Option<String> {
        match expr {
            Expr::Identifier(name) => {
                if let Some(tracked_type) = self.vars.get(name) {
                    return Some(tracked_type.clone());
                }
                let bt = BaseType::from_str(name);
                if bt != BaseType::Unknown {
                    Some(type_to_cpp(&bt))
                } else {
                    Some(crate::backend::cpp::stmt::cpp_safe_name(name))
                }
            }
            Expr::IndexAccess { object, .. } => {
                if matches!(**object, Expr::This) {
                    Some("char".to_string())
                } else if let Some(parent_type) = self.extract_cpp_type_from_expr(object) {
                    if parent_type.starts_with("fast_std::array<") && parent_type.ends_with('>') {
                        Some(
                            parent_type["fast_std::array<".len()..parent_type.len() - 1]
                                .to_string(),
                        )
                    } else if parent_type.starts_with("array<") && parent_type.ends_with('>') {
                        Some(parent_type["array<".len()..parent_type.len() - 1].to_string())
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            Expr::Call { callee, .. } => {
                if let Expr::PropertyAccess { object, property } = &**callee {
                    if property == "chain"
                        || property == "slice"
                        || property == "sort"
                        || property == "reverse"
                    {
                        self.extract_cpp_type_from_expr(object)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            Expr::PropertyAccess { object, property } => {
                if matches!(**object, Expr::This) {
                    if let Some(cls) = &self.current_class_name {
                        if let Some(fields) = self.struct_field_types.get(cls) {
                            if let Some(ft) = fields.get(property) {
                                return Some(ft.clone());
                            }
                        }
                    }
                    if let Some(ft) = self.vars.get(&format!("this->{}", property)) {
                        return Some(ft.clone());
                    }
                    if let Some(ft) = self.vars.get(property) {
                        return Some(ft.clone());
                    }
                } else if let Some(parent_type) = self.extract_cpp_type_from_expr(object) {
                    let clean_target = parent_type
                        .trim_start_matches("fast_std::")
                        .trim_end_matches('*')
                        .split('<')
                        .next()
                        .unwrap_or(&parent_type);
                    if let Some(fields) = self.struct_field_types.get(clean_target) {
                        if let Some(ft) = fields.get(property) {
                            return Some(ft.clone());
                        }
                    }
                }
                None
            }
            Expr::LiteralString(_) => Some("array<char>".to_string()),
            Expr::ArrayLiteral(_) => Some("array".to_string()),
            Expr::LiteralChar(_) => Some("char".to_string()),
            Expr::LiteralBool(_) => Some("bool".to_string()),
            Expr::LiteralInt(_) => Some("int32_t".to_string()),
            Expr::LiteralFloat(_) => Some("double".to_string()),
            Expr::IfExpr { then_branch, else_branch, .. } => {
                self.extract_cpp_type_from_expr(then_branch)
                    .or_else(|| self.extract_cpp_type_from_expr(else_branch))
            }
            Expr::BlockExpr { final_expr, .. } => {
                final_expr.as_ref().and_then(|f| self.extract_cpp_type_from_expr(f))
            }
            Expr::QuestionMark(inner) => self.extract_cpp_type_from_expr(inner),
            _ => None,
        }
    }
}
