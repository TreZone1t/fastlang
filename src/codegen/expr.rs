use crate::parser::ast::*;
use crate::codegen::generator::CodeGenerator;

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
                format!("std::vector{{{}}}", elems_code.join(", "))
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
                } else {
                    format!("{}({})", callee_code, args_code.join(", "))
                }
            }
            Expr::Instantiate { op, target, args } => {
                let target_code = self.visit_expression(target);
                let mut args_code = Vec::new();
                for arg in args {
                    args_code.push(self.visit_expression(arg));
                }
                if op == "new" {
                    let mut tc = target_code.clone();
                    if tc == "Node" {
                        tc = "Node<T>".to_string();
                    }
                    format!("(void*)new {}({})", tc, args_code.join(", "))
                } else if op == "copy" {
                    format!(
                        "new std::decay_t<decltype(*{})>(*{})",
                        target_code, target_code
                    ) // basic copy using decltype
                } else {
                    target_code // modify or other
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
                } else if property == "size" {
                    format!("{}.size()", obj_code)
                } else {
                    format!("{}.{}", obj_code, property) // we default to . since primitive objects might not be pointers, though shared_ptr requires ->
                }
            }
            Expr::IndexAccess { object, index } => {
                let obj_code = self.visit_expression(object);
                let idx_code = self.visit_expression(index);
                format!("{}[{}]", obj_code, idx_code)
            }
            _ => "/* unimplemented expr */".to_string(),
        }
    }

}
