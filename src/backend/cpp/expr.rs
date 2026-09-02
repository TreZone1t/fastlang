use crate::backend::cpp::generator::CodeGenerator;
use crate::backend::cpp::stmt::type_to_cpp;
use crate::frontend::parser::ast::*;

impl CodeGenerator {
    pub(crate) fn visit_expression(&mut self, expr: &Expr) -> String {
        match expr {
            Expr::LiteralInt(val) => val.to_string(),
            Expr::LiteralFloat(f) => format!("{:?}", f),
            Expr::LiteralString(s) => {
                let mut escaped = String::new();
                for ch in s.chars() {
                    match ch {
                        '\n' => escaped.push_str("\\n"),
                        '\t' => escaped.push_str("\\t"),
                        '\r' => escaped.push_str("\\r"),
                        '\\' => escaped.push_str("\\\\"),
                        '"' => escaped.push_str("\\\""),
                        _ => escaped.push(ch),
                    }
                }
                format!("\"{}\"", escaped)
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
            Expr::LiteralBool(val) => {
                if *val { "true".to_string() } else { "false".to_string() }
            }
            Expr::LiteralVoid => "fastlang_unit_t{}".to_string(),
            Expr::LiteralUndefined => "fastlang_undefined_t{}".to_string(),
            Expr::ArrayLiteral(elements) => {
                let has_spread = elements.iter().any(|e| matches!(e, Expr::Spread(_)));
                if has_spread {
                    let first = elements.iter().find(|e| !matches!(e, Expr::Spread(_))).or_else(|| elements.first());
                    let mut stmts = Vec::new();
                    let first_type_decl = if let Some(f) = first {
                        match f {
                            Expr::Spread(inner) => {
                                let c = self.visit_expression(inner);
                                format!("using __elem_t = std::decay_t<decltype(*std::begin({}))>;", c)
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
                    stmts.push("return __acc.to_slice();".to_string());
                    format!("([&]() {{\n    {}\n}}())", stmts.join("\n    "))
                } else {
                    let mut elems_code = Vec::new();
                    for el in elements {
                        elems_code.push(self.visit_expression(el));
                    }
                    format!("{{{}}}", elems_code.join(", "))
                }
            }
            Expr::Spread(inner) => {
                self.visit_expression(inner)
            }
            Expr::ObjectLiteral(stmts) => {
                let mut struct_code = "([]() { struct __Anon {\n".to_string();
                let mut temp_gen = CodeGenerator::new();
                temp_gen.indent_level = self.indent_level + 1;
                for s in stmts {
                    temp_gen.visit_statement(s);
                }
                struct_code.push_str(&temp_gen.output);
                struct_code.push_str(
                    &format!(
                        "{}}}; return std::make_shared<__Anon>(); }}())",
                        "    ".repeat(self.indent_level)
                    )
                );
                struct_code
            }
            Expr::Default(type_arg) => {
                if let Some(t) = type_arg {
                    format!("{}()", type_to_cpp(t))
                } else {
                    "{}".to_string()
                }
            }
            Expr::Identifier(name) => {
                match name.as_str() {
                    "bool" => "1".to_string(),
                    "int8" => "2".to_string(),
                    "int16" => "3".to_string(),
                    "int32" | "int" => "4".to_string(),
                    "int64" => "5".to_string(),
                    "uint8" => "7".to_string(),
                    "uint16" => "8".to_string(),
                    "uint32" | "uint" => "9".to_string(),
                    "uint64" => "10".to_string(),
                    "float32" => "12".to_string(),
                    "float64" | "float" => "13".to_string(),
                    "char" => "15".to_string(),
                    "byte" => "16".to_string(),
                    "usize" => "17".to_string(),
                    "isize" => "18".to_string(),
                    "string" => "19".to_string(),
                    "type" => "20".to_string(),
                    _ => name.clone(),
                }
            }
            Expr::This => {
                if self.in_primitive_impl || self.in_machine {
                    "__this".to_string()
                } else {
                    "this".to_string()
                }
            }
            Expr::Super => "super".to_string(), // will be handled in PropertyAccess
            Expr::Global => "::".to_string(),
            Expr::BinaryOp { left, operator, right } => {
                let l = self.visit_expression(left);
                let r = self.visit_expression(right);
                if operator == "->" {
                    format!("([&]() {{ fastlang_arrow({}, {}); return {}; }}())", l, r, l)
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
                let indices_code: Vec<String> = indices
                    .iter()
                    .map(|i| self.visit_expression(i))
                    .collect();
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
                if operator == "modify" {
                    format!("fastlang_modify(&{})", op_code)
                } else if operator == "copy" {
                    format!("fastlang_copy(&{})", op_code)
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

            Expr::Call { callee, args } => {
                if let Expr::PropertyAccess { object, property } = &**callee {
                    if let Some(target) = self.primitive_impl_methods.get(property).cloned() {
                        let obj_code = self.visit_expression(object);
                        let mut args_code = Vec::new();
                        for arg in args {
                            args_code.push(self.visit_expression(arg));
                        }
                        let mut all_args = vec![obj_code];
                        all_args.extend(args_code);
                        return format!("fastlang_{}_{}({})", target, property, all_args.join(", "));
                    }
                }

                let callee_code = match &**callee {
                    Expr::PropertyAccess { object, property } => {
                        let obj_code = self.visit_expression(object);
                        if obj_code == "::" {
                            format!("::{}", property)
                        } else if self.enum_types.contains(&obj_code) {
                            format!("{}::{}", obj_code, property)
                        } else if obj_code == "super" || obj_code == "this" {
                            format!("this->{}", property)
                        } else if self.pointer_vars.contains(&obj_code) || obj_code.ends_with("current") || obj_code.ends_with("head") || obj_code.ends_with("next") || obj_code.ends_with("temp") {
                            format!("{}->{}", obj_code, property)
                        } else {
                            format!("{}.{}", obj_code, property)
                        }
                    }
                    Expr::NamespaceAccess { namespace, property } => {
                        let prop_code = self.visit_expression(property);
                        let clean_prop = prop_code.trim_end_matches("()");
                        format!("{}::{}", namespace, clean_prop)
                    }
                    _ => self.visit_expression(callee),
                };

                let mut args_code = Vec::new();
                for arg in args {
                    args_code.push(self.visit_expression(arg));
                }

                if self.custom_scope_types.contains(&callee_code) {
                    format!("{}()({})", callee_code, args_code.join(", "))
                } else {
                    format!("{}({})", callee_code, args_code.join(", "))
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
                                        Stmt::Declaration(Decl::VarDecl { name, value, assign_op, .. }) => {
                                            let v = temp_gen.visit_expression(value);
                                            if !matches!(value, Expr::Default(None)) && v != "{}" {
                                                let op = if assign_op.is_empty() { "=" } else { assign_op.as_str() };
                                                temp_gen.emit(&format!("__payload.{} {} {};", name, op, v));
                                            }
                                        }
                                        Stmt::ReassignStmt { target, value, op } => {
                                            let t = temp_gen.visit_expression(target);
                                            let v = temp_gen.visit_expression(value);
                                            temp_gen.emit(&format!("__payload.{} {} {};", t, op, v));
                                        }
                                        _ => {
                                            temp_gen.visit_statement(s);
                                        }
                                    }
                                }
                                struct_code.push_str(&temp_gen.output);
                                struct_code.push_str(&format!("{}__obj.data = __payload;\n{}return __obj;\n{}}}())", "    ".repeat(self.indent_level + 1), "    ".repeat(self.indent_level + 1), "    ".repeat(self.indent_level)));
                                return struct_code;
                            }
                        }
                        let mut struct_code = format!("([&]() {{\n    {} __obj;\n", target_code);
                        let mut temp_gen = CodeGenerator::new();
                        temp_gen.indent_level = self.indent_level + 1;
                        for s in stmts {
                            match s {
                                Stmt::Declaration(Decl::VarDecl { name, value, assign_op, .. }) => {
                                    let v = temp_gen.visit_expression(value);
                                    if !matches!(value, Expr::Default(None)) && v != "{}" {
                                        let op = if assign_op.is_empty() { "=" } else { assign_op.as_str() };
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
                        struct_code.push_str(&format!("{}return __obj;\n{}}}())", "    ".repeat(self.indent_level + 1), "    ".repeat(self.indent_level)));
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
                let obj_code = self.visit_expression(object);
                if obj_code == "::" {
                    format!("::{}", property)
                } else if self.enum_types.contains(&obj_code) {
                    format!("{}::{}", obj_code, property)
                } else if obj_code == "super" || obj_code == "this" {
                    format!("this->{}", property)
                } else if self.pointer_vars.contains(&obj_code) || obj_code.ends_with("current") || obj_code.ends_with("head") || obj_code.ends_with("next") || obj_code.ends_with("temp") {
                    format!("{}->{}", obj_code, property)
                } else if property == "length" || property == "len" {
                    format!("((int32_t)fastlang_len({}))", obj_code)
                } else if property == "printable" || property == "is_printable" {
                    format!("(has_display<std::decay_t<decltype({})>>::value || std::is_arithmetic_v<std::decay_t<decltype({})>> || std::is_same_v<std::decay_t<decltype({})>, std::string>)", obj_code, obj_code, obj_code)
                } else if property == "throwable" || property == "is_throwable" {
                    format!("(is_throwable<std::decay_t<decltype({})>>::value || std::is_base_of_v<std::exception, std::decay_t<decltype({})>>)", obj_code, obj_code)
                } else if property == "compilable" || property == "is_compilable" {
                    "false".to_string()
                } else {
                    format!("{}.{}", obj_code, property)
                }
            }
            Expr::NamespaceAccess { namespace, property } => {
                let prop_code = self.visit_expression(property);
                if self.payload_enum_types.contains(namespace) {
                    format!("{}::{}()", namespace, prop_code)
                } else {
                    format!("{}::{}", namespace, prop_code)
                }
            }
            Expr::ArrayAllocate { type_node, size, length } => {
                let cpp_type: String = type_to_cpp(type_node);
                if let Some(init) = length {
                    let init_code = self.visit_expression(init);
                    format!("new {}[]{}", cpp_type, init_code)
                } else {
                    let size_code = self.visit_expression(size);
                    format!("new {}[{}]", cpp_type, size_code)
                }
            }
            Expr::New { type_node, target } => {
                if let BaseType::Array { base_type, size } = type_node {
                    let cpp_elem = type_to_cpp(base_type);
                    if let Some(s) = size.as_ref() {
                        let size_code = self.visit_expression(s);
                        return format!("fastlang_slice<{}>(new {}[{}], (size_t)({}))", cpp_elem, cpp_elem, size_code, size_code);
                    } else {
                        return format!("fastlang_slice<{}>()", cpp_elem);
                    }
                }
                let cpp_type = type_to_cpp(type_node);
                match &**target {
                    Expr::Instantiate { args, .. } => {
                        let arg_strs: Vec<String> = args.iter().map(|a| self.visit_expression(a)).collect();
                        format!("new {}({})", cpp_type, arg_strs.join(", "))
                    }
                    Expr::ArrayLiteral(elems) => {
                        let elem_strs: Vec<String> = elems.iter().map(|e| self.visit_expression(e)).collect();
                        format!("new {}[]{{{}}}", cpp_type, elem_strs.join(", "))
                    }
                    _ => {
                        let target_code = self.visit_expression(target);
                        if target_code == "__default__" || target_code == "{}" || target_code.is_empty() {
                            format!("new {}()", cpp_type)
                        } else {
                            format!("new {}({})", cpp_type, target_code)
                        }
                    }
                }
            }
            Expr::Lambda { params, return_type, body } => {
                let p_list: Vec<String> = params
                    .iter()
                    .map(|p| {
                        if p.type_node == BaseType::Unknown {
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
            _ => "/* unimplemented expr */".to_string(),
        }
    }
}
