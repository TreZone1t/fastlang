use std::collections::HashMap;
use crate::frontend::parser::ast::{Stmt, Decl, Expr, TypeMetadata};
use crate::middle_end::ir::instruction::*;

pub struct IRBuilder<'a> {
    module_name: String,
    _metadata: &'a HashMap<String, TypeMetadata>,
    current_func: Option<IRFunction>,
    current_block: BlockID,
    // Maps variable names in the current scope to their allocated Pointer IRValue
    env: Vec<HashMap<String, IRValue>>,
    structs: HashMap<String, IRStruct>,
    var_types: HashMap<String, String>,
}

impl<'a> IRBuilder<'a> {
    pub fn new(module_name: String, metadata: &'a HashMap<String, TypeMetadata>) -> Self {
        Self {
            module_name,
            _metadata: metadata,
            current_func: None,
            current_block: 0,
            env: vec![HashMap::new()],
            structs: HashMap::new(),
            var_types: HashMap::new(),
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

        // 1. First pass: Collect all struct and blueprint declarations
        for stmt in stmts {
            if let Stmt::Declaration(Decl::BlueprintDecl { name, definition, .. }) = stmt {
                if let crate::frontend::parser::ast::BlueprintDef::Explicit(fields) = definition {
                    let mut offset = 0;
                    let mut ir_fields = Vec::new();
                    for f in fields {
                        let ir_ty = IRType::from_ast(&f.type_node);
                        let size = ir_ty.size_in_bytes();
                        ir_fields.push((f.name.clone(), ir_ty, offset));
                        offset += size;
                    }
                    let ir_struct = IRStruct {
                        name: name.clone(),
                        size: if offset == 0 { 8 } else { offset },
                        fields: ir_fields,
                    };
                    self.structs.insert(name.clone(), ir_struct);
                }
            }
        }

        // 2. Second pass: Collect functions, impl methods, extern blocks
        for stmt in stmts {
            match stmt {
                Stmt::Declaration(Decl::FnDecl { name, params, return_type, body, .. }) => {
                    let ir_ret = IRType::from_ast(return_type);
                    let mut ir_params = Vec::new();
                    for param in params {
                        let ty = IRType::from_ast(&param.type_node);
                        ir_params.push((param.name.clone(), ty));
                    }

                    let func = IRFunction::new(name.clone(), ir_params.clone(), ir_ret);
                    self.current_func = Some(func);
                    self.current_block = 0;
                    self.push_scope();

                    // Allocate parameters as local variables and store incoming parameter values
                    for (i, (param_name, param_ty)) in ir_params.iter().enumerate() {
                        let alloc_ptr = self.current_func.as_mut().unwrap().new_vreg();
                        self.current_func.as_mut().unwrap().add_inst(
                            self.current_block,
                            IRInstruction { id: Some(alloc_ptr), op: IROp::Alloc { ty: param_ty.clone() } },
                        );
                        self.current_func.as_mut().unwrap().add_inst(
                            self.current_block,
                            IRInstruction { id: None, op: IROp::StoreParam { param_idx: i, ptr: alloc_ptr } },
                        );
                        self.declare_var(param_name.clone(), alloc_ptr);
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
                Stmt::Declaration(Decl::ImplDecl { target, methods, .. }) => {
                    for m in methods {
                        if let Decl::FnDecl { name, params, return_type, body, .. } = m {
                            let ir_ret = IRType::from_ast(return_type);
                            let mut ir_params = vec![("this".to_string(), IRType::CustomScope(target.clone()))];
                            for param in params {
                                let ty = IRType::from_ast(&param.type_node);
                                ir_params.push((param.name.clone(), ty));
                            }
                            let mangled_name = format!("{}_{}", target, name);
                            let func = IRFunction::new(mangled_name, ir_params.clone(), ir_ret);
                            self.current_func = Some(func);
                            self.current_block = 0;
                            self.push_scope();

                            for (i, (param_name, param_ty)) in ir_params.iter().enumerate() {
                                let alloc_ptr = self.current_func.as_mut().unwrap().new_vreg();
                                self.current_func.as_mut().unwrap().add_inst(
                                    self.current_block,
                                    IRInstruction { id: Some(alloc_ptr), op: IROp::Alloc { ty: param_ty.clone() } },
                                );
                                self.current_func.as_mut().unwrap().add_inst(
                                    self.current_block,
                                    IRInstruction { id: None, op: IROp::StoreParam { param_idx: i, ptr: alloc_ptr } },
                                );
                                self.declare_var(param_name.clone(), alloc_ptr);
                                if param_name == "this" {
                                    self.var_types.insert("this".to_string(), target.clone());
                                }
                            }

                            self.visit_block(body);

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
                    }
                }
                Stmt::Declaration(Decl::ExternBlockDecl { decls, .. }) => {
                    for ext_decl in decls {
                        if let Decl::ExternFnDecl { name, params, return_type, .. } = ext_decl {
                            let ir_ret = IRType::from_ast(return_type);
                            let mut ir_params = Vec::new();
                            for param in params {
                                let ty = IRType::from_ast(&param.type_node);
                                ir_params.push((param.name.clone(), ty));
                            }
                            let func = IRFunction::new_extern(name.clone(), ir_params, ir_ret);
                            functions.push(func);
                        } else if let Decl::FnDecl { name, params, return_type, .. } = ext_decl {
                            let ir_ret = IRType::from_ast(return_type);
                            let mut ir_params = Vec::new();
                            for param in params {
                                let ty = IRType::from_ast(&param.type_node);
                                ir_params.push((param.name.clone(), ty));
                            }
                            let func = IRFunction::new_extern(name.clone(), ir_params, ir_ret);
                            functions.push(func);
                        }
                    }
                }
                Stmt::Declaration(Decl::ExternFnDecl { name, params, return_type, .. }) => {
                    let ir_ret = IRType::from_ast(return_type);
                    let mut ir_params = Vec::new();
                    for param in params {
                        let ty = IRType::from_ast(&param.type_node);
                        ir_params.push((param.name.clone(), ty));
                    }
                    let func = IRFunction::new_extern(name.clone(), ir_params, ir_ret);
                    functions.push(func);
                }
                _ => {}
            }
        }

        IRModule {
            name: self.module_name,
            functions,
            structs: self.structs,
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
                self.var_types.insert(name.clone(), type_node.get_name());
                
                match value {
                    Expr::LiteralVoid | Expr::Default(_) => {}
                    _ => {
                        let val = self.visit_expr(value);
                        self.current_func.as_mut().unwrap().add_inst(
                            self.current_block,
                            IRInstruction { id: None, op: IROp::Store { ptr, value: val } },
                        );
                    }
                }
            }
            Stmt::ReassignStmt { target, op, value } => {
                let val = self.visit_expr(value);
                if let Expr::Identifier(name) = target {
                    if let Some(ptr) = self.lookup_var(name) {
                        let final_val = match op.as_str() {
                            "=" => val,
                            "+=" => {
                                let curr = self.current_func.as_mut().unwrap().new_vreg();
                                self.current_func.as_mut().unwrap().add_inst(
                                    self.current_block,
                                    IRInstruction { id: Some(curr), op: IROp::Load { ptr, ty: IRType::Int32 } },
                                );
                                let res = self.current_func.as_mut().unwrap().new_vreg();
                                self.current_func.as_mut().unwrap().add_inst(
                                    self.current_block,
                                    IRInstruction { id: Some(res), op: IROp::Add(curr, val) },
                                );
                                res
                            }
                            "-=" => {
                                let curr = self.current_func.as_mut().unwrap().new_vreg();
                                self.current_func.as_mut().unwrap().add_inst(
                                    self.current_block,
                                    IRInstruction { id: Some(curr), op: IROp::Load { ptr, ty: IRType::Int32 } },
                                );
                                let res = self.current_func.as_mut().unwrap().new_vreg();
                                self.current_func.as_mut().unwrap().add_inst(
                                    self.current_block,
                                    IRInstruction { id: Some(res), op: IROp::Sub(curr, val) },
                                );
                                res
                            }
                            "*=" => {
                                let curr = self.current_func.as_mut().unwrap().new_vreg();
                                self.current_func.as_mut().unwrap().add_inst(
                                    self.current_block,
                                    IRInstruction { id: Some(curr), op: IROp::Load { ptr, ty: IRType::Int32 } },
                                );
                                let res = self.current_func.as_mut().unwrap().new_vreg();
                                self.current_func.as_mut().unwrap().add_inst(
                                    self.current_block,
                                    IRInstruction { id: Some(res), op: IROp::Mul(curr, val) },
                                );
                                res
                            }
                            _ => val,
                        };
                        self.current_func.as_mut().unwrap().add_inst(
                            self.current_block,
                            IRInstruction { id: None, op: IROp::Store { ptr, value: final_val } },
                        );
                    }
                } else if let Expr::PropertyAccess { object, property } = target {
                    let (obj_ptr, struct_name) = match &**object {
                        Expr::This => (self.lookup_var("this").unwrap(), self.var_types.get("this").cloned().unwrap_or_default()),
                        Expr::Identifier(n) => (self.lookup_var(n).unwrap(), self.var_types.get(n).cloned().unwrap_or_default()),
                        _ => panic!("Unsupported property access target"),
                    };
                    if let Some(st) = self.structs.get(&struct_name).cloned() {
                        if let Some((_, _field_ty, offset)) = st.fields.iter().find(|(fn_name, _, _)| fn_name == property) {
                            let field_ptr = self.current_func.as_mut().unwrap().new_vreg();
                            self.current_func.as_mut().unwrap().add_inst(
                                self.current_block,
                                IRInstruction { id: Some(field_ptr), op: IROp::GetFieldPtr { ptr: obj_ptr, offset: *offset as i32 } },
                            );
                            self.current_func.as_mut().unwrap().add_inst(
                                self.current_block,
                                IRInstruction { id: None, op: IROp::StoreMemory { ptr: field_ptr, value: val } },
                            );
                        }
                    }
                }
            }
            Stmt::IfStmt { condition, then_block, else_block } => {
                let cond_val = self.visit_expr(condition);
                let true_block = self.current_func.as_mut().unwrap().new_block();
                let false_block = self.current_func.as_mut().unwrap().new_block();
                let merge_block = self.current_func.as_mut().unwrap().new_block();

                let false_target = if else_block.is_some() { false_block } else { merge_block };

                self.current_func.as_mut().unwrap().add_inst(
                    self.current_block,
                    IRInstruction {
                        id: None,
                        op: IROp::BranchIf {
                            cond: cond_val,
                            true_block,
                            false_block: false_target,
                        },
                    },
                );

                // Build then branch
                self.current_block = true_block;
                self.visit_block(then_block);
                self.current_func.as_mut().unwrap().add_inst(
                    self.current_block,
                    IRInstruction { id: None, op: IROp::Jump(merge_block) },
                );

                // Build else branch if present
                if let Some(else_stmts) = else_block {
                    self.current_block = false_block;
                    self.visit_block(else_stmts);
                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction { id: None, op: IROp::Jump(merge_block) },
                    );
                }

                self.current_block = merge_block;
            }
            Stmt::WhileStmt { condition, body } => {
                let header_block = self.current_func.as_mut().unwrap().new_block();
                let body_block = self.current_func.as_mut().unwrap().new_block();
                let exit_block = self.current_func.as_mut().unwrap().new_block();

                self.current_func.as_mut().unwrap().add_inst(
                    self.current_block,
                    IRInstruction { id: None, op: IROp::Jump(header_block) },
                );

                self.current_block = header_block;
                let cond_val = self.visit_expr(condition);
                self.current_func.as_mut().unwrap().add_inst(
                    header_block,
                    IRInstruction {
                        id: None,
                        op: IROp::BranchIf {
                            cond: cond_val,
                            true_block: body_block,
                            false_block: exit_block,
                        },
                    },
                );

                self.current_block = body_block;
                match body {
                    crate::frontend::parser::ast::EitherBlock::Inline(stmts) => self.visit_block(stmts),
                    crate::frontend::parser::ast::EitherBlock::External(expr) => { self.visit_expr(expr); }
                }
                self.current_func.as_mut().unwrap().add_inst(
                    self.current_block,
                    IRInstruction { id: None, op: IROp::Jump(header_block) },
                );

                self.current_block = exit_block;
            }
            Stmt::ReturnStmt(expr) => {
                let val = self.visit_expr(expr);
                let curr = self.current_block;
                self.current_func.as_mut().unwrap().add_inst(
                    curr,
                    IRInstruction { id: None, op: IROp::Return(Some(val)) },
                );
            }
            Stmt::LeaveStmt => {
                let curr = self.current_block;
                self.current_func.as_mut().unwrap().add_inst(
                    curr,
                    IRInstruction { id: None, op: IROp::Return(None) },
                );
            }
            Stmt::ExpressionStmt(expr) => {
                self.visit_expr(expr);
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
            Expr::LiteralString(s) => {
                let v = self.current_func.as_mut().unwrap().new_vreg();
                self.current_func.as_mut().unwrap().add_inst(
                    self.current_block,
                    IRInstruction { id: Some(v), op: IROp::ConstString(s.clone()) },
                );
                v
            }
            Expr::This => {
                if let Some(ptr) = self.lookup_var("this") {
                    ptr
                } else {
                    panic!("IR: 'this' used outside of method");
                }
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
            Expr::PropertyAccess { object, property } => {
                let (obj_ptr, struct_name) = match &**object {
                    Expr::This => (self.lookup_var("this").unwrap(), self.var_types.get("this").cloned().unwrap_or_default()),
                    Expr::Identifier(n) => (self.lookup_var(n).unwrap(), self.var_types.get(n).cloned().unwrap_or_default()),
                    _ => panic!("Unsupported property access target"),
                };
                if let Some(st) = self.structs.get(&struct_name).cloned() {
                    if let Some((_, field_ty, offset)) = st.fields.iter().find(|(fn_name, _, _)| fn_name == property) {
                        let field_ptr = self.current_func.as_mut().unwrap().new_vreg();
                        self.current_func.as_mut().unwrap().add_inst(
                            self.current_block,
                            IRInstruction { id: Some(field_ptr), op: IROp::GetFieldPtr { ptr: obj_ptr, offset: *offset as i32 } },
                        );
                        let val = self.current_func.as_mut().unwrap().new_vreg();
                        self.current_func.as_mut().unwrap().add_inst(
                            self.current_block,
                            IRInstruction { id: Some(val), op: IROp::LoadMemory { ptr: field_ptr, ty: field_ty.clone() } },
                        );
                        val
                    } else {
                        panic!("Unknown property {}", property);
                    }
                } else {
                    panic!("Unknown struct {}", struct_name);
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
                if let Expr::PropertyAccess { object, property } = &**callee {
                    let (obj_ptr, struct_name) = match &**object {
                        Expr::This => (self.lookup_var("this").unwrap(), self.var_types.get("this").cloned().unwrap_or_default()),
                        Expr::Identifier(n) => (self.lookup_var(n).unwrap(), self.var_types.get(n).cloned().unwrap_or_default()),
                        _ => panic!("Unsupported method call target"),
                    };
                    let mangled_name = format!("{}_{}", struct_name, property);
                    let mut ir_args = vec![obj_ptr];
                    for arg in args {
                        ir_args.push(self.visit_expr(arg));
                    }
                    let v = self.current_func.as_mut().unwrap().new_vreg();
                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction { id: Some(v), op: IROp::Call { func: mangled_name, args: ir_args } },
                    );
                    v
                } else if let Expr::Identifier(func_name) = &**callee {
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
