use std::collections::HashMap;
use crate::frontend::parser::ast::*;
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
    array_elem_types: HashMap<String, IRType>,
    fn_signatures: HashMap<String, Vec<(String, Vec<IRType>)>>,
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
            array_elem_types: HashMap::new(),
            fn_signatures: HashMap::new(),
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

    fn infer_expr_type(&self, expr: &Expr) -> IRType {
        match expr {
            Expr::LiteralInt(_) => IRType::Int(Size::S32),
            Expr::LiteralFloat(_) => IRType::Float(Size::S64),
            Expr::LiteralBool(_) => IRType::Bool,
            Expr::LiteralChar(_) => IRType::Char,
            Expr::LiteralString(_) => IRType::Pointer(Box::new(IRType::Int(Size::S8))),
            Expr::Identifier(name) => {
                if let Some(t) = self.var_types.get(name) {
                    match t.as_str() {
                        "int8" => IRType::Int(Size::S8),
                        "int16" => IRType::Int(Size::S16),
                        "int32" | "int" => IRType::Int(Size::S32),
                        "int64" => IRType::Int(Size::S64),
                        "uint8" | "byte" => IRType::UInt(Size::S8),
                        "uint16" => IRType::UInt(Size::S16),
                        "uint32" => IRType::UInt(Size::S32),
                        "uint64" => IRType::UInt(Size::S64),
                        "float" | "float32" => IRType::Float(Size::S32),
                        "float64" => IRType::Float(Size::S64),
                        "bool" => IRType::Bool,
                        "char" => IRType::Char,
                        "string" => IRType::Pointer(Box::new(IRType::Int(Size::S8))),
                        _ => IRType::Int(Size::S32),
                    }
                } else {
                    IRType::Int(Size::S32)
                }
            }
            Expr::IndexAccess { object, .. } => {
                if let Expr::Identifier(name) = &**object {
                    self.array_elem_types.get(name).cloned().unwrap_or(IRType::Int(Size::S32))
                } else {
                    IRType::Int(Size::S32)
                }
            }
            _ => IRType::Int(Size::S32),
        }
    }

    fn resolve_callee(&self, name: &str, arg_types: &[IRType]) -> String {
        if let Some(overloads) = self.fn_signatures.get(name) {
            if overloads.len() == 1 {
                return overloads[0].0.clone();
            }
            // 1. Exact match
            for (mangled, params) in overloads {
                if
                    params.len() == arg_types.len() &&
                    params
                        .iter()
                        .zip(arg_types)
                        .all(|(p, a)| p == a)
                {
                    return mangled.clone();
                }
            }
            // 2. Compatible match
            for (mangled, params) in overloads {
                if
                    params.len() == arg_types.len() &&
                    params
                        .iter()
                        .zip(arg_types)
                        .all(|(p, a)| {
                            p == a ||
                                (*p == IRType::Int(Size::S64) && *a == IRType::Int(Size::S32)) ||
                                (*p == IRType::Float(Size::S64) &&
                                    *a == IRType::Float(Size::S32)) ||
                                (*p == IRType::Int(Size::S32) && *a == IRType::Char)
                        })
                {
                    return mangled.clone();
                }
            }
            return overloads[0].0.clone();
        }
        name.to_string()
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
                        size: if offset == 0 {
                            8
                        } else {
                            offset
                        },
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

                    let mangled_name = if name == "main" {
                        "main".to_string()
                    } else {
                        let sig: Vec<String> = ir_params
                            .iter()
                            .map(|(_, ty)| ty.to_string().replace("<", "_").replace(">", "_"))
                            .collect();
                        format!("{}_{}", name, sig.join("_"))
                    };
                    self.fn_signatures
                        .entry(name.clone())
                        .or_default()
                        .push((
                            mangled_name.clone(),
                            ir_params
                                .iter()
                                .map(|(_, ty)| ty.clone())
                                .collect(),
                        ));

                    let func = IRFunction::new(mangled_name, ir_params.clone(), ir_ret);
                    self.current_func = Some(func);
                    self.current_block = 0;
                    self.push_scope();

                    // Allocate parameters as local variables and store incoming parameter values
                    for (i, (param_name, param_ty)) in ir_params.iter().enumerate() {
                        let alloc_ptr = self.current_func.as_mut().unwrap().new_vreg();
                        self.current_func
                            .as_mut()
                            .unwrap()
                            .add_inst(self.current_block, IRInstruction {
                                id: Some(alloc_ptr),
                                op: IROp::Alloc { ty: param_ty.clone() },
                            });
                        self.current_func
                            .as_mut()
                            .unwrap()
                            .add_inst(self.current_block, IRInstruction {
                                id: None,
                                op: IROp::StoreParam { param_idx: i, ptr: alloc_ptr },
                            });
                        self.declare_var(param_name.clone(), alloc_ptr);
                    }

                    self.visit_block(body);

                    // Auto-return for void functions
                    if let Some(f) = &mut self.current_func {
                        if f.return_type == IRType::Void {
                            let last_block = self.current_block;
                            f.add_inst(last_block, IRInstruction {
                                id: None,
                                op: IROp::Return(None),
                            });
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
                            let mut ir_params = vec![(
                                "this".to_string(),
                                IRType::CustomScope(target.clone()),
                            )];
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
                                self.current_func
                                    .as_mut()
                                    .unwrap()
                                    .add_inst(self.current_block, IRInstruction {
                                        id: Some(alloc_ptr),
                                        op: IROp::Alloc { ty: param_ty.clone() },
                                    });
                                self.current_func
                                    .as_mut()
                                    .unwrap()
                                    .add_inst(self.current_block, IRInstruction {
                                        id: None,
                                        op: IROp::StoreParam { param_idx: i, ptr: alloc_ptr },
                                    });
                                self.declare_var(param_name.clone(), alloc_ptr);
                                if param_name == "this" {
                                    self.var_types.insert("this".to_string(), target.clone());
                                }
                            }

                            self.visit_block(body);

                            if let Some(f) = &mut self.current_func {
                                if f.return_type == IRType::Void {
                                    let last_block = self.current_block;
                                    f.add_inst(last_block, IRInstruction {
                                        id: None,
                                        op: IROp::Return(None),
                                    });
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
                        if
                            let Decl::ExternFnDecl { name, params, return_type, alias, .. } =
                                ext_decl
                        {
                            let sym_name = alias.as_ref().unwrap_or(name);
                            let ir_ret = IRType::from_ast(return_type);
                            let mut ir_params = Vec::new();
                            for param in params {
                                let ty = IRType::from_ast(&param.type_node);
                                ir_params.push((param.name.clone(), ty));
                            }
                            let func = IRFunction::new_extern(sym_name.clone(), ir_params, ir_ret);
                            functions.push(func);
                        } else if let Decl::FnDecl { name, params, return_type, .. } = ext_decl {
                            let ir_ret = IRType::from_ast(return_type);
                            let mut ir_params = Vec::new();
                            for param in params {
                                let ty = IRType::from_ast(&param.type_node);
                                ir_params.push((param.name.clone(), ty));
                            }
                            self.fn_signatures
                                .entry(name.clone())
                                .or_default()
                                .push((
                                    name.clone(),
                                    ir_params
                                        .iter()
                                        .map(|(_, ty)| ty.clone())
                                        .collect(),
                                ));
                            let func = IRFunction::new_extern(name.clone(), ir_params, ir_ret);
                            functions.push(func);
                        }
                    }
                }
                Stmt::Declaration(Decl::ExternFnDecl { name, params, return_type, alias, .. }) => {
                    let sym_name = alias.as_ref().unwrap_or(name);
                    let ir_ret = IRType::from_ast(return_type);
                    let mut ir_params = Vec::new();
                    for param in params {
                        let ty = IRType::from_ast(&param.type_node);
                        ir_params.push((param.name.clone(), ty));
                    }
                    self.fn_signatures
                        .entry(name.clone())
                        .or_default()
                        .push((
                            sym_name.clone(),
                            ir_params
                                .iter()
                                .map(|(_, ty)| ty.clone())
                                .collect(),
                        ));
                    let func = IRFunction::new_extern(sym_name.clone(), ir_params, ir_ret);
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

                self.current_func
                    .as_mut()
                    .unwrap()
                    .add_inst(self.current_block, IRInstruction {
                        id: Some(ptr),
                        op: IROp::Alloc { ty: ty.clone() },
                    });

                self.declare_var(name.clone(), ptr);
                self.var_types.insert(name.clone(), type_node.get_name());

                match value {
                    Expr::LiteralVoid | Expr::Default(_) => {}
                    _ => {
                        let val = self.visit_expr(value);
                        self.current_func
                            .as_mut()
                            .unwrap()
                            .add_inst(self.current_block, IRInstruction {
                                id: None,
                                op: IROp::Store { ptr, value: val },
                            });
                    }
                }
            }
            Stmt::Declaration(Decl::DestructureDecl { assignments, type_node, .. }) => {
                let ty = IRType::from_ast(&type_node);
                for (name, expr) in assignments {
                    let ptr = self.current_func.as_mut().unwrap().new_vreg();
                    self.current_func
                        .as_mut()
                        .unwrap()
                        .add_inst(self.current_block, IRInstruction {
                            id: Some(ptr),
                            op: IROp::Alloc { ty: ty.clone() },
                        });
                    self.declare_var(name.clone(), ptr);
                    self.var_types.insert(name.clone(), type_node.get_name());

                    match expr {
                        Expr::LiteralVoid | Expr::Default(_) => {}
                        _ => {
                            let val = self.visit_expr(expr);
                            self.current_func
                                .as_mut()
                                .unwrap()
                                .add_inst(self.current_block, IRInstruction {
                                    id: None,
                                    op: IROp::Store { ptr, value: val },
                                });
                        }
                    }
                }
            }
            Stmt::Declaration(Decl::ArrayDecl { name, type_node, length: _, value, .. }) => {
                let elem_ty = IRType::from_ast(&type_node);
                let elem_size = elem_ty.size_in_bytes();
                let array_ptr = self.current_func.as_mut().unwrap().new_vreg();

                let (arr_len, elems) = match value {
                    Expr::ArrayLiteral(elements) => (elements.len(), elements.as_slice()),
                    _ => (1, [].as_slice()),
                };
                let size_in_bytes = if arr_len == 0 { elem_size } else { arr_len * elem_size };

                self.current_func
                    .as_mut()
                    .unwrap()
                    .add_inst(self.current_block, IRInstruction {
                        id: Some(array_ptr),
                        op: IROp::AllocArray {
                            elem_ty: elem_ty.clone(),
                            size: size_in_bytes,
                        },
                    });

                self.declare_var(name.clone(), array_ptr);
                self.var_types.insert(name.clone(), format!("array<{}>", type_node.get_name()));
                self.array_elem_types.insert(name.clone(), elem_ty.clone());

                for (idx, el) in elems.iter().enumerate() {
                    let el_val = self.visit_expr(el);
                    let idx_vreg = self.current_func.as_mut().unwrap().new_vreg();
                    self.current_func
                        .as_mut()
                        .unwrap()
                        .add_inst(self.current_block, IRInstruction {
                            id: Some(idx_vreg),
                            op: IROp::ConstInt32(idx as i32),
                        });
                    let elem_ptr = self.current_func.as_mut().unwrap().new_vreg();
                    self.current_func
                        .as_mut()
                        .unwrap()
                        .add_inst(self.current_block, IRInstruction {
                            id: Some(elem_ptr),
                            op: IROp::GetElementPtr {
                                base_ptr: array_ptr,
                                index: idx_vreg,
                                elem_size,
                            },
                        });
                    self.current_func
                        .as_mut()
                        .unwrap()
                        .add_inst(self.current_block, IRInstruction {
                            id: None,
                            op: IROp::StoreMemory { ptr: elem_ptr, value: el_val },
                        });
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
                                self.current_func
                                    .as_mut()
                                    .unwrap()
                                    .add_inst(self.current_block, IRInstruction {
                                        id: Some(curr),
                                        op: IROp::Load { ptr, ty: IRType::Int(Size::S32) },
                                    });
                                let res = self.current_func.as_mut().unwrap().new_vreg();
                                self.current_func
                                    .as_mut()
                                    .unwrap()
                                    .add_inst(self.current_block, IRInstruction {
                                        id: Some(res),
                                        op: IROp::Add(curr, val),
                                    });
                                res
                            }
                            "-=" => {
                                let curr = self.current_func.as_mut().unwrap().new_vreg();
                                self.current_func
                                    .as_mut()
                                    .unwrap()
                                    .add_inst(self.current_block, IRInstruction {
                                        id: Some(curr),
                                        op: IROp::Load { ptr, ty: IRType::Int(Size::S32) },
                                    });
                                let res = self.current_func.as_mut().unwrap().new_vreg();
                                self.current_func
                                    .as_mut()
                                    .unwrap()
                                    .add_inst(self.current_block, IRInstruction {
                                        id: Some(res),
                                        op: IROp::Sub(curr, val),
                                    });
                                res
                            }
                            "*=" => {
                                let curr = self.current_func.as_mut().unwrap().new_vreg();
                                self.current_func
                                    .as_mut()
                                    .unwrap()
                                    .add_inst(self.current_block, IRInstruction {
                                        id: Some(curr),
                                        op: IROp::Load { ptr, ty: IRType::Int(Size::S32) },
                                    });
                                let res = self.current_func.as_mut().unwrap().new_vreg();
                                self.current_func
                                    .as_mut()
                                    .unwrap()
                                    .add_inst(self.current_block, IRInstruction {
                                        id: Some(res),
                                        op: IROp::Mul(curr, val),
                                    });
                                res
                            }
                            _ => val,
                        };
                        self.current_func
                            .as_mut()
                            .unwrap()
                            .add_inst(self.current_block, IRInstruction {
                                id: None,
                                op: IROp::Store { ptr, value: final_val },
                            });
                    }
                } else if let Expr::PropertyAccess { object, property } = target {
                    let (obj_ptr, struct_name) = match &**object {
                        Expr::This =>
                            (
                                self.lookup_var("this").unwrap(),
                                self.var_types.get("this").cloned().unwrap_or_default(),
                            ),
                        Expr::Identifier(n) =>
                            (
                                self.lookup_var(n).unwrap(),
                                self.var_types.get(n).cloned().unwrap_or_default(),
                            ),
                        _ => panic!("Unsupported property access target"),
                    };
                    if let Some(st) = self.structs.get(&struct_name).cloned() {
                        if
                            let Some((_, _field_ty, offset)) = st.fields
                                .iter()
                                .find(|(fn_name, _, _)| fn_name == property)
                        {
                            let field_ptr = self.current_func.as_mut().unwrap().new_vreg();
                            self.current_func
                                .as_mut()
                                .unwrap()
                                .add_inst(self.current_block, IRInstruction {
                                    id: Some(field_ptr),
                                    op: IROp::GetFieldPtr { ptr: obj_ptr, offset: *offset as i32 },
                                });
                            self.current_func
                                .as_mut()
                                .unwrap()
                                .add_inst(self.current_block, IRInstruction {
                                    id: None,
                                    op: IROp::StoreMemory { ptr: field_ptr, value: val },
                                });
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

                self.current_func
                    .as_mut()
                    .unwrap()
                    .add_inst(self.current_block, IRInstruction {
                        id: None,
                        op: IROp::BranchIf {
                            cond: cond_val,
                            true_block,
                            false_block: false_target,
                        },
                    });

                // Build then branch
                self.current_block = true_block;
                self.visit_block(then_block);
                self.current_func
                    .as_mut()
                    .unwrap()
                    .add_inst(self.current_block, IRInstruction {
                        id: None,
                        op: IROp::Jump(merge_block),
                    });

                // Build else branch if present
                if let Some(else_stmts) = else_block {
                    self.current_block = false_block;
                    self.visit_block(else_stmts);
                    self.current_func
                        .as_mut()
                        .unwrap()
                        .add_inst(self.current_block, IRInstruction {
                            id: None,
                            op: IROp::Jump(merge_block),
                        });
                }

                self.current_block = merge_block;
            }
            Stmt::WhileStmt { condition, body } => {
                let header_block = self.current_func.as_mut().unwrap().new_block();
                let body_block = self.current_func.as_mut().unwrap().new_block();
                let exit_block = self.current_func.as_mut().unwrap().new_block();

                self.current_func
                    .as_mut()
                    .unwrap()
                    .add_inst(self.current_block, IRInstruction {
                        id: None,
                        op: IROp::Jump(header_block),
                    });

                self.current_block = header_block;
                let cond_val = self.visit_expr(condition);
                self.current_func
                    .as_mut()
                    .unwrap()
                    .add_inst(header_block, IRInstruction {
                        id: None,
                        op: IROp::BranchIf {
                            cond: cond_val,
                            true_block: body_block,
                            false_block: exit_block,
                        },
                    });

                self.current_block = body_block;
                match body {
                    crate::frontend::parser::ast::EitherBlock::Inline(stmts) =>
                        self.visit_block(stmts),
                    crate::frontend::parser::ast::EitherBlock::External(expr) => {
                        self.visit_expr(expr);
                    }
                }
                self.current_func
                    .as_mut()
                    .unwrap()
                    .add_inst(self.current_block, IRInstruction {
                        id: None,
                        op: IROp::Jump(header_block),
                    });

                self.current_block = exit_block;
            }
            Stmt::ReturnStmt(expr) => {
                let val = self.visit_expr(
                    &expr.clone().unwrap_or(Expr::Identifier("".to_string()))
                );
                let curr = self.current_block;
                self.current_func
                    .as_mut()
                    .unwrap()
                    .add_inst(curr, IRInstruction { id: None, op: IROp::Return(Some(val)) });
            }
            Stmt::LeaveStmt => {
                let curr = self.current_block;
                self.current_func
                    .as_mut()
                    .unwrap()
                    .add_inst(curr, IRInstruction { id: None, op: IROp::Return(None) });
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
                self.current_func
                    .as_mut()
                    .unwrap()
                    .add_inst(self.current_block, IRInstruction {
                        id: Some(v),
                        op: IROp::ConstInt32(*i as i32),
                    });
                v
            }
            Expr::LiteralFloat(f) => {
                let v = self.current_func.as_mut().unwrap().new_vreg();
                self.current_func
                    .as_mut()
                    .unwrap()
                    .add_inst(self.current_block, IRInstruction {
                        id: Some(v),
                        op: IROp::ConstFloat64(*f),
                    });
                v
            }
            Expr::LiteralChar(c) => {
                let v = self.current_func.as_mut().unwrap().new_vreg();
                self.current_func
                    .as_mut()
                    .unwrap()
                    .add_inst(self.current_block, IRInstruction {
                        id: Some(v),
                        op: IROp::ConstInt32(*c as i32),
                    });
                v
            }
            Expr::LiteralBool(b) => {
                let v = self.current_func.as_mut().unwrap().new_vreg();
                self.current_func
                    .as_mut()
                    .unwrap()
                    .add_inst(self.current_block, IRInstruction {
                        id: Some(v),
                        op: IROp::ConstBool(*b),
                    });
                v
            }
            Expr::LiteralString(s) => {
                let v = self.current_func.as_mut().unwrap().new_vreg();
                self.current_func
                    .as_mut()
                    .unwrap()
                    .add_inst(self.current_block, IRInstruction {
                        id: Some(v),
                        op: IROp::ConstString(s.clone()),
                    });
                v
            }
            Expr::This => {
                if let Some(ptr) = self.lookup_var("this") {
                    ptr
                } else {
                    panic!("IR: 'this' used outside of method");
                }
            }
            Expr::IndexAccess { object, indices } => {
                let (base_ptr, elem_ty) = match &**object {
                    Expr::Identifier(n) => {
                        let ptr = self
                            .lookup_var(n)
                            .unwrap_or_else(|| panic!("IR: Unknown array '{}'", n));
                        let ty = self.array_elem_types
                            .get(n)
                            .cloned()
                            .unwrap_or(IRType::Int(Size::S32));
                        (ptr, ty)
                    }
                    _ => (self.visit_expr(object), IRType::Int(Size::S32)),
                };
                let elem_size = elem_ty.size_in_bytes();
                let idx_val = self.visit_expr(&indices[0]);
                let elem_ptr = self.current_func.as_mut().unwrap().new_vreg();
                self.current_func
                    .as_mut()
                    .unwrap()
                    .add_inst(self.current_block, IRInstruction {
                        id: Some(elem_ptr),
                        op: IROp::GetElementPtr {
                            base_ptr,
                            index: idx_val,
                            elem_size,
                        },
                    });
                let val = self.current_func.as_mut().unwrap().new_vreg();
                self.current_func
                    .as_mut()
                    .unwrap()
                    .add_inst(self.current_block, IRInstruction {
                        id: Some(val),
                        op: IROp::LoadMemory {
                            ptr: elem_ptr,
                            ty: elem_ty,
                        },
                    });
                val
            }
            Expr::TypeOf { target: _ } => {
                let v = self.current_func.as_mut().unwrap().new_vreg();
                self.current_func
                    .as_mut()
                    .unwrap()
                    .add_inst(self.current_block, IRInstruction {
                        id: Some(v),
                        op: IROp::ConstTypeID(4),
                    });
                v
            }
            Expr::SizeOf { target: _ } => {
                let v = self.current_func.as_mut().unwrap().new_vreg();
                self.current_func
                    .as_mut()
                    .unwrap()
                    .add_inst(self.current_block, IRInstruction {
                        id: Some(v),
                        op: IROp::ConstInt32(4),
                    });
                v
            }
            Expr::Identifier(name) => {
                match name.as_str() {
                    "bool" => {
                        let v = self.current_func.as_mut().unwrap().new_vreg();
                        self.current_func
                            .as_mut()
                            .unwrap()
                            .add_inst(self.current_block, IRInstruction {
                                id: Some(v),
                                op: IROp::ConstTypeID(1),
                            });
                        return v;
                    }
                    "int32" | "int" => {
                        let v = self.current_func.as_mut().unwrap().new_vreg();
                        self.current_func
                            .as_mut()
                            .unwrap()
                            .add_inst(self.current_block, IRInstruction {
                                id: Some(v),
                                op: IROp::ConstTypeID(4),
                            });
                        return v;
                    }
                    "int64" => {
                        let v = self.current_func.as_mut().unwrap().new_vreg();
                        self.current_func
                            .as_mut()
                            .unwrap()
                            .add_inst(self.current_block, IRInstruction {
                                id: Some(v),
                                op: IROp::ConstTypeID(5),
                            });
                        return v;
                    }
                    "string" => {
                        let v = self.current_func.as_mut().unwrap().new_vreg();
                        self.current_func
                            .as_mut()
                            .unwrap()
                            .add_inst(self.current_block, IRInstruction {
                                id: Some(v),
                                op: IROp::ConstTypeID(19),
                            });
                        return v;
                    }
                    _ => {}
                }
                if let Some(ptr) = self.lookup_var(name) {
                    let v = self.current_func.as_mut().unwrap().new_vreg();
                    // Just assume Int32 for now until we fully type the AST in the Builder
                    self.current_func
                        .as_mut()
                        .unwrap()
                        .add_inst(self.current_block, IRInstruction {
                            id: Some(v),
                            op: IROp::Load { ptr, ty: IRType::Int(Size::S32) },
                        });
                    v
                } else {
                    panic!("IR: Unknown variable '{}'", name);
                }
            }
            Expr::PropertyAccess { object, property } => {
                let (obj_ptr, struct_name) = match &**object {
                    Expr::This =>
                        (
                            self.lookup_var("this").unwrap(),
                            self.var_types.get("this").cloned().unwrap_or_default(),
                        ),
                    Expr::Identifier(n) =>
                        (
                            self.lookup_var(n).unwrap(),
                            self.var_types.get(n).cloned().unwrap_or_default(),
                        ),
                    _ => panic!("Unsupported property access target"),
                };
                if let Some(st) = self.structs.get(&struct_name).cloned() {
                    if
                        let Some((_, field_ty, offset)) = st.fields
                            .iter()
                            .find(|(fn_name, _, _)| fn_name == property)
                    {
                        let field_ptr = self.current_func.as_mut().unwrap().new_vreg();
                        self.current_func
                            .as_mut()
                            .unwrap()
                            .add_inst(self.current_block, IRInstruction {
                                id: Some(field_ptr),
                                op: IROp::GetFieldPtr { ptr: obj_ptr, offset: *offset as i32 },
                            });
                        let val = self.current_func.as_mut().unwrap().new_vreg();
                        self.current_func
                            .as_mut()
                            .unwrap()
                            .add_inst(self.current_block, IRInstruction {
                                id: Some(val),
                                op: IROp::LoadMemory { ptr: field_ptr, ty: field_ty.clone() },
                            });
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

                self.current_func
                    .as_mut()
                    .unwrap()
                    .add_inst(self.current_block, IRInstruction { id: Some(v), op });
                v
            }
            Expr::Call { callee, args } => {
                if let Expr::PropertyAccess { object, property } = &**callee {
                    let (obj_ptr, struct_name) = match &**object {
                        Expr::This =>
                            (
                                self.lookup_var("this").unwrap(),
                                self.var_types.get("this").cloned().unwrap_or_default(),
                            ),
                        Expr::Identifier(n) =>
                            (
                                self.lookup_var(n).unwrap(),
                                self.var_types.get(n).cloned().unwrap_or_default(),
                            ),
                        _ => panic!("Unsupported method call target"),
                    };
                    let mangled_name = format!("{}_{}", struct_name, property);
                    let mut ir_args = vec![obj_ptr];
                    for arg in args {
                        ir_args.push(self.visit_expr(arg));
                    }
                    let v = self.current_func.as_mut().unwrap().new_vreg();
                    self.current_func
                        .as_mut()
                        .unwrap()
                        .add_inst(self.current_block, IRInstruction {
                            id: Some(v),
                            op: IROp::Call { func: mangled_name, args: ir_args },
                        });
                    v
                } else if let Expr::Identifier(func_name) = &**callee {
                    let mut ir_args = Vec::new();
                    let mut arg_types = Vec::new();
                    for arg in args {
                        arg_types.push(self.infer_expr_type(arg));
                        ir_args.push(self.visit_expr(arg));
                    }
                    let resolved_func = self.resolve_callee(func_name, &arg_types);
                    let v = self.current_func.as_mut().unwrap().new_vreg();
                    self.current_func
                        .as_mut()
                        .unwrap()
                        .add_inst(self.current_block, IRInstruction {
                            id: Some(v),
                            op: IROp::Call { func: resolved_func, args: ir_args },
                        });
                    v
                } else {
                    panic!("IR: Complex callee not yet supported");
                }
            }
            _ => {
                // Fallback for unsupported expressions returning a dummy value (for incomplete IR draft)
                let v = self.current_func.as_mut().unwrap().new_vreg();
                self.current_func
                    .as_mut()
                    .unwrap()
                    .add_inst(self.current_block, IRInstruction {
                        id: Some(v),
                        op: IROp::ConstInt32(0),
                    });
                v
            }
        }
    }
}
