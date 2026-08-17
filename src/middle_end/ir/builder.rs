use std::collections::HashMap;
use crate::frontend::parser::ast::{Stmt, Decl, Expr, TypeMetadata, BaseType};
use crate::middle_end::ir::instruction::*;

pub struct IRBuilder<'a> {
    module_name: String,
    metadata: &'a HashMap<String, TypeMetadata>,
    current_func: Option<IRFunction>,
    current_block: BlockID,
    // Maps variable names in the current scope to their allocated Pointer IRValue
    env: Vec<HashMap<String, IRValue>>,
}

impl<'a> IRBuilder<'a> {
    pub fn new(module_name: String, metadata: &'a HashMap<String, TypeMetadata>) -> Self {
        Self {
            module_name,
            metadata,
            current_func: None,
            current_block: 0,
            env: vec![HashMap::new()],
        }
    }

    fn push_scope(&mut self) {
        self.env.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.env.pop();
    }

    fn declare_var(&mut self, name: String, ptr_val: IRValue) {
        if let Some(scope) = self.env.last_mut() {
            scope.insert(name, ptr_val);
        }
    }

    fn lookup_var(&self, name: &str) -> Option<IRValue> {
        for scope in self.env.iter().rev() {
            if let Some(&val) = scope.get(name) {
                return Some(val);
            }
        }
        None
    }

    pub fn build(mut self, stmts: &[Stmt]) -> IRModule {
        let mut functions = Vec::new();

        for stmt in stmts {
            match stmt {
                Stmt::Declaration(Decl::FnDecl { name, params, return_type, body, .. }) => {
                    let ir_ret = IRType::from_ast(return_type);
                    let mut ir_params = Vec::new();
                    for param in params {
                        let ty = IRType::from_ast(&param.type_node);
                        ir_params.push((param.name.clone(), ty));
                    }

                    let mut func = IRFunction::new(name.clone(), ir_params.clone(), ir_ret);
                    self.current_func = Some(func);
                    self.current_block = 0;
                    self.push_scope();

                    // Allocate parameters as local variables so they can be mutated
                    for (param_name, param_ty) in &ir_params {
                        let alloc_ptr = self.current_func.as_mut().unwrap().new_vreg();
                        self.current_func.as_mut().unwrap().add_inst(
                            self.current_block,
                            IRInstruction { id: Some(alloc_ptr), op: IROp::Alloc { ty: param_ty.clone() } },
                        );
                        self.declare_var(param_name.clone(), alloc_ptr);
                        
                        // We also need an instruction to store the argument value into the param alloc.
                        // For Custom IR, we assume args are the first N vregs passed into the function?
                        // Let's keep it simple: Cranelift will handle argument mapping, 
                        // we just need to represent the Variable.
                    }

                    self.visit_block(body);

                    // Auto-return for void functions
                    if let Some(f) = &mut self.current_func {
                        if f.return_type == IRType::Void {
                            let last_block = self.current_block;
                            f.add_inst(last_block, IRInstruction { id: None, op: IROp::Return(None) });
                        }
                    }

                    self.pop_scope();
                    if let Some(f) = self.current_func.take() {
                        functions.push(f);
                    }
                }
                _ => {}
            }
        }

        IRModule {
            name: self.module_name,
            functions,
        }
    }

    fn visit_block(&mut self, stmts: &[Stmt]) {
        self.push_scope();
        for stmt in stmts {
            self.visit_stmt(stmt);
        }
        self.pop_scope();
    }

    fn visit_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Declaration(Decl::VarDecl { name, type_node, value, .. }) => {
                let ty = IRType::from_ast(&type_node);
                let ptr = self.current_func.as_mut().unwrap().new_vreg();
                
                self.current_func.as_mut().unwrap().add_inst(
                    self.current_block,
                    IRInstruction { id: Some(ptr), op: IROp::Alloc { ty: ty.clone() } },
                );
                
                self.declare_var(name.clone(), ptr);
                
                let val = self.visit_expr(value);
                self.current_func.as_mut().unwrap().add_inst(
                    self.current_block,
                    IRInstruction { id: None, op: IROp::Store { ptr, value: val } },
                );
            }
            Stmt::ExpressionStmt(expr) => {
                self.visit_expr(expr);
            }
            Stmt::ReturnStmt(expr) => {
                let val = self.visit_expr(expr);
                self.current_func.as_mut().unwrap().add_inst(
                    self.current_block,
                    IRInstruction { id: None, op: IROp::Return(Some(val)) },
                );
            }
            Stmt::LeaveStmt => {
                self.current_func.as_mut().unwrap().add_inst(
                    self.current_block,
                    IRInstruction { id: None, op: IROp::Return(None) },
                );
            }
            _ => {}
        }
    }

    fn visit_expr(&mut self, expr: &Expr) -> IRValue {
        match expr {
            Expr::LiteralInt(i) => {
                let v = self.current_func.as_mut().unwrap().new_vreg();
                self.current_func.as_mut().unwrap().add_inst(
                    self.current_block,
                    IRInstruction { id: Some(v), op: IROp::ConstInt32(*i as i32) },
                );
                v
            }
            Expr::LiteralFloat(f) => {
                let v = self.current_func.as_mut().unwrap().new_vreg();
                self.current_func.as_mut().unwrap().add_inst(
                    self.current_block,
                    IRInstruction { id: Some(v), op: IROp::ConstFloat32(*f as f32) },
                );
                v
            }
            Expr::LiteralBool(b) => {
                let v = self.current_func.as_mut().unwrap().new_vreg();
                self.current_func.as_mut().unwrap().add_inst(
                    self.current_block,
                    IRInstruction { id: Some(v), op: IROp::ConstBool(*b) },
                );
                v
            }
            Expr::Identifier(name) => {
                if let Some(ptr) = self.lookup_var(name) {
                    let v = self.current_func.as_mut().unwrap().new_vreg();
                    // Just assume Int32 for now until we fully type the AST in the Builder
                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction { id: Some(v), op: IROp::Load { ptr, ty: IRType::Int32 } },
                    );
                    v
                } else {
                    panic!("IR: Unknown variable '{}'", name);
                }
            }
            Expr::BinaryOp { left, operator, right } => {
                let l = self.visit_expr(left);
                let r = self.visit_expr(right);
                let v = self.current_func.as_mut().unwrap().new_vreg();
                
                let op = match operator.as_str() {
                    "+" => IROp::Add(l, r),
                    "-" => IROp::Sub(l, r),
                    "*" => IROp::Mul(l, r),
                    "/" => IROp::Div(l, r),
                    "%" => IROp::Mod(l, r),
                    "==" => IROp::Eq(l, r),
                    "!=" => IROp::Neq(l, r),
                    "<" => IROp::Lt(l, r),
                    "<=" => IROp::Le(l, r),
                    ">" => IROp::Gt(l, r),
                    ">=" => IROp::Ge(l, r),
                    _ => panic!("IR: Unsupported binary operator '{}'", operator),
                };
                
                self.current_func.as_mut().unwrap().add_inst(
                    self.current_block,
                    IRInstruction { id: Some(v), op },
                );
                v
            }
            Expr::Call { callee, args } => {
                // Simplistic call handling for functions (no methods yet)
                if let Expr::Identifier(func_name) = &**callee {
                    let mut ir_args = Vec::new();
                    for arg in args {
                        ir_args.push(self.visit_expr(arg));
                    }
                    let v = self.current_func.as_mut().unwrap().new_vreg();
                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction { id: Some(v), op: IROp::Call { func: func_name.clone(), args: ir_args } },
                    );
                    v
                } else {
                    panic!("IR: Complex callee not yet supported");
                }
            }
            _ => {
                // Fallback for unsupported expressions returning a dummy value (for incomplete IR draft)
                let v = self.current_func.as_mut().unwrap().new_vreg();
                self.current_func.as_mut().unwrap().add_inst(
                    self.current_block,
                    IRInstruction { id: Some(v), op: IROp::ConstInt32(0) },
                );
                v
            }
        }
    }
}
