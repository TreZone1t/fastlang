pub mod expr;
pub mod stmt;
pub mod types;

use crate::frontend::parser::ast::*;
use crate::middle_end::ir::instruction::*;
use std::collections::HashMap;

pub struct IRBuilder<'a> {
    pub(crate) module_name: String,
    pub(crate) _metadata: &'a HashMap<String, TypeMetadata>,
    pub(crate) current_func: Option<IRFunction>,
    pub(crate) current_block: BlockID,
    // Maps variable names in the current scope to their allocated Pointer IRValue
    pub(crate) env: Vec<HashMap<String, IRValue>>,
    pub(crate) structs: HashMap<String, IRStruct>,
    pub(crate) var_types: HashMap<String, String>,
    pub(crate) array_elem_types: HashMap<String, IRType>,
    pub(crate) fn_signatures: HashMap<String, Vec<(String, Vec<IRType>)>>,
    pub(crate) fn_param_defaults: HashMap<String, Vec<Option<Expr>>>,
    pub(crate) fn_return_types: HashMap<String, IRType>,
    pub(crate) loop_stack: Vec<(BlockID, BlockID)>,
    pub(crate) called_functions: std::collections::HashMap<String, Vec<IRType>>,
    pub enum_variant_tags: HashMap<String, usize>,
    pub enum_has_payload: std::collections::HashSet<String>,
    pub enum_variant_fields: HashMap<String, Vec<String>>,
    pub array_lens: HashMap<String, usize>,
    pub target_backend: String,
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
            fn_param_defaults: HashMap::new(),
            fn_return_types: HashMap::new(),
            loop_stack: Vec::new(),
            called_functions: std::collections::HashMap::new(),
            enum_variant_tags: HashMap::new(),
            enum_has_payload: std::collections::HashSet::new(),
            enum_variant_fields: HashMap::new(),
            array_lens: HashMap::new(),
            target_backend: "cranelift".to_string(),
        }
    }

    pub fn import_declarations(
        &mut self,
        structs: &HashMap<String, IRStruct>,
        signatures: &HashMap<String, Vec<(String, Vec<IRType>)>>,
        param_defaults: &HashMap<String, Vec<Option<Expr>>>,
        return_types: &HashMap<String, IRType>,
        enum_tags: &HashMap<String, usize>,
        enum_payloads: &std::collections::HashSet<String>,
        enum_fields: &HashMap<String, Vec<String>>,
    ) {
        self.structs.extend(structs.clone());
        for (name, sigs) in signatures {
            self.fn_signatures
                .entry(name.clone())
                .or_default()
                .extend(sigs.clone());
        }
        self.fn_param_defaults.extend(param_defaults.clone());
        self.fn_return_types.extend(return_types.clone());
        self.enum_variant_tags.extend(enum_tags.clone());
        self.enum_has_payload.extend(enum_payloads.clone());
        self.enum_variant_fields.extend(enum_fields.clone());
    }

    pub fn export_fn_signatures(&self) -> HashMap<String, Vec<(String, Vec<IRType>)>> {
        self.fn_signatures.clone()
    }

    pub fn export_fn_return_types(&self) -> HashMap<String, IRType> {
        self.fn_return_types.clone()
    }

    pub fn export_param_defaults(&self) -> HashMap<String, Vec<Option<Expr>>> {
        self.fn_param_defaults.clone()
    }

    pub fn export_enum_tags(&self) -> HashMap<String, usize> {
        self.enum_variant_tags.clone()
    }

    pub fn export_enum_payloads(&self) -> std::collections::HashSet<String> {
        self.enum_has_payload.clone()
    }

    pub fn export_enum_fields(&self) -> HashMap<String, Vec<String>> {
        self.enum_variant_fields.clone()
    }

    pub(crate) fn push_scope(&mut self) {
        self.env.push(HashMap::new());
    }

    pub(crate) fn pop_scope(&mut self) {
        self.env.pop();
    }

    pub(crate) fn declare_var(&mut self, name: String, ptr_val: IRValue) {
        if let Some(scope) = self.env.last_mut() {
            scope.insert(name, ptr_val);
        }
    }

    pub(crate) fn lookup_var(&self, name: &str) -> Option<IRValue> {
        for scope in self.env.iter().rev() {
            if let Some(&val) = scope.get(name) {
                return Some(val);
            }
        }
        None
    }

    pub fn build(&mut self, stmts: &[Stmt]) -> IRModule {
        let mut functions = Vec::new();

        // 1. First pass: Collect all struct, class, and blueprint declarations
        for stmt in stmts {
            match stmt {
                Stmt::Declaration(Decl::BlueprintDecl {
                    name, definition, ..
                }) => {
                    let mut offset = 0;
                    let mut ir_fields = Vec::new();
                    match definition {
                        crate::frontend::parser::ast::BlueprintDef::Explicit(fields) => {
                            for f in fields {
                                let ir_ty = IRType::from_ast(&f.type_node);
                                let size = ir_ty.size_in_bytes().max(1);
                                ir_fields.push((f.name.clone(), ir_ty, offset));
                                offset += size;
                            }
                        }
                        crate::frontend::parser::ast::BlueprintDef::FromTemporaryObject(fields) => {
                            for f in fields {
                                let ir_ty = self.infer_expr_type(&f.value);
                                let size = ir_ty.size_in_bytes().max(1);
                                ir_fields.push((f.name.clone(), ir_ty, offset));
                                offset += size;
                            }
                        }
                        _ => {}
                    }
                    let ir_struct = IRStruct {
                        name: name.clone(),
                        size: if offset == 0 { 8 } else { offset },
                        fields: ir_fields,
                    };
                    self.structs.insert(name.clone(), ir_struct);
                }
                Stmt::Declaration(Decl::ClassDecl {
                    name,
                    public_block,
                    private_block,
                    ..
                })
                | Stmt::Declaration(Decl::StructDecl {
                    name,
                    public_block,
                    private_block,
                    ..
                }) => {
                    let mut offset = 0;
                    let mut ir_fields = Vec::new();
                    for d in public_block.iter().chain(private_block.iter()) {
                        if let Decl::VarDecl {
                            name: f_name,
                            type_node,
                            ..
                        } = d
                        {
                            let ir_ty = IRType::from_ast(type_node);
                            let size = ir_ty.size_in_bytes().max(1);
                            ir_fields.push((f_name.clone(), ir_ty, offset));
                            offset += size;
                        }
                    }
                    let ir_struct = IRStruct {
                        name: name.clone(),
                        size: if offset == 0 { 8 } else { offset },
                        fields: ir_fields,
                    };
                    self.structs.insert(name.clone(), ir_struct);
                }
                _ => {}
            }
        }

        // Builtin C-runtime external symbols for I/O and syscalls (registered for selective extern emission)
        let runtime_externs = vec![
            (
                "fast_alloc".to_string(),
                vec![("size".to_string(), IRType::UInt(Size::S64))],
                IRType::Pointer(Box::new(IRType::Int(Size::S8))),
            ),
            (
                "fast_free".to_string(),
                vec![(
                    "ptr".to_string(),
                    IRType::Pointer(Box::new(IRType::Int(Size::S8))),
                )],
                IRType::Void,
            ),
            (
                "fast_open".to_string(),
                vec![
                    (
                        "path".to_string(),
                        IRType::Pointer(Box::new(IRType::Int(Size::S8))),
                    ),
                    ("flags".to_string(), IRType::Int(Size::S32)),
                    ("mode".to_string(), IRType::Int(Size::S32)),
                ],
                IRType::Int(Size::S32),
            ),
            (
                "fast_close".to_string(),
                vec![("fd".to_string(), IRType::Int(Size::S32))],
                IRType::Int(Size::S32),
            ),
            (
                "fast_read".to_string(),
                vec![
                    ("fd".to_string(), IRType::Int(Size::S32)),
                    (
                        "buf".to_string(),
                        IRType::Pointer(Box::new(IRType::Int(Size::S8))),
                    ),
                    ("count".to_string(), IRType::UInt(Size::S64)),
                ],
                IRType::UInt(Size::S64),
            ),
            (
                "fast_write".to_string(),
                vec![
                    ("fd".to_string(), IRType::Int(Size::S32)),
                    (
                        "buf".to_string(),
                        IRType::Pointer(Box::new(IRType::Int(Size::S8))),
                    ),
                    ("count".to_string(), IRType::UInt(Size::S64)),
                ],
                IRType::UInt(Size::S64),
            ),
            (
                "fast_exit".to_string(),
                vec![("code".to_string(), IRType::Int(Size::S32))],
                IRType::Void,
            ),
            (
                "fast_println".to_string(),
                vec![(
                    "s".to_string(),
                    IRType::Pointer(Box::new(IRType::Int(Size::S8))),
                )],
                IRType::Void,
            ),
            (
                "fast_print".to_string(),
                vec![(
                    "s".to_string(),
                    IRType::Pointer(Box::new(IRType::Int(Size::S8))),
                )],
                IRType::Void,
            ),
        ];
        for (name, params, ret) in runtime_externs {
            self.fn_signatures.entry(name.clone()).or_default().push((
                name.clone(),
                params.iter().map(|(_, ty)| ty.clone()).collect(),
            ));
            self.fn_return_types.insert(name.clone(), ret.clone());
        }

        // 2. Second pass: Collect functions, impl methods, extern blocks
        for stmt in stmts {
            match stmt {
                Stmt::Declaration(Decl::FnDecl {
                    name,
                    params,
                    return_type,
                    body,
                    ..
                }) => {
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
                    self.fn_signatures.entry(name.clone()).or_default().push((
                        mangled_name.clone(),
                        ir_params.iter().map(|(_, ty)| ty.clone()).collect(),
                    ));
                    self.fn_param_defaults.insert(
                        mangled_name.clone(),
                        params.iter().map(|p| p.default_value.clone()).collect(),
                    );
                    self.fn_return_types.insert(name.clone(), ir_ret.clone());
                    self.fn_return_types.insert(mangled_name.clone(), ir_ret.clone());

                    let func = IRFunction::new(mangled_name, ir_params.clone(), ir_ret);
                    self.current_func = Some(func);
                    self.current_block = 0;
                    self.push_scope();

                    // Allocate parameters as local variables and store incoming parameter values
                    for (i, (param_name, param_ty)) in ir_params.iter().enumerate() {
                        let alloc_ptr = self.current_func.as_mut().unwrap().new_vreg();
                        self.current_func.as_mut().unwrap().add_inst(
                            self.current_block,
                            IRInstruction {
                                id: Some(alloc_ptr),
                                op: IROp::Alloc {
                                    ty: param_ty.clone(),
                                },
                            },
                        );
                        self.current_func.as_mut().unwrap().add_inst(
                            self.current_block,
                            IRInstruction {
                                id: None,
                                op: IROp::StoreParam {
                                    param_idx: i,
                                    ptr: alloc_ptr,
                                },
                            },
                        );
                        self.declare_var(param_name.clone(), alloc_ptr);
                    }

                    self.visit_block(body);

                    // Auto-return for void functions
                    if let Some(f) = &mut self.current_func {
                        if f.return_type == IRType::Void {
                            let last_block = self.current_block;
                            f.add_inst(
                                last_block,
                                IRInstruction {
                                    id: None,
                                    op: IROp::Return(None),
                                },
                            );
                        }
                    }

                    self.pop_scope();
                    if let Some(f) = self.current_func.take() {
                        functions.push(f);
                    }
                }
                Stmt::Declaration(Decl::ImplDecl {
                    target,
                    target_generics,
                    methods,
                    handle_block,
                    ..
                }) => {
                    let gen_suffix = if target_generics.is_empty() {
                        String::new()
                    } else {
                        format!(
                            "_{}",
                            target_generics
                                .iter()
                                .map(|g| g.get_name())
                                .collect::<Vec<_>>()
                                .join("_")
                        )
                    };
                    for m in methods.iter().chain(handle_block.iter()) {
                        if let Decl::FnDecl {
                            name,
                            params,
                            return_type,
                            body,
                            ..
                        } = m
                        {
                            let ir_ret = IRType::from_ast(return_type);
                            let mut ir_params =
                                vec![("this".to_string(), IRType::CustomScope(target.clone()))];
                            for param in params {
                                let ty = IRType::from_ast(&param.type_node);
                                ir_params.push((param.name.clone(), ty));
                            }
                            let mangled_name = format!("{}{}_{}", target, gen_suffix, name);
                            let func = IRFunction::new(mangled_name, ir_params.clone(), ir_ret);
                            self.current_func = Some(func);
                            self.current_block = 0;
                            self.push_scope();

                            for (i, (param_name, param_ty)) in ir_params.iter().enumerate() {
                                let alloc_ptr = self.current_func.as_mut().unwrap().new_vreg();
                                self.current_func.as_mut().unwrap().add_inst(
                                    self.current_block,
                                    IRInstruction {
                                        id: Some(alloc_ptr),
                                        op: IROp::Alloc {
                                            ty: param_ty.clone(),
                                        },
                                    },
                                );
                                self.current_func.as_mut().unwrap().add_inst(
                                    self.current_block,
                                    IRInstruction {
                                        id: None,
                                        op: IROp::StoreParam {
                                            param_idx: i,
                                            ptr: alloc_ptr,
                                        },
                                    },
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
                                    f.add_inst(
                                        last_block,
                                        IRInstruction {
                                            id: None,
                                            op: IROp::Return(None),
                                        },
                                    );
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
                        if let Decl::ExternFnDecl {
                            name,
                            params,
                            return_type,
                            alias,
                            ..
                        } = ext_decl
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
                        } else if let Decl::FnDecl {
                            name,
                            params,
                            return_type,
                            ..
                        } = ext_decl
                        {
                            let ir_ret = IRType::from_ast(return_type);
                            let mut ir_params = Vec::new();
                            for param in params {
                                let ty = IRType::from_ast(&param.type_node);
                                ir_params.push((param.name.clone(), ty));
                            }
                            self.fn_signatures.entry(name.clone()).or_default().push((
                                name.clone(),
                                ir_params.iter().map(|(_, ty)| ty.clone()).collect(),
                            ));
                            let func = IRFunction::new_extern(name.clone(), ir_params, ir_ret);
                            functions.push(func);
                        }
                    }
                }
                Stmt::Declaration(Decl::ExternFnDecl {
                    name,
                    params,
                    return_type,
                    alias,
                    ..
                }) => {
                    let sym_name = alias.as_ref().unwrap_or(name);
                    let ir_ret = IRType::from_ast(return_type);
                    let mut ir_params = Vec::new();
                    for param in params {
                        let ty = IRType::from_ast(&param.type_node);
                        ir_params.push((param.name.clone(), ty));
                    }
                    self.fn_signatures.entry(name.clone()).or_default().push((
                        sym_name.clone(),
                        ir_params.iter().map(|(_, ty)| ty.clone()).collect(),
                    ));
                    let func = IRFunction::new_extern(sym_name.clone(), ir_params, ir_ret);
                    functions.push(func);
                }
                Stmt::Declaration(Decl::ClassDecl {
                    name,
                    public_block,
                    private_block,
                    handle_block,
                    constructor,
                    ..
                }) => {
                    for m in public_block
                        .iter()
                        .chain(private_block.iter())
                        .chain(handle_block.iter())
                    {
                        if let Decl::FnDecl {
                            name: m_name,
                            params,
                            return_type,
                            body,
                            ..
                        } = m
                        {
                            let ir_ret = IRType::from_ast(return_type);
                            let mut ir_params =
                                vec![("this".to_string(), IRType::CustomScope(name.clone()))];
                            for param in params {
                                let ty = IRType::from_ast(&param.type_node);
                                ir_params.push((param.name.clone(), ty));
                            }
                            let mangled_name = format!("{}_{}", name, m_name);
                            self.fn_signatures
                                .entry(mangled_name.clone())
                                .or_default()
                                .push((
                                    mangled_name.clone(),
                                    ir_params.iter().map(|(_, ty)| ty.clone()).collect(),
                                ));
                            self.fn_signatures
                                .entry(format!("{}::{}", name, m_name))
                                .or_default()
                                .push((
                                    mangled_name.clone(),
                                    ir_params.iter().map(|(_, ty)| ty.clone()).collect(),
                                ));

                            let func = IRFunction::new(mangled_name, ir_params.clone(), ir_ret);
                            self.current_func = Some(func);
                            self.current_block = 0;
                            self.push_scope();

                            for (i, (param_name, param_ty)) in ir_params.iter().enumerate() {
                                let alloc_ptr = self.current_func.as_mut().unwrap().new_vreg();
                                self.current_func.as_mut().unwrap().add_inst(
                                    self.current_block,
                                    IRInstruction {
                                        id: Some(alloc_ptr),
                                        op: IROp::Alloc {
                                            ty: param_ty.clone(),
                                        },
                                    },
                                );
                                self.current_func.as_mut().unwrap().add_inst(
                                    self.current_block,
                                    IRInstruction {
                                        id: None,
                                        op: IROp::StoreParam {
                                            param_idx: i,
                                            ptr: alloc_ptr,
                                        },
                                    },
                                );
                                self.declare_var(param_name.clone(), alloc_ptr);
                                if param_name == "this" {
                                    self.var_types.insert("this".to_string(), name.clone());
                                }
                            }

                            self.visit_block(body);

                            if let Some(f) = &mut self.current_func {
                                if f.return_type == IRType::Void {
                                    let last_block = self.current_block;
                                    f.add_inst(
                                        last_block,
                                        IRInstruction {
                                            id: None,
                                            op: IROp::Return(None),
                                        },
                                    );
                                }
                            }

                            self.pop_scope();
                            if let Some(f) = self.current_func.take() {
                                functions.push(f);
                            }
                        }
                    }

                    if let Some(constructors) = constructor {
                        for ctor in constructors {
                            let mut ir_params =
                                vec![("this".to_string(), IRType::CustomScope(name.clone()))];
                            for param in &ctor.params {
                                let ty = IRType::from_ast(&param.type_node);
                                ir_params.push((param.name.clone(), ty));
                            }
                            let ctor_names =
                                vec![format!("{}_new", name), format!("{}_init", name)];
                            for ctor_mangled in ctor_names {
                                self.fn_signatures
                                    .entry(ctor_mangled.clone())
                                    .or_default()
                                    .push((
                                        ctor_mangled.clone(),
                                        ir_params.iter().map(|(_, ty)| ty.clone()).collect(),
                                    ));

                                let func =
                                    IRFunction::new(ctor_mangled, ir_params.clone(), IRType::Void);
                                self.current_func = Some(func);
                                self.current_block = 0;
                                self.push_scope();

                                for (i, (param_name, param_ty)) in ir_params.iter().enumerate() {
                                    let alloc_ptr = self.current_func.as_mut().unwrap().new_vreg();
                                    self.current_func.as_mut().unwrap().add_inst(
                                        self.current_block,
                                        IRInstruction {
                                            id: Some(alloc_ptr),
                                            op: IROp::Alloc {
                                                ty: param_ty.clone(),
                                            },
                                        },
                                    );
                                    self.current_func.as_mut().unwrap().add_inst(
                                        self.current_block,
                                        IRInstruction {
                                            id: None,
                                            op: IROp::StoreParam {
                                                param_idx: i,
                                                ptr: alloc_ptr,
                                            },
                                        },
                                    );
                                    self.declare_var(param_name.clone(), alloc_ptr);
                                    if param_name == "this" {
                                        self.var_types.insert("this".to_string(), name.clone());
                                    }
                                }

                                if let Some(st) = self.structs.get(name).cloned() {
                                    for d in public_block.iter().chain(private_block.iter()) {
                                        if let Decl::VarDecl {
                                            name: f_name,
                                            value,
                                            ..
                                        } = d
                                        {
                                            if let Some((_, _, offset)) =
                                                st.fields.iter().find(|(n, _, _)| n == f_name)
                                            {
                                                let val = self.visit_expr(value);
                                                let this_ptr = self.lookup_var("this").unwrap();
                                                let this_val =
                                                    self.current_func.as_mut().unwrap().new_vreg();
                                                self.current_func.as_mut().unwrap().add_inst(
                                                    self.current_block,
                                                    IRInstruction {
                                                        id: Some(this_val),
                                                        op: IROp::Load {
                                                            ptr: this_ptr,
                                                            ty: IRType::Int(Size::S64),
                                                        },
                                                    },
                                                );
                                                let field_ptr =
                                                    self.current_func.as_mut().unwrap().new_vreg();
                                                self.current_func.as_mut().unwrap().add_inst(
                                                    self.current_block,
                                                    IRInstruction {
                                                        id: Some(field_ptr),
                                                        op: IROp::GetFieldPtr {
                                                            ptr: this_val,
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
                                }

                                self.visit_block(&ctor.body);

                                if let Some(f) = &mut self.current_func {
                                    let last_block = self.current_block;
                                    f.add_inst(
                                        last_block,
                                        IRInstruction {
                                            id: None,
                                            op: IROp::Return(None),
                                        },
                                    );
                                }

                                self.pop_scope();
                                if let Some(f) = self.current_func.take() {
                                    functions.push(f);
                                }
                            }
                        }
                    }
                }
                Stmt::Declaration(Decl::EnumDecl {
                    name: enum_name,
                    variants,
                    ..
                }) => {
                    let any_has_payload = variants.iter().any(|v| {
                        !matches!(
                            v.payload,
                            crate::frontend::parser::ast::EnumVariantPayload::None
                        )
                    });
                    if any_has_payload {
                        self.enum_has_payload.insert(enum_name.clone());
                    }
                    for (variant_idx, variant) in variants.iter().enumerate() {
                        self.enum_variant_tags
                            .insert(variant.name.clone(), variant_idx);
                        self.enum_variant_tags
                            .insert(format!("{}::{}", enum_name, variant.name), variant_idx);
                        self.enum_variant_tags
                            .insert(format!("{}_{}", enum_name, variant.name), variant_idx);

                        let (ir_params, field_names) = match &variant.payload {
                            crate::frontend::parser::ast::EnumVariantPayload::None => {
                                (Vec::new(), Vec::new())
                            }
                            crate::frontend::parser::ast::EnumVariantPayload::Tuple(tys) => {
                                let p: Vec<(String, IRType)> = tys
                                    .iter()
                                    .enumerate()
                                    .map(|(i, ty)| (format!("arg{}", i), IRType::from_ast(ty)))
                                    .collect();
                                let f: Vec<String> =
                                    (0..tys.len()).map(|i| format!("arg{}", i)).collect();
                                (p, f)
                            }
                            crate::frontend::parser::ast::EnumVariantPayload::Struct(params) => {
                                let p: Vec<(String, IRType)> = params
                                    .iter()
                                    .map(|param| {
                                        (param.name.clone(), IRType::from_ast(&param.type_node))
                                    })
                                    .collect();
                                let f: Vec<String> =
                                    params.iter().map(|param| param.name.clone()).collect();
                                (p, f)
                            }
                        };
                        self.enum_variant_fields
                            .insert(variant.name.clone(), field_names.clone());
                        self.enum_variant_fields.insert(
                            format!("{}::{}", enum_name, variant.name),
                            field_names.clone(),
                        );
                        self.enum_variant_fields
                            .insert(format!("{}_{}", enum_name, variant.name), field_names);

                        let func_names = vec![
                            variant.name.clone(),
                            format!("{}_{}", enum_name, variant.name),
                        ];
                        for fn_alias in func_names {
                            self.fn_signatures
                                .entry(fn_alias.clone())
                                .or_default()
                                .push((
                                    fn_alias.clone(),
                                    ir_params.iter().map(|(_, ty)| ty.clone()).collect(),
                                ));

                            let mut var_func = IRFunction::new(
                                fn_alias.clone(),
                                ir_params.clone(),
                                IRType::Int(Size::S64),
                            );
                            let b_id = 0;
                            if any_has_payload {
                                let total_bytes = 8 + (ir_params.len() * 8).max(8);
                                let size_reg = var_func.new_vreg();
                                var_func.add_inst(
                                    b_id,
                                    IRInstruction {
                                        id: Some(size_reg),
                                        op: IROp::ConstInt64(total_bytes as i64),
                                    },
                                );
                                let ptr = var_func.new_vreg();
                                self.called_functions
                                    .entry("fast_alloc".to_string())
                                    .or_insert_with(|| vec![IRType::UInt(Size::S64)]);
                                var_func.add_inst(
                                    b_id,
                                    IRInstruction {
                                        id: Some(ptr),
                                        op: IROp::Call {
                                            func: "fast_alloc".to_string(),
                                            args: vec![size_reg],
                                        },
                                    },
                                );
                                let tag_val = var_func.new_vreg();
                                var_func.add_inst(
                                    b_id,
                                    IRInstruction {
                                        id: Some(tag_val),
                                        op: IROp::ConstInt64(variant_idx as i64),
                                    },
                                );
                                var_func.add_inst(
                                    b_id,
                                    IRInstruction {
                                        id: None,
                                        op: IROp::StoreMemory {
                                            ptr,
                                            value: tag_val,
                                        },
                                    },
                                );
                                for (p_idx, (_p_name, p_ty)) in ir_params.iter().enumerate() {
                                    let alloc_ptr = var_func.new_vreg();
                                    var_func.add_inst(
                                        b_id,
                                        IRInstruction {
                                            id: Some(alloc_ptr),
                                            op: IROp::Alloc { ty: p_ty.clone() },
                                        },
                                    );
                                    var_func.add_inst(
                                        b_id,
                                        IRInstruction {
                                            id: None,
                                            op: IROp::StoreParam {
                                                param_idx: p_idx,
                                                ptr: alloc_ptr,
                                            },
                                        },
                                    );
                                    let param_val = var_func.new_vreg();
                                    var_func.add_inst(
                                        b_id,
                                        IRInstruction {
                                            id: Some(param_val),
                                            op: IROp::Load {
                                                ptr: alloc_ptr,
                                                ty: p_ty.clone(),
                                            },
                                        },
                                    );
                                    let field_offset = 8 + (p_idx * 8);
                                    let field_ptr = var_func.new_vreg();
                                    var_func.add_inst(
                                        b_id,
                                        IRInstruction {
                                            id: Some(field_ptr),
                                            op: IROp::GetFieldPtr {
                                                ptr,
                                                offset: field_offset as i32,
                                            },
                                        },
                                    );
                                    var_func.add_inst(
                                        b_id,
                                        IRInstruction {
                                            id: None,
                                            op: IROp::StoreMemory {
                                                ptr: field_ptr,
                                                value: param_val,
                                            },
                                        },
                                    );
                                }
                                var_func.add_inst(
                                    b_id,
                                    IRInstruction {
                                        id: None,
                                        op: IROp::Return(Some(ptr)),
                                    },
                                );
                            } else {
                                let tag_val = var_func.new_vreg();
                                var_func.add_inst(
                                    b_id,
                                    IRInstruction {
                                        id: Some(tag_val),
                                        op: IROp::ConstInt64(variant_idx as i64),
                                    },
                                );
                                var_func.add_inst(
                                    b_id,
                                    IRInstruction {
                                        id: None,
                                        op: IROp::Return(Some(tag_val)),
                                    },
                                );
                            }
                            functions.push(var_func);
                        }
                    }
                }
                _ => {}
            }
        }

        // Register only functions that were ACTUALLY CALLED in this module/file as externs!
        for (func_name, called_args) in &self.called_functions {
            if !functions.iter().any(|f| f.name == *func_name) {
                let mut found_params = None;
                for (_name, sigs) in &self.fn_signatures {
                    if let Some((_, p)) = sigs.iter().find(|(m, _)| m == func_name) {
                        found_params = Some(p.clone());
                        break;
                    }
                }
                let param_tys = found_params.unwrap_or_else(|| called_args.clone());
                let ir_params = param_tys
                    .iter()
                    .enumerate()
                    .map(|(i, ty)| (format!("arg{}", i), ty.clone()))
                    .collect();
                let ret_ty = if func_name.starts_with("fast_print")
                    || func_name == "fast_free"
                    || func_name == "fast_exit"
                {
                    IRType::Void
                } else if func_name == "fast_alloc" {
                    IRType::Pointer(Box::new(IRType::Int(Size::S8)))
                } else if func_name == "fast_open" || func_name == "fast_close" {
                    IRType::Int(Size::S32)
                } else if func_name == "fast_read" || func_name == "fast_write" {
                    IRType::UInt(Size::S64)
                } else {
                    IRType::Int(Size::S64)
                };
                let ext_func = IRFunction::new_extern(func_name.clone(), ir_params, ret_ty);
                functions.push(ext_func);
            }
        }

        IRModule {
            name: self.module_name.clone(),
            functions,
            structs: self.structs.clone(),
        }
    }
}
