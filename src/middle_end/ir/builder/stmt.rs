use super::IRBuilder;
use crate::frontend::parser::ast::*;
use crate::middle_end::ir::instruction::*;

impl<'a> IRBuilder<'a> {
    pub(crate) fn visit_block(&mut self, stmts: &[Stmt]) {
        self.push_scope();
        for stmt in stmts {
            self.visit_stmt(stmt);
        }
        self.pop_scope();
    }

    pub(crate) fn visit_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Declaration(Decl::VarDecl {
                name,
                type_node,
                value,
                ..
            }) => {
                let inferred_ty = if matches!(type_node, BaseType::Auto | BaseType::Unknown) {
                    self.infer_expr_type(value)
                } else {
                    IRType::from_ast(&type_node)
                };
                let type_name = if matches!(type_node, BaseType::Auto | BaseType::Unknown) {
                    match &inferred_ty {
                        IRType::Float(_) => "float64".to_string(),
                        IRType::Bool => "bool".to_string(),
                        IRType::Char => "char".to_string(),
                        IRType::CustomScope(s) => s.clone(),
                        IRType::UInt(Size::S8) => "uint8".to_string(),
                        IRType::UInt(Size::S16) => "uint16".to_string(),
                        IRType::UInt(Size::S32) => "uint32".to_string(),
                        IRType::UInt(Size::S64) => "uint64".to_string(),
                        IRType::Int(Size::S8) => "int8".to_string(),
                        IRType::Int(Size::S16) => "int16".to_string(),
                        IRType::Int(Size::S32) => "int32".to_string(),
                        IRType::Int(Size::S64) => "int64".to_string(),
                        _ => "int32".to_string(),
                    }
                } else {
                    type_node.get_name()
                };
                let ptr = self.current_func.as_mut().unwrap().new_vreg();

                self.current_func.as_mut().unwrap().add_inst(
                    self.current_block,
                    IRInstruction {
                        id: Some(ptr),
                        op: IROp::Alloc { ty: inferred_ty },
                    },
                );

                self.declare_var(name.clone(), ptr);
                self.var_types.insert(name.clone(), type_name);

                match value {
                    Expr::LiteralVoid | Expr::Default(_) => {}
                    _ => {
                        let val = self.visit_expr(value);
                        self.current_func.as_mut().unwrap().add_inst(
                            self.current_block,
                            IRInstruction {
                                id: None,
                                op: IROp::Store { ptr, value: val },
                            },
                        );
                    }
                }
            }
            Stmt::Declaration(Decl::DestructureDecl {
                assignments,
                type_node,
                ..
            }) => {
                for (name, expr) in assignments {
                    let inferred_ty = if matches!(type_node, BaseType::Auto | BaseType::Unknown) {
                        self.infer_expr_type(expr)
                    } else {
                        IRType::from_ast(&type_node)
                    };
                    let type_name = if matches!(type_node, BaseType::Auto | BaseType::Unknown) {
                        match &inferred_ty {
                            IRType::Float(_) => "float64".to_string(),
                            IRType::Bool => "bool".to_string(),
                            IRType::Char => "char".to_string(),
                            IRType::CustomScope(s) => s.clone(),
                            IRType::UInt(Size::S8) => "uint8".to_string(),
                            IRType::UInt(Size::S16) => "uint16".to_string(),
                            IRType::UInt(Size::S32) => "uint32".to_string(),
                            IRType::UInt(Size::S64) => "uint64".to_string(),
                            IRType::Int(Size::S8) => "int8".to_string(),
                            IRType::Int(Size::S16) => "int16".to_string(),
                            IRType::Int(Size::S32) => "int32".to_string(),
                            IRType::Int(Size::S64) => "int64".to_string(),
                            _ => "int32".to_string(),
                        }
                    } else {
                        type_node.get_name()
                    };
                    let ptr = self.current_func.as_mut().unwrap().new_vreg();
                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction {
                            id: Some(ptr),
                            op: IROp::Alloc { ty: inferred_ty },
                        },
                    );
                    self.declare_var(name.clone(), ptr);
                    self.var_types.insert(name.clone(), type_name);

                    match expr {
                        Expr::LiteralVoid | Expr::Default(_) => {}
                        _ => {
                            let val = self.visit_expr(expr);
                            self.current_func.as_mut().unwrap().add_inst(
                                self.current_block,
                                IRInstruction {
                                    id: None,
                                    op: IROp::Store { ptr, value: val },
                                },
                            );
                        }
                    }
                }
            }
            Stmt::Declaration(Decl::ObjectDestructureDecl {
                fields,
                type_name,
                rhs,
                ..
            }) => {
                let obj_val = self.visit_expr(rhs);
                let mut st_name = type_name.clone();
                if st_name.is_none() {
                    let rhs_ty = self.infer_expr_type(rhs);
                    if let IRType::CustomScope(s) = rhs_ty {
                        st_name = Some(s);
                    }
                }
                let st_opt = st_name
                    .as_ref()
                    .and_then(|tn| self.structs.get(tn).cloned());
                for (idx, (field_type, var_name)) in fields.iter().enumerate() {
                    let inferred_ty = if matches!(field_type, BaseType::Auto | BaseType::Unknown) {
                        if let Some(st) = &st_opt {
                            st.fields
                                .get(idx)
                                .map(|(_, ty, _)| ty.clone())
                                .unwrap_or(IRType::Int(Size::S32))
                        } else {
                            IRType::Int(Size::S32)
                        }
                    } else {
                        IRType::from_ast(field_type)
                    };
                    let offset = if let Some(st) = &st_opt {
                        st.fields
                            .get(idx)
                            .map(|(_, _, off)| *off)
                            .unwrap_or(idx * 4)
                    } else {
                        idx * 4
                    };

                    let field_ptr = self.current_func.as_mut().unwrap().new_vreg();
                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction {
                            id: Some(field_ptr),
                            op: IROp::GetFieldPtr {
                                ptr: obj_val,
                                offset: offset as i32,
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
                                ty: inferred_ty.clone(),
                            },
                        },
                    );
                    let var_ptr = self.current_func.as_mut().unwrap().new_vreg();
                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction {
                            id: Some(var_ptr),
                            op: IROp::Alloc {
                                ty: inferred_ty.clone(),
                            },
                        },
                    );
                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction {
                            id: None,
                            op: IROp::Store {
                                ptr: var_ptr,
                                value: val,
                            },
                        },
                    );
                    self.declare_var(var_name.clone(), var_ptr);
                    self.var_types
                        .insert(var_name.clone(), field_type.get_name());
                }
            }
            Stmt::Declaration(Decl::ArrayDecl {
                name,
                type_node,
                length: _,
                value,
                ..
            }) => {
                let elem_ty = IRType::from_ast(&type_node);
                let elem_size = elem_ty.size_in_bytes();
                let array_ptr = self.current_func.as_mut().unwrap().new_vreg();

                let (arr_len, elems) = match value {
                    Expr::ArrayLiteral(elements) => (elements.len(), elements.as_slice()),
                    _ => (1, [].as_slice()),
                };
                let size_in_bytes = if arr_len == 0 {
                    elem_size
                } else {
                    arr_len * elem_size
                };

                self.current_func.as_mut().unwrap().add_inst(
                    self.current_block,
                    IRInstruction {
                        id: Some(array_ptr),
                        op: IROp::AllocArray {
                            elem_ty: elem_ty.clone(),
                            size: size_in_bytes,
                        },
                    },
                );

                self.declare_var(name.clone(), array_ptr);
                self.var_types
                    .insert(name.clone(), format!("array<{}>", type_node.get_name()));
                self.array_elem_types.insert(name.clone(), elem_ty.clone());
                self.array_lens.insert(name.clone(), arr_len);

                for (idx, el) in elems.iter().enumerate() {
                    let el_val = self.visit_expr(el);
                    let idx_vreg = self.current_func.as_mut().unwrap().new_vreg();
                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction {
                            id: Some(idx_vreg),
                            op: IROp::ConstInt32(idx as i32),
                        },
                    );
                    let elem_ptr = self.current_func.as_mut().unwrap().new_vreg();
                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction {
                            id: Some(elem_ptr),
                            op: IROp::GetElementPtr {
                                base_ptr: array_ptr,
                                index: idx_vreg,
                                elem_size,
                            },
                        },
                    );
                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction {
                            id: None,
                            op: IROp::StoreMemory {
                                ptr: elem_ptr,
                                value: el_val,
                            },
                        },
                    );
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
                                    IRInstruction {
                                        id: Some(curr),
                                        op: IROp::Load {
                                            ptr,
                                            ty: IRType::Int(Size::S32),
                                        },
                                    },
                                );
                                let res = self.current_func.as_mut().unwrap().new_vreg();
                                self.current_func.as_mut().unwrap().add_inst(
                                    self.current_block,
                                    IRInstruction {
                                        id: Some(res),
                                        op: IROp::Add(curr, val),
                                    },
                                );
                                res
                            }
                            "-=" => {
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
                                let res = self.current_func.as_mut().unwrap().new_vreg();
                                self.current_func.as_mut().unwrap().add_inst(
                                    self.current_block,
                                    IRInstruction {
                                        id: Some(res),
                                        op: IROp::Sub(curr, val),
                                    },
                                );
                                res
                            }
                            "*=" => {
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
                                let res = self.current_func.as_mut().unwrap().new_vreg();
                                self.current_func.as_mut().unwrap().add_inst(
                                    self.current_block,
                                    IRInstruction {
                                        id: Some(res),
                                        op: IROp::Mul(curr, val),
                                    },
                                );
                                res
                            }
                            _ => val,
                        };
                        self.current_func.as_mut().unwrap().add_inst(
                            self.current_block,
                            IRInstruction {
                                id: None,
                                op: IROp::Store {
                                    ptr,
                                    value: final_val,
                                },
                            },
                        );
                    }
                } else if let Expr::PropertyAccess { object, property } = target {
                    if let Some((obj_ptr, struct_name)) = self.eval_object_ptr_and_type(object) {
                        if let Some(st) = self.structs.get(&struct_name).cloned() {
                            if let Some((_, _field_ty, offset)) =
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
                    }
                } else if let Expr::IndexAccess { object, indices } = target {
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
                    if !indices.is_empty() {
                        let idx_val = self.visit_expr(&indices[0]);
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
                        self.current_func.as_mut().unwrap().add_inst(
                            self.current_block,
                            IRInstruction {
                                id: None,
                                op: IROp::StoreMemory {
                                    ptr: elem_ptr,
                                    value: val,
                                },
                            },
                        );
                    }
                }
            }
            Stmt::IfStmt {
                condition,
                then_block,
                else_block,
            } => {
                let cond_val = self.visit_expr(condition);
                let true_block = self.current_func.as_mut().unwrap().new_block();
                let false_block = self.current_func.as_mut().unwrap().new_block();
                let merge_block = self.current_func.as_mut().unwrap().new_block();

                let false_target = if else_block.is_some() {
                    false_block
                } else {
                    merge_block
                };

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
                    IRInstruction {
                        id: None,
                        op: IROp::Jump(merge_block),
                    },
                );

                // Build else branch if present
                if let Some(else_stmts) = else_block {
                    self.current_block = false_block;
                    self.visit_block(else_stmts);
                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction {
                            id: None,
                            op: IROp::Jump(merge_block),
                        },
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
                    IRInstruction {
                        id: None,
                        op: IROp::Jump(header_block),
                    },
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

                self.loop_stack.push((header_block, exit_block));
                self.current_block = body_block;
                match body {
                    crate::frontend::parser::ast::EitherBlock::Inline(stmts) => {
                        self.visit_block(stmts)
                    }
                    crate::frontend::parser::ast::EitherBlock::External(expr) => {
                        self.visit_expr(expr);
                    }
                }
                self.current_func.as_mut().unwrap().add_inst(
                    self.current_block,
                    IRInstruction {
                        id: None,
                        op: IROp::Jump(header_block),
                    },
                );
                self.loop_stack.pop();

                self.current_block = exit_block;
            }
            Stmt::DoWhileStmt { body, condition } => {
                let body_block = self.current_func.as_mut().unwrap().new_block();
                let cond_block = self.current_func.as_mut().unwrap().new_block();
                let exit_block = self.current_func.as_mut().unwrap().new_block();

                self.current_func.as_mut().unwrap().add_inst(
                    self.current_block,
                    IRInstruction {
                        id: None,
                        op: IROp::Jump(body_block),
                    },
                );

                self.loop_stack.push((cond_block, exit_block));
                self.current_block = body_block;
                match body {
                    crate::frontend::parser::ast::EitherBlock::Inline(stmts) => {
                        self.visit_block(stmts)
                    }
                    crate::frontend::parser::ast::EitherBlock::External(expr) => {
                        self.visit_expr(expr);
                    }
                }
                self.current_func.as_mut().unwrap().add_inst(
                    self.current_block,
                    IRInstruction {
                        id: None,
                        op: IROp::Jump(cond_block),
                    },
                );
                self.loop_stack.pop();

                self.current_block = cond_block;
                let cond_val = self.visit_expr(condition);
                self.current_func.as_mut().unwrap().add_inst(
                    cond_block,
                    IRInstruction {
                        id: None,
                        op: IROp::BranchIf {
                            cond: cond_val,
                            true_block: body_block,
                            false_block: exit_block,
                        },
                    },
                );

                self.current_block = exit_block;
            }
            Stmt::SwitchStmt {
                condition, cases, ..
            } => {
                let cond_val = self.visit_expr(condition);
                let merge_block = self.current_func.as_mut().unwrap().new_block();

                fn collect_alts(expr: &Expr, out: &mut Vec<Expr>) {
                    if let Expr::BinaryOp {
                        operator,
                        left,
                        right,
                    } = expr
                    {
                        if operator == "|" {
                            collect_alts(left, out);
                            collect_alts(right, out);
                            return;
                        }
                    }
                    out.push(expr.clone());
                }

                let is_enum_match = cases.iter().any(|c| {
                    if let Stmt::CaseStmt { option, .. } = c {
                        match option {
                            Expr::Call { callee, .. } => {
                                if let Expr::Identifier(n) = &**callee {
                                    self.enum_variant_tags.contains_key(n)
                                } else {
                                    false
                                }
                            }
                            Expr::Instantiate { target, .. } => {
                                let name = match &**target {
                                    Expr::Identifier(n) => n.clone(),
                                    Expr::NamespaceAccess { property, .. } => {
                                        if let Expr::Identifier(p) = &**property {
                                            p.clone()
                                        } else {
                                            String::new()
                                        }
                                    }
                                    _ => String::new(),
                                };
                                self.enum_variant_tags.contains_key(&name)
                            }
                            Expr::Identifier(n) => self.enum_variant_tags.contains_key(n),
                            Expr::NamespaceAccess { property, .. } => {
                                if let Expr::Identifier(p) = &**property {
                                    self.enum_variant_tags.contains_key(p)
                                } else {
                                    false
                                }
                            }
                            _ => false,
                        }
                    } else {
                        false
                    }
                });

                let tag_val = if is_enum_match {
                    let tv = self.current_func.as_mut().unwrap().new_vreg();
                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction {
                            id: Some(tv),
                            op: IROp::LoadMemory {
                                ptr: cond_val,
                                ty: IRType::Int(Size::S64),
                            },
                        },
                    );
                    Some(tv)
                } else {
                    None
                };

                for case in cases {
                    if let Stmt::CaseStmt { option, body, .. } = case {
                        let is_default = match option {
                            Expr::Identifier(n) => n == "void" || n == "_",
                            _ => false,
                        };

                        if is_default {
                            let default_block = self.current_func.as_mut().unwrap().new_block();
                            self.current_func.as_mut().unwrap().add_inst(
                                self.current_block,
                                IRInstruction {
                                    id: None,
                                    op: IROp::Jump(default_block),
                                },
                            );
                            self.current_block = default_block;
                            self.visit_block(body);
                            self.current_func.as_mut().unwrap().add_inst(
                                self.current_block,
                                IRInstruction {
                                    id: None,
                                    op: IROp::Jump(merge_block),
                                },
                            );
                            break;
                        }

                        let mut alternatives = Vec::new();
                        collect_alts(option, &mut alternatives);

                        if alternatives.len() > 1 {
                            let case_body_block = self.current_func.as_mut().unwrap().new_block();
                            let next_check_block = self.current_func.as_mut().unwrap().new_block();

                            let mut combined_cmp = None;
                            for alt in alternatives {
                                let alt_val = self.visit_expr(&alt);
                                let cmp = self.current_func.as_mut().unwrap().new_vreg();
                                self.current_func.as_mut().unwrap().add_inst(
                                    self.current_block,
                                    IRInstruction {
                                        id: Some(cmp),
                                        op: IROp::Eq(cond_val, alt_val),
                                    },
                                );
                                if let Some(prev) = combined_cmp {
                                    let or_v = self.current_func.as_mut().unwrap().new_vreg();
                                    self.current_func.as_mut().unwrap().add_inst(
                                        self.current_block,
                                        IRInstruction {
                                            id: Some(or_v),
                                            op: IROp::Or(prev, cmp),
                                        },
                                    );
                                    combined_cmp = Some(or_v);
                                } else {
                                    combined_cmp = Some(cmp);
                                }
                            }

                            self.current_func.as_mut().unwrap().add_inst(
                                self.current_block,
                                IRInstruction {
                                    id: None,
                                    op: IROp::BranchIf {
                                        cond: combined_cmp.unwrap(),
                                        true_block: case_body_block,
                                        false_block: next_check_block,
                                    },
                                },
                            );

                            self.current_block = case_body_block;
                            self.visit_block(body);
                            self.current_func.as_mut().unwrap().add_inst(
                                self.current_block,
                                IRInstruction {
                                    id: None,
                                    op: IROp::Jump(merge_block),
                                },
                            );
                            self.current_block = next_check_block;
                            continue;
                        }

                        // Pattern: variant with tuple payload, e.g. Pending(code)
                        if let Expr::Call { callee, args, .. } = option {
                            let var_name = if let Expr::Identifier(n) = &**callee {
                                n.clone()
                            } else if let Expr::NamespaceAccess { property, .. } = &**callee {
                                if let Expr::Identifier(p) = &**property {
                                    p.clone()
                                } else {
                                    String::new()
                                }
                            } else {
                                String::new()
                            };

                            if let Some(&tag_idx) = self.enum_variant_tags.get(&var_name) {
                                let case_body_block =
                                    self.current_func.as_mut().unwrap().new_block();
                                let next_check_block =
                                    self.current_func.as_mut().unwrap().new_block();

                                let curr_tag = tag_val.unwrap_or(cond_val);
                                let expected_tag = self.current_func.as_mut().unwrap().new_vreg();
                                self.current_func.as_mut().unwrap().add_inst(
                                    self.current_block,
                                    IRInstruction {
                                        id: Some(expected_tag),
                                        op: IROp::ConstInt64(tag_idx as i64),
                                    },
                                );
                                let cmp = self.current_func.as_mut().unwrap().new_vreg();
                                self.current_func.as_mut().unwrap().add_inst(
                                    self.current_block,
                                    IRInstruction {
                                        id: Some(cmp),
                                        op: IROp::Eq(curr_tag, expected_tag),
                                    },
                                );
                                self.current_func.as_mut().unwrap().add_inst(
                                    self.current_block,
                                    IRInstruction {
                                        id: None,
                                        op: IROp::BranchIf {
                                            cond: cmp,
                                            true_block: case_body_block,
                                            false_block: next_check_block,
                                        },
                                    },
                                );

                                self.current_block = case_body_block;
                                self.push_scope();
                                for (p_idx, arg_expr) in args.iter().enumerate() {
                                    if let Expr::Identifier(bind_name) = arg_expr {
                                        let field_offset = 8 + (p_idx * 8);
                                        let field_ptr =
                                            self.current_func.as_mut().unwrap().new_vreg();
                                        self.current_func.as_mut().unwrap().add_inst(
                                            self.current_block,
                                            IRInstruction {
                                                id: Some(field_ptr),
                                                op: IROp::GetFieldPtr {
                                                    ptr: cond_val,
                                                    offset: field_offset as i32,
                                                },
                                            },
                                        );
                                        let payload_val =
                                            self.current_func.as_mut().unwrap().new_vreg();
                                        self.current_func.as_mut().unwrap().add_inst(
                                            self.current_block,
                                            IRInstruction {
                                                id: Some(payload_val),
                                                op: IROp::LoadMemory {
                                                    ptr: field_ptr,
                                                    ty: IRType::Int(Size::S32),
                                                },
                                            },
                                        );
                                        let var_ptr =
                                            self.current_func.as_mut().unwrap().new_vreg();
                                        self.current_func.as_mut().unwrap().add_inst(
                                            self.current_block,
                                            IRInstruction {
                                                id: Some(var_ptr),
                                                op: IROp::Alloc {
                                                    ty: IRType::Int(Size::S32),
                                                },
                                            },
                                        );
                                        self.current_func.as_mut().unwrap().add_inst(
                                            self.current_block,
                                            IRInstruction {
                                                id: None,
                                                op: IROp::Store {
                                                    ptr: var_ptr,
                                                    value: payload_val,
                                                },
                                            },
                                        );
                                        self.declare_var(bind_name.clone(), var_ptr);
                                        self.var_types
                                            .insert(bind_name.clone(), "int32".to_string());
                                    }
                                }
                                for stmt in body {
                                    self.visit_stmt(stmt);
                                }
                                self.pop_scope();
                                self.current_func.as_mut().unwrap().add_inst(
                                    self.current_block,
                                    IRInstruction {
                                        id: None,
                                        op: IROp::Jump(merge_block),
                                    },
                                );
                                self.current_block = next_check_block;
                                continue;
                            }
                        }

                        // Pattern: variant with struct payload, e.g. AppEvent::Click { x, y }
                        if let Expr::Instantiate { target, args } = option {
                            let var_name = match &**target {
                                Expr::Identifier(n) => n.clone(),
                                Expr::NamespaceAccess { property, .. } => {
                                    if let Expr::Identifier(p) = &**property {
                                        p.clone()
                                    } else {
                                        String::new()
                                    }
                                }
                                _ => String::new(),
                            };

                            if let Some(&tag_idx) = self.enum_variant_tags.get(&var_name) {
                                let case_body_block =
                                    self.current_func.as_mut().unwrap().new_block();
                                let next_check_block =
                                    self.current_func.as_mut().unwrap().new_block();

                                let curr_tag = tag_val.unwrap_or(cond_val);
                                let expected_tag = self.current_func.as_mut().unwrap().new_vreg();
                                self.current_func.as_mut().unwrap().add_inst(
                                    self.current_block,
                                    IRInstruction {
                                        id: Some(expected_tag),
                                        op: IROp::ConstInt64(tag_idx as i64),
                                    },
                                );
                                let cmp = self.current_func.as_mut().unwrap().new_vreg();
                                self.current_func.as_mut().unwrap().add_inst(
                                    self.current_block,
                                    IRInstruction {
                                        id: Some(cmp),
                                        op: IROp::Eq(curr_tag, expected_tag),
                                    },
                                );
                                self.current_func.as_mut().unwrap().add_inst(
                                    self.current_block,
                                    IRInstruction {
                                        id: None,
                                        op: IROp::BranchIf {
                                            cond: cmp,
                                            true_block: case_body_block,
                                            false_block: next_check_block,
                                        },
                                    },
                                );

                                self.current_block = case_body_block;
                                self.push_scope();

                                let field_names = self
                                    .enum_variant_fields
                                    .get(&var_name)
                                    .cloned()
                                    .unwrap_or_default();
                                for arg in args {
                                    if let Expr::ObjectLiteral(stmts) = arg {
                                        for s in stmts {
                                            let bind_name = match s {
                                                Stmt::ExpressionStmt(Expr::Identifier(n)) => {
                                                    n.clone()
                                                }
                                                Stmt::Declaration(Decl::VarDecl {
                                                    name: n,
                                                    ..
                                                }) => n.clone(),
                                                _ => String::new(),
                                            };
                                            if !bind_name.is_empty() {
                                                let p_idx = field_names
                                                    .iter()
                                                    .position(|f| f == &bind_name)
                                                    .unwrap_or(0);
                                                let field_offset = 8 + (p_idx * 8);
                                                let field_ptr =
                                                    self.current_func.as_mut().unwrap().new_vreg();
                                                self.current_func.as_mut().unwrap().add_inst(
                                                    self.current_block,
                                                    IRInstruction {
                                                        id: Some(field_ptr),
                                                        op: IROp::GetFieldPtr {
                                                            ptr: cond_val,
                                                            offset: field_offset as i32,
                                                        },
                                                    },
                                                );
                                                let payload_val =
                                                    self.current_func.as_mut().unwrap().new_vreg();
                                                self.current_func.as_mut().unwrap().add_inst(
                                                    self.current_block,
                                                    IRInstruction {
                                                        id: Some(payload_val),
                                                        op: IROp::LoadMemory {
                                                            ptr: field_ptr,
                                                            ty: IRType::Int(Size::S32),
                                                        },
                                                    },
                                                );
                                                let var_ptr =
                                                    self.current_func.as_mut().unwrap().new_vreg();
                                                self.current_func.as_mut().unwrap().add_inst(
                                                    self.current_block,
                                                    IRInstruction {
                                                        id: Some(var_ptr),
                                                        op: IROp::Alloc {
                                                            ty: IRType::Int(Size::S32),
                                                        },
                                                    },
                                                );
                                                self.current_func.as_mut().unwrap().add_inst(
                                                    self.current_block,
                                                    IRInstruction {
                                                        id: None,
                                                        op: IROp::Store {
                                                            ptr: var_ptr,
                                                            value: payload_val,
                                                        },
                                                    },
                                                );
                                                self.declare_var(bind_name.clone(), var_ptr);
                                                self.var_types
                                                    .insert(bind_name.clone(), "int32".to_string());
                                            }
                                        }
                                    }
                                }

                                for stmt in body {
                                    self.visit_stmt(stmt);
                                }
                                self.pop_scope();
                                self.current_func.as_mut().unwrap().add_inst(
                                    self.current_block,
                                    IRInstruction {
                                        id: None,
                                        op: IROp::Jump(merge_block),
                                    },
                                );
                                self.current_block = next_check_block;
                                continue;
                            }
                        }

                        // Pattern: variant without payload (e.g. Active or AppEvent::Quit)
                        let opt_variant_name = match option {
                            Expr::Identifier(n) => Some(n.clone()),
                            Expr::NamespaceAccess { property, .. } => {
                                if let Expr::Identifier(p) = &**property {
                                    Some(p.clone())
                                } else {
                                    None
                                }
                            }
                            _ => None,
                        };

                        if let Some(ref v_name) = opt_variant_name {
                            if let Some(&tag_idx) = self.enum_variant_tags.get(v_name) {
                                let case_body_block =
                                    self.current_func.as_mut().unwrap().new_block();
                                let next_check_block =
                                    self.current_func.as_mut().unwrap().new_block();

                                let curr_tag = tag_val.unwrap_or(cond_val);
                                let expected_tag = self.current_func.as_mut().unwrap().new_vreg();
                                self.current_func.as_mut().unwrap().add_inst(
                                    self.current_block,
                                    IRInstruction {
                                        id: Some(expected_tag),
                                        op: IROp::ConstInt64(tag_idx as i64),
                                    },
                                );
                                let cmp = self.current_func.as_mut().unwrap().new_vreg();
                                self.current_func.as_mut().unwrap().add_inst(
                                    self.current_block,
                                    IRInstruction {
                                        id: Some(cmp),
                                        op: IROp::Eq(curr_tag, expected_tag),
                                    },
                                );
                                self.current_func.as_mut().unwrap().add_inst(
                                    self.current_block,
                                    IRInstruction {
                                        id: None,
                                        op: IROp::BranchIf {
                                            cond: cmp,
                                            true_block: case_body_block,
                                            false_block: next_check_block,
                                        },
                                    },
                                );

                                self.current_block = case_body_block;
                                self.visit_block(body);
                                self.current_func.as_mut().unwrap().add_inst(
                                    self.current_block,
                                    IRInstruction {
                                        id: None,
                                        op: IROp::Jump(merge_block),
                                    },
                                );
                                self.current_block = next_check_block;
                                continue;
                            }
                        }

                        // Regular value matching
                        let case_body_block = self.current_func.as_mut().unwrap().new_block();
                        let next_check_block = self.current_func.as_mut().unwrap().new_block();

                        let opt_val = self.visit_expr(option);
                        let cmp = self.current_func.as_mut().unwrap().new_vreg();
                        self.current_func.as_mut().unwrap().add_inst(
                            self.current_block,
                            IRInstruction {
                                id: Some(cmp),
                                op: IROp::Eq(cond_val, opt_val),
                            },
                        );
                        self.current_func.as_mut().unwrap().add_inst(
                            self.current_block,
                            IRInstruction {
                                id: None,
                                op: IROp::BranchIf {
                                    cond: cmp,
                                    true_block: case_body_block,
                                    false_block: next_check_block,
                                },
                            },
                        );

                        self.current_block = case_body_block;
                        self.visit_block(body);
                        self.current_func.as_mut().unwrap().add_inst(
                            self.current_block,
                            IRInstruction {
                                id: None,
                                op: IROp::Jump(merge_block),
                            },
                        );
                        self.current_block = next_check_block;
                    }
                }

                self.current_func.as_mut().unwrap().add_inst(
                    self.current_block,
                    IRInstruction {
                        id: None,
                        op: IROp::Jump(merge_block),
                    },
                );
                self.current_block = merge_block;
            }
            Stmt::ForStmt {
                init,
                condition,
                increment,
                body,
            } => {
                if let Some(init_stmt) = init {
                    self.visit_stmt(init_stmt);
                }
                let header_block = self.current_func.as_mut().unwrap().new_block();
                let body_block = self.current_func.as_mut().unwrap().new_block();
                let step_block = self.current_func.as_mut().unwrap().new_block();
                let exit_block = self.current_func.as_mut().unwrap().new_block();

                self.current_func.as_mut().unwrap().add_inst(
                    self.current_block,
                    IRInstruction {
                        id: None,
                        op: IROp::Jump(header_block),
                    },
                );

                self.current_block = header_block;
                if let Some(cond_expr) = condition {
                    let cond_val = self.visit_expr(cond_expr);
                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction {
                            id: None,
                            op: IROp::BranchIf {
                                cond: cond_val,
                                true_block: body_block,
                                false_block: exit_block,
                            },
                        },
                    );
                } else {
                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction {
                            id: None,
                            op: IROp::Jump(body_block),
                        },
                    );
                }

                self.current_block = body_block;
                self.loop_stack.push((step_block, exit_block));
                self.push_scope();
                match body {
                    EitherBlock::Inline(stmts) => {
                        for s in stmts {
                            self.visit_stmt(s);
                        }
                    }
                    EitherBlock::External(expr) => {
                        self.visit_expr(expr);
                    }
                }
                self.pop_scope();
                self.loop_stack.pop();
                self.current_func.as_mut().unwrap().add_inst(
                    self.current_block,
                    IRInstruction {
                        id: None,
                        op: IROp::Jump(step_block),
                    },
                );

                self.current_block = step_block;
                if let Some(inc_stmt) = increment {
                    self.visit_stmt(inc_stmt);
                }
                self.current_func.as_mut().unwrap().add_inst(
                    self.current_block,
                    IRInstruction {
                        id: None,
                        op: IROp::Jump(header_block),
                    },
                );

                self.current_block = exit_block;
            }
            Stmt::ForInStmt {
                item,
                iterable,
                body,
            } => {
                let item_name = match &**item {
                    Stmt::Declaration(Decl::VarDecl { name, .. }) => name.clone(),
                    Stmt::ExpressionStmt(Expr::Identifier(name)) => name.clone(),
                    _ => "item".to_string(),
                };

                let is_array = match &iterable {
                    Expr::Identifier(n) => {
                        self.array_lens.contains_key(n) || self.array_elem_types.contains_key(n)
                    }
                    Expr::ArrayLiteral(_) => true,
                    _ => false,
                };

                if is_array {
                    let (arr_ptr, arr_len, elem_ty) = match &iterable {
                        Expr::Identifier(n) => {
                            let ptr = self.lookup_var(n).unwrap_or(0);
                            let len = self.array_lens.get(n).copied().unwrap_or(0);
                            let ty = self
                                .array_elem_types
                                .get(n)
                                .cloned()
                                .unwrap_or(IRType::Int(Size::S32));
                            (ptr, len, ty)
                        }
                        Expr::ArrayLiteral(elems) => {
                            let val = self.visit_expr(&iterable);
                            let ty = if !elems.is_empty() {
                                self.infer_expr_type(&elems[0])
                            } else {
                                IRType::Int(Size::S32)
                            };
                            (val, elems.len(), ty)
                        }
                        _ => (0, 0, IRType::Int(Size::S32)),
                    };

                    let elem_size = elem_ty.size_in_bytes().max(1);

                    let idx_ptr = self.current_func.as_mut().unwrap().new_vreg();
                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction {
                            id: Some(idx_ptr),
                            op: IROp::Alloc {
                                ty: IRType::Int(Size::S32),
                            },
                        },
                    );
                    let zero_val = self.current_func.as_mut().unwrap().new_vreg();
                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction {
                            id: Some(zero_val),
                            op: IROp::ConstInt32(0),
                        },
                    );
                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction {
                            id: None,
                            op: IROp::Store {
                                ptr: idx_ptr,
                                value: zero_val,
                            },
                        },
                    );

                    let header_block = self.current_func.as_mut().unwrap().new_block();
                    let body_block = self.current_func.as_mut().unwrap().new_block();
                    let step_block = self.current_func.as_mut().unwrap().new_block();
                    let exit_block = self.current_func.as_mut().unwrap().new_block();

                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction {
                            id: None,
                            op: IROp::Jump(header_block),
                        },
                    );

                    self.current_block = header_block;
                    let idx_val = self.current_func.as_mut().unwrap().new_vreg();
                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction {
                            id: Some(idx_val),
                            op: IROp::Load {
                                ptr: idx_ptr,
                                ty: IRType::Int(Size::S32),
                            },
                        },
                    );
                    let len_val = self.current_func.as_mut().unwrap().new_vreg();
                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction {
                            id: Some(len_val),
                            op: IROp::ConstInt32(arr_len as i32),
                        },
                    );
                    let cmp = self.current_func.as_mut().unwrap().new_vreg();
                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction {
                            id: Some(cmp),
                            op: IROp::Lt(idx_val, len_val),
                        },
                    );
                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction {
                            id: None,
                            op: IROp::BranchIf {
                                cond: cmp,
                                true_block: body_block,
                                false_block: exit_block,
                            },
                        },
                    );

                    self.current_block = body_block;
                    self.loop_stack.push((step_block, exit_block));
                    self.push_scope();

                    let cur_idx = self.current_func.as_mut().unwrap().new_vreg();
                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction {
                            id: Some(cur_idx),
                            op: IROp::Load {
                                ptr: idx_ptr,
                                ty: IRType::Int(Size::S32),
                            },
                        },
                    );
                    let elem_addr = self.current_func.as_mut().unwrap().new_vreg();
                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction {
                            id: Some(elem_addr),
                            op: IROp::GetElementPtr {
                                base_ptr: arr_ptr,
                                index: cur_idx,
                                elem_size,
                            },
                        },
                    );
                    let elem_val = self.current_func.as_mut().unwrap().new_vreg();
                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction {
                            id: Some(elem_val),
                            op: IROp::LoadMemory {
                                ptr: elem_addr,
                                ty: elem_ty.clone(),
                            },
                        },
                    );
                    let item_var = self.current_func.as_mut().unwrap().new_vreg();
                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction {
                            id: Some(item_var),
                            op: IROp::Alloc {
                                ty: elem_ty.clone(),
                            },
                        },
                    );
                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction {
                            id: None,
                            op: IROp::Store {
                                ptr: item_var,
                                value: elem_val,
                            },
                        },
                    );
                    self.declare_var(item_name.clone(), item_var);
                    self.var_types.insert(
                        item_name.clone(),
                        match elem_ty {
                            IRType::Int(Size::S64) => "int64".to_string(),
                            IRType::Float(Size::S64) => "float64".to_string(),
                            _ => "int32".to_string(),
                        },
                    );

                    match body {
                        EitherBlock::Inline(stmts) => {
                            for s in stmts {
                                self.visit_stmt(s);
                            }
                        }
                        EitherBlock::External(expr) => {
                            self.visit_expr(expr);
                        }
                    }

                    self.pop_scope();
                    self.loop_stack.pop();
                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction {
                            id: None,
                            op: IROp::Jump(step_block),
                        },
                    );

                    self.current_block = step_block;
                    let old_idx = self.current_func.as_mut().unwrap().new_vreg();
                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction {
                            id: Some(old_idx),
                            op: IROp::Load {
                                ptr: idx_ptr,
                                ty: IRType::Int(Size::S32),
                            },
                        },
                    );
                    let one_val = self.current_func.as_mut().unwrap().new_vreg();
                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction {
                            id: Some(one_val),
                            op: IROp::ConstInt32(1),
                        },
                    );
                    let new_idx = self.current_func.as_mut().unwrap().new_vreg();
                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction {
                            id: Some(new_idx),
                            op: IROp::Add(old_idx, one_val),
                        },
                    );
                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction {
                            id: None,
                            op: IROp::Store {
                                ptr: idx_ptr,
                                value: new_idx,
                            },
                        },
                    );
                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction {
                            id: None,
                            op: IROp::Jump(header_block),
                        },
                    );

                    self.current_block = exit_block;
                } else {
                    let (col_ptr, struct_name) = self
                        .eval_object_ptr_and_type(&iterable)
                        .unwrap_or((0, String::new()));
                    let iter_func = format!("{}_iter", struct_name);
                    let it_ptr = self.current_func.as_mut().unwrap().new_vreg();
                    self.called_functions
                        .entry(iter_func.clone())
                        .or_insert_with(|| {
                            vec![IRType::Pointer(Box::new(IRType::CustomScope(
                                struct_name.clone(),
                            )))]
                        });
                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction {
                            id: Some(it_ptr),
                            op: IROp::Call {
                                func: iter_func,
                                args: vec![col_ptr],
                            },
                        },
                    );

                    let iter_struct_name = format!("{}Iter", struct_name);

                    let header_block = self.current_func.as_mut().unwrap().new_block();
                    let body_block = self.current_func.as_mut().unwrap().new_block();
                    let exit_block = self.current_func.as_mut().unwrap().new_block();

                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction {
                            id: None,
                            op: IROp::Jump(header_block),
                        },
                    );

                    self.current_block = header_block;
                    let is_done_func = format!("{}_is_done_iter", iter_struct_name);
                    let is_done_val = self.current_func.as_mut().unwrap().new_vreg();
                    self.called_functions
                        .entry(is_done_func.clone())
                        .or_insert_with(|| {
                            vec![IRType::Pointer(Box::new(IRType::CustomScope(
                                iter_struct_name.clone(),
                            )))]
                        });
                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction {
                            id: Some(is_done_val),
                            op: IROp::Call {
                                func: is_done_func,
                                args: vec![it_ptr],
                            },
                        },
                    );
                    let not_done = self.current_func.as_mut().unwrap().new_vreg();
                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction {
                            id: Some(not_done),
                            op: IROp::Not(is_done_val),
                        },
                    );
                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction {
                            id: None,
                            op: IROp::BranchIf {
                                cond: not_done,
                                true_block: body_block,
                                false_block: exit_block,
                            },
                        },
                    );

                    self.current_block = body_block;
                    self.loop_stack.push((header_block, exit_block));
                    self.push_scope();

                    let next_func = format!("{}_next", iter_struct_name);
                    let next_val = self.current_func.as_mut().unwrap().new_vreg();
                    self.called_functions
                        .entry(next_func.clone())
                        .or_insert_with(|| {
                            vec![IRType::Pointer(Box::new(IRType::CustomScope(
                                iter_struct_name.clone(),
                            )))]
                        });
                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction {
                            id: Some(next_val),
                            op: IROp::Call {
                                func: next_func,
                                args: vec![it_ptr],
                            },
                        },
                    );

                    let item_var = self.current_func.as_mut().unwrap().new_vreg();
                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction {
                            id: Some(item_var),
                            op: IROp::Alloc {
                                ty: IRType::Int(Size::S32),
                            },
                        },
                    );
                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction {
                            id: None,
                            op: IROp::Store {
                                ptr: item_var,
                                value: next_val,
                            },
                        },
                    );
                    self.declare_var(item_name.clone(), item_var);
                    self.var_types
                        .insert(item_name.clone(), "int32".to_string());

                    match body {
                        EitherBlock::Inline(stmts) => {
                            for s in stmts {
                                self.visit_stmt(s);
                            }
                        }
                        EitherBlock::External(expr) => {
                            self.visit_expr(expr);
                        }
                    }

                    self.pop_scope();
                    self.loop_stack.pop();
                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction {
                            id: None,
                            op: IROp::Jump(header_block),
                        },
                    );

                    self.current_block = exit_block;
                }
            }
            Stmt::BreakStmt => {
                if let Some((_, exit_block)) = self.loop_stack.last() {
                    let curr = self.current_block;
                    self.current_func.as_mut().unwrap().add_inst(
                        curr,
                        IRInstruction {
                            id: None,
                            op: IROp::Jump(*exit_block),
                        },
                    );
                    let dead_block = self.current_func.as_mut().unwrap().new_block();
                    self.current_block = dead_block;
                }
            }
            Stmt::ContinueStmt => {
                if let Some((header_block, _)) = self.loop_stack.last() {
                    let curr = self.current_block;
                    self.current_func.as_mut().unwrap().add_inst(
                        curr,
                        IRInstruction {
                            id: None,
                            op: IROp::Jump(*header_block),
                        },
                    );
                    let dead_block = self.current_func.as_mut().unwrap().new_block();
                    self.current_block = dead_block;
                }
            }
            Stmt::LoopStmt { count, body } => {
                let header_block = self.current_func.as_mut().unwrap().new_block();
                let body_block = self.current_func.as_mut().unwrap().new_block();
                let exit_block = self.current_func.as_mut().unwrap().new_block();

                if let Some(cnt_expr) = count {
                    let total = self.visit_expr(cnt_expr);
                    let counter_ptr = self.current_func.as_mut().unwrap().new_vreg();
                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction {
                            id: Some(counter_ptr),
                            op: IROp::Alloc {
                                ty: IRType::Int(Size::S32),
                            },
                        },
                    );
                    let zero = self.current_func.as_mut().unwrap().new_vreg();
                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction {
                            id: Some(zero),
                            op: IROp::ConstInt32(0),
                        },
                    );
                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction {
                            id: None,
                            op: IROp::Store {
                                ptr: counter_ptr,
                                value: zero,
                            },
                        },
                    );
                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction {
                            id: None,
                            op: IROp::Jump(header_block),
                        },
                    );

                    self.current_block = header_block;
                    let curr_cnt = self.current_func.as_mut().unwrap().new_vreg();
                    self.current_func.as_mut().unwrap().add_inst(
                        header_block,
                        IRInstruction {
                            id: Some(curr_cnt),
                            op: IROp::Load {
                                ptr: counter_ptr,
                                ty: IRType::Int(Size::S32),
                            },
                        },
                    );
                    let cond = self.current_func.as_mut().unwrap().new_vreg();
                    self.current_func.as_mut().unwrap().add_inst(
                        header_block,
                        IRInstruction {
                            id: Some(cond),
                            op: IROp::Lt(curr_cnt, total),
                        },
                    );
                    self.current_func.as_mut().unwrap().add_inst(
                        header_block,
                        IRInstruction {
                            id: None,
                            op: IROp::BranchIf {
                                cond,
                                true_block: body_block,
                                false_block: exit_block,
                            },
                        },
                    );

                    let step_block = self.current_func.as_mut().unwrap().new_block();
                    self.loop_stack.push((step_block, exit_block));
                    self.current_block = body_block;
                    match body {
                        crate::frontend::parser::ast::EitherBlock::Inline(stmts) => {
                            self.visit_block(stmts)
                        }
                        crate::frontend::parser::ast::EitherBlock::External(expr) => {
                            self.visit_expr(expr);
                        }
                    }
                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction {
                            id: None,
                            op: IROp::Jump(step_block),
                        },
                    );
                    self.loop_stack.pop();

                    self.current_block = step_block;
                    let c_val = self.current_func.as_mut().unwrap().new_vreg();
                    self.current_func.as_mut().unwrap().add_inst(
                        step_block,
                        IRInstruction {
                            id: Some(c_val),
                            op: IROp::Load {
                                ptr: counter_ptr,
                                ty: IRType::Int(Size::S32),
                            },
                        },
                    );
                    let one = self.current_func.as_mut().unwrap().new_vreg();
                    self.current_func.as_mut().unwrap().add_inst(
                        step_block,
                        IRInstruction {
                            id: Some(one),
                            op: IROp::ConstInt32(1),
                        },
                    );
                    let next_c = self.current_func.as_mut().unwrap().new_vreg();
                    self.current_func.as_mut().unwrap().add_inst(
                        step_block,
                        IRInstruction {
                            id: Some(next_c),
                            op: IROp::Add(c_val, one),
                        },
                    );
                    self.current_func.as_mut().unwrap().add_inst(
                        step_block,
                        IRInstruction {
                            id: None,
                            op: IROp::Store {
                                ptr: counter_ptr,
                                value: next_c,
                            },
                        },
                    );
                    self.current_func.as_mut().unwrap().add_inst(
                        step_block,
                        IRInstruction {
                            id: None,
                            op: IROp::Jump(header_block),
                        },
                    );
                } else {
                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction {
                            id: None,
                            op: IROp::Jump(header_block),
                        },
                    );
                    self.loop_stack.push((header_block, exit_block));
                    self.current_block = header_block;
                    match body {
                        crate::frontend::parser::ast::EitherBlock::Inline(stmts) => {
                            self.visit_block(stmts)
                        }
                        crate::frontend::parser::ast::EitherBlock::External(expr) => {
                            self.visit_expr(expr);
                        }
                    }
                    self.current_func.as_mut().unwrap().add_inst(
                        self.current_block,
                        IRInstruction {
                            id: None,
                            op: IROp::Jump(header_block),
                        },
                    );
                    self.loop_stack.pop();
                }

                self.current_block = exit_block;
            }
            Stmt::Block(stmts) | Stmt::ThisBlock(stmts) => {
                self.visit_block(stmts);
            }
            Stmt::ReturnStmt(expr) => {
                let val =
                    self.visit_expr(&expr.clone().unwrap_or(Expr::Identifier("".to_string())));
                let curr = self.current_block;
                self.current_func.as_mut().unwrap().add_inst(
                    curr,
                    IRInstruction {
                        id: None,
                        op: IROp::Return(Some(val)),
                    },
                );
            }
            Stmt::LeaveStmt => {
                let curr = self.current_block;
                self.current_func.as_mut().unwrap().add_inst(
                    curr,
                    IRInstruction {
                        id: None,
                        op: IROp::Return(None),
                    },
                );
            }
            Stmt::ExpressionStmt(expr) => {
                self.visit_expr(expr);
            }
            _ => {}
        }
    }
}
