use crate::backend::cpp::generator::CodeGenerator;
use crate::backend::cpp::stmt::type_to_cpp;
use crate::frontend::parser::ast::*;

impl CodeGenerator {
    pub(crate) fn visit_expression(&mut self, expr: &Expr) -> String {
        match expr {
            Expr::LiteralInt(val) => val.to_string(),
            Expr::LiteralFloat(f) => format!("{:?}", f),
            Expr::LiteralString(s) => format!("\"{}\"", s),
            Expr::LiteralChar(c) => format!("'{}'", c),
            Expr::LiteralBool(val) => {
                if *val { "true".to_string() } else { "false".to_string() }
            }
            Expr::ArrayLiteral(elements) => {
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
                struct_code.push_str(
                    &format!(
                        "{}}}; return std::make_shared<__Anon>(); }}())",
                        "    ".repeat(self.indent_level)
                    )
                );
                struct_code
            }
            Expr::Identifier(name) => {
                if name == "null" || name == "void" {
                    "nullptr".to_string()
                } else if name == "None" {
                    "std::nullopt".to_string()
                } else if name == "__default__" {
                    "{}".to_string()
                } else {
                    name.clone()
                }
            }
            Expr::This => "this".to_string(),
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

                if self.custom_scope_types.contains(&callee_code) {
                    format!("{}({})()", callee_code, args_code.join(", "))
                } else {
                    format!("{}({})", callee_code, args_code.join(", "))
                }
            }
            Expr::Instantiate { target, args } => {
                let target_code = self.visit_expression(target);
                if args.len() == 1 && matches!(args[0], Expr::ObjectLiteral(_)) {
                    if let Expr::ObjectLiteral(ref stmts) = args[0] {
                        let mut struct_code = format!("([&]() {{\n    {} __obj;\n", target_code);
                        let mut temp_gen = CodeGenerator::new();
                        temp_gen.indent_level = self.indent_level + 1;
                        for s in stmts {
                            match s {
                                Stmt::Declaration(Decl::VarDecl { name, value, assign_op, .. }) => {
                                    let v = temp_gen.visit_expression(value);
                                    if v != "__default__" {
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
                } else if self.pointer_vars.contains(&obj_code) {
                    format!("{}->{}", obj_code, property)
                } else if property == "length" {
                    format!("((int32_t)fastlang_len({}))", obj_code)
                } else {
                    format!("{}.{}", obj_code, property)
                }
            }
            Expr::NamespaceAccess { namespace, property } => {
                let prop_code = self.visit_expression(property);
                format!("{}::{}", namespace, prop_code)
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
                let cpp_type = type_to_cpp(type_node);
                let target_code = self.visit_expression(target);
                if target_code == "__default__" || target_code == "{}" || target_code.is_empty() {
                    format!("new {}()", cpp_type)
                } else if target_code.starts_with('{') && target_code.ends_with('}') {
                    format!("new {}[]{}", cpp_type, target_code)
                } else {
                    format!("new {}({})", cpp_type, target_code)
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
