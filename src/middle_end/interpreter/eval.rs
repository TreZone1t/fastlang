use crate::frontend::parser::ast::*;
use crate::middle_end::interpreter::env::InterpreterEnv;
use crate::middle_end::interpreter::fast_type::FastType;
use crate::middle_end::interpreter::intrinsics::CompilerIntrinsics;
use crate::middle_end::semantic::analyzer::resolve_call_generics;
use crate::middle_end::semantic::environment::{Environment, FnSignature, SymbolKind};
use colored::Colorize;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub fn is_literal(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::LiteralInt(_)
            | Expr::LiteralUInt(_)
            | Expr::LiteralFloat(_)
            | Expr::LiteralBool(_)
            | Expr::LiteralChar(_)
            | Expr::LiteralString(_)
            | Expr::LiteralVoid
    )
}

pub fn is_known_type_name(s: &str) -> bool {
    matches!(
        s,
        "int8"
            | "int16"
            | "int32"
            | "int"
            | "int64"
            | "int128"
            | "uint8"
            | "uint16"
            | "uint32"
            | "uint"
            | "uint64"
            | "uint128"
            | "usize"
            | "isize"
            | "float32"
            | "float"
            | "float64"
            | "float128"
            | "char"
            | "str"
            | "bool"
            | "void"
            | "type"
    )
}

#[inline]
pub fn strip_generic_wrapper<'a>(s: &'a str, prefix: &str) -> &'a str {
    if let Some(rest) = s.strip_prefix(prefix) {
        if let Some(inner) = rest.strip_suffix('>') {
            return inner;
        }
    }
    s
}

#[inline]
pub fn strip_type_wrapper(s: &str) -> &str {
    strip_generic_wrapper(s, "type<")
}

pub fn is_typeof_callee(callee: &Expr) -> bool {
    match callee {
        Expr::Identifier(id) => id == "typeof",
        Expr::NamespaceAccess {
            namespace,
            property,
        } if namespace == "@compile" => match &**property {
            Expr::Identifier(id) => id == "typeof",
            _ => false,
        },
        _ => false,
    }
}

pub fn has_compile_operations(stmts: &[Stmt]) -> bool {
    fn check_expr(e: &Expr) -> bool {
        match e {
            Expr::NamespaceAccess { namespace, .. } if namespace == "@compile" => true,
            Expr::Call { callee, args, .. } => {
                if check_expr(callee) {
                    return true;
                }
                args.iter().any(check_expr)
            }
            Expr::BinaryOp { left, right, .. } => check_expr(left) || check_expr(right),
            Expr::UnaryOp { operand, .. } => check_expr(operand),
            Expr::Cast { expr, .. } => check_expr(expr),
            Expr::PropertyAccess { object, .. } => check_expr(object),
            Expr::ArrayLiteral(elements) => elements.iter().any(check_expr),
            _ => false,
        }
    }

    fn check_stmt(s: &Stmt) -> bool {
        match s {
            Stmt::CompileValidation { .. } => true,
            Stmt::Declaration(Decl::CompileDecl { .. }) => true,
            Stmt::CallStmt(e) | Stmt::ExpressionStmt(e) => check_expr(e),
            Stmt::ReturnStmt(Some(e)) => check_expr(e),
            Stmt::ReassignStmt { value, .. } => check_expr(value),
            Stmt::IfStmt {
                condition,
                then_block,
                else_block,
            } => {
                check_expr(condition)
                    || then_block.iter().any(check_stmt)
                    || else_block
                        .as_ref()
                        .map_or(false, |eb| eb.iter().any(check_stmt))
            }
            Stmt::WhileStmt {
                condition, body, ..
            } => {
                check_expr(condition)
                    || match body {
                        EitherBlock::Inline(stmts) => stmts.iter().any(check_stmt),
                        EitherBlock::External(e) => check_expr(e),
                    }
            }
            Stmt::LoopStmt { count, body } => {
                count.as_ref().map_or(false, check_expr)
                    || match body {
                        EitherBlock::Inline(stmts) => stmts.iter().any(check_stmt),
                        EitherBlock::External(e) => check_expr(e),
                    }
            }
            Stmt::ForStmt { body, .. } => match body {
                EitherBlock::Inline(stmts) => stmts.iter().any(check_stmt),
                EitherBlock::External(e) => check_expr(e),
            },
            Stmt::Block(stmts) | Stmt::ThisBlock(stmts) => stmts.iter().any(check_stmt),
            _ => false,
        }
    }

    stmts.iter().any(check_stmt)
}

pub struct Interpreter {
    pub env: InterpreterEnv,
    pub functions: HashMap<String, (Vec<BaseType>, Vec<Param>, Vec<Stmt>)>,
    pub var_types: HashMap<String, String>,
    pub current_env: Option<Rc<RefCell<Environment>>>,
    pub fn_overloads: HashMap<String, Vec<FnSignature>>,
}

impl Interpreter {
    pub fn new() -> Self {
        Self {
            env: InterpreterEnv::new(),
            functions: HashMap::new(),
            var_types: HashMap::new(),
            current_env: None,
            fn_overloads: HashMap::new(),
        }
    }

    pub fn get_fast_type(&self, name: &str) -> FastType {
        let clean = strip_type_wrapper(name);
        let base_name = if clean.contains('<') {
            clean.split('<').next().unwrap_or(clean)
        } else {
            clean
        };
        if let Some(ref env_rc) = self.current_env {
            if let Some(bp) = env_rc.borrow().lookup_blueprint(base_name) {
                let has_display = bp.handles.iter().any(|h| h.as_str() == "display");
                let has_copy = bp.handles.iter().any(|h| h.as_str() == "copy" || matches!(h, HandleMethods::Copy));
                let has_cast = bp.handles.iter().any(|h| h.as_str() == "cast" || matches!(h, HandleMethods::Cast));
                let has_throw = bp.handles.iter().any(|h| h.as_str() == "throw" || h.as_str() == "$throw" || matches!(h, HandleMethods::Throw));
                let has_default = bp.handles.iter().any(|h| h.as_str() == "default" || matches!(h, HandleMethods::Default));
                return FastType::with_all_handles(
                    BaseType::from_str(clean),
                    has_display,
                    has_copy,
                    has_cast,
                    has_throw,
                    has_default,
                );
            }
        }
        FastType::from_name(clean)
    }

    pub fn default_for_type(&self, name: &str) -> Option<Expr> {
        let clean = strip_type_wrapper(name);
        match clean {
            "int8" | "int16" | "int32" | "int64" | "int" | "uint8" | "uint16" | "uint32"
            | "uint64" | "uint" | "usize" | "isize" => Some(Expr::LiteralInt(0)),
            "float32" | "float64" | "float" => Some(Expr::LiteralFloat(0.0)),
            "bool" => Some(Expr::LiteralBool(false)),
            "char" => Some(Expr::LiteralChar('\0')),
            "str" => Some(Expr::LiteralString("".to_string())),
            _ => {
                let base_name = if clean.contains('<') {
                    clean.split('<').next().unwrap_or(clean)
                } else {
                    clean
                };
                if let Some(ref env_rc) = self.current_env {
                    if let Some(bp) = env_rc.borrow().lookup_blueprint(base_name) {
                        let has_default = bp.handles.iter().any(|h| h.as_str() == "default" || matches!(h, HandleMethods::Default));
                        if has_default || bp.is_class {
                            return Some(Expr::Default(Some(BaseType::from_str(clean))));
                        }
                    }
                }
                None
            }
        }
    }

    pub fn eval_macro(
        &mut self,
        body: &[Stmt],
        params: &[Param],
        args: &[Expr],
    ) -> Result<(), String> {
        // Bind args to params
        for (i, param) in params.iter().enumerate() {
            if param.is_variadic {
                let rest: Vec<Expr> = args[i..].to_vec();
                self.env
                    .define(param.name.clone(), Expr::ArrayLiteral(rest));
                break;
            }
            if let Some(arg) = args.get(i) {
                // Try to evaluate the argument (simple resolution)
                let val = self.eval_expr(arg).unwrap_or_else(|_| arg.clone());
                self.env.define(param.name.clone(), val);
            }
        }

        for stmt in body {
            self.eval_stmt(stmt)?;
        }
        Ok(())
    }

    pub fn eval_method(
        &mut self,
        body: &[Stmt],
        params: &[Param],
        args: &[Expr],
    ) -> Result<Option<Expr>, String> {
        // Bind args to params
        for (i, param) in params.iter().enumerate() {
            if param.is_variadic {
                let rest: Vec<Expr> = args[i..].to_vec();
                self.env
                    .define(param.name.clone(), Expr::ArrayLiteral(rest));
                break;
            }
            if let Some(arg) = args.get(i) {
                let val = self.eval_expr(arg).unwrap_or_else(|_| arg.clone());
                self.env.define(param.name.clone(), val);
            }
        }

        for stmt in body {
            if let Some(ret) = self.eval_stmt(stmt)? {
                return Ok(Some(ret));
            }
        }
        Ok(None)
    }

    pub fn eval_stmt(&mut self, stmt: &Stmt) -> Result<Option<Expr>, String> {
        match stmt {
            Stmt::CompileValidation { body, args } => {
                self.env.push_scope();
                for (i, arg) in args.iter().enumerate() {
                    let val = self.eval_expr(arg).unwrap_or_else(|_| arg.clone());
                    if let Expr::Identifier(id) = arg {
                        self.env.define(id.clone(), val.clone());
                    }
                    self.env.define(format!("arg_{}", i), val);
                }
                let mut res = Ok(None);
                for s in body {
                    if let Err(e) = self.eval_stmt(s) {
                        res = Err(e);
                        break;
                    }
                }
                self.env.pop_scope();
                res
            }
            Stmt::ForInStmt {
                item,
                iterable,
                body,
            } => {
                let iter_val = self.eval_expr(iterable)?;
                let elements = match iter_val {
                    Expr::ArrayLiteral(elems) => elems,
                    Expr::Identifier(id) => {
                        if let Ok(Expr::ArrayLiteral(elems)) = self.env.get(&id) {
                            elems
                        } else {
                            Vec::new()
                        }
                    }
                    _ => Vec::new(),
                };

                let var_name = match &**item {
                    Stmt::Declaration(Decl::VarDecl { name, .. }) => Some(name.clone()),
                    Stmt::ExpressionStmt(Expr::Identifier(name)) => Some(name.clone()),
                    _ => None,
                };

                if let Some(var_name) = var_name {
                    for elem in elements {
                        self.env.define(var_name.clone(), elem);
                        match body {
                            EitherBlock::Inline(stmts) => {
                                for s in stmts {
                                    if let Some(ret) = self.eval_stmt(s)? {
                                        return Ok(Some(ret));
                                    }
                                }
                            }
                            EitherBlock::External(expr) => {
                                self.eval_expr(expr)?;
                            }
                        }
                    }
                }
                Ok(None)
            }
            Stmt::ThrowStmt(expr) => {
                let val = self.eval_expr(expr)?;
                let msg = self.extract_string_from_expr(&val);
                self.fastlang_throw(&msg)?;
                Ok(None)
            }
            Stmt::ReturnStmt(opt_expr) => {
                let val = if let Some(expr) = opt_expr {
                    self.eval_expr(expr)?
                } else {
                    Expr::LiteralVoid
                };
                Ok(Some(val))
            }
            Stmt::Declaration(Decl::VarDecl { name, value, .. }) => {
                let val = self.eval_expr(value)?;
                self.env.define(name.clone(), val);
                Ok(None)
            }
            Stmt::ReassignStmt { target, value, .. } => {
                let val = self.eval_expr(value)?;
                if let Expr::Identifier(name) = target {
                    self.env.assign(&name, val)?;
                }
                Ok(None)
            }
            Stmt::IfStmt {
                condition,
                then_block,
                else_block,
            } => {
                let cond_val = self.eval_expr(condition)?;
                let is_true = match cond_val {
                    Expr::LiteralBool(b) => b,
                    _ => false, // Simplistic
                };

                if is_true {
                    for s in then_block {
                        if let Some(ret) = self.eval_stmt(s)? {
                            return Ok(Some(ret));
                        }
                    }
                } else if let Some(else_branch_stmts) = else_block {
                    for s in else_branch_stmts {
                        if let Some(ret) = self.eval_stmt(s)? {
                            return Ok(Some(ret));
                        }
                    }
                }
                Ok(None)
            }
            Stmt::CallStmt(expr) | Stmt::ExpressionStmt(expr) => {
                self.eval_expr(expr)?;
                Ok(None)
            }
            _ => Ok(None), // Ignore other statements for now
        }
    }

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
                    BaseType::Name(n) => n.as_str(),
                    BaseType::New(n) => n.clone(),
                    _ => target_type.as_str(),
                };
                let resolved_target = if let Ok(Expr::LiteralString(s)) = self.env.get(&target_name) {
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
            Expr::PropertyAccess { object, property } => {
                let obj_val = self.eval_expr(object).unwrap_or_else(|_| *object.clone());
                let type_name = match obj_val {
                    Expr::LiteralString(s) => strip_type_wrapper(&s).to_string(),
                    Expr::Identifier(id) => {
                        if let Ok(Expr::LiteralString(s)) = self.env.get(&id) {
                            strip_type_wrapper(&s).to_string()
                        } else {
                            id
                        }
                    }
                    _ => "".to_string(),
                };
                let ft = self.get_fast_type(&type_name);
                match property.as_str() {
                    "is_primitive" => Ok(Expr::LiteralBool(ft.is_primitive())),
                    "is_pointer" => Ok(Expr::LiteralBool(ft.is_pointer())),
                    "is_array" => Ok(Expr::LiteralBool(ft.is_array())),
                    "printable" | "is_printable" => Ok(Expr::LiteralBool(ft.printable())),
                    "throwable" | "is_throwable" => Ok(Expr::LiteralBool(ft.throwable())),
                    "copyable" | "is_copyable" => Ok(Expr::LiteralBool(ft.copyable())),
                    "castable" | "is_castable" => Ok(Expr::LiteralBool(ft.castable())),
                    "as_str" => Ok(Expr::LiteralString(ft.as_str())),
                    "size" => Ok(Expr::LiteralInt(ft.size() as i128)),
                    "default" => {
                        if let Some(def) = self.default_for_type(&type_name) {
                            Ok(def)
                        } else {
                            Ok(Expr::Default(Some(BaseType::from_str(&type_name))))
                        }
                    }
                    _ => Ok(Expr::LiteralUndefined),
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
                    (Expr::LiteralChar(c), "+", Expr::LiteralString(b)) => {
                        let mut s = c.to_string();
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
                    let type_name = match &**object {
                        Expr::Identifier(id) => {
                            if let Ok(Expr::LiteralString(s)) = self.env.get(id) {
                                strip_type_wrapper(&s).to_string()
                            } else {
                                id.clone()
                            }
                        }
                        Expr::LiteralString(s) => strip_type_wrapper(s).to_string(),
                        _ => "".to_string(),
                    };
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
                        "copyable" | "is_copyable" => return Ok(Expr::LiteralBool(ft.copyable())),
                        "castable" | "is_castable" => return Ok(Expr::LiteralBool(ft.castable())),
                        "as_str" => return Ok(Expr::LiteralString(ft.as_str())),
                        "size" => return Ok(Expr::LiteralInt(ft.size() as i128)),
                        "default" => {
                            if let Some(def) = self.default_for_type(&type_name) {
                                return Ok(def);
                            } else {
                                return Ok(Expr::Default(Some(BaseType::from_str(&type_name))));
                            }
                        }
                        "castable_to" => {
                            if let Some(target_expr) = args.first() {
                                let evaled = self.eval_expr(target_expr).unwrap_or_else(|_| target_expr.clone());
                                let target_name = match evaled {
                                    Expr::LiteralString(s) => strip_type_wrapper(&s).to_string(),
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
                        _ => {}
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

                if target_fn_name.starts_with("@compile::")
                    && !self.functions.contains_key(&target_fn_name)
                {
                    let clean = target_fn_name.trim_start_matches("@compile::");
                    if !CompilerIntrinsics::is_compile_member(clean) {
                        eprintln!("STACKTRACE:\n{:?}", std::backtrace::Backtrace::force_capture());
                        return self.fastlang_throw(&format!(
                            "Unknown compile intrinsic '{}'",
                            target_fn_name
                        ));
                    }
                }

                let is_compilable = target_fn_name.starts_with("@compile::")
                    || {
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
                            var_types: self.var_types.clone(),
                            current_env: self.current_env.clone(),
                            fn_overloads: self.fn_overloads.clone(),
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
                            if let Some(arg_val) = eval_args.get(i) {
                                func_interp.env.define(param.name.clone(), arg_val.clone());
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
        match expr {
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
            _ => "Compile-time Error".to_string(),
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
                _ => Err(format!("Compile Error: Cannot cast {:?} to uint", resolved)),
            },
            BaseType::Float(_) => match resolved {
                Expr::LiteralFloat(f) => Ok(Expr::LiteralFloat(f)),
                Expr::LiteralInt(i) => Ok(Expr::LiteralFloat(i as f64)),
                Expr::LiteralUInt(u) => Ok(Expr::LiteralFloat(u as f64)),
                Expr::LiteralBool(b) => Ok(Expr::LiteralFloat(if b { 1.0 } else { 0.0 })),
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
                _ => Err(format!("Compile Error: Cannot cast {:?} to bool", resolved)),
            },
            BaseType::Char => match resolved {
                Expr::LiteralChar(c) => Ok(Expr::LiteralChar(c)),
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
            BaseType::Str => match resolved {
                Expr::LiteralString(s) => Ok(Expr::LiteralString(s)),
                Expr::LiteralInt(i) => Ok(Expr::LiteralString(i.to_string())),
                Expr::LiteralUInt(u) => Ok(Expr::LiteralString(u.to_string())),
                Expr::LiteralFloat(f) => Ok(Expr::LiteralString(f.to_string())),
                Expr::LiteralBool(b) => Ok(Expr::LiteralString(b.to_string())),
                Expr::LiteralChar(c) => Ok(Expr::LiteralString(c.to_string())),
                _ => Err(format!("Compile Error: Cannot cast {:?} to str", resolved)),
            },
            _ => Err(format!(
                "Compile Error: Compile-time cast to '{}' is not supported",
                target_type.as_str()
            )),
        }
    }

    pub fn load_from_program(
        &mut self,
        main_ast: &[Stmt],
        modules: &[crate::loader::LoadedModule],
    ) {
        let mut all_stmts = Vec::new();
        for m in modules {
            all_stmts.extend(m.ast.clone());
        }
        all_stmts.extend(main_ast.to_vec());

        for s in all_stmts {
            if let Stmt::Declaration(decl) = s {
                match decl {
                    Decl::FnDecl {
                        name,
                        generics,
                        params,
                        body,
                        ..
                    } => {
                        self.functions.insert(name, (generics, params, body));
                    }
                    Decl::ImplDecl {
                        methods,
                        handle_block,
                        ..
                    } => {
                        for m in methods {
                            if let Decl::FnDecl {
                                name,
                                generics,
                                params,
                                body,
                                ..
                            } = m
                            {
                                self.functions.insert(name, (generics, params, body));
                            }
                        }
                        for h in handle_block {
                            if let Decl::FnDecl {
                                name,
                                generics,
                                params,
                                body,
                                ..
                            } = h
                            {
                                self.functions.insert(name, (generics, params, body));
                            }
                        }
                    }
                    Decl::CompileDecl { name, decls } => {
                        for d in decls {
                            match d {
                                Decl::FnDecl {
                                    name: fn_name,
                                    generics,
                                    params,
                                    body,
                                    ..
                                } => {
                                    self.functions.insert(
                                        format!("{}::{}", name, fn_name),
                                        (generics.clone(), params.clone(), body.clone()),
                                    );
                                }
                                Decl::MacroDecl {
                                    name: m_name,
                                    params,
                                    body,
                                    ..
                                } => {
                                    let clean_m = m_name.trim_start_matches('$');
                                    self.functions.insert(
                                        format!("{}::{}", name, m_name),
                                        (vec![], params.clone(), body.clone()),
                                    );
                                    self.functions.insert(
                                        format!("{}::{}", name, clean_m),
                                        (vec![], params.clone(), body.clone()),
                                    );
                                    self.functions.insert(
                                        m_name.clone(),
                                        (vec![], params.clone(), body.clone()),
                                    );
                                    self.functions.insert(
                                        clean_m.to_string(),
                                        (vec![], params.clone(), body.clone()),
                                    );
                                }
                                Decl::MicroDecl {
                                    name: m_name,
                                    params,
                                    body,
                                    generics,
                                    ..
                                } => {
                                    let gen_types: Vec<BaseType> = generics
                                        .as_ref()
                                        .map(|v| {
                                            v.iter().map(|s| BaseType::from_str(s)).collect()
                                        })
                                        .unwrap_or_default();
                                    self.functions.insert(
                                        format!("{}::{}", name, m_name),
                                        (gen_types.clone(), params.clone(), body.clone()),
                                    );
                                    self.functions.insert(
                                        m_name.clone(),
                                        (gen_types, params, body),
                                    );
                                }
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    pub fn load_functions_from_env(&mut self, env: &Environment) {
        for (name, info) in &env.symbols {
            if let SymbolKind::Function {
                generics,
                params,
                body: Some(body),
                ..
            } = &info.kind
            {
                self.functions.entry(name.clone()).or_insert_with(|| {
                    (generics.clone(), params.clone(), body.clone())
                });
            } else if let SymbolKind::Macro {
                params,
                body,
                ..
            } = &info.kind
            {
                self.functions.entry(name.clone()).or_insert_with(|| {
                    (vec![], params.clone(), body.clone())
                });
            }
        }
        if let Some(ref parent) = env.parent {
            self.load_functions_from_env(&parent.borrow());
        }
    }

    pub fn resolve_type_str(&self, ty: &str) -> String {
        let mut current = ty.to_string();
        for _ in 0..10 {
            if let Some(ref env_rc) = self.current_env {
                if let Some(info) = env_rc.borrow().lookup(&current) {
                    if let SymbolKind::Variable { type_node, .. } = &info.kind {
                        if let BaseType::Type(inner) = type_node {
                            current = inner.as_str();
                            continue;
                        }
                    }
                }
            }
            break;
        }
        current
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
                    || (id.contains('<') && id.ends_with('>'))
                    || (self.current_env.is_some()
                        && {
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
            Expr::LiteralBool(_) => "bool".to_string(),
            Expr::LiteralChar(_) => "char".to_string(),
            Expr::LiteralVoid => "void".to_string(),
            Expr::Identifier(id) => {
                let ty = if is_known_type_name(id) {
                    id.clone()
                } else if let Some(t) = self.var_types.get(id) {
                    t.clone()
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
            Expr::ArrayLiteral(_) => "array".to_string(),
            _ => "unknown".to_string(),
        };
        self.resolve_type_str(&raw)
    }

    pub fn lookup_fn(&self, name: &str) -> Option<(Vec<BaseType>, Vec<Param>)> {
        if let Some(sigs) = self.fn_overloads.get(name) {
            if let Some(sig) = sigs.iter().find(|s| !s.generics.is_empty()) {
                return Some((sig.generics.clone(), sig.params.clone()));
            }
            if let Some(sig) = sigs.first() {
                return Some((sig.generics.clone(), sig.params.clone()));
            }
        }
        if let Some(ref env_rc) = self.current_env {
            if let Some(info) = env_rc.borrow().lookup(name) {
                if let SymbolKind::Function {
                    generics, params, ..
                } = &info.kind
                {
                    return Some((generics.clone(), params.clone()));
                }
            }
        }
        if let Some((generics, params, _)) = self.functions.get(name) {
            return Some((generics.clone(), params.clone()));
        }
        None
    }

    pub fn evaluate_ast(&mut self, stmts: &mut Vec<Stmt>) -> Result<(), String> {
        let mut new_stmts = Vec::with_capacity(stmts.len());
        for mut stmt in stmts.drain(..) {
            match &mut stmt {
                Stmt::CompileValidation { body: _, args } => {
                    for a in args.iter_mut() {
                        self.fold_expr(a)?;
                    }
                    self.eval_stmt(&stmt)?;
                    // Compile-time validation block succeeded! Strip from AST so C++ codegen never sees it.
                    continue;
                }
                Stmt::Declaration(decl) => {
                    self.evaluate_decl(decl)?;
                    new_stmts.push(stmt);
                }
                Stmt::ReassignStmt { target, value, .. } => {
                    self.fold_expr(value)?;
                    if let Expr::Identifier(id) = target {
                        if is_literal(value) {
                            self.env.define(id.clone(), value.clone());
                        } else {
                            self.env.remove(id);
                        }
                    }
                    new_stmts.push(stmt);
                }
                Stmt::ReturnStmt(opt_expr) => {
                    if let Some(e) = opt_expr {
                        self.fold_expr(e)?;
                    }
                    new_stmts.push(stmt);
                }
                Stmt::IfStmt {
                    condition,
                    then_block,
                    else_block,
                } => {
                    self.fold_expr(condition)?;
                    if let Expr::LiteralBool(b) = *condition {
                        if b {
                            self.evaluate_ast(then_block)?;
                            new_stmts.append(then_block);
                        } else if let Some(eb) = else_block {
                            self.evaluate_ast(eb)?;
                            new_stmts.append(eb);
                        }
                    } else {
                        // Dynamic condition!
                        if has_compile_operations(then_block)
                            || else_block
                                .as_ref()
                                .map_or(false, |eb| has_compile_operations(eb))
                        {
                            return Err("Compile Error: Dynamic conditions cannot contain @compile blocks or @compile::throw".to_string());
                        }
                        self.evaluate_ast(then_block)?;
                        if let Some(eb) = else_block {
                            self.evaluate_ast(eb)?;
                        }
                        new_stmts.push(stmt);
                    }
                }
                Stmt::WhileStmt {
                    condition, body, ..
                } => {
                    self.fold_expr(condition)?;
                    self.evaluate_either_block(body)?;
                    new_stmts.push(stmt);
                }
                Stmt::LoopStmt { count, body } => {
                    if let Some(c) = count {
                        self.fold_expr(c)?;
                    }
                    self.evaluate_either_block(body)?;
                    new_stmts.push(stmt);
                }
                Stmt::ForStmt { body, .. } => {
                    self.evaluate_either_block(body)?;
                    new_stmts.push(stmt);
                }
                Stmt::Block(inner) | Stmt::ThisBlock(inner) => {
                    self.env.push_scope();
                    self.evaluate_ast(inner)?;
                    self.env.pop_scope();
                    new_stmts.push(stmt);
                }
                Stmt::CallStmt(e) | Stmt::ExpressionStmt(e) => {
                    self.fold_expr(e)?;
                    new_stmts.push(stmt);
                }
                _ => {
                    new_stmts.push(stmt);
                }
            }
        }
        *stmts = new_stmts;
        Ok(())
    }

    pub fn evaluate_decl(&mut self, decl: &mut Decl) -> Result<(), String> {
        match decl {
            Decl::VarDecl {
                name,
                value,
                type_node,
                ..
            } => {
                self.fold_expr(value)?;
                let ty_name = self.resolve_type_str(&type_node.get_name());
                self.var_types.insert(name.clone(), ty_name.clone());
                if is_literal(value) {
                    self.env.define(name.clone(), value.clone());
                } else if ty_name == "type"
                    || ty_name.starts_with("type<")
                    || type_node.as_str() == "type"
                {
                    if let Expr::LiteralString(ref s) = value {
                        if s.starts_with("type<") {
                            self.env.define(name.clone(), value.clone());
                        }
                    }
                }
            }
            Decl::ArrayDecl {
                name, type_node, ..
            } => {
                self.var_types
                    .insert(name.clone(), format!("{}[]", type_node.get_name()));
            }
            Decl::FnDecl {
                name,
                generics,
                params,
                body,
                ..
            } => {
                self.functions.insert(name.clone(), (generics.clone(), params.clone(), body.clone()));
                if generics.is_empty() {
                    self.evaluate_ast(body)?;
                }
            }
            Decl::ImplDecl {
                methods,
                handle_block,
                ..
            } => {
                for m in methods {
                    if let Decl::FnDecl { name, generics, params, body, .. } = m {
                        self.functions.insert(name.clone(), (generics.clone(), params.clone(), body.clone()));
                    }
                }
                for h in handle_block {
                    if let Decl::FnDecl { name, generics, params, body, .. } = h {
                        self.functions.insert(name.clone(), (generics.clone(), params.clone(), body.clone()));
                    }
                }
            }
            Decl::CompileDecl { name, decls } => {
                for d in decls {
                    self.evaluate_decl(d)?;
                    match d {
                        Decl::FnDecl { name: fn_name, generics, params, body, .. } => {
                            self.functions.insert(format!("{}::{}", name, fn_name), (generics.clone(), params.clone(), body.clone()));
                            self.functions.insert(fn_name.clone(), (generics.clone(), params.clone(), body.clone()));
                        }
                        Decl::MacroDecl { name: m_name, params, body, .. } => {
                            let clean_m = m_name.trim_start_matches('$');
                            self.functions.insert(format!("{}::{}", name, m_name), (vec![], params.clone(), body.clone()));
                            self.functions.insert(format!("{}::{}", name, clean_m), (vec![], params.clone(), body.clone()));
                            self.functions.insert(m_name.clone(), (vec![], params.clone(), body.clone()));
                            self.functions.insert(clean_m.to_string(), (vec![], params.clone(), body.clone()));
                        }
                        Decl::MicroDecl { name: m_name, params, body, generics, .. } => {
                            let gen_types: Vec<BaseType> = generics.as_ref().map(|v| v.iter().map(|s| BaseType::from_str(s)).collect()).unwrap_or_default();
                            self.functions.insert(format!("{}::{}", name, m_name), (gen_types.clone(), params.clone(), body.clone()));
                            self.functions.insert(m_name.clone(), (gen_types, params.clone(), body.clone()));
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub fn evaluate_either_block(&mut self, block: &mut EitherBlock) -> Result<(), String> {
        match block {
            EitherBlock::Inline(stmts) => self.evaluate_ast(stmts),
            EitherBlock::External(expr) => self.fold_expr(expr),
        }
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
            Expr::Cast {
                expr: inner,
                target_type,
            } => {
                self.fold_expr(inner)?;
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
                let obj_type_name = self.extract_type_string(object);
                if let Some(t_name) = obj_type_name {
                    let ft = self.get_fast_type(&t_name);
                    match property.as_str() {
                        "is_primitive" => *expr = Expr::LiteralBool(ft.is_primitive()),
                        "is_pointer" => *expr = Expr::LiteralBool(ft.is_pointer()),
                        "is_array" => *expr = Expr::LiteralBool(ft.is_array()),
                        "printable" | "is_printable" => *expr = Expr::LiteralBool(ft.printable()),
                        "throwable" | "is_throwable" => *expr = Expr::LiteralBool(ft.throwable()),
                        "copyable" | "is_copyable" => *expr = Expr::LiteralBool(ft.copyable()),
                        "castable" | "is_castable" => *expr = Expr::LiteralBool(ft.castable()),
                        "size" => *expr = Expr::LiteralInt(ft.size() as i128),
                        "as_str" => *expr = Expr::LiteralString(ft.as_str()),
                        "default" => {
                            if let Some(def) = self.default_for_type(&t_name) {
                                *expr = def;
                            } else {
                                *expr = Expr::Default(Some(BaseType::from_str(&t_name)));
                            }
                        }
                        _ => {}
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
                            _ => {}
                        }
                    }
                }

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

                if fn_name == "print"
                    && matches!(&**callee, Expr::NamespaceAccess { namespace, .. } if namespace == "@compile")
                {
                    let mut parts = Vec::new();
                    for arg in args.iter() {
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
                    *expr = Expr::LiteralVoid;
                    return Ok(());
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
                        let is_generic = matches!(
                            target_type,
                            BaseType::GenericParam(_) | BaseType::Unknown
                        ) || (!FastType::from_name(&target_str).is_primitive()
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

                // Evaluate via interpreter if compile-time function
                let is_compilable = match &**callee {
                    Expr::NamespaceAccess { namespace, .. } if namespace == "@compile" => true,
                    Expr::Identifier(id) => {
                        id.starts_with("@compile::")
                            || {
                                if let Some((_, _, body)) = self.functions.get(id) {
                                    crate::middle_end::semantic::analyzer::detect_execution_mode(
                                        body,
                                    ) == ExecutionMode::FullyCompilable
                                } else {
                                    false
                                }
                            }
                    }
                    _ => false,
                };

                if is_compilable {
                    match self.eval_expr(expr) {
                        Ok(res) => {
                            if is_literal(&res) {
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
            _ => {}
        }
        Ok(())
    }
}
