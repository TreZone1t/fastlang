use crate::frontend::parser::ast::*;
use crate::middle_end::interpreter::env::InterpreterEnv;

pub struct Interpreter {
    pub env: InterpreterEnv,
}

impl Interpreter {
    pub fn new() -> Self {
        Self {
            env: InterpreterEnv::new(),
        }
    }

    pub fn eval_macro(&mut self, body: &[Stmt], params: &[Param], args: &[Expr]) -> Result<(), String> {
        // Bind args to params
        for (i, param) in params.iter().enumerate() {
            if let Some(arg) = args.get(i) {
                // Try to evaluate the argument (simple resolution)
                let val = self.eval_expr(arg).unwrap_or(arg.clone());
                self.env.define(param.name.clone(), val);
            }
        }
        
        for stmt in body {
            self.eval_stmt(stmt)?;
        }
        Ok(())
    }

    pub fn eval_stmt(&mut self, stmt: &Stmt) -> Result<(), String> {
        match stmt {
            Stmt::ThrowStmt(expr) => {
                let val = self.eval_expr(expr)?;
                let msg = self.extract_string_from_expr(&val);
                Err(format!("Compile Error: {}", msg))
            }
            Stmt::Declaration(Decl::VarDecl { name, value, .. }) => {
                let val = self.eval_expr(value)?;
                self.env.define(name.clone(), val);
                Ok(())
            }
            Stmt::ReassignStmt { target, value, .. } => {
                let val = self.eval_expr(value)?;
                if let Expr::Identifier(name) = target {
                    self.env.assign(&name, val)?;
                }
                Ok(())
            }
            Stmt::IfStmt { condition, then_block, else_block } => {
                let cond_val = self.eval_expr(condition)?;
                let is_true = match cond_val {
                    Expr::LiteralBool(b) => b,
                    _ => false, // Simplistic
                };
                
                if is_true {
                    for s in then_block {
                        self.eval_stmt(s)?;
                    }
                } else if let Some(else_branch_stmts) = else_block {
                    for s in else_branch_stmts {
                        self.eval_stmt(s)?;
                    }
                }
                Ok(())
            }
            Stmt::CallStmt(expr) | Stmt::ExpressionStmt(expr) => {
                self.eval_expr(expr)?;
                Ok(())
            }
            _ => Ok(()) // Ignore other statements for now
        }
    }

    pub fn eval_expr(&mut self, expr: &Expr) -> Result<Expr, String> {
        match expr {
            Expr::LiteralInt(_) | Expr::LiteralFloat(_) | Expr::LiteralString(_) 
            | Expr::LiteralBool(_) | Expr::LiteralChar(_) | Expr::LiteralVoid | Expr::LiteralUndefined => {
                Ok(expr.clone())
            }
            Expr::Identifier(name) => {
                self.env.get(name)
            }
            Expr::SizeOf { .. } => {
                // simple sizeof mock
                Ok(Expr::LiteralInt(8))
            }
            Expr::TypeOf { .. } => {
                // simple typeof mock
                Ok(Expr::LiteralString("unknown".to_string()))
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
            Expr::Call { callee, args } => {
                if let Expr::NamespaceAccess { namespace, property } = &**callee {
                    if namespace == "@compile" {
                        if let Expr::Identifier(prop_name) = &**property {
                            if prop_name == "throw" { // todo : remove this hard-coded sh*t code
                                let error_msg = if let Some(arg) = args.first() {
                                    self.extract_string_from_expr(arg)
                                } else {
                                    "Compile error thrown".to_string()
                                };
                                return Err(error_msg);
                            }
                        }
                    }
                }
                // Basic mock for compile-time function evaluation 
                // Currently just returning undefined, will be expanded later
                Ok(Expr::LiteralUndefined)
            }
            _ => Ok(expr.clone())
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
            _ => "Compile-time Error".to_string()
        }
    }
}
