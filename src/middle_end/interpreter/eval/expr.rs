use super::*;
use crate::frontend::parser::ast::*;
use crate::middle_end::semantic::environment::SymbolKind;
use colored::Colorize;

impl Interpreter {
    pub fn eval_expr(&mut self, expr: &Expr) -> Result<Expr, String> {
        match expr {
            Expr::LiteralInt(_)
            | Expr::LiteralUInt(_)
            | Expr::LiteralFloat(_)
            | Expr::LiteralString(_)
            | Expr::LiteralBool(_)
            | Expr::LiteralChar(_)
            | Expr::LiteralVoid
            | Expr::LiteralUndefined => Ok(expr.clone()),
            Expr::Identifier(name) => self.env.get(name),
            Expr::Cast { expr, target_type } => {
                let val = self.eval_expr(expr)?;
                let target_name = match target_type {
                    BaseType::GenericParam(g_name) => g_name.clone(),
                    BaseType::Blueprint { name, .. } => name.clone(),
                    BaseType::New(n) => n.clone(),
                    _ => target_type.as_str(),
                };
                let resolved_target = if let Ok(Expr::LiteralString(s)) = self.env.get(&target_name)
                {
                    let clean = if s.starts_with("type<") && s.ends_with('>') {
                        &s[5..s.len() - 1]
                    } else {
                        &s
                    };
                    BaseType::from_str(clean)
                } else {
                    target_type.clone()
                };
                self.fastlang_cast(&val, &resolved_target)
            }
            Expr::UnaryOp { operator, operand } => {
                let val = self.eval_expr(operand)?;
                match (operator.as_str(), val) {
                    ("!", Expr::LiteralBool(b)) => Ok(Expr::LiteralBool(!b)),
                    ("-", Expr::LiteralInt(i)) => Ok(Expr::LiteralInt(-i)),
                    ("-", Expr::LiteralFloat(f)) => Ok(Expr::LiteralFloat(-f)),
                    _ => Ok(Expr::LiteralUndefined),
                }
            }
            Expr::IfExpr {
                condition,
                then_branch,
                else_branch,
            } => {
                let cond_val = self.eval_expr(condition)?;
                let is_true = match cond_val {
                    Expr::LiteralBool(b) => b,
                    Expr::LiteralInt(i) => i != 0,
                    _ => false,
                };
                if is_true {
                    self.eval_expr(then_branch)
                } else {
                    self.eval_expr(else_branch)
                }
            }
            Expr::BlockExpr {
                statements,
                final_expr,
            } => {
                self.env.push_scope();
                for stmt in statements {
                    let _ = self.eval_stmt(stmt)?;
                }
                let res = if let Some(f) = final_expr {
                    self.eval_expr(f)
                } else {
                    Ok(Expr::LiteralVoid)
                };
                self.env.pop_scope();
                res
            }
            Expr::PropertyAccess { object, property } => {
                let obj_val = self.eval_expr(object).unwrap_or_else(|_| *object.clone());
                let type_name = match &obj_val {
                    Expr::LiteralString(s) => strip_type_wrapper(s).to_string(),
                    Expr::Identifier(id) => {
                        if let Ok(Expr::LiteralString(s)) = self.env.get(id) {
                            strip_type_wrapper(&s).to_string()
                        } else {
                            id.clone()
                        }
                    }
                    _ => "".to_string(),
                };
                let is_type_reflection = matches!(obj_val, Expr::LiteralString(ref s) if s.starts_with("type<"))
                    || (!type_name.is_empty() && is_known_type_name(&type_name));
                if !is_type_reflection {
                    return Err(
                        "PropertyAccess on non-type cannot be evaluated at compile time"
                            .to_string(),
                    );
                }
                let ft = self.get_fast_type(&type_name);
                match property.as_str() {
                    "is_primitive" => Ok(Expr::LiteralBool(ft.is_primitive())),
                    "is_pointer" => Ok(Expr::LiteralBool(ft.is_pointer())),
                    "is_array" => Ok(Expr::LiteralBool(ft.is_array())),
                    "printable" | "is_printable" => Ok(Expr::LiteralBool(ft.printable())),
                    "throwable" | "is_throwable" => Ok(Expr::LiteralBool(ft.throwable())),
                    "shareable" | "is_shareable" => Ok(Expr::LiteralBool(ft.shareable())),
                    "copyable" | "is_copyable" => Ok(Expr::LiteralBool(ft.copyable())),
                    "castable" | "is_castable" => Ok(Expr::LiteralBool(ft.castable())),
                    "is_constant" | "is_const" => Ok(Expr::LiteralBool(ft.is_constant())),
                    "is_runtime" => Ok(Expr::LiteralBool(ft.is_runtime())),
                    "is_comptime" => Ok(Expr::LiteralBool(ft.is_comptime())),
                    "is_undefined" => Ok(Expr::LiteralBool(ft.is_undefined())),
                    "on_heap" => Ok(Expr::LiteralBool(ft.on_heap())),
                    "on_stack" => Ok(Expr::LiteralBool(ft.on_stack())),
                    "as_str" => {
                        if matches!(obj_val, Expr::LiteralString(ref s) if s.starts_with("type<")) {
                            Ok(Expr::LiteralString(ft.as_str()))
                        } else {
                            Err("as_str only supported on type reflection in comptime eval"
                                .to_string())
                        }
                    }
                    "size" => Ok(Expr::LiteralInt(ft.size() as i128)),
                    "default" => {
                        if let Some(def) = self.default_for_type(&type_name) {
                            Ok(def)
                        } else {
                            Ok(Expr::Default(Some(BaseType::from_str(&type_name))))
                        }
                    }
                    _ => {
                        if let Some(method_name) = property.strip_prefix("has_") {
                            Ok(Expr::LiteralBool(
                                ft.has_method(method_name)
                                    || ft.has_handle(method_name)
                                    || (method_name == "as_str"
                                        && (ft.printable()
                                            || ft.has_method("as_str")
                                            || ft.has_handle("display"))),
                            ))
                        } else {
                            Ok(Expr::LiteralUndefined)
                        }
                    }
                }
            }
            Expr::BinaryOp {
                left,
                operator,
                right,
            } => {
                if operator == "==" || operator == "!=" {
                    let left_ty = self.extract_type_string(left);
                    let right_ty = self.extract_type_string(right);
                    if let (Some(lt), Some(rt)) = (left_ty, right_ty) {
                        let clean_lt = strip_type_wrapper(&lt).to_string();
                        let clean_rt = strip_type_wrapper(&rt).to_string();
                        let eq = clean_lt == clean_rt;
                        let res = if operator == "==" { eq } else { !eq };
                        return Ok(Expr::LiteralBool(res));
                    }
                }

                let l = self.eval_expr(left)?;
                let r = self.eval_expr(right)?;
                match (l, operator.as_str(), r) {
                    (Expr::LiteralInt(a), "+", Expr::LiteralInt(b)) => Ok(Expr::LiteralInt(a + b)),
                    (Expr::LiteralInt(a), "-", Expr::LiteralInt(b)) => Ok(Expr::LiteralInt(a - b)),
                    (Expr::LiteralInt(a), "*", Expr::LiteralInt(b)) => Ok(Expr::LiteralInt(a * b)),
                    (Expr::LiteralInt(a), "/", Expr::LiteralInt(b)) if b != 0 => {
                        Ok(Expr::LiteralInt(a / b))
                    }
                    (Expr::LiteralFloat(a), "+", Expr::LiteralFloat(b)) => {
                        Ok(Expr::LiteralFloat(a + b))
                    }
                    (Expr::LiteralFloat(a), "-", Expr::LiteralFloat(b)) => {
                        Ok(Expr::LiteralFloat(a - b))
                    }
                    (Expr::LiteralFloat(a), "*", Expr::LiteralFloat(b)) => {
                        Ok(Expr::LiteralFloat(a * b))
                    }
                    (Expr::LiteralFloat(a), "/", Expr::LiteralFloat(b)) if b != 0.0 => {
                        Ok(Expr::LiteralFloat(a / b))
                    }
                    (Expr::LiteralInt(a), "+", Expr::LiteralFloat(b)) => {
                        Ok(Expr::LiteralFloat(a as f64 + b))
                    }
                    (Expr::LiteralFloat(a), "+", Expr::LiteralInt(b)) => {
                        Ok(Expr::LiteralFloat(a + b as f64))
                    }
                    (Expr::LiteralString(mut a), "+", Expr::LiteralString(b)) => {
                        a.push_str(&b);
                        Ok(Expr::LiteralString(a))
                    }
                    (Expr::LiteralString(mut a), "+", Expr::LiteralChar(c)) => {
                        a.push(c);
                        Ok(Expr::LiteralString(a))
                    }
                    (Expr::LiteralString(mut a), "+", Expr::LiteralUChar(c)) => {
                        if let Some(ch) = std::char::from_u32(c) {
                            a.push(ch);
                        }
                        Ok(Expr::LiteralString(a))
                    }
                    (Expr::LiteralChar(c), "+", Expr::LiteralString(b)) => {
                        let mut s = c.to_string();
                        s.push_str(&b);
                        Ok(Expr::LiteralString(s))
                    }
                    (Expr::LiteralUChar(c), "+", Expr::LiteralString(b)) => {
                        let mut s = String::new();
                        if let Some(ch) = std::char::from_u32(c) {
                            s.push(ch);
                        }
                        s.push_str(&b);
                        Ok(Expr::LiteralString(s))
                    }
                    (Expr::LiteralBool(a), "&&", Expr::LiteralBool(b)) => {
                        Ok(Expr::LiteralBool(a && b))
                    }
                    (Expr::LiteralBool(a), "||", Expr::LiteralBool(b)) => {
                        Ok(Expr::LiteralBool(a || b))
                    }
                    (Expr::LiteralInt(a), "==", Expr::LiteralInt(b)) => {
                        Ok(Expr::LiteralBool(a == b))
                    }
                    (Expr::LiteralInt(a), "!=", Expr::LiteralInt(b)) => {
                        Ok(Expr::LiteralBool(a != b))
                    }
                    (Expr::LiteralInt(a), "<", Expr::LiteralInt(b)) => Ok(Expr::LiteralBool(a < b)),
                    (Expr::LiteralInt(a), "<=", Expr::LiteralInt(b)) => {
                        Ok(Expr::LiteralBool(a <= b))
                    }
                    (Expr::LiteralInt(a), ">", Expr::LiteralInt(b)) => Ok(Expr::LiteralBool(a > b)),
                    (Expr::LiteralInt(a), ">=", Expr::LiteralInt(b)) => {
                        Ok(Expr::LiteralBool(a >= b))
                    }
                    (Expr::LiteralFloat(a), "==", Expr::LiteralFloat(b)) => {
                        Ok(Expr::LiteralBool(a == b))
                    }
                    (Expr::LiteralFloat(a), "!=", Expr::LiteralFloat(b)) => {
                        Ok(Expr::LiteralBool(a != b))
                    }
                    (Expr::LiteralFloat(a), "<", Expr::LiteralFloat(b)) => {
                        Ok(Expr::LiteralBool(a < b))
                    }
                    (Expr::LiteralFloat(a), "<=", Expr::LiteralFloat(b)) => {
                        Ok(Expr::LiteralBool(a <= b))
                    }
                    (Expr::LiteralFloat(a), ">", Expr::LiteralFloat(b)) => {
                        Ok(Expr::LiteralBool(a > b))
                    }
                    (Expr::LiteralFloat(a), ">=", Expr::LiteralFloat(b)) => {
                        Ok(Expr::LiteralBool(a >= b))
                    }
                    (Expr::LiteralInt(a), "<", Expr::LiteralFloat(b)) => {
                        Ok(Expr::LiteralBool((a as f64) < b))
                    }
                    (Expr::LiteralInt(a), "<=", Expr::LiteralFloat(b)) => {
                        Ok(Expr::LiteralBool((a as f64) <= b))
                    }
                    (Expr::LiteralInt(a), ">", Expr::LiteralFloat(b)) => {
                        Ok(Expr::LiteralBool((a as f64) > b))
                    }
                    (Expr::LiteralInt(a), ">=", Expr::LiteralFloat(b)) => {
                        Ok(Expr::LiteralBool((a as f64) >= b))
                    }
                    (Expr::LiteralFloat(a), "<", Expr::LiteralInt(b)) => {
                        Ok(Expr::LiteralBool(a < (b as f64)))
                    }
                    (Expr::LiteralFloat(a), "<=", Expr::LiteralInt(b)) => {
                        Ok(Expr::LiteralBool(a <= (b as f64)))
                    }
                    (Expr::LiteralFloat(a), ">", Expr::LiteralInt(b)) => {
                        Ok(Expr::LiteralBool(a > (b as f64)))
                    }
                    (Expr::LiteralFloat(a), ">=", Expr::LiteralInt(b)) => {
                        Ok(Expr::LiteralBool(a >= (b as f64)))
                    }
                    (Expr::LiteralChar(a), "==", Expr::LiteralChar(b)) => {
                        Ok(Expr::LiteralBool(a == b))
                    }
                    (Expr::LiteralChar(a), "!=", Expr::LiteralChar(b)) => {
                        Ok(Expr::LiteralBool(a != b))
                    }
                    (Expr::LiteralChar(a), "<", Expr::LiteralChar(b)) => {
                        Ok(Expr::LiteralBool(a < b))
                    }
                    (Expr::LiteralChar(a), "<=", Expr::LiteralChar(b)) => {
                        Ok(Expr::LiteralBool(a <= b))
                    }
                    (Expr::LiteralChar(a), ">", Expr::LiteralChar(b)) => {
                        Ok(Expr::LiteralBool(a > b))
                    }
                    (Expr::LiteralChar(a), ">=", Expr::LiteralChar(b)) => {
                        Ok(Expr::LiteralBool(a >= b))
                    }
                    (Expr::LiteralUChar(a), "==", Expr::LiteralUChar(b)) => {
                        Ok(Expr::LiteralBool(a == b))
                    }
                    (Expr::LiteralUChar(a), "!=", Expr::LiteralUChar(b)) => {
                        Ok(Expr::LiteralBool(a != b))
                    }
                    (Expr::LiteralUChar(a), "<", Expr::LiteralUChar(b)) => {
                        Ok(Expr::LiteralBool(a < b))
                    }
                    (Expr::LiteralUChar(a), "<=", Expr::LiteralUChar(b)) => {
                        Ok(Expr::LiteralBool(a <= b))
                    }
                    (Expr::LiteralUChar(a), ">", Expr::LiteralUChar(b)) => {
                        Ok(Expr::LiteralBool(a > b))
                    }
                    (Expr::LiteralUChar(a), ">=", Expr::LiteralUChar(b)) => {
                        Ok(Expr::LiteralBool(a >= b))
                    }
                    (l_val, op, r_val) => Ok(Expr::BinaryOp {
                        left: Box::new(l_val),
                        operator: op.to_string(),
                        right: Box::new(r_val),
                    }),
                }
            }
            Expr::New { target, .. } => {
                // For 'new Error("msg")', the target is often Instantiate
                self.eval_expr(target)
            }
            Expr::Instantiate { args, .. } => {
                // Just bubble up the first argument if it's a string, or undefined
                if let Some(first) = args.first() {
                    self.eval_expr(first)
                } else {
                    Ok(Expr::LiteralUndefined)
                }
            }
            Expr::Call {
                callee,
                generics,
                args,
            } => {
                if let Expr::PropertyAccess { object, property } = &**callee {
                    let obj_val = self.eval_expr(object).unwrap_or_else(|_| *object.clone());
                    let type_name = match &obj_val {
                        Expr::LiteralString(s) => strip_type_wrapper(s).to_string(),
                        Expr::Identifier(id) => {
                            if let Ok(Expr::LiteralString(s)) = self.env.get(id) {
                                strip_type_wrapper(&s).to_string()
                            } else {
                                id.clone()
                            }
                        }
                        _ => "".to_string(),
                    };
                    let is_type_reflection = matches!(obj_val, Expr::LiteralString(ref s) if s.starts_with("type<"))
                        || (!type_name.is_empty() && is_known_type_name(&type_name));
                    if is_type_reflection {
                        let ft = self.get_fast_type(&type_name);
                        match property.as_str() {
                            "is_primitive" => return Ok(Expr::LiteralBool(ft.is_primitive())),
                            "is_pointer" => return Ok(Expr::LiteralBool(ft.is_pointer())),
                            "is_array" => return Ok(Expr::LiteralBool(ft.is_array())),
                            "printable" | "is_printable" => {
                                return Ok(Expr::LiteralBool(ft.printable()))
                            }
                            "throwable" | "is_throwable" => {
                                return Ok(Expr::LiteralBool(ft.throwable()))
                            }
                            "shareable" | "is_shareable" => {
                                return Ok(Expr::LiteralBool(ft.shareable()))
                            }
                            "copyable" | "is_copyable" => {
                                return Ok(Expr::LiteralBool(ft.copyable()))
                            }
                            "castable" | "is_castable" => {
                                return Ok(Expr::LiteralBool(ft.castable()))
                            }
                            "is_constant" | "is_const" => {
                                return Ok(Expr::LiteralBool(ft.is_constant()))
                            }
                            "is_runtime" => return Ok(Expr::LiteralBool(ft.is_runtime())),
                            "is_comptime" => return Ok(Expr::LiteralBool(ft.is_comptime())),
                            "is_undefined" => return Ok(Expr::LiteralBool(ft.is_undefined())),
                            "on_heap" => return Ok(Expr::LiteralBool(ft.on_heap())),
                            "on_stack" => return Ok(Expr::LiteralBool(ft.on_stack())),
                            "has_handle" => {
                                if let Some(arg) = args.first() {
                                    let evaled =
                                        self.eval_expr(arg).unwrap_or_else(|_| arg.clone());
                                    let s1 = self.extract_type_or_name(&evaled);
                                    let s2 = args.get(1).map(|a| {
                                        let v = self.eval_expr(a).unwrap_or_else(|_| a.clone());
                                        self.extract_type_or_name(&v)
                                    });
                                    return Ok(Expr::LiteralBool(
                                        ft.has_handle_query(&s1, s2.as_deref()),
                                    ));
                                }
                                return Ok(Expr::LiteralBool(false));
                            }
                            "has_method" => {
                                if let Some(arg) = args.first() {
                                    let evaled =
                                        self.eval_expr(arg).unwrap_or_else(|_| arg.clone());
                                    let s1 = self.extract_type_or_name(&evaled);
                                    let s2 = args.get(1).map(|a| {
                                        let v = self.eval_expr(a).unwrap_or_else(|_| a.clone());
                                        self.extract_type_or_name(&v)
                                    });
                                    return Ok(Expr::LiteralBool(
                                        ft.has_method_query(&s1, s2.as_deref()),
                                    ));
                                }
                                return Ok(Expr::LiteralBool(false));
                            }
                            "has_field" => {
                                if let Some(arg) = args.first() {
                                    let evaled =
                                        self.eval_expr(arg).unwrap_or_else(|_| arg.clone());
                                    let s1 = self.extract_type_or_name(&evaled);
                                    let s2 = args.get(1).map(|a| {
                                        let v = self.eval_expr(a).unwrap_or_else(|_| a.clone());
                                        self.extract_type_or_name(&v)
                                    });
                                    return Ok(Expr::LiteralBool(
                                        ft.has_field_query(&s1, s2.as_deref()),
                                    ));
                                }
                                return Ok(Expr::LiteralBool(false));
                            }
                            "as_str" => {
                                if matches!(obj_val, Expr::LiteralString(ref s) if s.starts_with("type<"))
                                {
                                    return Ok(Expr::LiteralString(ft.as_str()));
                                } else if is_literal(&obj_val) {
                                    return Ok(Expr::LiteralString(
                                        self.format_expr_for_print(&obj_val),
                                    ));
                                } else {
                                    return Err(
                                        "as_str only supported on literal or type in comptime eval"
                                            .to_string(),
                                    );
                                }
                            }
                            "size" => {
                                if matches!(obj_val, Expr::LiteralString(ref s) if s.starts_with("type<"))
                                {
                                    return Ok(Expr::LiteralInt(ft.size() as i128));
                                } else {
                                    return Err("size method only supported on type reflection at compile time".to_string());
                                }
                            }
                            "default" => {
                                if let Some(def) = self.default_for_type(&type_name) {
                                    return Ok(def);
                                } else {
                                    return Ok(Expr::Default(Some(BaseType::from_str(&type_name))));
                                }
                            }
                            "castable_to" => {
                                if let Some(target_expr) = args.first() {
                                    let evaled = self
                                        .eval_expr(target_expr)
                                        .unwrap_or_else(|_| target_expr.clone());
                                    let target_name = match evaled {
                                        Expr::LiteralString(s) => {
                                            strip_type_wrapper(&s).to_string()
                                        }
                                        Expr::Identifier(id) => {
                                            if let Ok(Expr::LiteralString(s)) = self.env.get(&id) {
                                                strip_type_wrapper(&s).to_string()
                                            } else {
                                                id
                                            }
                                        }
                                        _ => "".to_string(),
                                    };
                                    let target_ft = self.get_fast_type(&target_name);
                                    return Ok(Expr::LiteralBool(ft.castable_to(&target_ft)));
                                }
                            }
                            _ => {
                                if let Some(method_name) = property.strip_prefix("has_") {
                                    return Ok(Expr::LiteralBool(
                                        ft.has_method(method_name)
                                            || ft.has_handle(method_name)
                                            || (method_name == "as_str"
                                                && (ft.printable()
                                                    || ft.has_method("as_str")
                                                    || ft.has_handle("display"))),
                                    ));
                                }
                            }
                        }
                    }
                }

                let target_fn_name = match &**callee {
                    Expr::NamespaceAccess {
                        namespace,
                        property,
                    } if namespace == "@compile" => {
                        if let Expr::Identifier(p) = &**property {
                            format!("@compile::{}", p)
                        } else {
                            "".to_string()
                        }
                    }
                    Expr::Identifier(id) => id.clone(),
                    _ => "".to_string(),
                };

                if target_fn_name == "@compile::typeof" || target_fn_name == "typeof" {
                    if let Some(arg) = args.first() {
                        let t = self.fastlang_typeof(arg);
                        if t != "unknown" {
                            return Ok(Expr::LiteralString(format!("type<{}>", t)));
                        } else {
                            return Ok(Expr::LiteralString("type<unknown>".to_string()));
                        }
                    }
                    return Ok(expr.clone());
                }

                if target_fn_name == "@compile::sizeof" || target_fn_name == "sizeof" {
                    if let Some(arg) = args.first() {
                        let sz = self.fastlang_sizeof(arg);
                        if sz > 0 {
                            return Ok(Expr::LiteralInt(sz as i128));
                        }
                    }
                    return Ok(expr.clone());
                }
                if target_fn_name == "@compile::print" {
                    let mut parts = Vec::new();
                    for arg in args {
                        let val = self.eval_expr(arg).unwrap_or_else(|_| arg.clone());
                        if let Expr::ArrayLiteral(elems) = val {
                            for elem in elems {
                                parts.push(self.format_expr_for_print(&elem));
                            }
                        } else {
                            parts.push(self.format_expr_for_print(&val));
                        }
                    }
                    println!("{}", parts.join(" "));
                    return Ok(Expr::LiteralVoid);
                }

                if target_fn_name == "@compile::rand" || target_fn_name == "rand" {
                    let target_type = generics
                        .first()
                        .cloned()
                        .unwrap_or(BaseType::Int(crate::frontend::parser::ast::Size::S32));
                    if !CompilerIntrinsics::is_numeric_type(&target_type) {
                        return self.fastlang_throw(&format!(
                            "@compile::rand only supports numeric types, found '{}'",
                            target_type.as_str()
                        ));
                    }
                    let evaled_args: Vec<Expr> = args
                        .iter()
                        .map(|a| self.eval_expr(a).unwrap_or_else(|_| a.clone()))
                        .collect();
                    return self.fastlang_rand(&target_type, &evaled_args);
                }

                if target_fn_name == "@compile::access" || target_fn_name == "access" {
                    if args.len() < 2 {
                        return self.fastlang_throw(
                            "@compile::access requires at least 2 arguments: (target, prop)",
                        );
                    }
                    let target_expr = &args[0];
                    let prop_expr = &args[1];

                    if expr_contains_this(target_expr) {
                        return Ok(expr.clone());
                    }

                    let (prop_name, expected_ty) = self.extract_prop_name_and_type(prop_expr);
                    if prop_name.is_empty() {
                        return self.fastlang_throw(
                            "@compile::access requires a valid member name in member specification",
                        );
                    }

                    let target_type = self.get_expr_type(target_expr);
                    let clean_target = self.extract_clean_blueprint_name(&target_type);

                    if let Some(ref env_rc) = self.current_env.clone() {
                        let env = env_rc.borrow();
                        if let Some(bp) = env.lookup_blueprint(&clean_target) {
                            let has_field = bp.fields.get(&prop_name);
                            let has_method = bp.methods.get(&prop_name);
                            let has_handle = bp.handle_signatures.get(&prop_name);
                            if has_field.is_none() && has_method.is_none() && has_handle.is_none() {
                                return self.fastlang_throw(&format!(
                                    "Member '{}' not found on type '{}'",
                                    prop_name, clean_target
                                ));
                            }
                            if let Some(ref expected) = expected_ty {
                                if let Some(field_ty) = has_field {
                                    let clean_actual =
                                        self.extract_clean_blueprint_name(&field_ty.as_str());
                                    let clean_expected =
                                        self.extract_clean_blueprint_name(expected);
                                    if clean_actual != clean_expected
                                        && field_ty.as_str() != *expected
                                    {
                                        return self.fastlang_throw(&format!(
                                            "Member '{}' on type '{}' has type '{}', but expected '{}'",
                                            prop_name, clean_target, field_ty.as_str(), expected
                                        ));
                                    }
                                }
                            }
                        }
                    }

                    return Ok(Expr::PropertyAccess {
                        object: Box::new(target_expr.clone()),
                        property: prop_name,
                    });
                }

                if target_fn_name.starts_with("@compile::")
                    && !self.functions.contains_key(&target_fn_name)
                {
                    let clean = target_fn_name.trim_start_matches("@compile::");
                    if !CompilerIntrinsics::is_compile_member(clean) {
                        eprintln!(
                            "STACKTRACE:\n{:?}",
                            std::backtrace::Backtrace::force_capture()
                        );
                        return self.fastlang_throw(&format!(
                            "Unknown compile intrinsic '{}'",
                            target_fn_name
                        ));
                    }
                }
                if let Some((params, body)) = self.micros.get(&target_fn_name).cloned() {
                    let mut eval_args = Vec::new();
                    for a in args {
                        eval_args.push(self.eval_expr(a).unwrap_or_else(|_| a.clone()));
                    }
                    let mut micro_interp = Interpreter {
                        env: InterpreterEnv::with_parent(self.env.clone()),
                        functions: self.functions.clone(),
                        micros: self.micros.clone(),
                        var_types: self.var_types.clone(),
                        current_env: self.current_env.clone(),
                        fn_overloads: self.fn_overloads.clone(),
                        split_functions: self.split_functions.clone(),
                        in_function_body: true,
                        current_switch_type: None,
                        current_type: self.current_type.clone(),
                        target_backend: self.target_backend.clone(),
                    };
                    for (i, param) in params.iter().enumerate() {
                        if let Some(arg_val) = eval_args.get(i) {
                            micro_interp.env.define(param.name.clone(), arg_val.clone());
                            let arg_t = self.get_expr_type(arg_val);
                            micro_interp.var_types.insert(param.name.clone(), arg_t);
                        }
                    }
                    if let Some(ret_val) = micro_interp.eval_method(&body, &params, &eval_args)? {
                        return Ok(ret_val);
                    } else {
                        return Ok(Expr::LiteralVoid);
                    }
                }

                let is_compilable = target_fn_name.starts_with("@compile::") || {
                    if let Some((_, _, body)) = self.functions.get(&target_fn_name) {
                        crate::middle_end::semantic::analyzer::detect_execution_mode(body)
                            == ExecutionMode::FullyCompilable
                    } else {
                        false
                    }
                };

                if is_compilable {
                    if let Some((fn_generics, params, body)) =
                        self.functions.get(&target_fn_name).cloned()
                    {
                        let mut eval_args = Vec::new();
                        for a in args {
                            eval_args.push(self.eval_expr(a).unwrap_or_else(|_| a.clone()));
                        }
                        let arg_types: Vec<String> =
                            eval_args.iter().map(|a| self.fastlang_typeof(a)).collect();
                        let generic_map =
                            crate::middle_end::semantic::analyzer::resolve_call_generics(
                                &fn_generics,
                                &params,
                                generics,
                                &arg_types,
                            );

                        let mut func_interp = Interpreter {
                            env: InterpreterEnv::with_parent(self.env.clone()),
                            functions: self.functions.clone(),
                            micros: self.micros.clone(),
                            var_types: self.var_types.clone(),
                            current_env: self.current_env.clone(),
                            fn_overloads: self.fn_overloads.clone(),
                            split_functions: self.split_functions.clone(),
                            in_function_body: true,
                            current_switch_type: None,
                            current_type: self.current_type.clone(),
                            target_backend: self.target_backend.clone(),
                        };
                        for (g_name, g_type) in &generic_map {
                            func_interp.env.define(
                                g_name.clone(),
                                Expr::LiteralString(format!("type<{}>", g_type.as_str())),
                            );
                        }
                        for (i, param) in params.iter().enumerate() {
                            if param.is_variadic {
                                let rest: Vec<Expr> = eval_args[i..].to_vec();
                                func_interp
                                    .env
                                    .define(param.name.clone(), Expr::ArrayLiteral(rest));
                                break;
                            }
                            let arg_val = if let Some(val) = eval_args.get(i) {
                                Some(val.clone())
                            } else if let Some(ref def) = param.default_value {
                                let folded =
                                    func_interp.eval_expr(def).unwrap_or_else(|_| def.clone());
                                Some(folded)
                            } else {
                                None
                            };
                            if let Some(val) = arg_val {
                                func_interp.env.define(param.name.clone(), val.clone());
                                let arg_t = func_interp.get_expr_type(&val);
                                func_interp.var_types.insert(param.name.clone(), arg_t);
                            }
                        }

                        if let Some(ret_val) =
                            func_interp.eval_method(&body, &params, &eval_args)?
                        {
                            return Ok(ret_val);
                        } else {
                            return Ok(Expr::LiteralVoid);
                        }
                    }
                }

                Ok(expr.clone())
            }
            _ => Ok(expr.clone()),
        }
    }

    pub fn fastlang_typeof(&mut self, expr: &Expr) -> String {
        self.get_expr_type(expr)
    }

    pub fn fastlang_sizeof(&mut self, expr: &Expr) -> u32 {
        let t = self.get_expr_type(expr);
        FastType::from_name(&t).size()
    }

    pub fn fastlang_throw(&mut self, msg: &str) -> Result<Expr, String> {
        Err(format!("{}: {}", "Compile Error".red().bold(), msg))
    }

    pub fn format_expr_for_print(&self, expr: &Expr) -> String {
        match expr {
            Expr::LiteralString(s) => s.clone(),
            Expr::LiteralInt(i) => i.to_string(),
            Expr::LiteralUInt(u) => u.to_string(),
            Expr::LiteralFloat(f) => f.to_string(),
            Expr::LiteralBool(b) => b.to_string(),
            Expr::LiteralChar(c) => c.to_string(),
            Expr::LiteralVoid => "()".to_string(),
            Expr::ArrayLiteral(elems) => {
                let inner: Vec<String> = elems
                    .iter()
                    .map(|e| self.format_expr_for_print(e))
                    .collect();
                format!("[{}]", inner.join(", "))
            }
            Expr::Identifier(id) => {
                if let Ok(val) = self.env.get(id) {
                    self.format_expr_for_print(&val)
                } else {
                    id.clone()
                }
            }
            _ => format!("{:?}", expr),
        }
    }

    pub fn expr_to_i128(&self, expr: &Expr) -> Result<i128, String> {
        match expr {
            Expr::LiteralInt(i) => Ok(*i),
            Expr::LiteralUInt(u) => Ok(*u as i128),
            Expr::LiteralFloat(f) => Ok(*f as i128),
            Expr::UnaryOp { operator, operand } if operator == "-" => {
                let inner = self.expr_to_i128(operand)?;
                Ok(-inner)
            }
            Expr::Identifier(id) => {
                if let Ok(v) = self.env.get(id) {
                    self.expr_to_i128(&v)
                } else {
                    Err(format!(
                        "Identifier '{}' is not a constant integer in rand()",
                        id
                    ))
                }
            }
            _ => Err(format!("Non-constant expression '{:?}' in rand()", expr)),
        }
    }

    pub fn expr_to_f64(&self, expr: &Expr) -> Result<f64, String> {
        match expr {
            Expr::LiteralFloat(f) => Ok(*f),
            Expr::LiteralInt(i) => Ok(*i as f64),
            Expr::LiteralUInt(u) => Ok(*u as f64),
            Expr::UnaryOp { operator, operand } if operator == "-" => {
                let inner = self.expr_to_f64(operand)?;
                Ok(-inner)
            }
            Expr::Identifier(id) => {
                if let Ok(v) = self.env.get(id) {
                    self.expr_to_f64(&v)
                } else {
                    Err(format!(
                        "Identifier '{}' is not a constant float in rand()",
                        id
                    ))
                }
            }
            _ => Err(format!("Non-constant expression '{:?}' in rand()", expr)),
        }
    }

    pub fn fastlang_rand(&mut self, target_type: &BaseType, args: &[Expr]) -> Result<Expr, String> {
        use std::time::{SystemTime, UNIX_EPOCH};
        static SEED_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(123456789);
        let counter = SEED_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let mut z = nanos.wrapping_add(counter.wrapping_mul(0x9e3779b97f4a7c15));
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        let seed = z ^ (z >> 31);

        let is_float = matches!(target_type, BaseType::Float(_));

        if is_float {
            let (min, max, step) = match args.len() {
                0 => (0.0, 1.0, None),
                1 => {
                    let max = self.expr_to_f64(&args[0])?;
                    (0.0, max, None)
                }
                2 => {
                    let min = self.expr_to_f64(&args[0])?;
                    let max = self.expr_to_f64(&args[1])?;
                    (min, max, None)
                }
                3 => {
                    let min = self.expr_to_f64(&args[0])?;
                    let max = self.expr_to_f64(&args[1])?;
                    let step = self.expr_to_f64(&args[2])?;
                    if step <= 0.0 {
                        return self.fastlang_throw("rand step must be greater than 0");
                    }
                    (min, max, Some(step))
                }
                _ => return self.fastlang_throw("rand takes at most 3 arguments (min, max, step)"),
            };

            let (min, max) = if min > max { (max, min) } else { (min, max) };
            let norm = (seed as f64) / (u64::MAX as f64);
            let val = if let Some(s) = step {
                let num_steps = ((max - min) / s).floor() as i64;
                let step_idx = (norm * ((num_steps + 1) as f64)).floor() as i64;
                min + (step_idx as f64) * s
            } else {
                min + norm * (max - min)
            };
            Ok(Expr::LiteralFloat(val))
        } else {
            let (min, max, step) = match args.len() {
                0 => (0i128, 100i128, None),
                1 => {
                    let max = self.expr_to_i128(&args[0])?;
                    (0i128, max, None)
                }
                2 => {
                    let min = self.expr_to_i128(&args[0])?;
                    let max = self.expr_to_i128(&args[1])?;
                    (min, max, None)
                }
                3 => {
                    let min = self.expr_to_i128(&args[0])?;
                    let max = self.expr_to_i128(&args[1])?;
                    let step = self.expr_to_i128(&args[2])?;
                    if step <= 0 {
                        return self.fastlang_throw("rand step must be greater than 0");
                    }
                    (min, max, Some(step))
                }
                _ => return self.fastlang_throw("rand takes at most 3 arguments (min, max, step)"),
            };

            let (min, max) = if min > max { (max, min) } else { (min, max) };
            let r = (seed as i128).abs();
            let val = if let Some(s) = step {
                let num_steps = (max - min) / s;
                let step_idx = if num_steps > 0 {
                    r % (num_steps + 1)
                } else {
                    0
                };
                min + step_idx * s
            } else {
                let range = max - min + 1;
                if range > 0 {
                    min + (r % range)
                } else {
                    min
                }
            };

            match target_type {
                BaseType::UInt(_) | BaseType::USize => Ok(Expr::LiteralUInt(val as u128)),
                _ => Ok(Expr::LiteralInt(val)),
            }
        }
    }

    pub fn extract_string_from_expr(&mut self, expr: &Expr) -> String {
        let evaled = self.eval_expr(expr).unwrap_or_else(|_| expr.clone());
        match &evaled {
            Expr::LiteralString(s) => s.clone(),
            Expr::Identifier(id) => {
                if let Ok(val) = self.env.get(id) {
                    self.extract_string_from_expr(&val)
                } else {
                    id.clone()
                }
            }
            Expr::New { target, .. } => self.extract_string_from_expr(target),
            Expr::Instantiate { args, .. } => {
                if let Some(first) = args.first() {
                    self.extract_string_from_expr(first)
                } else {
                    "Unknown Error".to_string()
                }
            }
            Expr::BinaryOp { .. } => {
                if let Ok(res) = self.eval_expr(&evaled) {
                    if let Expr::LiteralString(s) = res {
                        return s;
                    }
                }
                "Compile-time Error".to_string()
            }
            _ => self.format_expr_for_print(&evaled),
        }
    }

    pub fn extract_clean_blueprint_name(&self, type_str: &str) -> String {
        let mut clean = type_str.trim();
        loop {
            let mut changed = false;
            for prefix in &["blueprint::", "struct::", "class::", "enum::", "fast_std::"] {
                if clean.starts_with(prefix) {
                    clean = &clean[prefix.len()..];
                    changed = true;
                }
            }
            for prefix in &["type<"] {
                if clean.starts_with(prefix) && clean.ends_with('>') {
                    clean = &clean[prefix.len()..clean.len() - 1];
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        let base = clean.split('<').next().unwrap_or(clean);
        base.trim_end_matches('*')
            .trim_end_matches('&')
            .trim()
            .to_string()
    }


    pub fn extract_prop_name(&mut self, expr: &Expr) -> String {
        self.extract_prop_name_and_type(expr).0
    }

    pub fn extract_prop_name_and_type(&mut self, expr: &Expr) -> (String, Option<String>) {
        match expr {
            Expr::LiteralString(s) => (s.clone(), None),
            Expr::Identifier(id) => {
                if let Ok(val) = self.env.get(id) {
                    match &val {
                        Expr::LiteralString(s) => (s.clone(), None),
                        Expr::ObjectLiteral(_) | Expr::Instantiate { .. } => {
                            self.extract_prop_name_and_type(&val)
                        }
                        _ => (id.clone(), None),
                    }
                } else {
                    (id.clone(), None)
                }
            }
            Expr::ObjectLiteral(stmts) => {
                let mut name_opt = None;
                let mut type_opt = None;
                for stmt in stmts {
                    match stmt {
                        Stmt::Declaration(Decl::VarDecl { name, value, .. }) => {
                            if name == "name" {
                                name_opt = Some(self.extract_string_from_expr(value));
                            } else if name == "ty" || name == "type" {
                                type_opt = Some(self.extract_type_or_name(value));
                            }
                        }
                        Stmt::ReassignStmt { target, value, .. } => {
                            if let Expr::Identifier(field_name) = target {
                                if field_name == "name" {
                                    name_opt = Some(self.extract_string_from_expr(value));
                                } else if field_name == "ty" || field_name == "type" {
                                    type_opt = Some(self.extract_type_or_name(value));
                                }
                            }
                        }
                        Stmt::ExpressionStmt(Expr::BinaryOp {
                            left,
                            operator,
                            right,
                        }) if operator == "=" => {
                            if let Expr::Identifier(field_name) = &**left {
                                if field_name == "name" {
                                    name_opt = Some(self.extract_string_from_expr(right));
                                } else if field_name == "ty" || field_name == "type" {
                                    type_opt = Some(self.extract_type_or_name(right));
                                }
                            }
                        }
                        _ => {}
                    }
                }
                (name_opt.unwrap_or_default(), type_opt)
            }
            Expr::Instantiate { args, .. } => {
                for arg in args {
                    let (n, t) = self.extract_prop_name_and_type(arg);
                    if !n.is_empty() {
                        return (n, t);
                    }
                }
                ("".to_string(), None)
            }
            Expr::PropertyAccess { property, .. } => (property.clone(), None),
            _ => (self.extract_string_from_expr(expr), None),
        }
    }

    pub fn fastlang_cast(&mut self, val: &Expr, target_type: &BaseType) -> Result<Expr, String> {
        let resolved = match val {
            Expr::Identifier(id) => {
                if let Ok(v) = self.env.get(id) {
                    v
                } else {
                    val.clone()
                }
            }
            _ => val.clone(),
        };

        if matches!(resolved, Expr::LiteralUndefined) {
            eprintln!("Warning: Compile-time cast on undefined value. Consider using runtime 'as' operator instead.");
            return Ok(Expr::LiteralUndefined);
        }

        match target_type {
            BaseType::Int(_) | BaseType::ISize => match resolved {
                Expr::LiteralInt(i) => Ok(Expr::LiteralInt(i)),
                Expr::LiteralUInt(u) => Ok(Expr::LiteralInt(u as i128)),
                Expr::LiteralFloat(f) => Ok(Expr::LiteralInt(f as i128)),
                Expr::LiteralBool(b) => Ok(Expr::LiteralInt(if b { 1 } else { 0 })),
                Expr::LiteralChar(c) => Ok(Expr::LiteralInt(c as u32 as i128)),
                Expr::LiteralUChar(c) => Ok(Expr::LiteralInt(c as i128)),
                Expr::LiteralString(ref s) => s
                    .trim()
                    .parse::<i128>()
                    .map(Expr::LiteralInt)
                    .map_err(|_| format!("Compile Error: Cannot cast string \"{}\" to int", s)),
                _ => Err(format!("Compile Error: Cannot cast {:?} to int", resolved)),
            },
            BaseType::UInt(_) | BaseType::USize => match resolved {
                Expr::LiteralUInt(u) => Ok(Expr::LiteralUInt(u)),
                Expr::LiteralInt(i) => Ok(Expr::LiteralUInt(i as u128)),
                Expr::LiteralFloat(f) => Ok(Expr::LiteralUInt(f as u128)),
                Expr::LiteralBool(b) => Ok(Expr::LiteralUInt(if b { 1 } else { 0 })),
                Expr::LiteralChar(c) => Ok(Expr::LiteralUInt(c as u32 as u128)),
                Expr::LiteralUChar(c) => Ok(Expr::LiteralUInt(c as u128)),
                _ => Err(format!("Compile Error: Cannot cast {:?} to uint", resolved)),
            },
            BaseType::Float(_) => match resolved {
                Expr::LiteralFloat(f) => Ok(Expr::LiteralFloat(f)),
                Expr::LiteralInt(i) => Ok(Expr::LiteralFloat(i as f64)),
                Expr::LiteralUInt(u) => Ok(Expr::LiteralFloat(u as f64)),
                Expr::LiteralBool(b) => Ok(Expr::LiteralFloat(if b { 1.0 } else { 0.0 })),
                Expr::LiteralChar(c) => Ok(Expr::LiteralFloat(c as u32 as f64)),
                Expr::LiteralUChar(c) => Ok(Expr::LiteralFloat(c as f64)),
                _ => Err(format!(
                    "Compile Error: Cannot cast {:?} to float",
                    resolved
                )),
            },
            BaseType::Bool => match resolved {
                Expr::LiteralBool(b) => Ok(Expr::LiteralBool(b)),
                Expr::LiteralInt(i) => Ok(Expr::LiteralBool(i != 0)),
                Expr::LiteralUInt(u) => Ok(Expr::LiteralBool(u != 0)),
                Expr::LiteralFloat(f) => Ok(Expr::LiteralBool(f != 0.0)),
                Expr::LiteralChar(c) => Ok(Expr::LiteralBool(c != '\0')),
                Expr::LiteralUChar(c) => Ok(Expr::LiteralBool(c != 0)),
                _ => Err(format!("Compile Error: Cannot cast {:?} to bool", resolved)),
            },
            BaseType::Char => match resolved {
                Expr::LiteralChar(c) => Ok(Expr::LiteralChar(c)),
                Expr::LiteralUChar(c) => {
                    let ch = char::from_u32(c).unwrap_or('\0');
                    Ok(Expr::LiteralChar(ch))
                }
                Expr::LiteralInt(i) => {
                    let ch = char::from_u32(i as u32).unwrap_or('\0');
                    Ok(Expr::LiteralChar(ch))
                }
                Expr::LiteralUInt(u) => {
                    let ch = char::from_u32(u as u32).unwrap_or('\0');
                    Ok(Expr::LiteralChar(ch))
                }
                _ => Err(format!("Compile Error: Cannot cast {:?} to char", resolved)),
            },
            BaseType::UChar => match resolved {
                Expr::LiteralUChar(c) => Ok(Expr::LiteralUChar(c)),
                Expr::LiteralChar(c) => Ok(Expr::LiteralUChar(c as u32)),
                Expr::LiteralInt(i) => Ok(Expr::LiteralUChar(i as u32)),
                Expr::LiteralUInt(u) => Ok(Expr::LiteralUChar(u as u32)),
                _ => Err(format!(
                    "Compile Error: Cannot cast {:?} to uchar",
                    resolved
                )),
            },
            BaseType::Str => match resolved {
                Expr::LiteralString(s) => Ok(Expr::LiteralString(s)),
                Expr::LiteralUString(s) => Ok(Expr::LiteralString(s)),
                _ => Err(format!("Compile Error: Cannot cast {:?} to str", resolved)),
            },
            BaseType::UStr => match resolved {
                Expr::LiteralUString(s) => Ok(Expr::LiteralUString(s)),
                Expr::LiteralString(s) => Ok(Expr::LiteralUString(s)),
                _ => Err(format!("Compile Error: Cannot cast {:?} to ustr", resolved)),
            },
            _ => Err(format!(
                "Compile Error: Compile-time cast to '{}' is not supported",
                target_type.as_str()
            )),
        }
    }


    pub fn infer_iterable_element_type(&mut self, iterable_type: &str, iterable: &Expr) -> String {
        let clean_iter = self.extract_clean_blueprint_name(iterable_type);
        if iterable_type.starts_with("array<") && iterable_type.ends_with('>') {
            let inner = &iterable_type[6..iterable_type.len() - 1];
            return self.resolve_type_str(inner.trim());
        }
        if iterable_type.ends_with("[]") {
            let inner = &iterable_type[..iterable_type.len() - 2];
            return self.resolve_type_str(inner.trim());
        }
        if iterable_type == "array" {
            if let Expr::ArrayLiteral(elements) = iterable {
                if let Some(first) = elements.first() {
                    let t = self.get_expr_type(first);
                    if t != "unknown" {
                        return t;
                    }
                }
            }
        }
        if let Some(ref env_rc) = self.current_env.clone() {
            let env = env_rc.borrow();
            if let Some(bp) = env.lookup_blueprint(&clean_iter) {
                // Check for iter() method or handle
                if let Some(iter_sig) = bp
                    .methods
                    .get("iter")
                    .or_else(|| bp.handle_signatures.get("iter"))
                {
                    let cursor_type = iter_sig.return_type.as_str();
                    let substituted_cursor = self.substitute_generics_in_type_str(
                        &cursor_type,
                        &bp.generics,
                        iterable_type,
                    );
                    let clean_cursor = self.extract_clean_blueprint_name(&substituted_cursor);
                    if let Some(cursor_bp) = env.lookup_blueprint(&clean_cursor) {
                        if let Some(next_sig) = cursor_bp
                            .methods
                            .get("next")
                            .or_else(|| cursor_bp.handle_signatures.get("next"))
                        {
                            let ret = next_sig.return_type.as_str();
                            let sub_ret = self.substitute_generics_in_type_str(
                                &ret,
                                &cursor_bp.generics,
                                &substituted_cursor,
                            );
                            return self.resolve_type_str(&sub_ret);
                        }
                    }
                }
                // Check for direct next() method or handle
                if let Some(next_sig) = bp
                    .methods
                    .get("next")
                    .or_else(|| bp.handle_signatures.get("next"))
                {
                    let ret = next_sig.return_type.as_str();
                    let sub_ret =
                        self.substitute_generics_in_type_str(&ret, &bp.generics, iterable_type);
                    return self.resolve_type_str(&sub_ret);
                }
            }
        }
        // Fallback if iterable_type has generic parameter (e.g. List<int32>)
        if iterable_type.contains('<') && iterable_type.ends_with('>') {
            let start = iterable_type.find('<').unwrap() + 1;
            let end = iterable_type.rfind('>').unwrap();
            let inner = iterable_type[start..end]
                .split(',')
                .next()
                .unwrap_or("")
                .trim();
            if !inner.is_empty() {
                return self.resolve_type_str(inner);
            }
        }
        "unknown".to_string()
    }

    pub fn extract_type_string(&mut self, expr: &Expr) -> Option<String> {
        match expr {
            Expr::LiteralString(s) if s.starts_with("type<") && s.ends_with('>') => {
                Some(strip_type_wrapper(s).to_string())
            }
            Expr::Call { callee, args, .. } if is_typeof_callee(callee) => {
                args.first().map(|a| self.get_expr_type(a))
            }
            Expr::Identifier(id) => {
                if let Ok(Expr::LiteralString(s)) = self.env.get(id) {
                    if s.starts_with("type<") && s.ends_with('>') {
                        return Some(strip_type_wrapper(&s).to_string());
                    }
                }
                if let Some(t) = self.var_types.get(id) {
                    if t == "type" || t.starts_with("type<") {
                        if let Ok(Expr::LiteralString(s)) = self.env.get(id) {
                            let clean = strip_type_wrapper(s.as_str());
                            if clean != s {
                                return Some(clean.to_string());
                            }
                        }
                        return Some(t.clone());
                    }
                    None
                } else if is_known_type_name(id)
                    || id.ends_with("[]")
                    || (id.contains('<') && id.ends_with('>'))
                    || (self.current_env.is_some() && {
                        let base = id.split('<').next().unwrap_or(id);
                        self.current_env
                            .as_ref()
                            .unwrap()
                            .borrow()
                            .lookup_blueprint(base)
                            .is_some()
                    })
                {
                    Some(self.resolve_type_str(id))
                } else {
                    let resolved = self.resolve_type_str(id);
                    if resolved != *id {
                        Some(resolved)
                    } else {
                        None
                    }
                }
            }
            _ => None,
        }
    }

    pub fn get_expr_type(&mut self, expr: &Expr) -> String {
        let raw = match expr {
            Expr::LiteralInt(_) => "int32".to_string(),
            Expr::LiteralUInt(_) => "uint32".to_string(),
            Expr::LiteralFloat(_) => "float64".to_string(),
            Expr::LiteralString(s) if s.starts_with("type<") && s.ends_with('>') => {
                strip_type_wrapper(s).to_string()
            }
            Expr::LiteralString(_) => "str".to_string(),
            Expr::LiteralUString(_) => "ustr".to_string(),
            Expr::LiteralBool(_) => "bool".to_string(),
            Expr::LiteralChar(_) => "char".to_string(),
            Expr::LiteralUChar(_) => "uchar".to_string(),
            Expr::LiteralVoid => "void".to_string(),
            Expr::This => {
                if let Some(ref t) = self.current_type {
                    t.clone()
                } else {
                    "unknown".to_string()
                }
            }
            Expr::Identifier(id) => {
                let ty = if is_known_type_name(id) {
                    id.clone()
                } else if let Some(t) = self.var_types.get(id) {
                    t.clone()
                } else if let Ok(val) = self.env.get(id) {
                    let val_ty = match &val {
                        Expr::LiteralString(s) if s.starts_with("type<") && s.ends_with('>') => {
                            strip_type_wrapper(s).to_string()
                        }
                        Expr::Identifier(other_id) if other_id == id => "unknown".to_string(),
                        _ => self.get_expr_type(&val),
                    };
                    if val_ty != "unknown" {
                        val_ty
                    } else if let Some(ref env_rc) = self.current_env {
                        if env_rc.borrow().lookup_blueprint(id).is_some() {
                            id.clone()
                        } else if let Some(info) = env_rc.borrow().lookup(id) {
                            if let SymbolKind::Variable { type_node, .. } = &info.kind {
                                type_node.as_str()
                            } else {
                                "unknown".to_string()
                            }
                        } else {
                            "unknown".to_string()
                        }
                    } else {
                        "unknown".to_string()
                    }
                } else if let Some(ref env_rc) = self.current_env {
                    if env_rc.borrow().lookup_blueprint(id).is_some() {
                        id.clone()
                    } else if let Some(info) = env_rc.borrow().lookup(id) {
                        if let SymbolKind::Variable { type_node, .. } = &info.kind {
                            type_node.as_str()
                        } else {
                            "unknown".to_string()
                        }
                    } else {
                        "unknown".to_string()
                    }
                } else {
                    "unknown".to_string()
                };
                self.resolve_type_str(&ty)
            }
            Expr::UnaryOp { operator, operand } if operator == "&" => {
                format!("{}*", self.get_expr_type(operand))
            }
            Expr::PropertyAccess { object, property } => {
                let obj_type = self.get_expr_type(object);
                let clean_obj = self.extract_clean_blueprint_name(&obj_type);
                if let Some(ref env_rc) = self.current_env.clone() {
                    let env = env_rc.borrow();
                    if let Some(bp) = env.lookup_blueprint(&clean_obj) {
                        if let Some(ft) = bp.fields.get(property) {
                            let ft_str = ft.as_str();
                            let substituted = self.substitute_generics_in_type_str(
                                &ft_str,
                                &bp.generics,
                                &obj_type,
                            );
                            return self.resolve_type_str(&substituted);
                        }
                        if let Some(sig) = bp
                            .methods
                            .get(property)
                            .or_else(|| bp.handle_signatures.get(property))
                        {
                            let ret_str = sig.return_type.as_str();
                            let substituted = self.substitute_generics_in_type_str(
                                &ret_str,
                                &bp.generics,
                                &obj_type,
                            );
                            return self.resolve_type_str(&substituted);
                        }
                    }
                }
                "unknown".to_string()
            }
            Expr::ArrayLiteral(elements) => {
                if let Some(first) = elements.first() {
                    let elem_t = self.get_expr_type(first);
                    if elem_t != "unknown" {
                        format!("array<{}>", elem_t)
                    } else {
                        "array".to_string()
                    }
                } else {
                    "array".to_string()
                }
            }
            Expr::Instantiate { target, .. } => match &**target {
                Expr::Identifier(id) => id.clone(),
                _ => self.get_expr_type(target),
            },
            Expr::New { type_node, .. } => type_node.as_str(),
            Expr::Cast { target_type, .. } => target_type.as_str(),
            Expr::IfExpr {
                then_branch,
                else_branch,
                ..
            } => {
                let t = self.get_expr_type(then_branch);
                if t != "unknown" {
                    t
                } else {
                    self.get_expr_type(else_branch)
                }
            }
            Expr::Call { callee, .. } => {
                let fn_name = match &**callee {
                    Expr::Identifier(id) => id.as_str(),
                    Expr::PropertyAccess { property, .. } => property.as_str(),
                    Expr::NamespaceAccess { property, .. } => {
                        if let Expr::Identifier(p) = &**property {
                            p.as_str()
                        } else {
                            ""
                        }
                    }
                    _ => "",
                };
                if let Some(sigs) = self.fn_overloads.get(fn_name) {
                    if let Some(sig) = sigs.first() {
                        return self.resolve_type_str(&sig.return_type.as_str());
                    }
                }
                if let Some(ref env_rc) = self.current_env {
                    if let Some(info) = env_rc.borrow().lookup(fn_name) {
                        if let SymbolKind::Function { return_type, .. } = &info.kind {
                            return self.resolve_type_str(&return_type.as_str());
                        }
                    }
                }
                "unknown".to_string()
            }
            _ => "unknown".to_string(),
        };
        self.resolve_type_str(&raw)
    }


    pub fn try_expand_access_handle(
        &mut self,
        object: &Expr,
        prop_name: &str,
        handle_name: &str,
    ) -> Result<Option<Expr>, String> {
        let obj_type = self.get_expr_type(object);
        if obj_type == "unknown" {
            return Ok(None);
        }
        let clean_obj = self.extract_clean_blueprint_name(&obj_type);
        let env_rc = match self.current_env.clone() {
            Some(e) => e,
            None => return Ok(None),
        };
        let bp = match env_rc.borrow().lookup_blueprint(&clean_obj) {
            Some(b) => b,
            None => return Ok(None),
        };
        if handle_name == "property_access" {
            if bp.fields.contains_key(prop_name)
                || bp.methods.contains_key(prop_name)
                || bp.handle_signatures.contains_key(prop_name)
            {
                return Ok(None);
            }
        } else if handle_name == "handle_access" {
            if bp.handle_signatures.contains_key(prop_name) || bp.methods.contains_key(prop_name) {
                return Ok(None);
            }
        }

        let sig = match bp.handle_signatures.get(handle_name) {
            Some(s) => s,
            None => return Ok(None),
        };

        let param_ty = self.extract_clean_blueprint_name(&sig.params[0].type_node.as_str());
        let ret_ty = sig.return_type.as_str();
        let is_access_sig = sig.params.len() == 1
            && (param_ty == "MemberType" || param_ty == "unknown")
            && ret_ty.ends_with('*');

        if !is_access_sig {
            return Ok(None);
        }

        let fn_key = format!("{}::{}", clean_obj, handle_name);
        let body = match self.functions.get(&fn_key) {
            Some((_, _, b)) => b.clone(),
            None => match self.functions.get(handle_name) {
                Some((_, _, b)) => b.clone(),
                None => return Ok(None),
            },
        };

        for stmt in &body {
            if let Stmt::ReturnStmt(Some(ret_expr)) = stmt {
                let target_opt: Option<&Expr> = match ret_expr {
                    Expr::Call { callee, args, .. } => {
                        let is_compile_access = match &**callee {
                            Expr::NamespaceAccess {
                                namespace,
                                property,
                            } => {
                                namespace == "@compile"
                                    && matches!(&**property, Expr::Identifier(p) if p == "access")
                            }
                            Expr::Identifier(id) => id == "@compile::access" || id == "access",
                            _ => false,
                        };
                        if is_compile_access && args.len() >= 2 {
                            Some(&args[0])
                        } else {
                            None
                        }
                    }
                    Expr::UnaryOp { operator, operand } if operator == "&" => {
                        if let Expr::PropertyAccess { object, .. } = &**operand {
                            Some(&**object)
                        } else {
                            None
                        }
                    }
                    _ => None,
                };

                if let Some(target) = target_opt {
                    let substituted_target = substitute_this_in_expr(target, object);
                    let target_type = self.get_expr_type(&substituted_target);
                    let clean_target = self.extract_clean_blueprint_name(&target_type);
                    if let Some(target_bp) = env_rc.borrow().lookup_blueprint(&clean_target) {
                        let has_member = target_bp.fields.contains_key(prop_name)
                            || target_bp.methods.contains_key(prop_name)
                            || target_bp.handle_signatures.contains_key(prop_name);
                        if !has_member {
                            return self
                                .fastlang_throw(&format!(
                                    "Member '{}' not found on type '{}'",
                                    prop_name, clean_target
                                ))
                                .map(|_| None);
                        }
                    }
                    return Ok(Some(Expr::PropertyAccess {
                        object: Box::new(substituted_target),
                        property: prop_name.to_string(),
                    }));
                }
            }
        }

        Ok(None)
    }

    pub fn fold_expr(&mut self, expr: &mut Expr) -> Result<(), String> {
        match expr {
            Expr::UnaryOp { operator, operand } => {
                self.fold_expr(operand)?;
                match (operator.as_str(), &**operand) {
                    ("!", Expr::LiteralBool(b)) => *expr = Expr::LiteralBool(!b),
                    ("-", Expr::LiteralInt(i)) => *expr = Expr::LiteralInt(-i),
                    ("-", Expr::LiteralFloat(f)) => *expr = Expr::LiteralFloat(-f),
                    _ => {}
                }
            }
            Expr::BinaryOp {
                left,
                operator,
                right,
            } => {
                self.fold_expr(left)?;
                self.fold_expr(right)?;

                if operator == "->" || operator == "=>" {
                    let handle_name = if operator == "->" {
                        "arrow"
                    } else {
                        "fat_arrow"
                    };
                    if let Expr::Identifier(prop_name) = &**right {
                        if let Some(expanded) =
                            self.try_expand_access_handle(left, prop_name, handle_name)?
                        {
                            *expr = expanded;
                            return self.fold_expr(expr);
                        }
                    } else if let Expr::Call {
                        callee,
                        generics,
                        args,
                    } = &**right
                    {
                        if let Expr::Identifier(prop_name) = &**callee {
                            if let Some(expanded) =
                                self.try_expand_access_handle(left, prop_name, handle_name)?
                            {
                                *expr = Expr::Call {
                                    callee: Box::new(expanded),
                                    generics: generics.clone(),
                                    args: args.clone(),
                                };
                                return self.fold_expr(expr);
                            }
                        }
                    }
                }

                // Namespace / Enum comparison, e.g. Backend::Cpp == Backend::Cpp
                if operator == "==" || operator == "!=" {
                    let extract_ns = |e: &Expr| -> Option<(String, String)> {
                        match e {
                            Expr::NamespaceAccess {
                                namespace,
                                property,
                            } => {
                                if let Expr::Identifier(p) = &**property {
                                    Some((namespace.clone(), p.clone()))
                                } else {
                                    None
                                }
                            }
                            _ => None,
                        }
                    };
                    if let (Some((l_ns, l_prop)), Some((r_ns, r_prop))) =
                        (extract_ns(left), extract_ns(right))
                    {
                        if l_ns == r_ns {
                            let eq = l_prop == r_prop;
                            let res = if operator == "==" { eq } else { !eq };
                            *expr = Expr::LiteralBool(res);
                            return Ok(());
                        }
                    }
                }

                if operator == "&&" {
                    if let (Expr::LiteralBool(l), Expr::LiteralBool(r)) = (&**left, &**right) {
                        *expr = Expr::LiteralBool(*l && *r);
                        return Ok(());
                    }
                }
                if operator == "||" {
                    if let (Expr::LiteralBool(l), Expr::LiteralBool(r)) = (&**left, &**right) {
                        *expr = Expr::LiteralBool(*l || *r);
                        return Ok(());
                    }
                }

                // Type comparisons, e.g. typeof(id) == int32 or t == int32
                if operator == "==" || operator == "!=" {
                    let left_ty = self.extract_type_string(left);
                    let right_ty = self.extract_type_string(right);
                    if let (Some(lt), Some(rt)) = (left_ty, right_ty) {
                        let clean_lt = strip_type_wrapper(&lt).to_string();
                        let clean_rt = strip_type_wrapper(&rt).to_string();
                        let eq = clean_lt == clean_rt;
                        let res = if operator == "==" { eq } else { !eq };
                        *expr = Expr::LiteralBool(res);
                        return Ok(());
                    }
                }

                if is_literal(left) && is_literal(right) {
                    if let (Expr::LiteralString(s), "+", Expr::LiteralChar(c)) =
                        (&**left, operator.as_str(), &**right)
                    {
                        *expr = Expr::LiteralString(format!("{}{}", s, c));
                        return Ok(());
                    }
                    if let (Expr::LiteralChar(c), "+", Expr::LiteralString(s)) =
                        (&**left, operator.as_str(), &**right)
                    {
                        *expr = Expr::LiteralString(format!("{}{}", c, s));
                        return Ok(());
                    }
                    if let Ok(res) = self.eval_expr(expr) {
                        if is_literal(&res) {
                            *expr = res;
                        }
                    }
                }
            }
            Expr::HandleCall {
                object,
                handle_name,
                args,
            } => {
                self.fold_expr(object)?;
                for a in args.iter_mut() {
                    self.fold_expr(a)?;
                }
                if args.is_empty() {
                    if let Some(expanded) =
                        self.try_expand_access_handle(object, handle_name, "handle_access")?
                    {
                        *expr = expanded;
                        return self.fold_expr(expr);
                    }
                }
            }
            Expr::NamespaceAccess {
                namespace,
                property,
            } => {
                if namespace == "@compile" {
                    if let Expr::Identifier(prop_name) = &**property {
                        if prop_name == "backend" {
                            let variant = if self.target_backend == "cranelift" {
                                "Cranelift"
                            } else {
                                "Cpp"
                            };
                            *expr = Expr::NamespaceAccess {
                                namespace: "Backend".to_string(),
                                property: Box::new(Expr::Identifier(variant.to_string())),
                            };
                            return Ok(());
                        } else if prop_name == "system" {
                            let variant = if cfg!(target_os = "windows") {
                                "Windows"
                            } else if cfg!(target_os = "macos") {
                                "MacOS"
                            } else {
                                "Linux"
                            };
                            *expr = Expr::NamespaceAccess {
                                namespace: "System".to_string(),
                                property: Box::new(Expr::Identifier(variant.to_string())),
                            };
                            return Ok(());
                        }
                    }
                }
                self.fold_expr(property)?;
                if let Expr::Identifier(prop_name) = &**property {
                    let obj_expr = Expr::Identifier(namespace.clone());
                    if let Some(expanded) =
                        self.try_expand_access_handle(&obj_expr, prop_name, "namespace_access")?
                    {
                        *expr = expanded;
                        return self.fold_expr(expr);
                    }
                }
            }
            Expr::Cast {
                expr: inner,
                target_type,
            } => {
                self.fold_expr(inner)?;

                // Execute @compile validation block from cast handle if defined (e.g. array::cast)
                let inner_type = self.get_expr_type(inner);
                let (base_name, elem_type) = if inner_type.ends_with("[]") {
                    (
                        "array".to_string(),
                        inner_type[..inner_type.len() - 2].trim().to_string(),
                    )
                } else if inner_type.starts_with("array<") && inner_type.ends_with('>') {
                    (
                        "array".to_string(),
                        inner_type["array<".len()..inner_type.len() - 1]
                            .trim()
                            .to_string(),
                    )
                } else {
                    (
                        self.extract_clean_blueprint_name(&inner_type),
                        inner_type.clone(),
                    )
                };

                if elem_type != "unknown"
                    && elem_type != "T"
                    && !matches!(target_type, BaseType::GenericParam(_))
                {
                    let cast_fn_key = format!("{}::cast", base_name);
                    let found_fn = self.functions.get(&cast_fn_key).cloned().or_else(|| {
                        if base_name == "array" {
                            self.functions.get("cast").cloned()
                        } else {
                            None
                        }
                    });

                    if let Some((_fn_generics, _params, body)) = found_fn {
                        for stmt in &body {
                            if let Stmt::CompileValidation { body: val_body, .. } = stmt {
                                let mut val_interp = Interpreter {
                                    env: InterpreterEnv::with_parent(self.env.clone()),
                                    functions: self.functions.clone(),
                                    micros: self.micros.clone(),
                                    var_types: self.var_types.clone(),
                                    current_env: self.current_env.clone(),
                                    fn_overloads: self.fn_overloads.clone(),
                                    split_functions: self.split_functions.clone(),
                                    in_function_body: true,
                                    current_switch_type: None,
                                    current_type: self.current_type.clone(),
                                    target_backend: self.target_backend.clone(),
                                };

                                if base_name == "array" {
                                    val_interp.env.define(
                                        "T".to_string(),
                                        Expr::LiteralString(format!("type<{}>", elem_type)),
                                    );
                                    val_interp
                                        .var_types
                                        .insert("T".to_string(), "type".to_string());
                                }
                                val_interp.env.define(
                                    "Target".to_string(),
                                    Expr::LiteralString(format!("type<{}>", target_type.as_str())),
                                );
                                val_interp
                                    .var_types
                                    .insert("Target".to_string(), "type".to_string());
                                val_interp.env.define("this".to_string(), (**inner).clone());

                                for s in val_body {
                                    val_interp.eval_stmt(s)?;
                                }
                            }
                        }
                    }
                }

                let resolved = if let Expr::Identifier(id) = &**inner {
                    self.env.get(id).unwrap_or_else(|_| *inner.clone())
                } else {
                    *inner.clone()
                };
                if is_literal(&resolved) {
                    if let Ok(res) = self.fastlang_cast(&resolved, target_type) {
                        *expr = res;
                    }
                }
            }
            Expr::PropertyAccess { object, property } => {
                self.fold_expr(object)?;
                if let Some(expanded) =
                    self.try_expand_access_handle(object, property, "property_access")?
                {
                    *expr = expanded;
                    return self.fold_expr(expr);
                }
                let obj_type_name = self.extract_type_string(object);
                if let Some(t_name) = obj_type_name {
                    let ft = self.get_fast_type(&t_name);
                    match property.as_str() {
                        "is_primitive" => *expr = Expr::LiteralBool(ft.is_primitive()),
                        "is_pointer" => *expr = Expr::LiteralBool(ft.is_pointer()),
                        "is_array" => *expr = Expr::LiteralBool(ft.is_array()),
                        "printable" | "is_printable" => *expr = Expr::LiteralBool(ft.printable()),
                        "throwable" | "is_throwable" => *expr = Expr::LiteralBool(ft.throwable()),
                        "shareable" | "is_shareable" => *expr = Expr::LiteralBool(ft.shareable()),
                        "copyable" | "is_copyable" => *expr = Expr::LiteralBool(ft.copyable()),
                        "castable" | "is_castable" => *expr = Expr::LiteralBool(ft.castable()),
                        "is_constant" | "is_const" => *expr = Expr::LiteralBool(ft.is_constant()),
                        "is_runtime" => *expr = Expr::LiteralBool(ft.is_runtime()),
                        "is_comptime" => *expr = Expr::LiteralBool(ft.is_comptime()),
                        "is_undefined" => *expr = Expr::LiteralBool(ft.is_undefined()),
                        "on_heap" => *expr = Expr::LiteralBool(ft.on_heap()),
                        "on_stack" => *expr = Expr::LiteralBool(ft.on_stack()),
                        "size" => *expr = Expr::LiteralInt(ft.size() as i128),
                        "as_str" => *expr = Expr::LiteralString(ft.as_str()),
                        "default" => {
                            if let Some(def) = self.default_for_type(&t_name) {
                                *expr = def;
                            } else {
                                *expr = Expr::Default(Some(BaseType::from_str(&t_name)));
                            }
                        }
                        _ => {
                            if let Some(method_name) = property.strip_prefix("has_") {
                                *expr = Expr::LiteralBool(
                                    ft.has_method(method_name)
                                        || ft.has_handle(method_name)
                                        || (method_name == "as_str"
                                            && (ft.printable()
                                                || ft.has_method("as_str")
                                                || ft.has_handle("display"))),
                                );
                            }
                        }
                    }
                } else if let Ok(res) = self.eval_expr(expr) {
                    if is_literal(&res) {
                        *expr = res;
                    }
                }
            }
            Expr::Call {
                callee,
                generics,
                args,
            } => {
                for a in args.iter_mut() {
                    self.fold_expr(a)?;
                }

                if let Expr::NamespaceAccess {
                    namespace,
                    property,
                } = &**callee
                {
                    if namespace == "@compile" {
                        if let Expr::Identifier(prop_name) = &**property {
                            if prop_name == "backend" {
                                let variant = if self.target_backend == "cranelift" {
                                    "Cranelift"
                                } else {
                                    "Cpp"
                                };
                                *expr = Expr::NamespaceAccess {
                                    namespace: "Backend".to_string(),
                                    property: Box::new(Expr::Identifier(variant.to_string())),
                                };
                                return Ok(());
                            } else if prop_name == "system" {
                                let variant = if cfg!(target_os = "windows") {
                                    "Windows"
                                } else if cfg!(target_os = "macos") {
                                    "MacOS"
                                } else {
                                    "Linux"
                                };
                                *expr = Expr::NamespaceAccess {
                                    namespace: "System".to_string(),
                                    property: Box::new(Expr::Identifier(variant.to_string())),
                                };
                                return Ok(());
                            }
                        }
                    }
                }

                // Expand micro calls in expression contexts
                let micro_info = {
                    let m_name = match &**callee {
                        Expr::Identifier(id) => Some(id.clone()),
                        Expr::NamespaceAccess {
                            namespace,
                            property,
                        } => {
                            if let Expr::Identifier(p) = &**property {
                                Some(format!("{}::{}", namespace, p))
                            } else {
                                None
                            }
                        }
                        _ => None,
                    };
                    if let Some(ref name) = m_name {
                        self.micros.get(name).cloned().or_else(|| {
                            if let Some(short) = name.split("::").last() {
                                self.micros.get(short).cloned()
                            } else {
                                None
                            }
                        })
                    } else {
                        None
                    }
                };

                if let Some((params, body)) = micro_info {
                    let ret_expr = body.iter().find_map(|s| match s {
                        Stmt::ReturnStmt(Some(e)) => Some(e.clone()),
                        Stmt::ExpressionStmt(e) | Stmt::CallStmt(e) => Some(e.clone()),
                        _ => None,
                    });
                    if let Some(r_expr) = ret_expr {
                        let param_map = self.build_micro_param_map(&params, args);
                        let mut substituted = substitute_expr(&r_expr, &param_map);
                        self.fold_expr(&mut substituted)?;
                        *expr = substituted;
                        return Ok(());
                    } else {
                        *expr = Expr::LiteralVoid;
                        return Ok(());
                    }
                }

                // Property method calls on types, e.g. typeof(id).printable(), typeof(id).castable_to(float64)
                if let Expr::PropertyAccess { object, property } = &mut **callee {
                    self.fold_expr(object)?;
                    let obj_type_name = self.extract_type_string(object);
                    if let Some(t_name) = obj_type_name {
                        let ft = self.get_fast_type(&t_name);
                        match property.as_str() {
                            "printable" | "is_printable" => {
                                *expr = Expr::LiteralBool(ft.printable());
                                return Ok(());
                            }
                            "throwable" | "is_throwable" => {
                                *expr = Expr::LiteralBool(ft.throwable());
                                return Ok(());
                            }
                            "shareable" | "is_shareable" => {
                                *expr = Expr::LiteralBool(ft.shareable());
                                return Ok(());
                            }
                            "copyable" | "is_copyable" => {
                                *expr = Expr::LiteralBool(ft.copyable());
                                return Ok(());
                            }
                            "castable" | "is_castable" => {
                                *expr = Expr::LiteralBool(ft.castable());
                                return Ok(());
                            }
                            "castable_to" => {
                                if let Some(target_arg) = args.first() {
                                    let target_name = self
                                        .extract_type_string(target_arg)
                                        .unwrap_or_else(|| self.get_expr_type(target_arg));
                                    let target_ft = self.get_fast_type(&target_name);
                                    *expr = Expr::LiteralBool(ft.castable_to(&target_ft));
                                    return Ok(());
                                }
                            }
                            "is_primitive" => {
                                *expr = Expr::LiteralBool(ft.is_primitive());
                                return Ok(());
                            }
                            "is_pointer" => {
                                *expr = Expr::LiteralBool(ft.is_pointer());
                                return Ok(());
                            }
                            "is_array" => {
                                *expr = Expr::LiteralBool(ft.is_array());
                                return Ok(());
                            }
                            "is_constant" | "is_const" => {
                                *expr = Expr::LiteralBool(ft.is_constant());
                                return Ok(());
                            }
                            "is_runtime" => {
                                *expr = Expr::LiteralBool(ft.is_runtime());
                                return Ok(());
                            }
                            "is_comptime" => {
                                *expr = Expr::LiteralBool(ft.is_comptime());
                                return Ok(());
                            }
                            "is_undefined" => {
                                *expr = Expr::LiteralBool(ft.is_undefined());
                                return Ok(());
                            }
                            "on_heap" => {
                                *expr = Expr::LiteralBool(ft.on_heap());
                                return Ok(());
                            }
                            "on_stack" => {
                                *expr = Expr::LiteralBool(ft.on_stack());
                                return Ok(());
                            }
                            "has_handle" => {
                                if let Some(target_arg) = args.first() {
                                    let val = self
                                        .eval_expr(target_arg)
                                        .unwrap_or_else(|_| target_arg.clone());
                                    let s1 = self.extract_type_or_name(&val);
                                    let s2 = args.get(1).map(|a| {
                                        let v = self.eval_expr(a).unwrap_or_else(|_| a.clone());
                                        self.extract_type_or_name(&v)
                                    });
                                    *expr =
                                        Expr::LiteralBool(ft.has_handle_query(&s1, s2.as_deref()));
                                    return Ok(());
                                }
                                *expr = Expr::LiteralBool(false);
                                return Ok(());
                            }
                            "has_method" => {
                                if let Some(target_arg) = args.first() {
                                    let val = self
                                        .eval_expr(target_arg)
                                        .unwrap_or_else(|_| target_arg.clone());
                                    let s1 = self.extract_type_or_name(&val);
                                    let s2 = args.get(1).map(|a| {
                                        let v = self.eval_expr(a).unwrap_or_else(|_| a.clone());
                                        self.extract_type_or_name(&v)
                                    });
                                    *expr =
                                        Expr::LiteralBool(ft.has_method_query(&s1, s2.as_deref()));
                                    return Ok(());
                                }
                                *expr = Expr::LiteralBool(false);
                                return Ok(());
                            }
                            "has_field" => {
                                if let Some(target_arg) = args.first() {
                                    let val = self
                                        .eval_expr(target_arg)
                                        .unwrap_or_else(|_| target_arg.clone());
                                    let s1 = self.extract_type_or_name(&val);
                                    let s2 = args.get(1).map(|a| {
                                        let v = self.eval_expr(a).unwrap_or_else(|_| a.clone());
                                        self.extract_type_or_name(&v)
                                    });
                                    *expr =
                                        Expr::LiteralBool(ft.has_field_query(&s1, s2.as_deref()));
                                    return Ok(());
                                }
                                *expr = Expr::LiteralBool(false);
                                return Ok(());
                            }
                            "size" => {
                                *expr = Expr::LiteralInt(ft.size() as i128);
                                return Ok(());
                            }
                            "default" => {
                                if let Some(def) = self.default_for_type(&t_name) {
                                    *expr = def;
                                } else {
                                    *expr = Expr::Default(Some(BaseType::from_str(&t_name)));
                                }
                                return Ok(());
                            }
                            _ => {
                                if let Some(method_name) = property.strip_prefix("has_") {
                                    *expr = Expr::LiteralBool(
                                        ft.has_method(method_name)
                                            || ft.has_handle(method_name)
                                            || (method_name == "as_str"
                                                && (ft.printable()
                                                    || ft.has_method("as_str")
                                                    || ft.has_handle("display"))),
                                    );
                                    return Ok(());
                                }
                            }
                        }
                    }
                }
                self.fold_expr(callee)?;

                let fn_name = match &**callee {
                    Expr::NamespaceAccess {
                        namespace,
                        property,
                    } if namespace == "@compile" => {
                        if let Expr::Identifier(p) = &**property {
                            p.as_str()
                        } else {
                            ""
                        }
                    }
                    Expr::Identifier(id) => id.as_str(),
                    _ => "",
                };

                if fn_name == "typeof" {
                    if let Some(arg) = args.first() {
                        let t = self.get_expr_type(arg);
                        if t != "unknown" {
                            *expr = Expr::LiteralString(format!("type<{}>", t));
                            return Ok(());
                        }
                    }
                }

                if fn_name == "sizeof" {
                    if let Some(arg) = args.first() {
                        let t = self.get_expr_type(arg);
                        if t != "unknown" {
                            let sz = FastType::from_name(&t).size();
                            *expr = Expr::LiteralInt(sz as i128);
                            return Ok(());
                        }
                    }
                }

                if fn_name == "rand" {
                    let target_type = generics
                        .first()
                        .cloned()
                        .unwrap_or(BaseType::Int(crate::frontend::parser::ast::Size::S32));
                    if CompilerIntrinsics::is_numeric_type(&target_type) {
                        let evaled_args: Vec<Expr> = args
                            .iter()
                            .map(|a| self.eval_expr(a).unwrap_or_else(|_| a.clone()))
                            .collect();
                        if let Ok(res) = self.fastlang_rand(&target_type, &evaled_args) {
                            *expr = res;
                            return Ok(());
                        }
                    }
                }

                if fn_name == "cast" {
                    if let (Some(target_type), Some(arg)) = (generics.first(), args.first()) {
                        let resolved = if let Expr::Identifier(id) = arg {
                            self.env.get(id).unwrap_or_else(|_| arg.clone())
                        } else {
                            arg.clone()
                        };
                        let target_str = target_type.as_str();
                        let is_generic =
                            matches!(target_type, BaseType::GenericParam(_) | BaseType::Unknown)
                                || (!FastType::from_name(&target_str).is_primitive()
                                    && self.env.get(&target_str).is_err());
                        if !is_generic && is_literal(&resolved) {
                            if let Ok(res) = self.fastlang_cast(&resolved, target_type) {
                                *expr = res;
                                return Ok(());
                            }
                        }
                    }
                }

                // Resolve generic arguments for generic functions only if generics were provided
                if !generics.is_empty() {
                    if let Expr::Identifier(fn_name) = &**callee {
                        let fn_info = self.lookup_fn(fn_name);
                        if let Some((fn_generics, params)) = fn_info {
                            if !fn_generics.is_empty() {
                                let arg_types: Vec<String> =
                                    args.iter().map(|a| self.get_expr_type(a)).collect();
                                let generic_map = resolve_call_generics(
                                    &fn_generics,
                                    &params,
                                    generics,
                                    &arg_types,
                                );
                                let mut full_generics = Vec::new();
                                for g in &fn_generics {
                                    let g_name = match g {
                                        BaseType::GenericParam(n) => n.clone(),
                                        BaseType::New(n) => n.clone(),
                                        _ => g.as_str(),
                                    };
                                    if let Some(concrete) = generic_map.get(&g_name) {
                                        full_generics.push(concrete.clone());
                                    } else {
                                        full_generics.push(g.clone());
                                    }
                                }
                                let all_concrete = full_generics.iter().all(|g| {
                                    !matches!(g, BaseType::GenericParam(_) | BaseType::Unknown)
                                        && !g.as_str().starts_with("...")
                                });
                                if all_concrete {
                                    *generics = full_generics;
                                }
                            }
                        }
                    }
                }

                // Check if this is a split function with compile-time branches
                let split_call_info = if let Expr::Identifier(id) = &**callee {
                    self.split_functions.get(id).cloned()
                } else {
                    None
                };

                if let Some((params, branches)) = split_call_info {
                    let mut eval_args = Vec::new();
                    for a in args.iter() {
                        eval_args.push(self.eval_expr(a).unwrap_or_else(|_| a.clone()));
                    }
                    let mut cond_interp = Interpreter {
                        env: InterpreterEnv::with_parent(self.env.clone()),
                        functions: self.functions.clone(),
                        micros: self.micros.clone(),
                        var_types: self.var_types.clone(),
                        current_env: self.current_env.clone(),
                        fn_overloads: self.fn_overloads.clone(),
                        split_functions: self.split_functions.clone(),
                        in_function_body: true,
                        current_switch_type: None,
                        current_type: self.current_type.clone(),
                        target_backend: self.target_backend.clone(),
                    };
                    for (i, p) in params.iter().enumerate() {
                        if let Some(arg_val) = eval_args.get(i) {
                            cond_interp.env.define(p.name.clone(), arg_val.clone());
                            let arg_t = self.get_expr_type(arg_val);
                            cond_interp.var_types.insert(p.name.clone(), arg_t);
                        }
                    }
                    let mut selected_fn: Option<String> = None;
                    for (cond_opt, br_fn) in branches {
                        if let Some(cond) = cond_opt {
                            if let Ok(Expr::LiteralBool(true)) = cond_interp.eval_expr(&cond) {
                                selected_fn = Some(br_fn);
                                break;
                            }
                        } else {
                            selected_fn = Some(br_fn);
                            break;
                        }
                    }
                    if let Some(chosen_fn) = selected_fn {
                        *callee = Box::new(Expr::Identifier(chosen_fn.clone()));
                        let is_chosen_compilable = chosen_fn.starts_with("@compile::") || {
                            if let Some((_, _, body)) = self.functions.get(&chosen_fn) {
                                crate::middle_end::semantic::analyzer::detect_execution_mode(body)
                                    == ExecutionMode::FullyCompilable
                            } else {
                                false
                            }
                        };
                        if is_chosen_compilable {
                            let _ = self.eval_expr(expr)?;
                        }
                        return Ok(());
                    }
                }

                // Check if called function has a compile-time validation block (@compile { ... }(args))
                let fn_candidates = match &**callee {
                    Expr::Identifier(id) => vec![id.clone()],
                    Expr::NamespaceAccess {
                        namespace,
                        property,
                    } => {
                        if let Expr::Identifier(p) = &**property {
                            vec![format!("{}::{}", namespace, p), p.clone()]
                        } else {
                            vec![]
                        }
                    }
                    Expr::PropertyAccess { object, property } => {
                        let obj_type = self.get_expr_type(object);
                        let clean_obj = self.extract_clean_blueprint_name(&obj_type);
                        vec![format!("{}::{}", clean_obj, property), property.clone()]
                    }
                    _ => vec![],
                };
                let mut found_fn = None;
                for name in &fn_candidates {
                    if let Some(info) = self.functions.get(name).cloned() {
                        found_fn = Some(info);
                        break;
                    }
                }
                if let Some((fn_generics, params, body)) = found_fn {
                    let is_fn_fully_compilable =
                        crate::middle_end::semantic::analyzer::detect_execution_mode(&body)
                            == ExecutionMode::FullyCompilable;
                    if !is_fn_fully_compilable {
                        for stmt in &body {
                            if let Stmt::CompileValidation {
                                body: val_body,
                                args: _,
                            } = stmt
                            {
                                let mut eval_args = Vec::new();
                                for a in args.iter() {
                                    eval_args.push(self.eval_expr(a).unwrap_or_else(|_| a.clone()));
                                }
                                let arg_types: Vec<String> =
                                    eval_args.iter().map(|a| self.fastlang_typeof(a)).collect();
                                let generic_map =
                                    crate::middle_end::semantic::analyzer::resolve_call_generics(
                                        &fn_generics,
                                        &params,
                                        generics,
                                        &arg_types,
                                    );
                                let mut val_interp = Interpreter {
                                    env: InterpreterEnv::with_parent(self.env.clone()),
                                    functions: self.functions.clone(),
                                    micros: self.micros.clone(),
                                    var_types: self.var_types.clone(),
                                    current_env: self.current_env.clone(),
                                    fn_overloads: self.fn_overloads.clone(),
                                    split_functions: self.split_functions.clone(),
                                    in_function_body: true,
                                    current_switch_type: None,
                                    current_type: self.current_type.clone(),
                                    target_backend: self.target_backend.clone(),
                                };
                                for (g_name, g_type) in &generic_map {
                                    val_interp.env.define(
                                        g_name.clone(),
                                        Expr::LiteralString(format!("type<{}>", g_type.as_str())),
                                    );
                                }
                                for (i, p) in params.iter().enumerate() {
                                    if p.is_variadic {
                                        let rest_args = if i < eval_args.len() {
                                            eval_args[i..].to_vec()
                                        } else {
                                            Vec::new()
                                        };
                                        val_interp
                                            .env
                                            .define(p.name.clone(), Expr::ArrayLiteral(rest_args));
                                        val_interp
                                            .var_types
                                            .insert(p.name.clone(), "array".to_string());
                                        break;
                                    }
                                    let arg_val = if let Some(val) = eval_args.get(i) {
                                        Some(val.clone())
                                    } else if let Some(ref def) = p.default_value {
                                        let folded = val_interp
                                            .eval_expr(def)
                                            .unwrap_or_else(|_| def.clone());
                                        Some(folded)
                                    } else {
                                        None
                                    };
                                    if let Some(val) = arg_val {
                                        val_interp.env.define(p.name.clone(), val.clone());
                                        let arg_t = val_interp.get_expr_type(&val);
                                        val_interp.var_types.insert(p.name.clone(), arg_t);
                                    }
                                }
                                for s in val_body {
                                    val_interp.eval_stmt(s)?;
                                }
                            }
                        }
                    }
                }

                // Evaluate via interpreter if compile-time function
                let is_compilable = match &**callee {
                    Expr::NamespaceAccess {
                        namespace,
                        property,
                    } if namespace == "@compile" => {
                        if let Expr::Identifier(p) = &**property {
                            if p == "print" {
                                !self.in_function_body
                            } else {
                                true
                            }
                        } else {
                            true
                        }
                    }
                    Expr::Identifier(id) => {
                        if id == "@compile::print" {
                            !self.in_function_body
                        } else if id.starts_with("@compile::") {
                            true
                        } else if let Some((_, _, body)) = self.functions.get(id) {
                            crate::middle_end::semantic::analyzer::detect_execution_mode(body)
                                == ExecutionMode::FullyCompilable
                        } else {
                            false
                        }
                    }
                    _ => false,
                };

                if is_compilable {
                    match self.eval_expr(expr) {
                        Ok(res) => {
                            if is_literal(&res)
                                || matches!(
                                    &res,
                                    Expr::UnaryOp { .. }
                                        | Expr::PropertyAccess { .. }
                                        | Expr::LiteralVoid
                                )
                            {
                                *expr = res;
                            }
                        }
                        Err(e) => {
                            if e.contains("Compile Error") {
                                return Err(e);
                            }
                        }
                    }
                }
            }
            Expr::QuestionMark(inner) => {
                self.fold_expr(inner)?;
            }
            Expr::IfExpr {
                condition,
                then_branch,
                else_branch,
            } => {
                self.fold_expr(condition)?;
                if let Expr::LiteralBool(b) = **condition {
                    if b {
                        self.fold_expr(then_branch)?;
                        *expr = (**then_branch).clone();
                    } else {
                        self.fold_expr(else_branch)?;
                        *expr = (**else_branch).clone();
                    }
                    return Ok(());
                } else {
                    self.fold_expr(then_branch)?;
                    self.fold_expr(else_branch)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}
