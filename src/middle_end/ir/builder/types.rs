use super::IRBuilder;
use crate::frontend::parser::ast::*;
use crate::middle_end::ir::instruction::*;

impl<'a> IRBuilder<'a> {
    pub(crate) fn infer_expr_type(&self, expr: &Expr) -> IRType {
        match expr {
            Expr::LiteralInt(_) => IRType::Int(Size::S32),
            Expr::LiteralUInt(_) => IRType::UInt(Size::S32),
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
                        _ => IRType::Int(Size::S32),
                    }
                } else {
                    IRType::Int(Size::S32)
                }
            }
            Expr::IndexAccess { object, .. } => {
                if let Expr::Identifier(name) = &**object {
                    self.array_elem_types
                        .get(name)
                        .cloned()
                        .unwrap_or(IRType::Int(Size::S32))
                } else {
                    IRType::Int(Size::S32)
                }
            }
            Expr::UnaryOp { operand, .. } => self.infer_expr_type(operand),
            Expr::BinaryOp { left, .. } => self.infer_expr_type(left),
            Expr::Cast { target_type, .. } => IRType::from_ast(target_type),
            Expr::New { type_node, .. } => IRType::from_ast(type_node),
            Expr::Instantiate { target, .. } => {
                if let Expr::Identifier(n) = &**target {
                    IRType::CustomScope(n.clone())
                } else {
                    IRType::Int(Size::S32)
                }
            }
            Expr::Call { callee, .. } => {
                if let Expr::Identifier(name) = &**callee {
                    if let Some(ret) = self.fn_return_types.get(name) {
                        return ret.clone();
                    }
                } else if let Expr::NamespaceAccess {
                    namespace,
                    property,
                } = &**callee
                {
                    if namespace == "@sys" {
                        if let Expr::Identifier(prop) = &**property {
                            match prop.as_str() {
                                "alloc" => return IRType::Pointer(Box::new(IRType::Int(Size::S8))),
                                "open" | "close" => return IRType::Int(Size::S32),
                                "read" | "write" => return IRType::UInt(Size::S64),
                                _ => return IRType::Void,
                            }
                        }
                    }
                }
                IRType::Int(Size::S64)
            }
            _ => IRType::Int(Size::S32),
        }
    }

    pub(crate) fn resolve_callee(&self, name: &str, arg_types: &[IRType]) -> String {
        if let Some(overloads) = self.fn_signatures.get(name) {
            if overloads.len() == 1 {
                return overloads[0].0.clone();
            }
            // 1. Exact match
            for (mangled, params) in overloads {
                if params.len() == arg_types.len()
                    && params.iter().zip(arg_types).all(|(p, a)| p == a)
                {
                    return mangled.clone();
                }
            }
            // 2. Prefix match (for default parameters)
            for (mangled, params) in overloads {
                if params.len() >= arg_types.len()
                    && params.iter().zip(arg_types).all(|(p, a)| p == a)
                {
                    return mangled.clone();
                }
            }
            // 3. Compatible match
            for (mangled, params) in overloads {
                if params.len() == arg_types.len()
                    && params.iter().zip(arg_types).all(|(p, a)| {
                        p == a
                            || (*p == IRType::Int(Size::S64) && *a == IRType::Int(Size::S32))
                            || (*p == IRType::Float(Size::S64) && *a == IRType::Float(Size::S32))
                            || (*p == IRType::Int(Size::S32) && *a == IRType::Char)
                    })
                {
                    return mangled.clone();
                }
            }
            return overloads[0].0.clone();
        }
        name.to_string()
    }

    /// Recursively evaluates an object expression (e.g. `this`, `p`, `this.counter`, `arr[i]`),
    /// returning the IRValue pointer to the object and its struct type name (if known).
    pub(crate) fn eval_object_ptr_and_type(&mut self, expr: &Expr) -> Option<(IRValue, String)> {
        match expr {
            Expr::This => {
                let ptr = self.lookup_var("this")?;
                let st_name = self.var_types.get("this").cloned().unwrap_or_default();
                Some((ptr, st_name))
            }
            Expr::Identifier(name) => {
                let ptr = self.lookup_var(name)?;
                let st_name = self.var_types.get(name).cloned().unwrap_or_default();
                Some((ptr, st_name))
            }
            Expr::PropertyAccess { object, property } => {
                let (parent_ptr, parent_struct) = self.eval_object_ptr_and_type(object)?;
                let st = self.structs.get(&parent_struct)?.clone();
                let (_, field_ty, offset) =
                    st.fields.iter().find(|(f, _, _)| f == property)?.clone();

                let field_ptr = self.current_func.as_mut()?.new_vreg();
                self.current_func.as_mut()?.add_inst(
                    self.current_block,
                    IRInstruction {
                        id: Some(field_ptr),
                        op: IROp::GetFieldPtr {
                            ptr: parent_ptr,
                            offset: offset as i32,
                        },
                    },
                );

                match field_ty {
                    IRType::CustomScope(name) => Some((field_ptr, name)),
                    IRType::Pointer(inner) => {
                        let struct_name = match inner.as_ref() {
                            IRType::CustomScope(ref name) => name.clone(),
                            _ => String::new(),
                        };
                        let loaded = self.current_func.as_mut()?.new_vreg();
                        self.current_func.as_mut()?.add_inst(
                            self.current_block,
                            IRInstruction {
                                id: Some(loaded),
                                op: IROp::LoadMemory {
                                    ptr: field_ptr,
                                    ty: IRType::Pointer(inner),
                                },
                            },
                        );
                        Some((loaded, struct_name))
                    }
                    _ => Some((field_ptr, String::new())),
                }
            }
            Expr::IndexAccess { object, indices } => {
                let (base_ptr, elem_ty) = match &**object {
                    Expr::Identifier(n) => {
                        let ptr = self.lookup_var(n)?;
                        let ty = self
                            .array_elem_types
                            .get(n)
                            .cloned()
                            .unwrap_or(IRType::Int(Size::S32));
                        (ptr, ty)
                    }
                    Expr::This => {
                        let ptr = self.lookup_var("this")?;
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
                    return None;
                };
                let elem_ptr = self.current_func.as_mut()?.new_vreg();
                self.current_func.as_mut()?.add_inst(
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
                let st_name = match elem_ty {
                    IRType::CustomScope(n) => n,
                    _ => String::new(),
                };
                Some((elem_ptr, st_name))
            }
            _ => {
                let val = self.visit_expr(expr);
                Some((val, String::new()))
            }
        }
    }
}
