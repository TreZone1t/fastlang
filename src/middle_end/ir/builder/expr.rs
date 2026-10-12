use super::IRBuilder;
use crate::frontend::parser::ast::*;
use crate::middle_end::ir::instruction::*;
use std::collections::HashMap;

impl<'a> IRBuilder<'a> {
    pub(crate) fn visit_expr(&mut self, expr: &Expr) -> IRValue {
        match expr {
            Expr::LiteralInt(i) => {
                let v = self.current_func.as_mut().unwrap().new_vreg();
                self.current_func.as_mut().unwrap().add_inst(
                    self.current_block,
                    IRInstruction {
                        id: Some(v),
                        op: IROp::ConstInt32(*i as i32),
                    },
                );
                v
            }
            Expr::LiteralUInt(u) => {
                let v = self.current_func.as_mut().unwrap().new_vreg();
                self.current_func.as_mut().unwrap().add_inst(
                    self.current_block,
                    IRInstruction {
                        id: Some(v),
                        op: IROp::ConstInt32(*u as i32),
                    },
                );
                v
            }
            Expr::LiteralFloat(f) => {
                let v = self.current_func.as_mut().unwrap().new_vreg();
                self.current_func.as_mut().unwrap().add_inst(
                    self.current_block,
                    IRInstruction {
                        id: Some(v),
                        op: IROp::ConstFloat64(*f),
                    },
                );
                v
            }
            Expr::LiteralChar(c) => {
                let v = self.current_func.as_mut().unwrap().new_vreg();
                self.current_func.as_mut().unwrap().add_inst(
                    self.current_block,
                    IRInstruction {
                        id: Some(v),
                        op: IROp::ConstInt32(*c as i32),
                    },
                );
                v
            }
            Expr::LiteralBool(b) => {
                let v = self.current_func.as_mut().unwrap().new_vreg();
                self.current_func.as_mut().unwrap().add_inst(
                    self.current_block,
                    IRInstruction {
                        id: Some(v),
                        op: IROp::ConstBool(*b),
                    },
                );
                v
            }
            Expr::LiteralString(s) => {
                let v = self.current_func.as_mut().unwrap().new_vreg();
                self.current_func.as_mut().unwrap().add_inst(
                    self.current_block,
                    IRInstruction {
                        id: Some(v),
                        op: IROp::ConstString(s.clone()),
                    },
                );
                v
            }
            Expr::This => {
                if let Some(ptr) = self.lookup_var("this") {
                    ptr
                } else {
                    let v = self.current_func.as_mut().unwrap().new_vreg();
                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction {
                            id: Some(v),
                            op: IROp::ConstInt64(0),
                        },
                    );
                    v
                }
            }
            Expr::IndexAccess { object, indices } => {
                let (base_ptr, elem_ty) = match &**object {
                    Expr::Identifier(n) => {
                        let ptr = self.lookup_var(n).unwrap_or(0);
                        let ty = self
                            .array_elem_types
                            .get(n)
                            .cloned()
                            .unwrap_or(IRType::Int(Size::S32));
                        (ptr, ty)
                    }
                    Expr::This => {
                        let ptr = self.lookup_var("this").unwrap_or(0);
                        let ty = self
                            .array_elem_types
                            .get("this")
                            .cloned()
                            .unwrap_or(IRType::Int(Size::S32));
                        (ptr, ty)
                    }
                    _ => (self.visit_expr(object), IRType::Int(Size::S32)),
                };
                let elem_size = elem_ty.size_in_bytes().max(1);
                let idx_val = if !indices.is_empty() {
                    self.visit_expr(&indices[0])
                } else {
                    let v = self.current_func.as_mut().unwrap().new_vreg();
                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction {
                            id: Some(v),
                            op: IROp::ConstInt32(0),
                        },
                    );
                    return v;
                };
                let elem_ptr = self.current_func.as_mut().unwrap().new_vreg();
                self.current_func.as_mut().unwrap().add_inst(
                    self.current_block,
                    IRInstruction {
                        id: Some(elem_ptr),
                        op: IROp::GetElementPtr {
                            base_ptr,
                            index: idx_val,
                            elem_size,
                        },
                    },
                );
                let val = self.current_func.as_mut().unwrap().new_vreg();
                self.current_func.as_mut().unwrap().add_inst(
                    self.current_block,
                    IRInstruction {
                        id: Some(val),
                        op: IROp::LoadMemory {
                            ptr: elem_ptr,
                            ty: elem_ty,
                        },
                    },
                );
                val
            }
            Expr::Identifier(name) => {
                match name.as_str() {
                    "bool" => {
                        let v = self.current_func.as_mut().unwrap().new_vreg();
                        self.current_func.as_mut().unwrap().add_inst(
                            self.current_block,
                            IRInstruction {
                                id: Some(v),
                                op: IROp::ConstTypeID(1),
                            },
                        );
                        return v;
                    }
                    "int32" | "int" => {
                        let v = self.current_func.as_mut().unwrap().new_vreg();
                        self.current_func.as_mut().unwrap().add_inst(
                            self.current_block,
                            IRInstruction {
                                id: Some(v),
                                op: IROp::ConstTypeID(4),
                            },
                        );
                        return v;
                    }
                    "int64" => {
                        let v = self.current_func.as_mut().unwrap().new_vreg();
                        self.current_func.as_mut().unwrap().add_inst(
                            self.current_block,
                            IRInstruction {
                                id: Some(v),
                                op: IROp::ConstTypeID(5),
                            },
                        );
                        return v;
                    }
                    _ => {}
                }
                if let Some(ptr) = self.lookup_var(name) {
                    let v = self.current_func.as_mut().unwrap().new_vreg();
                    let ty = self
                        .var_types
                        .get(name)
                        .map(|t| match t.as_str() {
                            "float" | "float32" => IRType::Float(Size::S32),
                            "float64" => IRType::Float(Size::S64),
                            "int64" | "uint64" | "usize" => IRType::Int(Size::S64),
                            "bool" => IRType::Bool,
                            "char" => IRType::Char,
                            _ => IRType::Int(Size::S32),
                        })
                        .unwrap_or(IRType::Int(Size::S32));
                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction {
                            id: Some(v),
                            op: IROp::Load { ptr, ty },
                        },
                    );
                    v
                } else if let Some(sigs) = self.fn_signatures.get(name) {
                    if let Some((mangled, _)) = sigs.iter().find(|(_, p)| p.is_empty()) {
                        let v = self.current_func.as_mut().unwrap().new_vreg();
                        self.called_functions
                            .entry(mangled.clone())
                            .or_insert_with(Vec::new);
                        self.current_func.as_mut().unwrap().add_inst(
                            self.current_block,
                            IRInstruction {
                                id: Some(v),
                                op: IROp::Call {
                                    func: mangled.clone(),
                                    args: vec![],
                                },
                            },
                        );
                        v
                    } else {
                        let v = self.current_func.as_mut().unwrap().new_vreg();
                        self.current_func.as_mut().unwrap().add_inst(
                            self.current_block,
                            IRInstruction {
                                id: Some(v),
                                op: IROp::ConstInt32(0),
                            },
                        );
                        v
                    }
                } else {
                    let v = self.current_func.as_mut().unwrap().new_vreg();
                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction {
                            id: Some(v),
                            op: IROp::ConstInt32(0),
                        },
                    );
                    v
                }
            }
            Expr::PropertyAccess { object, property } => {
                if let Some((obj_ptr, struct_name)) = self.eval_object_ptr_and_type(object) {
                    if let Some(st) = self.structs.get(&struct_name).cloned() {
                        if let Some((_, field_ty, offset)) =
                            st.fields.iter().find(|(fn_name, _, _)| fn_name == property)
                        {
                            let field_ptr = self.current_func.as_mut().unwrap().new_vreg();
                            self.current_func.as_mut().unwrap().add_inst(
                                self.current_block,
                                IRInstruction {
                                    id: Some(field_ptr),
                                    op: IROp::GetFieldPtr {
                                        ptr: obj_ptr,
                                        offset: *offset as i32,
                                    },
                                },
                            );
                            let val = self.current_func.as_mut().unwrap().new_vreg();
                            self.current_func.as_mut().unwrap().add_inst(
                                self.current_block,
                                IRInstruction {
                                    id: Some(val),
                                    op: IROp::LoadMemory {
                                        ptr: field_ptr,
                                        ty: field_ty.clone(),
                                    },
                                },
                            );
                            return val;
                        }
                    }
                }
                let v = self.current_func.as_mut().unwrap().new_vreg();
                self.current_func.as_mut().unwrap().add_inst(
                    self.current_block,
                    IRInstruction {
                        id: Some(v),
                        op: IROp::ConstInt32(0),
                    },
                );
                v
            }
            Expr::UnaryOp { operator, operand } => {
                let val = self.visit_expr(operand);
                let v = self.current_func.as_mut().unwrap().new_vreg();
                let op = match operator.as_str() {
                    "!" => IROp::Not(val),
                    "-" => IROp::Neg(val),
                    "+" => return val,
                    "~" => IROp::Not(val),
                    _ => return val,
                };
                self.current_func
                    .as_mut()
                    .unwrap()
                    .add_inst(self.current_block, IRInstruction { id: Some(v), op });
                v
            }
            Expr::PrefixUpdate { operator, right } => {
                if let Expr::Identifier(name) = &**right {
                    if let Some(ptr) = self.lookup_var(name) {
                        let curr = self.current_func.as_mut().unwrap().new_vreg();
                        self.current_func.as_mut().unwrap().add_inst(
                            self.current_block,
                            IRInstruction {
                                id: Some(curr),
                                op: IROp::Load {
                                    ptr,
                                    ty: IRType::Int(Size::S32),
                                },
                            },
                        );
                        let one = self.current_func.as_mut().unwrap().new_vreg();
                        self.current_func.as_mut().unwrap().add_inst(
                            self.current_block,
                            IRInstruction {
                                id: Some(one),
                                op: IROp::ConstInt32(1),
                            },
                        );
                        let res = self.current_func.as_mut().unwrap().new_vreg();
                        let op = if operator == "++" {
                            IROp::Add(curr, one)
                        } else {
                            IROp::Sub(curr, one)
                        };
                        self.current_func
                            .as_mut()
                            .unwrap()
                            .add_inst(self.current_block, IRInstruction { id: Some(res), op });
                        self.current_func.as_mut().unwrap().add_inst(
                            self.current_block,
                            IRInstruction {
                                id: None,
                                op: IROp::Store { ptr, value: res },
                            },
                        );
                        return res;
                    }
                }
                self.visit_expr(right)
            }
            Expr::PostfixUpdate { operator, left } => {
                if let Expr::Identifier(name) = &**left {
                    if let Some(ptr) = self.lookup_var(name) {
                        let curr = self.current_func.as_mut().unwrap().new_vreg();
                        self.current_func.as_mut().unwrap().add_inst(
                            self.current_block,
                            IRInstruction {
                                id: Some(curr),
                                op: IROp::Load {
                                    ptr,
                                    ty: IRType::Int(Size::S32),
                                },
                            },
                        );
                        let one = self.current_func.as_mut().unwrap().new_vreg();
                        self.current_func.as_mut().unwrap().add_inst(
                            self.current_block,
                            IRInstruction {
                                id: Some(one),
                                op: IROp::ConstInt32(1),
                            },
                        );
                        let next_val = self.current_func.as_mut().unwrap().new_vreg();
                        let op = if operator == "++" {
                            IROp::Add(curr, one)
                        } else {
                            IROp::Sub(curr, one)
                        };
                        self.current_func.as_mut().unwrap().add_inst(
                            self.current_block,
                            IRInstruction {
                                id: Some(next_val),
                                op,
                            },
                        );
                        self.current_func.as_mut().unwrap().add_inst(
                            self.current_block,
                            IRInstruction {
                                id: None,
                                op: IROp::Store {
                                    ptr,
                                    value: next_val,
                                },
                            },
                        );
                        return curr;
                    }
                }
                self.visit_expr(left)
            }
            Expr::IfExpr {
                condition,
                then_branch,
                else_branch,
            } => {
                let cond_val = self.visit_expr(condition);
                let true_block = self.current_func.as_mut().unwrap().new_block();
                let false_block = self.current_func.as_mut().unwrap().new_block();
                let merge_block = self.current_func.as_mut().unwrap().new_block();

                let res_ptr = self.current_func.as_mut().unwrap().new_vreg();
                self.current_func.as_mut().unwrap().add_inst(
                    self.current_block,
                    IRInstruction {
                        id: Some(res_ptr),
                        op: IROp::Alloc {
                            ty: IRType::Int(Size::S32),
                        },
                    },
                );

                self.current_func.as_mut().unwrap().add_inst(
                    self.current_block,
                    IRInstruction {
                        id: None,
                        op: IROp::BranchIf {
                            cond: cond_val,
                            true_block,
                            false_block,
                        },
                    },
                );

                self.current_block = true_block;
                let true_val = self.visit_expr(then_branch);
                self.current_func.as_mut().unwrap().add_inst(
                    self.current_block,
                    IRInstruction {
                        id: None,
                        op: IROp::Store {
                            ptr: res_ptr,
                            value: true_val,
                        },
                    },
                );
                self.current_func.as_mut().unwrap().add_inst(
                    self.current_block,
                    IRInstruction {
                        id: None,
                        op: IROp::Jump(merge_block),
                    },
                );

                self.current_block = false_block;
                let false_val = self.visit_expr(else_branch);
                self.current_func.as_mut().unwrap().add_inst(
                    self.current_block,
                    IRInstruction {
                        id: None,
                        op: IROp::Store {
                            ptr: res_ptr,
                            value: false_val,
                        },
                    },
                );
                self.current_func.as_mut().unwrap().add_inst(
                    self.current_block,
                    IRInstruction {
                        id: None,
                        op: IROp::Jump(merge_block),
                    },
                );

                self.current_block = merge_block;
                let final_val = self.current_func.as_mut().unwrap().new_vreg();
                self.current_func.as_mut().unwrap().add_inst(
                    merge_block,
                    IRInstruction {
                        id: Some(final_val),
                        op: IROp::Load {
                            ptr: res_ptr,
                            ty: IRType::Int(Size::S32),
                        },
                    },
                );
                final_val
            }
            Expr::Cast { expr, .. } => self.visit_expr(expr),
            Expr::QuestionMark(inner) => self.visit_expr(inner),
            Expr::New { type_node, target } => {
                let raw_name = type_node.get_name();
                let struct_name = if !raw_name.is_empty() {
                    raw_name
                } else {
                    match &**target {
                        Expr::Identifier(n) => n.clone(),
                        Expr::Call { callee, .. } => {
                            if let Expr::Identifier(n) = &**callee {
                                n.clone()
                            } else {
                                String::new()
                            }
                        }
                        _ => String::new(),
                    }
                };
                let v = self.current_func.as_mut().unwrap().new_vreg();
                self.current_func.as_mut().unwrap().add_inst(
                    self.current_block,
                    IRInstruction {
                        id: Some(v),
                        op: IROp::Alloc {
                            ty: IRType::CustomScope(struct_name.clone()),
                        },
                    },
                );
                if let Expr::Call { args, .. } = &**target {
                    let constructor_name = format!("{}_new", struct_name);
                    let mut call_args = vec![v];
                    let mut call_tys = vec![IRType::Pointer(Box::new(IRType::CustomScope(
                        struct_name.clone(),
                    )))];
                    for a in args {
                        call_tys.push(self.infer_expr_type(a));
                        call_args.push(self.visit_expr(a));
                    }
                    self.called_functions
                        .entry(constructor_name.clone())
                        .or_insert(call_tys);
                    let call_res = self.current_func.as_mut().unwrap().new_vreg();
                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction {
                            id: Some(call_res),
                            op: IROp::Call {
                                func: constructor_name,
                                args: call_args,
                            },
                        },
                    );
                }
                v
            }
            Expr::Instantiate { target, args } => {
                let (struct_name, prop_name) = match &**target {
                    Expr::Identifier(n) => (n.clone(), n.clone()),
                    Expr::NamespaceAccess {
                        namespace,
                        property,
                    } => {
                        let p = if let Expr::Identifier(id) = &**property {
                            id.clone()
                        } else {
                            String::new()
                        };
                        (format!("{}_{}", namespace, p), p)
                    }
                    _ => (String::new(), String::new()),
                };

                let enum_variant_name = if self.enum_variant_tags.contains_key(&struct_name) {
                    Some(struct_name.clone())
                } else if self.enum_variant_tags.contains_key(&prop_name) {
                    Some(prop_name.clone())
                } else {
                    None
                };

                if let Some(variant_name) = enum_variant_name {
                    let field_order = self
                        .enum_variant_fields
                        .get(&variant_name)
                        .cloned()
                        .unwrap_or_default();
                    let mut field_vals: HashMap<String, IRValue> = HashMap::new();
                    for arg in args {
                        if let Expr::ObjectLiteral(stmts) = arg {
                            for s in stmts {
                                match s {
                                    Stmt::Declaration(Decl::VarDecl {
                                        name: f_name,
                                        value,
                                        ..
                                    }) => {
                                        let val = self.visit_expr(value);
                                        field_vals.insert(f_name.clone(), val);
                                    }
                                    Stmt::ReassignStmt {
                                        target: Expr::Identifier(f_name),
                                        value,
                                        ..
                                    } => {
                                        let val = self.visit_expr(value);
                                        field_vals.insert(f_name.clone(), val);
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    let mut call_args = Vec::new();
                    for f in &field_order {
                        if let Some(&v) = field_vals.get(f) {
                            call_args.push(v);
                        } else {
                            let zero = self.current_func.as_mut().unwrap().new_vreg();
                            self.current_func.as_mut().unwrap().add_inst(
                                self.current_block,
                                IRInstruction {
                                    id: Some(zero),
                                    op: IROp::ConstInt32(0),
                                },
                            );
                            call_args.push(zero);
                        }
                    }
                    let constructor_fn = if self.fn_signatures.contains_key(&struct_name) {
                        struct_name
                    } else {
                        variant_name
                    };
                    let v = self.current_func.as_mut().unwrap().new_vreg();
                    let arg_tys = vec![IRType::Int(Size::S32); call_args.len()];
                    self.called_functions
                        .entry(constructor_fn.clone())
                        .or_insert_with(|| arg_tys);
                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction {
                            id: Some(v),
                            op: IROp::Call {
                                func: constructor_fn,
                                args: call_args,
                            },
                        },
                    );
                    return v;
                }

                let obj_ptr = self.current_func.as_mut().unwrap().new_vreg();
                self.current_func.as_mut().unwrap().add_inst(
                    self.current_block,
                    IRInstruction {
                        id: Some(obj_ptr),
                        op: IROp::Alloc {
                            ty: IRType::CustomScope(struct_name.clone()),
                        },
                    },
                );

                if let Some(st) = self.structs.get(&struct_name).cloned() {
                    for arg in args {
                        if let Expr::ObjectLiteral(stmts) = arg {
                            for s in stmts {
                                match s {
                                    Stmt::Declaration(Decl::VarDecl {
                                        name: f_name,
                                        value,
                                        ..
                                    }) => {
                                        if let Some((_, _, offset)) =
                                            st.fields.iter().find(|(n, _, _)| n == f_name)
                                        {
                                            let val = self.visit_expr(value);
                                            let field_ptr =
                                                self.current_func.as_mut().unwrap().new_vreg();
                                            self.current_func.as_mut().unwrap().add_inst(
                                                self.current_block,
                                                IRInstruction {
                                                    id: Some(field_ptr),
                                                    op: IROp::GetFieldPtr {
                                                        ptr: obj_ptr,
                                                        offset: *offset as i32,
                                                    },
                                                },
                                            );
                                            self.current_func.as_mut().unwrap().add_inst(
                                                self.current_block,
                                                IRInstruction {
                                                    id: None,
                                                    op: IROp::StoreMemory {
                                                        ptr: field_ptr,
                                                        value: val,
                                                    },
                                                },
                                            );
                                        }
                                    }
                                    Stmt::ReassignStmt {
                                        target: Expr::Identifier(f_name),
                                        value,
                                        ..
                                    } => {
                                        if let Some((_, _, offset)) =
                                            st.fields.iter().find(|(n, _, _)| n == f_name)
                                        {
                                            let val = self.visit_expr(value);
                                            let field_ptr =
                                                self.current_func.as_mut().unwrap().new_vreg();
                                            self.current_func.as_mut().unwrap().add_inst(
                                                self.current_block,
                                                IRInstruction {
                                                    id: Some(field_ptr),
                                                    op: IROp::GetFieldPtr {
                                                        ptr: obj_ptr,
                                                        offset: *offset as i32,
                                                    },
                                                },
                                            );
                                            self.current_func.as_mut().unwrap().add_inst(
                                                self.current_block,
                                                IRInstruction {
                                                    id: None,
                                                    op: IROp::StoreMemory {
                                                        ptr: field_ptr,
                                                        value: val,
                                                    },
                                                },
                                            );
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                obj_ptr
            }
            Expr::BinaryOp {
                left,
                operator,
                right,
            } => {
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
                    "&&" | "&" => IROp::And(l, r),
                    "||" | "|" => IROp::Or(l, r),
                    _ => IROp::Add(l, r),
                };

                self.current_func
                    .as_mut()
                    .unwrap()
                    .add_inst(self.current_block, IRInstruction { id: Some(v), op });
                v
            }
            Expr::Call { callee, args, .. } => {
                if let Expr::Identifier(func_name) = &**callee {
                    if func_name == "println" || func_name == "print" {
                        for arg in args {
                            let arg_ty = self.infer_expr_type(arg);
                            let arg_val = self.visit_expr(arg);
                            let helper_name = match arg_ty {
                                IRType::Pointer(_) | IRType::Array(_) => "fast_print_str",
                                IRType::Float(_) => "fast_print_double",
                                IRType::Bool => "fast_print_bool",
                                IRType::Char => "fast_print_char",
                                IRType::UInt(_) => "fast_print_uint",
                                _ => "fast_print_int",
                            };
                            let dummy = self.current_func.as_mut().unwrap().new_vreg();
                            self.called_functions
                                .entry(helper_name.to_string())
                                .or_insert_with(|| vec![arg_ty]);
                            self.current_func.as_mut().unwrap().add_inst(
                                self.current_block,
                                IRInstruction {
                                    id: Some(dummy),
                                    op: IROp::Call {
                                        func: helper_name.to_string(),
                                        args: vec![arg_val],
                                    },
                                },
                            );
                        }
                        if func_name == "println" {
                            let empty_str = self.current_func.as_mut().unwrap().new_vreg();
                            self.current_func.as_mut().unwrap().add_inst(
                                self.current_block,
                                IRInstruction {
                                    id: Some(empty_str),
                                    op: IROp::ConstString("".to_string()),
                                },
                            );
                            let dummy = self.current_func.as_mut().unwrap().new_vreg();
                            self.called_functions
                                .entry("fast_println".to_string())
                                .or_insert_with(|| {
                                    vec![IRType::Pointer(Box::new(IRType::Int(Size::S8)))]
                                });
                            self.current_func.as_mut().unwrap().add_inst(
                                self.current_block,
                                IRInstruction {
                                    id: Some(dummy),
                                    op: IROp::Call {
                                        func: "fast_println".to_string(),
                                        args: vec![empty_str],
                                    },
                                },
                            );
                        }
                        let v = self.current_func.as_mut().unwrap().new_vreg();
                        self.current_func.as_mut().unwrap().add_inst(
                            self.current_block,
                            IRInstruction {
                                id: Some(v),
                                op: IROp::ConstInt32(0),
                            },
                        );
                        return v;
                    }

                    let mut ir_args = Vec::new();
                    let mut arg_types = Vec::new();
                    for arg in args {
                        arg_types.push(self.infer_expr_type(arg));
                        ir_args.push(self.visit_expr(arg));
                    }
                    let resolved_func = self.resolve_callee(func_name, &arg_types);
                    if let Some(defaults) = self.fn_param_defaults.get(&resolved_func).cloned() {
                        for default_opt in defaults.into_iter().skip(args.len()) {
                            if let Some(def_expr) = default_opt {
                                ir_args.push(self.visit_expr(&def_expr));
                            }
                        }
                    }
                    let v = self.current_func.as_mut().unwrap().new_vreg();
                    self.called_functions
                        .entry(resolved_func.clone())
                        .or_insert_with(|| arg_types.clone());
                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction {
                            id: Some(v),
                            op: IROp::Call {
                                func: resolved_func,
                                args: ir_args,
                            },
                        },
                    );
                    v
                } else if let Expr::PropertyAccess { object, property } = &**callee {
                    let (obj_ptr, struct_name) = self
                        .eval_object_ptr_and_type(object)
                        .unwrap_or((0, String::new()));
                    let mangled_name = format!("{}_{}", struct_name, property);
                    let mut ir_args = vec![obj_ptr];
                    let mut arg_types = vec![IRType::Pointer(Box::new(IRType::CustomScope(
                        struct_name.clone(),
                    )))];
                    for arg in args {
                        arg_types.push(self.infer_expr_type(arg));
                        ir_args.push(self.visit_expr(arg));
                    }
                    let v = self.current_func.as_mut().unwrap().new_vreg();
                    self.called_functions
                        .entry(mangled_name.clone())
                        .or_insert_with(|| arg_types);
                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction {
                            id: Some(v),
                            op: IROp::Call {
                                func: mangled_name,
                                args: ir_args,
                            },
                        },
                    );
                    v
                } else if let Expr::NamespaceAccess {
                    namespace,
                    property,
                } = &**callee
                {
                    let prop_name = if let Expr::Identifier(p) = &**property {
                        p.clone()
                    } else {
                        String::new()
                    };
                    if namespace == "@compile" {
                        if prop_name == "backend" {
                            let val = if self.target_backend == "cranelift" {
                                1
                            } else {
                                0
                            };
                            let v = self.current_func.as_mut().unwrap().new_vreg();
                            self.current_func.as_mut().unwrap().add_inst(
                                self.current_block,
                                IRInstruction {
                                    id: Some(v),
                                    op: IROp::ConstInt32(val),
                                },
                            );
                            return v;
                        } else if prop_name == "system" {
                            let val = if cfg!(target_os = "windows") {
                                1
                            } else if cfg!(target_os = "macos") {
                                2
                            } else {
                                0
                            };
                            let v = self.current_func.as_mut().unwrap().new_vreg();
                            self.current_func.as_mut().unwrap().add_inst(
                                self.current_block,
                                IRInstruction {
                                    id: Some(v),
                                    op: IROp::ConstInt32(val),
                                },
                            );
                            return v;
                        }
                    }
                    let qualified = format!("{}_{}", namespace, prop_name);
                    let mut ir_args = Vec::new();
                    let mut arg_types = Vec::new();
                    for arg in args {
                        arg_types.push(self.infer_expr_type(arg));
                        ir_args.push(self.visit_expr(arg));
                    }
                    let resolved_func = if namespace == "@sys" {
                        match prop_name.as_str() {
                            "alloc" => "fast_alloc".to_string(),
                            "free" => "fast_free".to_string(),
                            "open" => "fast_open".to_string(),
                            "close" => "fast_close".to_string(),
                            "write" => "fast_write".to_string(),
                            "read" => "fast_read".to_string(),
                            "exit" => "fast_exit".to_string(),
                            _ => format!("fast_{}", prop_name),
                        }
                    } else if self.fn_signatures.contains_key(&qualified) {
                        self.resolve_callee(&qualified, &arg_types)
                    } else {
                        self.resolve_callee(&prop_name, &arg_types)
                    };
                    let v = self.current_func.as_mut().unwrap().new_vreg();
                    self.called_functions
                        .entry(resolved_func.clone())
                        .or_insert_with(|| arg_types.clone());
                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction {
                            id: Some(v),
                            op: IROp::Call {
                                func: resolved_func,
                                args: ir_args,
                            },
                        },
                    );
                    v
                } else {
                    let v = self.current_func.as_mut().unwrap().new_vreg();
                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction {
                            id: Some(v),
                            op: IROp::ConstInt32(0),
                        },
                    );
                    v
                }
            }
            Expr::NamespaceAccess {
                namespace,
                property,
            } => {
                let prop_name = if let Expr::Identifier(p) = &**property {
                    p.clone()
                } else {
                    String::new()
                };
                if namespace == "Backend" {
                    let val = if prop_name == "Cranelift" { 1 } else { 0 };
                    let v = self.current_func.as_mut().unwrap().new_vreg();
                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction {
                            id: Some(v),
                            op: IROp::ConstInt32(val),
                        },
                    );
                    return v;
                } else if namespace == "System" {
                    let val = if prop_name == "Windows" {
                        1
                    } else if prop_name == "MacOS" {
                        2
                    } else {
                        0
                    };
                    let v = self.current_func.as_mut().unwrap().new_vreg();
                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction {
                            id: Some(v),
                            op: IROp::ConstInt32(val),
                        },
                    );
                    return v;
                }
                let qualified = format!("{}_{}", namespace, prop_name);
                let resolved_func = if self.fn_signatures.contains_key(&qualified) {
                    Some(qualified)
                } else if self.fn_signatures.contains_key(&prop_name) {
                    Some(prop_name)
                } else {
                    None
                };
                if let Some(fn_name) = resolved_func {
                    let v = self.current_func.as_mut().unwrap().new_vreg();
                    self.called_functions
                        .entry(fn_name.clone())
                        .or_insert_with(Vec::new);
                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction {
                            id: Some(v),
                            op: IROp::Call {
                                func: fn_name,
                                args: vec![],
                            },
                        },
                    );
                    return v;
                }
                let v = self.current_func.as_mut().unwrap().new_vreg();
                self.current_func.as_mut().unwrap().add_inst(
                    self.current_block,
                    IRInstruction {
                        id: Some(v),
                        op: IROp::ConstInt32(0),
                    },
                );
                v
            }
            _ => {
                // Fallback for unsupported expressions returning a dummy value
                let v = self.current_func.as_mut().unwrap().new_vreg();
                self.current_func.as_mut().unwrap().add_inst(
                    self.current_block,
                    IRInstruction {
                        id: Some(v),
                        op: IROp::ConstInt32(0),
                    },
                );
                v
            }
        }
    }
}
