use crate::backend::codegen::generator::CodeGenerator;
use crate::frontend::parser::ast::*;

impl CodeGenerator {
    pub(crate) fn visit_expression(&mut self, expr: &Expr) -> String {
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
            Expr::PrefixUpdate { right, operator } => {
                let r = self.visit_expression(right);
                format!("{}{}", operator, r)
            }
            Expr::UnaryOp { operator, operand } => {
                let op_code = self.visit_expression(operand);
                if operator == "&" {
                    // In FastLang, '&' means dereference! (pull data)
                    format!("(*{})", op_code)
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

                if callee_code == "Some" {
                    format!("std::optional{{{}}}", args_code.join(", "))
                } else if self.custom_scopes.contains(&callee_code) {
                    format!("{}({}).call()", callee_code, args_code.join(", "))
                } else {
                    format!("{}({})", callee_code, args_code.join(", "))
                }
            }
            Expr::Instantiate { target, args } => {
                let target_code = self.visit_expression(target);
                let mut args_code = Vec::new();
                for arg in args {
                    args_code.push(self.visit_expression(arg));
                }
                // Value instantiation in C++ (stack allocation)
                format!("{}({})", target_code, args_code.join(", "))
            }
            Expr::Modify { target } => {
                // In FastLang, 'modify' means address-of (take the name)
                let t_code = self.visit_expression(target);
                format!("(&{})", t_code)
            }
            Expr::Copy { target } => self.visit_expression(target),
            Expr::MagicReference {
                target,
                kind,
                access_mode,
            } => {
                let target_code = self.visit_expression(target);
                let const_prefix = match access_mode {
                    AccessMode::ReadOnly => "const ",
                    AccessMode::ReadWrite => "",
                };

                match kind {
                    ReferenceKind::Name => {
                        format!("({}void*)&({})", const_prefix, target_code)
                    }
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
                } else {
                    format!("{}.{}", obj_code, property) // we default to . since primitive objects might not be pointers, though shared_ptr requires ->
                }
            }
            Expr::NamespaceAccess {
                namespace,
                property,
            } => {
                let prop_code = self.visit_expression(property);
                format!("{}::{}", namespace, prop_code)
            }
            Expr::ArrayAllocate {
                type_node,
                size,
                length,
            } => {
                let cpp_type = match type_node {
                    BaseType::Int8 => "int8_t",
                    BaseType::Int16 => "int16_t",
                    BaseType::Int32 => "int32_t",
                    BaseType::Int64 => "int64_t",
                    BaseType::Int128 => "__int128",
                    BaseType::Float32 => "float",
                    BaseType::Float64 => "double",
                    BaseType::Char => "char",
                    BaseType::Bool => "bool",
                    _ => "auto",
                };
                if let Some(init) = length {
                    let init_code = self.visit_expression(init);
                    format!("new {}[]{}", cpp_type, init_code)
                } else {
                    let size_code = self.visit_expression(size);
                    format!("new {}[{}]", cpp_type, size_code)
                }
            }
            _ => "/* unimplemented expr */".to_string(),
        }
    }
}
