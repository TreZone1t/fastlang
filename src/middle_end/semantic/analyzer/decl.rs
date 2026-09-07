use super::*;

pub fn make_sig_key(name: &str, params: &[Param]) -> String {
    let types: Vec<String> = params.iter().map(|p| p.type_node.as_str()).collect();
    format!("{}#{}", name, types.join(","))
}

impl SemanticAnalyzer {
    pub fn pre_register_decl(&mut self, decl: &Decl) -> Result<(), String> {
        match decl {
            Decl::CompileDecl { name, decls } => {
                for d in decls {
                    let mut namespaced_d = d.clone();
                    match &mut namespaced_d {
                        Decl::FnDecl { name: fn_name, .. } => {
                            *fn_name = format!("{}::{}", name, fn_name);
                        }
                        Decl::MicroDecl { name: m_name, .. } => {
                            *m_name = format!("{}::{}", name, m_name);
                        }
                        Decl::MacroDecl { name: m_name, .. } => {
                            *m_name = format!("{}::{}", name, m_name);
                        }
                        _ => {}
                    }
                    if let Decl::MacroDecl {
                        name: ref m_name,
                        params,
                        body,
                        visibility,
                        ..
                    } = namespaced_d
                    {
                        let prefix = format!("{}::", name);
                        let short_name = m_name.replace(&prefix, "");
                        let without_dollar = short_name.trim_start_matches('$').to_string();
                        let aliases = vec![
                            m_name.clone(),
                            format!("{}::{}", name, without_dollar),
                            short_name.clone(),
                            without_dollar.clone(),
                        ];
                        for alias in aliases {
                            let alias_info = SymbolInfo {
                                name: alias.clone(),
                                kind: SymbolKind::Macro {
                                    params: params.clone(),
                                    return_type: BaseType::Void,
                                    body: body.clone(),
                                },
                                visibility: visibility.clone(),
                                dependencies: vec![],
                                is_used: false,
                                is_param: false,
                                is_uninitialized: false,
                                is_compilable: true,
                            };
                            self.current_env.borrow_mut().define_or_update(alias, alias_info);
                        }
                    } else if let Decl::FnDecl {
                        name: ref f_name,
                        generics,
                        params,
                        return_type,
                        body,
                        visibility: _,
                        ..
                    } = namespaced_d
                    {
                        let fn_info = SymbolInfo {
                            name: f_name.clone(),
                            kind: SymbolKind::Function {
                                params: params.clone(),
                                return_type: return_type.clone(),
                                generics: generics.clone(),
                                body: Some(body.clone()),
                            },
                            visibility: Visibility::Public,
                            dependencies: vec![],
                            is_used: false,
                            is_param: false,
                            is_uninitialized: false,
                            is_compilable: true,
                        };
                        self.current_env.borrow_mut().define_or_update(f_name.clone(), fn_info);
                    } else if let Decl::ImplDecl {
                        target,
                        methods,
                        handle_block,
                        ..
                    } = d
                    {
                        let entry = self.global_metadata.entry(target.clone()).or_insert_with(|| TypeMetadata {
                            name: target.clone(),
                            ty: BaseType::from_str(target),
                            fields: std::collections::HashMap::new(),
                            methods: std::collections::HashMap::new(),
                            constructor: None,
                            handles: vec![],
                            vars: std::collections::HashMap::new(),
                            variants: None,
                        });
                        for m in methods.iter().chain(handle_block.iter()) {
                            if let Decl::FnDecl { name, generics, params, return_type, .. } = m {
                                entry.methods.insert(
                                    name.clone(),
                                    FnType {
                                        name: name.clone(),
                                        generics: generics.clone(),
                                        params: params.clone(),
                                        return_type: return_type.clone(),
                                        mode: ExecutionMode::Runtime,
                                    },
                                );
                            }
                        }
                    }
                }
            }
            Decl::MacroDecl {
                name,
                params,
                body,
                visibility,
                ..
            } => {
                let without_dollar = name.trim_start_matches('$').to_string();
                let aliases = vec![name.clone(), without_dollar];
                for alias in aliases {
                    let alias_info = SymbolInfo {
                        name: alias.clone(),
                        kind: SymbolKind::Macro {
                            params: params.clone(),
                            return_type: BaseType::Void,
                            body: body.clone(),
                        },
                        visibility: visibility.clone(),
                        dependencies: vec![],
                        is_used: false,
                        is_param: false,
                        is_uninitialized: false,
                        is_compilable: true,
                    };
                    self.current_env.borrow_mut().define_or_update(alias, alias_info);
                }
            }
            Decl::ImplDecl {
                target,
                methods,
                handle_block,
                ..
            } => {
                let entry = self.global_metadata.entry(target.clone()).or_insert_with(|| TypeMetadata {
                    name: target.clone(),
                    ty: BaseType::from_str(target),
                    fields: std::collections::HashMap::new(),
                    methods: std::collections::HashMap::new(),
                    constructor: None,
                    handles: vec![],
                    vars: std::collections::HashMap::new(),
                    variants: None,
                });
                for m in methods.iter().chain(handle_block.iter()) {
                    if let Decl::FnDecl { name, generics, params, return_type, .. } = m {
                        entry.methods.insert(
                            name.clone(),
                            FnType {
                                name: name.clone(),
                                generics: generics.clone(),
                                params: params.clone(),
                                return_type: return_type.clone(),
                                mode: ExecutionMode::Runtime,
                            },
                        );
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub(crate) fn visit_declaration(&mut self, decl: &Decl) -> Result<(), String> {
        match decl {
            Decl::VarDecl {
                visibility,
                editability,
                type_node,
                place: _,
                name,
                value,
                assign_op,
            } => {
                self.analyze_var_decl(visibility, editability, type_node, name, value, assign_op)?;
            }

            Decl::DestructureDecl {
                visibility,
                editability,
                type_node,
                assignments,
                assign_op,
                ..
            } => {
                for (name, val) in assignments {
                    self.analyze_var_decl(
                        visibility,
                        editability,
                        type_node,
                        name,
                        val,
                        assign_op,
                    )?;
                }
            }

            Decl::ObjectDestructureDecl {
                visibility,
                editability,
                type_name,
                fields,
                rhs,
                ..
            } => {
                let rhs_type = self.visit_expression(rhs)?;
                let clean_rhs = if let Some(t) = type_name {
                    t.clone()
                } else {
                    extract_blueprint_name_from_type(&rhs_type).unwrap_or_else(|| rhs_type.clone())
                };
                let bp_data = self.current_env.borrow().lookup_blueprint(&clean_rhs);
                let bp_meta = self.global_metadata.get(&clean_rhs).cloned();
                for (idx, (type_node, name)) in fields.iter().enumerate() {
                    let actual_type = if matches!(type_node, BaseType::Unknown) {
                        if let Some(ref bp) = bp_data {
                            if let Some(ty) = bp.fields.get(name) {
                                ty.clone()
                            } else {
                                BaseType::Unknown
                            }
                        } else if let Some(ref meta) = bp_meta {
                            if let Some(ty) = meta.fields.get(name) {
                                ty.clone()
                            } else {
                                // Try getting from constructor params
                                let params = meta
                                    .constructor
                                    .as_ref()
                                    .and_then(|ctors| ctors.first())
                                    .map(|c| &c.params[..]);
                                if let Some(p) = params {
                                    if idx < p.len() {
                                        p[idx].type_node.clone()
                                    } else {
                                        BaseType::Unknown
                                    }
                                } else {
                                    BaseType::Unknown
                                }
                            }
                        } else {
                            BaseType::Unknown
                        }
                    } else {
                        type_node.clone()
                    };
                    let info = SymbolInfo {
                        name: name.clone(),
                        kind: SymbolKind::Variable {
                            type_node: actual_type,
                            editability: editability.clone(),
                            is_array: false,
                        },
                        visibility: visibility.clone(),
                        dependencies: vec![],
                        is_used: false,
                        is_param: false,
                        is_uninitialized: false,
                        is_compilable: false,
                    };
                    self.current_env.borrow_mut().define(name.clone(), info)?;
                }
            }

            Decl::ArrayDecl {
                visibility,
                editability,
                type_node,
                name,
                length,
                value,
                ..
            } => {
                let expr_type = self.visit_expression(value)?;
                self.visit_expression(length)?;

                let declared_type = type_node.as_str();
                let array_inner = if expr_type.starts_with("array<") {
                    expr_type
                        .trim_start_matches("array<")
                        .trim_end_matches('>')
                        .to_string()
                } else {
                    expr_type.clone()
                };
                let array_inner_base = array_inner.split('[').next().unwrap_or(&array_inner);

                if expr_type != "unknown"
                    && expr_type != "default"
                    && !self.types_are_compatible(&declared_type, &expr_type)
                    && !self.types_are_compatible(&declared_type, &array_inner)
                    && !self.types_are_compatible(&declared_type, array_inner_base)
                    && !(declared_type.contains("char") && expr_type == "str")
                {
                    return Err(format!(
                        "Semantic Error: Type mismatch for array '{}'. Declared '{}', got '{}'",
                        name, declared_type, expr_type
                    ));
                }

                let info = SymbolInfo {
                    name: name.clone(),
                    kind: SymbolKind::Variable {
                        type_node: type_node.clone(),
                        editability: editability.clone(),
                        is_array: true,
                    },
                    visibility: visibility.clone(),
                    dependencies: vec![],
                    is_used: false,
                    is_param: false,
                    is_uninitialized: false,
                    is_compilable: false,
                };
                self.current_env.borrow_mut().define(name.clone(), info)?;
                if matches!(value, Expr::Instantiate { .. }) || type_node.as_str().ends_with('*') {
                    self.heap_allocated_vars.insert(name.clone());
                }
            }

            Decl::ClassDecl {
                visibility,
                name,
                extends,
                handles,
                public_block,
                private_block,
                static_block,
                handle_block,
                constructor,
                ..
            } => {
                self.analyze_class_or_struct_decl(
                    visibility.clone(),
                    name,
                    true,
                    extends.as_deref(),
                    handles,
                    public_block,
                    private_block,
                    static_block,
                    handle_block,
                    constructor,
                )?;
            }

            Decl::StructDecl {
                visibility,
                name,
                handles,
                public_block,
                private_block,
                handle_block,
                static_block,
                constructor,
                ..
            } => {
                self.analyze_class_or_struct_decl(
                    visibility.clone(),
                    name,
                    false,
                    None,
                    handles,
                    public_block,
                    private_block,
                    static_block,
                    handle_block,
                    constructor,
                )?;
            }

            Decl::EnumDecl {
                visibility,
                name,
                handle_block,
                variants,
                ..
            } => {
                let info = self.make_blueprint_symbol(name, visibility.clone());
                self.current_env.borrow_mut().define(name.clone(), info)?;
                for v in variants {
                    self.dependency_graph.entry(v.name.clone()).or_default().insert(name.clone());
                    self.dependency_graph.entry(name.clone()).or_default().insert(v.name.clone());
                    let var_symbol = SymbolInfo {
                        name: v.name.clone(),
                        kind: SymbolKind::Variable {
                            type_node: BaseType::from_str(name),
                            editability: Editability::NotEditable,
                            is_array: false,
                        },
                        visibility: visibility.clone(),
                        dependencies: vec![],
                        is_used: false,
                        is_param: false,
                        is_uninitialized: false,
                        is_compilable: false,
                    };
                    let _ = self
                        .current_env
                        .borrow_mut()
                        .define(v.name.clone(), var_symbol);
                }
                self.enter_scope();
                let prev = self.in_custom_scope;
                self.in_custom_scope = true;
                let prev_type = self.current_type_name.clone();
                self.current_type_name = Some(name.clone());
                for d in handle_block {
                    if let Decl::FnDecl { is_virtual: true, .. } = d {
                        return Err(format!(
                            "Semantic Error: 'virtual' modifier is only allowed in 'class' methods, not in enum '{}'",
                            name
                        ));
                    }
                    self.visit_declaration(d)?;
                }
                self.current_type_name = prev_type;
                self.leave_scope();
                self.in_custom_scope = prev;
            }

            Decl::BlockDecl {
                visibility,
                name,
                return_type,
                statements,
            } => {
                let mut bp = BlueprintData::new(name);
                let prev_flags = self.active_flags.clone();
                let prev_return = self.active_return_type.clone();
                self.active_flags.push("+has_yield".to_string());
                self.active_flags.push("+has_leave".to_string());
                self.active_flags.push("+has_return".to_string());
                self.active_flags.push("+has_break".to_string());
                self.active_flags.push("+has_continue".to_string());
                self.active_flags.push("+has_throw".to_string());
                self.active_return_type = return_type.clone();

                self.enter_scope();
                for s in statements {
                    if let Stmt::Declaration(Decl::VarDecl {
                        name: f_name,
                        type_node,
                        ..
                    }) = s
                    {
                        bp.fields.insert(f_name.clone(), type_node.clone());
                    }
                    if let Stmt::Declaration(Decl::FnDecl {
                        name: fn_name,
                        params,
                        return_type,
                        generics,
                        ..
                    }) = s
                    {
                        bp.methods.insert(
                            fn_name.clone(),
                            FnSignature {
                                name: fn_name.clone(),
                                generics: generics.clone(),
                                params: params.clone(),
                                return_type: return_type.clone(),
                                is_virtual: false,
                                is_abstract: false,
                            },
                        );
                    }
                    self.visit_statement(s)?;
                }
                self.leave_scope();
                self.active_flags = prev_flags;
                self.active_return_type = prev_return;

                let mut block_fields = std::collections::HashMap::new();
                for (fname, ftype) in bp.fields.iter() {
                    block_fields.insert(fname.clone(), ftype.clone());
                }
                let mut block_methods = std::collections::HashMap::new();
                for (mname, msig) in bp.methods.iter() {
                    block_methods.insert(
                        mname.clone(),
                        FnType {
                            name: mname.clone(),
                            generics: vec![],
                            params: msig.params.clone(),
                            return_type: msig.return_type.clone(),
                            mode: ExecutionMode::Runtime,
                        },
                    );
                }

                self.current_env
                    .borrow_mut()
                    .define_blueprint(name.to_string(), bp);

                let info = SymbolInfo {
                    name: name.clone(),
                    kind: SymbolKind::Variable {
                        type_node: BaseType::Block {
                            name: name.clone(),
                            fields: Box::new(block_fields),
                            methods: Box::new(block_methods),
                        },
                        editability: Editability::Editable,
                        is_array: false,
                    },
                    visibility: visibility.clone(),
                    dependencies: vec![],
                    is_used: false,
                    is_param: false,
                    is_uninitialized: false,
                    is_compilable: false,
                };
                self.current_env
                    .borrow_mut()
                    .define(name.to_string(), info)?;
            }

            Decl::MachineDecl {
                visibility,
                name,
                return_type: _,
                labels,
                handle_block,
            } => {
                // Register the machine as a custom blueprint type
                let mut bp = BlueprintData::new(name);

                let mut label_map = std::collections::HashMap::new();
                for l in labels.iter() {
                    if let Decl::LabelDecl {
                        name: lbl_name,
                        body: stmts,
                        ..
                    } = l
                    {
                        let mut lbl_fields = std::collections::HashMap::new();
                        for s in stmts {
                            if let Stmt::Declaration(Decl::VarDecl {
                                name: var_name,
                                type_node,
                                ..
                            }) = s
                            {
                                lbl_fields.insert(var_name.clone(), type_node.clone());
                            }
                        }
                        label_map.insert(
                            lbl_name.clone(),
                            BaseType::Label {
                                name: lbl_name.clone(),
                                fields: Box::new(lbl_fields),
                            },
                        );
                    }
                }

                let info = SymbolInfo {
                    name: name.clone(),
                    kind: SymbolKind::Variable {
                        type_node: BaseType::Machine {
                            name: name.clone(),
                            fields: Box::new(std::collections::HashMap::new()),
                            methods: Box::new(std::collections::HashMap::new()),
                            labels: Box::new(label_map),
                            mode: ExecutionMode::Runtime,
                        },
                        editability: Editability::Editable,
                        is_array: false,
                    },
                    visibility: visibility.clone(),
                    dependencies: vec![],
                    is_used: false,
                    is_param: false,
                    is_uninitialized: false,
                    is_compilable: false,
                };
                self.current_env
                    .borrow_mut()
                    .define(name.to_string(), info)?;

                if let Some(Decl::LabelDecl {
                    name: last_lbl_name,
                    body: last_body,
                }) = labels.last()
                {
                    fn stmts_contain_continue(stmts: &[Stmt]) -> bool {
                        for s in stmts {
                            match s {
                                Stmt::ContinueStmt => return true,
                                Stmt::Block(inner) | Stmt::ThisBlock(inner) => {
                                    if stmts_contain_continue(inner) {
                                        return true;
                                    }
                                }
                                Stmt::IfStmt {
                                    then_block,
                                    else_block,
                                    ..
                                } => {
                                    if stmts_contain_continue(then_block) {
                                        return true;
                                    }
                                    if let Some(eb) = else_block {
                                        if stmts_contain_continue(eb) {
                                            return true;
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                        false
                    }

                    if stmts_contain_continue(last_body) {
                        return Err(format!(
                            "Semantic Error: Cannot 'continue' in final label '{}' of machine '{}' as there is no subsequent label to advance to.",
                            last_lbl_name, name
                        ));
                    }
                }

                let mut label_vars: HashMap<String, HashMap<String, BaseType>> = HashMap::new();
                for label_decl in labels {
                    if let Decl::LabelDecl { name: l_name, body } = label_decl {
                        let clean_name = l_name.replace("@", "");
                        let mut vars = HashMap::new();
                        for s in body {
                            if let Stmt::Declaration(Decl::VarDecl {
                                name: v_name,
                                type_node,
                                ..
                            }) = s
                            {
                                vars.insert(v_name.clone(), type_node.clone());
                            }
                        }
                        label_vars.insert(clean_name.clone(), vars.clone());
                        label_vars.insert(l_name.clone(), vars);
                    }
                }
                self.machine_labels.insert(name.clone(), label_vars);
                let prev_machine = self.current_machine.clone();
                self.current_machine = Some(name.clone());

                let prev = self.in_custom_scope;
                self.in_custom_scope = true;
                self.enter_scope();

                // Analyze each label body
                for label_decl in labels {
                    if let Decl::LabelDecl { body, .. } = label_decl {
                        for s in body {
                            let _ = self.visit_statement(s);
                        }
                    }
                }

                // Analyze handle block (only call/leave allowed, enforced by parser)
                for d in handle_block {
                    if let Decl::FnDecl {
                        name: f_name,
                        params,
                        return_type,
                        is_virtual,
                        is_abstract,
                        generics,
                        ..
                    } = d
                    {
                        if *is_virtual {
                            return Err(format!(
                                "Semantic Error: 'virtual' modifier is only allowed in 'class' methods, not in machine '{}'",
                                name
                            ));
                        }
                        bp.methods.insert(
                            f_name.clone(),
                            crate::middle_end::semantic::environment::FnSignature {
                                name: f_name.clone(),
                                generics: generics.clone(),
                                params: params.clone(),
                                return_type: return_type.clone(),
                                is_virtual: *is_virtual,
                                is_abstract: *is_abstract,
                            },
                        );
                    }
                    let _ = self.visit_declaration(d);
                }

                self.leave_scope();
                self.in_custom_scope = prev;
                self.current_machine = prev_machine;
                self.current_env
                    .borrow_mut()
                    .define_blueprint(name.to_string(), bp);
            }

            Decl::FnDecl {
                visibility,
                name,
                params,
                return_type,
                body,
                is_virtual,
                is_abstract,
                generics,
                ..
            } => {
                if *is_virtual && self.current_context.is_none() && !self.in_custom_scope {
                    return Err(format!(
                        "Semantic Error: 'virtual' modifier is only allowed in 'class' methods, not in standalone function '{}'",
                        name
                    ));
                }
                let mut resolved_params = params.clone();
                for p in &mut resolved_params {
                    if let Some(ref def_val) = p.default_value {
                        let inferred = self.visit_expression(def_val)?;
                        if matches!(p.type_node, BaseType::Unknown) {
                            if let Some(bp) = self.current_env.borrow().lookup_blueprint(&inferred)
                            {
                                if bp.is_class {
                                    p.type_node = BaseType::Class {
                                        name: inferred.clone(),
                                        fields: Box::new(HashMap::new()),
                                        methods: Box::new(HashMap::new()),
                                        constructor: None,
                                        generics: Vec::new(),
                                    };
                                } else {
                                    p.type_node = BaseType::Struct {
                                        name: inferred.clone(),
                                        fields: Box::new(HashMap::new()),
                                        methods: Box::new(HashMap::new()),
                                        generics: Vec::new(),
                                    };
                                }
                            } else if let Some(meta) = self.global_metadata.get(&inferred) {
                                if meta.ty.as_str().starts_with("class") {
                                    p.type_node = BaseType::Class {
                                        name: inferred.clone(),
                                        fields: Box::new(HashMap::new()),
                                        methods: Box::new(HashMap::new()),
                                        constructor: None,
                                        generics: Vec::new(),
                                    };
                                } else {
                                    p.type_node = BaseType::Struct {
                                        name: inferred.clone(),
                                        fields: Box::new(HashMap::new()),
                                        methods: Box::new(HashMap::new()),
                                        generics: Vec::new(),
                                    };
                                }
                            } else {
                                p.type_node = match inferred.as_str() {
                                    "int" | "int32" => BaseType::Int(Size::S32),
                                    "int8" => BaseType::Int(Size::S8),
                                    "int16" => BaseType::Int(Size::S16),
                                    "int64" => BaseType::Int(Size::S64),
                                    "uint" | "uint32" => BaseType::UInt(Size::S32),
                                    "uint8" => BaseType::UInt(Size::S8),
                                    "uint16" => BaseType::UInt(Size::S16),
                                    "uint64" => BaseType::UInt(Size::S64),
                                    "float" | "float32" => BaseType::Float(Size::S32),
                                    "float64" => BaseType::Float(Size::S64),
                                    "bool" => BaseType::Bool,
                                    "str" => BaseType::Str,
                                    "char" => BaseType::Char,
                                    _ => BaseType::Blueprint {
                                        name: inferred.clone(),
                                        fields: Box::new(HashMap::new()),
                                        methods: Box::new(HashMap::new()),
                                        generics: Vec::new(),
                                    },
                                };
                            }
                        } else if !self.types_are_compatible(&p.type_node.as_str(), &inferred) {
                            return Err(format!(
                                "Semantic Error: Default value for parameter '{}' has type '{}', incompatible with declared type '{}'.",
                                p.name, inferred, p.type_node.as_str()
                            ));
                        }
                    }
                }

                let mut seen_default = false;
                for p in &resolved_params {
                    if p.default_value.is_some() {
                        seen_default = true;
                    } else if seen_default {
                        return Err(format!(
                            "Semantic Error: Required parameter '{}' cannot follow a parameter with a default value in function '{}'.",
                            p.name, name
                        ));
                    }
                }

                let is_member_method = self.current_type_name.is_some()
                    || self.in_custom_scope
                    || self.in_class
                    || self.in_struct;

                if !is_member_method {
                    let is_var = resolved_params
                        .last()
                        .map(|p| p.is_variadic)
                        .unwrap_or(false);
                    let min_args = resolved_params
                        .iter()
                        .take_while(|p| p.default_value.is_none() && !p.is_variadic)
                        .count();
                    let _max_args = if is_var {
                        usize::MAX
                    } else {
                        resolved_params.len()
                    };

                    if let Some(existing_sigs) = self.fn_overloads.get(name) {
                        for ex in existing_sigs {
                            let _ex_var = ex.params.last().map(|p| p.is_variadic).unwrap_or(false);
                            let ex_min = ex
                                .params
                                .iter()
                                .take_while(|p| p.default_value.is_none() && !p.is_variadic)
                                .count();
                            let ex_max = ex.params.len();
                            for ex_len in ex_min..=ex_max {
                                let ex_prefix: Vec<String> = ex.params[..ex_len]
                                    .iter()
                                    .map(|p| p.type_node.as_str())
                                    .collect();
                                for len in min_args..=resolved_params.len() {
                                    let prefix: Vec<String> = resolved_params[..len]
                                        .iter()
                                        .map(|p| p.type_node.as_str())
                                        .collect();
                                    if prefix == ex_prefix {
                                        return Err(format!(
                                            "Semantic Error: Overload conflict for function '{}': default parameters create an ambiguity with existing overload.",
                                            name
                                        ));
                                    }
                                }
                            }
                        }
                    }

                    self.fn_overloads
                        .entry(name.clone())
                        .or_default()
                        .push(FnSignature {
                            name: name.clone(),
                            generics: generics.clone(),
                            params: resolved_params.clone(),
                            return_type: return_type.clone(),
                            is_virtual: *is_virtual,
                            is_abstract: *is_abstract,
                        });
                }
                let exec_mode = detect_execution_mode(body);
                let is_compilable_fn = name.starts_with("@compile::") || exec_mode == ExecutionMode::FullyCompilable;

                let fn_info = SymbolInfo {
                    name: name.clone(),
                    kind: SymbolKind::Function {
                        params: resolved_params.clone(),
                        return_type: return_type.clone(),
                        generics: generics.clone(),
                        body: Some(body.clone()),
                    },
                    visibility: visibility.clone(),
                    dependencies: vec![],
                    is_used: false,
                    is_param: false,
                    is_uninitialized: false,
                    is_compilable: is_compilable_fn,
                };
                if self.in_custom_scope {
                    self.current_env
                        .borrow_mut()
                        .define_or_update(name.clone(), fn_info);
                } else {
                    self.current_env
                        .borrow_mut()
                        .define(name.clone(), fn_info)?;
                }

                let prev_in_generic = self.in_generic_template;
                if !generics.is_empty() {
                    self.in_generic_template = true;
                }
                let prev_flags = self.active_flags.clone();
                let prev_return = self.active_return_type.clone();
                let prev_context = self.current_context.clone();
                let prev_deleted = self.deleted_vars.clone();
                let prev_heap = self.heap_allocated_vars.clone();
                self.deleted_vars.clear();
                self.heap_allocated_vars.clear();
                let ctx_name = if let Some(ref tname) = self.current_type_name {
                    format!("{}::{}", tname, name)
                } else {
                    name.clone()
                };
                self.current_context = Some(ctx_name.clone());
                let sig_key = make_sig_key(name, &resolved_params);
                self.dependency_graph.entry(sig_key).or_default().insert(ctx_name.clone());
                self.active_flags.push("+has_return".to_string());
                self.active_flags.push("+has_throw".to_string());
                self.active_flags.push("+has_yield".to_string());
                self.active_flags.push("+has_leave".to_string());
                self.active_return_type = Some(return_type.clone());

                self.enter_scope();
                for g in generics {
                    let g_name = match g {
                        BaseType::GenericParam(name) => name.clone(),
                        BaseType::New(name) => name.clone(),
                        _ => g.as_str(),
                    };
                    if !g_name.is_empty() && g_name != "unknown" {
                        let clean_name = g_name.trim_start_matches("...").to_string();
                        let g_info = SymbolInfo {
                            name: clean_name.clone(),
                            kind: SymbolKind::Variable {
                                type_node: BaseType::Type(Box::new(BaseType::GenericParam(clean_name.clone()))),
                                editability: Editability::NotEditable,
                                is_array: false,
                            },
                            visibility: Visibility::Public,
                            dependencies: vec![],
                            is_used: true,
                            is_param: false,
                            is_uninitialized: false,
                            is_compilable: false,
                        };
                        let _ = self.current_env.borrow_mut().define(clean_name.clone(), g_info.clone());
                        if g_name != clean_name {
                            let _ = self.current_env.borrow_mut().define(g_name, g_info);
                        }
                    }
                }
                for p in &resolved_params {
                    for dep in extract_all_type_names(&p.type_node.as_str()) {
                        self.record_dependency(dep);
                    }
                    let (param_type, is_arr) = if p.is_variadic {
                        (
                            BaseType::Array {
                                base_type: Box::new(p.type_node.clone()),
                                size: Box::new(None),
                            },
                            true,
                        )
                    } else {
                        (p.type_node.clone(), false)
                    };
                    let param_info = SymbolInfo {
                        name: p.name.clone(),
                        kind: SymbolKind::Variable {
                            type_node: param_type,
                            editability: Editability::Editable,
                            is_array: is_arr,
                        },
                        visibility: Visibility::Public,
                        dependencies: vec![],
                        is_used: body.is_empty() || *is_abstract || p.name.starts_with('_'),
                        is_param: true,
                        is_uninitialized: false,
                        is_compilable: false,
                    };
                    self.current_env
                        .borrow_mut()
                        .define(p.name.clone(), param_info)?;
                }
                for dep in extract_all_type_names(&return_type.as_str()) {
                    self.record_dependency(dep);
                }
                for s in body {
                    self.visit_statement(s)?;
                }

                if !self.in_custom_scope
                    && !body.is_empty()
                    && *return_type != BaseType::Void
                    && !self.block_always_terminates(body)
                {
                    return Err(
                        format!(
                            "Semantic Error: Not all control paths return a value in function '{}' (declared return type '{}').",
                            name,
                            return_type.as_str()
                        )
                    );
                }

                self.leave_scope();

                self.active_flags = prev_flags;
                self.active_return_type = prev_return;
                self.current_context = prev_context;
                self.deleted_vars = prev_deleted;
                self.heap_allocated_vars = prev_heap;
                self.in_generic_template = prev_in_generic;
            }

            Decl::MicroDecl {
                visibility,
                name,
                params,
                return_type,
                body,
                generics,
            } => {
                let prev_in_generic = self.in_generic_template;
                if generics.as_ref().map(|g| !g.is_empty()).unwrap_or(false) {
                    self.in_generic_template = true;
                }
                let is_compilable_micro = name.starts_with("@compile::");
                let micro_info = SymbolInfo {
                    name: name.clone(),
                    kind: SymbolKind::Function {
                        params: params.clone(),
                        return_type: return_type.clone().unwrap_or(BaseType::Void),
                        generics: generics
                            .as_ref()
                            .map(|v| v.iter().map(|s| BaseType::from_str(s)).collect())
                            .unwrap_or_default(),
                        body: Some(body.clone()),
                    },
                    visibility: visibility.clone(),
                    dependencies: vec![],
                    is_used: false,
                    is_param: false,
                    is_uninitialized: false,
                    is_compilable: is_compilable_micro,
                };
                self.current_env
                    .borrow_mut()
                    .define(name.clone(), micro_info)?;

                let prev_flags = self.active_flags.clone();
                let prev_return = self.active_return_type.clone();
                let prev_context = self.current_context.clone();
                self.current_context = Some(name.clone());
                self.active_flags.push("+has_break".to_string());
                self.active_flags.push("+has_continue".to_string());
                self.active_flags.push("+has_throw".to_string());
                self.active_flags.push("+has_yield".to_string());
                self.active_flags.push("+has_leave".to_string());
                self.active_flags.push("+has_return".to_string());
                self.active_return_type = return_type.clone();

                self.enter_scope();
                for p in params {
                    let param_info = SymbolInfo {
                        name: p.name.clone(),
                        kind: SymbolKind::Variable {
                            type_node: p.type_node.clone(),
                            editability: Editability::Editable,
                            is_array: false,
                        },
                        visibility: Visibility::Public,
                        dependencies: vec![],
                        is_used: false,
                        is_param: true,
                        is_uninitialized: false,
                        is_compilable: false,
                    };
                    self.current_env
                        .borrow_mut()
                        .define(p.name.clone(), param_info)?;
                }
                for s in body {
                    self.visit_statement(s)?;
                }
                self.leave_scope();

                self.active_flags = prev_flags;
                self.active_return_type = prev_return;
                self.current_context = prev_context;
                self.in_generic_template = prev_in_generic;
            }

            Decl::LabelDecl { name, body } => {
                let info = SymbolInfo {
                    name: name.clone(),
                    kind: SymbolKind::Label,
                    visibility: Visibility::Private,
                    dependencies: vec![],
                    is_used: false,
                    is_param: false,
                    is_uninitialized: false,
                    is_compilable: false,
                };
                self.current_env.borrow_mut().define(name.clone(), info)?;
                let prev_flags = self.active_flags.clone();
                self.active_flags.push("+has_yield".to_string());
                self.active_flags.push("+has_leave".to_string());
                self.active_flags.push("+has_return".to_string());
                self.enter_scope();
                for s in body {
                    self.visit_statement(s)?;
                }
                self.leave_scope();
                self.active_flags = prev_flags;
            }

            Decl::BlueprintDecl {
                name, visibility, ..
            } => {
                let info = self.make_blueprint_symbol(name, visibility.clone());
                self.current_env.borrow_mut().define(name.clone(), info)?;
            }

            Decl::ImplDecl {
                target,
                target_generics,
                is_handle_impl: _,
                methods,
                handle_block,
            } => {
                let is_builtin = Self::is_primitive_numeric(target)
                    || matches!(
                        target.as_str(),
                        "char"
                            | "str"
                            | "bool"
                            | "flag"
                            | "array"
                            | "byte"
                            | "usize"
                            | "isize"
                            | "type"
                    );
                let symbol_opt = self.current_env.borrow().lookup(target);
                let meta_opt = self.global_metadata.get(target).cloned();

                if !is_builtin && symbol_opt.is_none() && meta_opt.is_none() {
                    return Err(format!(
                        "Semantic Error: Target '{}' not found for impl block",
                        target
                    ));
                }

                let expected_generics = if let Some(meta) = &meta_opt {
                    match &meta.ty {
                        BaseType::Enum { generics, .. }
                        | BaseType::Blueprint { generics, .. }
                        | BaseType::Class { generics, .. }
                        | BaseType::Struct { generics, .. } => generics.len(),
                        BaseType::Array { .. } => 1,
                        _ => 0,
                    }
                } else if let Some(bp) = self.current_env.borrow().lookup_blueprint(target) {
                    bp.generics.len()
                } else {
                    0
                };

                if expected_generics > 0 && target_generics.len() != expected_generics {
                    return Err(format!(
                        "Semantic Error: Type '{}' requires {} generic parameter(s) in 'impl {}<...>', found {}",
                        target, expected_generics, target, target_generics.len()
                    ));
                }

                let is_target_class = if let Some(meta) = &meta_opt {
                    matches!(&meta.ty, BaseType::Class { .. })
                } else if let Some(bp) = self.current_env.borrow().lookup_blueprint(target) {
                    bp.is_class
                } else {
                    false
                };

                if !is_target_class {
                    for m in methods.iter().chain(handle_block.iter()) {
                        if let Decl::FnDecl { is_virtual: true, .. } = m {
                            return Err(format!(
                                "Semantic Error: 'virtual' modifier is only allowed in 'class' methods, not in '{}'",
                                target
                            ));
                        }
                    }
                }

                if self.global_metadata.get(target).is_none() {
                    self.global_metadata.insert(
                        target.clone(),
                        TypeMetadata {
                            name: target.clone(),
                            ty: BaseType::from_str(target),
                            fields: std::collections::HashMap::new(),
                            methods: std::collections::HashMap::new(),
                            constructor: None,
                            handles: vec![],
                            vars: std::collections::HashMap::new(),
                            variants: None,
                        },
                    );
                }

                let is_fn = symbol_opt
                    .as_ref()
                    .map(|s| matches!(s.kind, SymbolKind::Function { .. }))
                    .unwrap_or(false);
                let is_block = symbol_opt
                    .as_ref()
                    .map(
                        |s|
                            matches!(s.kind, SymbolKind::Variable { ref type_node, .. } if matches!(type_node, BaseType::Block { .. } | BaseType::Machine { .. }))
                    )
                    .unwrap_or(false);

                let op_names = [
                    "add",
                    "sub",
                    "mul",
                    "div",
                    "mod",
                    "index_access",
                    "index_add",
                    "index_sub",
                    "index_mul",
                    "index_div",
                    "index_mod",
                    "arrow",
                    "arrow_assign",
                    "equal",
                    "not_equal",
                    "less_than",
                    "greater_than",
                    "less_than_equal",
                    "greater_than_equal",
                ];

                for h in methods.iter().chain(handle_block.iter()) {
                    if let Decl::FnDecl {
                        name: h_name,
                        params,
                        return_type,
                        ..
                    } = h
                    {
                        if h_name == "error" {
                            self.record_dependency("Error".to_string());
                        }
                        for dep in extract_all_type_names(&return_type.as_str()) {
                            self.record_dependency(dep);
                        }
                        for p in params {
                            for dep in extract_all_type_names(&p.type_node.as_str()) {
                                self.record_dependency(dep);
                            }
                        }
                        if is_fn {
                            if op_names.contains(&h_name.as_str()) || h_name == "call" {
                                return Err(
                                    format!("Semantic Error: Operator overloading and 'call' handles are not allowed on function '{}'", target)
                                );
                            }
                        } else if is_block {
                            if op_names.contains(&h_name.as_str()) {
                                return Err(
                                    format!("Semantic Error: Operator overloading handles are not allowed on block '{}'", target)
                                );
                            }
                        }

                        if matches!(
                            h_name.as_str(),
                            "display"
                                | "iterator"
                                | "iter"
                                | "copy"
                                | "next"
                                | "break"
                                | "continue"
                        ) {
                            if !params.is_empty() {
                                return Err(
                                    format!(
                                        "Semantic Error: Handle method '{}' cannot take any parameters on target '{}'",
                                        h_name,
                                        target
                                    )
                                );
                            }
                            if h_name == "display" {
                                let is_valid = match return_type {
                                    BaseType::Str | BaseType::Char => true,
                                    BaseType::Array { base_type, .. } => {
                                        matches!(base_type.as_ref(), BaseType::Char)
                                    }
                                    _ => false,
                                };
                                if !is_valid {
                                    return Err(
                                        format!(
                                            "Semantic Error: Handle method 'display' must return 'str', 'char', or 'char[]', found '{}' on target '{}'",
                                            return_type.as_str(),
                                            target
                                        )
                                    );
                                }
                            }
                        }
                    }
                }

                self.enter_scope();
                let prev_in_generic = self.in_generic_template;
                if !target_generics.is_empty() {
                    self.in_generic_template = true;
                }
                self.active_flags.push("+has_return".to_string());
                let prev = self.in_struct;
                self.in_struct = true;

                let this_type = if target == "array" {
                    BaseType::Array {
                        base_type: Box::new(
                            target_generics
                                .first()
                                .cloned()
                                .unwrap_or(BaseType::Unknown),
                        ),
                        size: Box::new(None),
                    }
                } else if target == "char" {
                    BaseType::Char
                } else if target == "str" {
                    BaseType::Str
                } else if target == "bool" || target == "flag" {
                    BaseType::Bool
                } else if target.starts_with("int") {
                    BaseType::Int(Size::S32)
                } else if target.starts_with("uint") {
                    BaseType::UInt(Size::S32)
                } else if target.starts_with("float") {
                    BaseType::Float(Size::S64)
                } else {
                    BaseType::from_str(target)
                };

                let prev_type = self.current_type_name.clone();
                self.current_type_name = Some(this_type.as_str());

                for g in target_generics {
                    let g_name = g.as_str();
                    let g_info = SymbolInfo {
                        name: g_name.clone(),
                        kind: SymbolKind::Variable {
                            type_node: BaseType::Type(Box::new(g.clone())),
                            editability: Editability::NotEditable,
                            is_array: false,
                        },
                        visibility: Visibility::Public,
                        dependencies: vec![],
                        is_used: true,
                        is_param: false,
                        is_uninitialized: false,
                        is_compilable: false,
                    };
                    let _ = self.current_env.borrow_mut().define(g_name, g_info);
                }

                let this_info = SymbolInfo {
                    name: "this".to_string(),
                    kind: SymbolKind::Variable {
                        type_node: this_type,
                        editability: Editability::Editable,
                        is_array: target == "array",
                    },
                    visibility: Visibility::Private,
                    dependencies: vec![],
                    is_used: false,
                    is_param: true,
                    is_uninitialized: false,
                    is_compilable: false,
                };
                let _ = self
                    .current_env
                    .borrow_mut()
                    .define("this".to_string(), this_info);

                for m in methods {
                    self.visit_declaration(m)?;
                    if let Decl::FnDecl {
                        name,
                        generics,
                        params,
                        return_type,
                        ..
                    } = m
                    {
                        if let Some(meta) = self.global_metadata.get_mut(target) {
                            meta.methods.insert(
                                name.clone(),
                                FnType {
                                    name: name.clone(),
                                    generics: generics.clone(),
                                    params: params.clone(),
                                    return_type: return_type.clone(),
                                    mode: ExecutionMode::Runtime,
                                },
                            );
                        }
                    }
                }

                for h in handle_block {
                    self.visit_declaration(h)?;
                    if let Decl::FnDecl {
                        name,
                        ..
                    } = h
                    {
                        let hk = HandleMethods::from_str(name.as_str());
                        if let Some(meta) = self.global_metadata.get_mut(target) {
                            if hk != HandleMethods::NotFound && !meta.handles.contains(&hk) {
                                meta.handles.push(hk);
                            }
                        }
                    }
                }

                self.current_type_name = prev_type;
                self.in_struct = prev;
                self.active_flags.retain(|x| x != "+has_return");
                self.leave_scope();
                self.in_generic_template = prev_in_generic;
            }

            Decl::ExternFnDecl {
                name,
                params,
                return_type,
                alias,
                ..
            } => {
                let sym_name = alias.as_ref().unwrap_or(name);
                let info = SymbolInfo {
                    name: sym_name.clone(),
                    kind: SymbolKind::Function {
                        return_type: return_type.clone(),
                        params: params.clone(),
                        generics: vec![],
                        body: None,
                    },
                    visibility: Visibility::Public,
                    dependencies: vec![],
                    is_used: false,
                    is_param: false,
                    is_uninitialized: false,
                    is_compilable: false,
                };
                self.current_env
                    .borrow_mut()
                    .define(sym_name.clone(), info)?;
            }

            Decl::ExternBlockDecl { decls, .. } => {
                for d in decls {
                    self.visit_declaration(d)?;
                }
            }

            Decl::Import { .. } => {}
            Decl::DefineDecl {
                name,
                type_alias,
                value,
                ..
            } => {
                if let Some(t) = type_alias {
                    let info = SymbolInfo {
                        name: name.clone(),
                        kind: SymbolKind::Variable {
                            type_node: BaseType::Type(Box::new(t.clone())),
                            editability: Editability::NotEditable,
                            is_array: false,
                        },
                        visibility: Visibility::Public,
                        dependencies: vec![],
                        is_used: false,
                        is_param: false,
                        is_uninitialized: false,
                        is_compilable: false,
                    };
                    self.current_env.borrow_mut().define(name.clone(), info)?;
                } else if let Some(expr) = value {
                    let _val_type = self.visit_expression(expr)?;
                    let info = SymbolInfo {
                        name: name.clone(),
                        kind: SymbolKind::Variable {
                            type_node: BaseType::Int(Size::S32),
                            editability: Editability::NotEditable,
                            is_array: false,
                        },
                        visibility: Visibility::Public,
                        dependencies: vec![],
                        is_used: false,
                        is_param: false,
                        is_uninitialized: false,
                        is_compilable: false,
                    };
                    self.current_env.borrow_mut().define(name.clone(), info)?;
                }
            }
            Decl::CompileDecl { name, decls } => {
                for d in decls {
                    let mut namespaced_d = d.clone();
                    match &mut namespaced_d {
                        Decl::FnDecl { name: fn_name, visibility, .. } => {
                            *fn_name = format!("{}::{}", name, fn_name);
                            *visibility = Visibility::Public;
                        }
                        Decl::MicroDecl { name: m_name, visibility, .. } => {
                            *m_name = format!("{}::{}", name, m_name);
                            *visibility = Visibility::Public;
                        }
                        Decl::MacroDecl { name: m_name, visibility, .. } => {
                            *m_name = format!("{}::{}", name, m_name);
                            *visibility = Visibility::Public;
                        }
                        _ => {}
                    }
                    self.visit_declaration(&namespaced_d)?;
                    if let Decl::MacroDecl {
                        name: ref m_name,
                        params,
                        body,
                        visibility,
                        ..
                    } = namespaced_d
                    {
                        let prefix = format!("{}::", name);
                        let short_name = m_name.replace(&prefix, "");
                        let without_dollar = short_name.trim_start_matches('$').to_string();
                        let aliases = vec![
                            format!("{}::{}", name, without_dollar),
                            short_name.clone(),
                            without_dollar.clone(),
                        ];
                        for alias in aliases {
                            if alias != *m_name {
                                let alias_info = SymbolInfo {
                                    name: alias.clone(),
                                    kind: SymbolKind::Macro {
                                        params: params.clone(),
                                        return_type: BaseType::Void,
                                        body: body.clone(),
                                    },
                                    visibility: visibility.clone(),
                                    dependencies: vec![],
                                    is_used: false,
                                    is_param: false,
                                    is_uninitialized: false,
                                    is_compilable: false,
                                };
                                let _ = self.current_env.borrow_mut().define(alias, alias_info);
                            }
                        }
                    }
                }
            }
            Decl::MacroDecl {
                name,
                params,
                body,
                visibility,
                ..
            } => {
                let info = SymbolInfo {
                    name: name.clone(),
                    kind: SymbolKind::Macro {
                        params: params.clone(),
                        return_type: BaseType::Void,
                        body: body.clone(),
                    },
                    visibility: visibility.clone(),
                    dependencies: vec![],
                    is_used: false,
                    is_param: false,
                    is_uninitialized: false,
                    is_compilable: false,
                };
                let _ = self.current_env.borrow_mut().define(name.clone(), info);
            }
            _ => {}
        }
        Ok(())
    }
    // ----------------------------------------------------------
    // analyze_var_decl - handle-aware VarDecl analysis
    //
    // Core logic:
    //   1. Evaluate value expression type
    //   2. If assign_op != "=" and declared type is complex ->
    //      look up handle for the operator in the blueprint
    //   3. Otherwise do normal type compatibility check
    // ----------------------------------------------------------
    pub(crate) fn analyze_var_decl(
        &mut self,
        visibility: &Visibility,
        editability: &Editability,
        type_node: &BaseType,
        name: &str,
        value: &Expr,
        assign_op: &str,
    ) -> Result<(), String> {
        let prev_context = self.current_context.clone();
        self.current_context = Some(name.to_string());

        if matches!(type_node, BaseType::Flag) {
            return Err(
                format!("Semantic Error: 'flag' is a reserved state inspection type and cannot be declared as a user variable '{}'. Use 'bool' instead.", name)
            );
        }

        if assign_op == ":=" {
            if self.current_env.borrow().symbols.contains_key(name) {
                return Err(
                    format!(
                        "Semantic Error: Variable '{}' is already defined in this scope. Cannot redeclare using ':='.",
                        name
                    )
                );
            }
        }

        let expr_type = self.visit_expression(value)?;
        if assign_op == ":=" && expr_type == "undefined" {
            return Err(
                format!(
                    "Semantic Error: Cannot infer type for variable '{}' from 'undefined'. Specify an explicit type (e.g. 'Type {} = undefined;').",
                    name, name
                )
            );
        }
        let declared_type = type_node.as_str();
        for dep in extract_all_type_names(&declared_type) {
            self.record_dependency(dep);
        }

        // Propagate deps recorded under the var name into the containing function context.
        // This ensures tree-shaking can trace: fn_context -> deps_used_in_var_init.
        if let Some(ref prev_ctx) = prev_context.clone() {
            let var_deps: Vec<String> = self
                .dependency_graph
                .get(name)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .collect();
            for dep in var_deps {
                self.dependency_graph
                    .entry(prev_ctx.clone())
                    .or_default()
                    .insert(dep);
            }
        }

        if expr_type != "unknown" {
            let base_decl: &str = declared_type.split('<').next().unwrap_or(&declared_type);

            match base_decl {
                // smart pointer - accept anything, infer inner if needed
                // smart pointers - accept anything, infer inner if needed
                "name" | "modify" | "copy" => {
                    let is_modify = base_decl == "modify";
                    let is_copy = base_decl == "copy";
                    let is_arr = expr_type.starts_with("array<");

                    let inner_type_str = if declared_type.contains("<name<") {
                        let extracted = if is_modify {
                            declared_type
                                .trim_start_matches("modify<name<")
                                .trim_end_matches(">>")
                        } else {
                            declared_type
                                .trim_start_matches("copy<name<")
                                .trim_end_matches(">>")
                        };
                        if extracted == "unknown" && expr_type != "unknown" {
                            if is_arr {
                                expr_type.trim_start_matches("array<").trim_end_matches('>')
                            } else {
                                &expr_type
                            }
                        } else {
                            extracted
                        }
                    } else if declared_type.starts_with("name<") {
                        let extracted = declared_type
                            .trim_start_matches("name<")
                            .trim_end_matches('>');
                        if extracted == "unknown" && expr_type != "unknown" {
                            if is_arr {
                                expr_type.trim_start_matches("array<").trim_end_matches('>')
                            } else {
                                &expr_type
                            }
                        } else {
                            extracted
                        }
                    } else if is_arr {
                        expr_type.trim_start_matches("array<").trim_end_matches('>')
                    } else {
                        &expr_type
                    };

                    let base_inner = BaseType::Name(Box::new(BaseType::from_str(inner_type_str)));
                    let final_type_node = if is_modify {
                        BaseType::Modify(Box::new(base_inner))
                    } else if is_copy {
                        BaseType::Copy(Box::new(base_inner))
                    } else {
                        base_inner
                    };

                    let deps = self
                        .dependency_graph
                        .get(name)
                        .cloned()
                        .unwrap_or_default()
                        .into_iter()
                        .collect();
                    if !is_modify && !is_copy && !self.is_valid_pointer_rhs(value, &expr_type) {
                        return Err(
                            format!("Semantic Error: Invalid assignment to smart pointer '{}'. Must be a reference (&), 'new' allocation, or another smart pointer.", name)
                        );
                    }
                    let info = SymbolInfo {
                        name: name.to_string(),
                        kind: SymbolKind::Variable {
                            type_node: final_type_node,
                            editability: editability.clone(),
                            is_array: is_arr,
                        },
                        visibility: visibility.clone(),
                        dependencies: deps,
                        is_used: false,
                        is_param: false,
                        is_uninitialized: false,
                        is_compilable: false,
                    };
                    self.current_env
                        .borrow_mut()
                        .define(name.to_string(), info)?;
                    self.heap_allocated_vars.insert(name.to_string());
                    self.current_context = prev_context;
                    return Ok(());
                }
                // pointer type
                "pointer" => {
                    let is_arr = expr_type.starts_with("array<");
                    let inner_type_str = if declared_type.starts_with("pointer<") {
                        declared_type
                            .trim_start_matches("pointer<")
                            .trim_end_matches('>')
                    } else {
                        "unknown"
                    };
                    let expr_inner = if is_arr {
                        expr_type.trim_start_matches("array<").trim_end_matches('>')
                    } else if expr_type.starts_with("pointer<") {
                        expr_type
                            .trim_start_matches("pointer<")
                            .trim_end_matches('>')
                    } else {
                        &expr_type
                    };

                    if expr_type != "default"
                        && inner_type_str != "unknown"
                        && !self.types_are_compatible(inner_type_str, expr_inner)
                    {
                        return Err(
                            format!(
                                "Semantic Error: Type mismatch for pointer '{}'. Declared '{}', got '{}'",
                                name,
                                declared_type,
                                expr_type
                            )
                        );
                    }

                    let deps = self
                        .dependency_graph
                        .get(name)
                        .cloned()
                        .unwrap_or_default()
                        .into_iter()
                        .collect();
                    if expr_type != "default" && !self.is_valid_pointer_rhs(value, &expr_type) {
                        return Err(
                            format!("Semantic Error: Invalid assignment to a pointer '{}'. Must be a reference (&), 'new' allocation, or another  pointer.", name)
                        );
                    }
                    let info = SymbolInfo {
                        name: name.to_string(),
                        kind: SymbolKind::Variable {
                            type_node: BaseType::Pointer(Box::new(BaseType::from_str(
                                inner_type_str,
                            ))),
                            editability: editability.clone(),
                            is_array: is_arr,
                        },
                        visibility: visibility.clone(),
                        dependencies: deps,
                        is_used: false,
                        is_param: false,
                        is_uninitialized: false,
                        is_compilable: false,
                    };
                    self.current_env
                        .borrow_mut()
                        .define(name.to_string(), info)?;
                    self.heap_allocated_vars.insert(name.to_string());
                    self.current_context = prev_context;
                    return Ok(());
                }

                // complex types with possible handle overloading
                "custom" | "class" | "struct" | "enum" => {
                    let bp_name = extract_blueprint_name_from_type(&declared_type)
                        .unwrap_or_else(|| declared_type.clone());

                    // Check if bp_name is a type alias defined via define
                    let is_alias_compatible =
                        if let Some(info) = self.current_env.borrow().lookup(&bp_name) {
                            if let SymbolKind::Variable { type_node, .. } = &info.kind {
                                if let BaseType::Type(inner) = type_node {
                                    self.types_are_compatible(&inner.as_str(), &expr_type)
                                } else {
                                    false
                                }
                            } else {
                                false
                            }
                        } else {
                            false
                        };

                    // Direct compatibility: X = X
                    let directly_compatible = is_alias_compatible
                        || self.types_are_compatible(&declared_type, &expr_type)
                        || expr_type == bp_name;

                    // If types are directly compatible, always accept regardless of op
                    // e.g. MathScope a -> new MathScope() is fine even with ->
                    if directly_compatible {
                        // accepted
                    } else if assign_op == "=" {
                        return Err(
                            format!(
                                "Semantic Error [1796]: Type mismatch for '{}'. Declared '{}', got '{}' (value: {:?})",
                                name,
                                declared_type,
                                expr_type,
                                value
                            )
                        );
                    } else {
                        // operator overloading via handle
                        let effective_op = if assign_op == "->" {
                            "arrow_assign"
                        } else {
                            assign_op
                        };
                        match resolve_handle_for_op(&self.current_env, &bp_name, effective_op) {
                            HandleLookupResult::Found(bp) => {
                                let handle = op_to_handle(effective_op);
                                if !bp.handle_accepts_type(handle, &expr_type) {
                                    return Err(
                                        format!(
                                            "Semantic Error: Handle '{}' in '{}' does not accept type '{}'. Expected a compatible type for the '{}' operator.",
                                            handle.as_str(),
                                            bp_name,
                                            expr_type,
                                            assign_op
                                        )
                                    );
                                }
                                // OK - handle overloading accepts this
                            }
                            HandleLookupResult::BlueprintNotFound => {
                                // Not in env yet (generic or not-yet-defined) - allow
                            }
                            HandleLookupResult::HandleMissing { handle } => {
                                return Err(
                                    format!(
                                        "Semantic Error: Type '{}' does not support the '{}' operator. Handle '{}' is not defined in its handle block.",
                                        bp_name,
                                        assign_op,
                                        handle.as_str()
                                    )
                                );
                            }
                            HandleLookupResult::UnknownOp => {
                                // Unknown operator - allow
                            }
                        }
                    }
                }

                // bare type name (e.g. "MathScope" instead of "custom<MathScope>")
                // could be a user-defined type - look up in blueprints
                name_key
                    if self
                        .current_env
                        .borrow()
                        .lookup_blueprint(name_key)
                        .is_some() =>
                {
                    // It is a known blueprint type used without "custom<>" prefix
                    if assign_op != "=" {
                        let effective_op = if assign_op == "->" {
                            "arrow_assign"
                        } else {
                            assign_op
                        };
                        match resolve_handle_for_op(&self.current_env, name_key, effective_op) {
                            HandleLookupResult::Found(bp) => {
                                let handle = op_to_handle(effective_op);
                                if !bp.handle_accepts_type(handle, &expr_type) {
                                    return Err(
                                        format!(
                                            "Semantic Error: Handle '{}' in '{}' does not accept type '{}'.",
                                            handle.as_str(),
                                            name_key,
                                            expr_type
                                        )
                                    );
                                }
                            }
                            HandleLookupResult::BlueprintNotFound => {}
                            HandleLookupResult::HandleMissing { handle } => {
                                return Err(
                                    format!(
                                        "Semantic Error: Type '{}' does not support '{}' (missing handle '{}').",
                                        name_key,
                                        assign_op,
                                        handle.as_str()
                                    )
                                );
                            }
                            HandleLookupResult::UnknownOp => {}
                        }
                    }
                    // For "=", just accept - they're using the type by its raw name
                }

                // simple primitive types
                _ => {
                    if let Expr::Lambda { params, .. } = value {
                        let is_lambda_untyped = matches!(type_node, BaseType::Unknown)
                            || matches!(type_node, BaseType::Lambda { params, .. } if params.is_empty());
                        if is_lambda_untyped
                            && params.iter().any(|p| p.type_node == BaseType::Unknown)
                        {
                            return Err(format!(
                                "Semantic Error: Lambda parameters must have explicit type annotations when target type is not declared (e.g. '|x: int, y: int|')"
                            ));
                        }
                    }

                    // Allow -> operator for any type (it's used as "default init" syntax)
                    // Allow := operator (inferred type)
                    if assign_op != "->"
                        && assign_op != ":="
                        && declared_type != "unknown"
                        && expr_type != "default"
                        && !self.types_are_compatible(&declared_type, &expr_type)
                    {
                        return Err(
                            format!(
                                "Semantic Error [1892]: Type mismatch for '{}'. Declared '{}', got '{}' (value: {:?})",
                                name,
                                declared_type,
                                expr_type,
                                value
                            )
                        );
                    }
                }
            }
        }

        // Register symbol
        let deps = self
            .dependency_graph
            .get(name)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .collect();

        let final_type_node = if matches!(type_node, BaseType::Unknown) {
            if expr_type == "undefined" || expr_type == "unknown" {
                BaseType::Unknown
            } else if expr_type == "int" {
                BaseType::Int(Size::S32)
            } else if expr_type == "float" {
                BaseType::Float(Size::S64)
            } else if expr_type == "bool" {
                BaseType::Bool
            } else if expr_type == "char" {
                BaseType::Char
            } else if expr_type == "str" {
                BaseType::Str
            } else if expr_type == "array<char>" {
                BaseType::Array {
                    base_type: Box::new(BaseType::Char),
                    size: Box::new(None),
                }
            } else {
                BaseType::from_str(&expr_type)
            }
        } else {
            type_node.clone()
        };

        for dep in extract_all_type_names(&final_type_node.as_str()) {
            self.record_dependency(dep);
        }

        let is_custom_or_struct = match &final_type_node {
            BaseType::Class { .. }
            | BaseType::Struct { .. }
            | BaseType::Blueprint { .. }
            | BaseType::Enum { .. }
            | BaseType::Array { .. } => true,
            BaseType::Name(inner) => {
                matches!(
                    &**inner,
                    BaseType::Class { .. }
                        | BaseType::Struct { .. }
                        | BaseType::Blueprint { .. }
                        | BaseType::Enum { .. }
                ) || self
                    .current_env
                    .borrow()
                    .lookup_blueprint(&inner.as_str())
                    .is_some()
            }
            _ => self
                .current_env
                .borrow()
                .lookup_blueprint(&final_type_node.as_str())
                .is_some(),
        };

        let is_uninit = match value {
            Expr::LiteralUndefined => true,
            Expr::Default(None) => !is_custom_or_struct,
            _ => false,
        };
        let info = SymbolInfo {
            name: name.to_string(),
            kind: SymbolKind::Variable {
                type_node: final_type_node,
                editability: editability.clone(),
                is_array: false,
            },
            visibility: visibility.clone(),
            dependencies: deps,
            is_used: false,
            is_param: false,
            is_compilable: false,
            is_uninitialized: is_uninit,
        };
        self.current_env
            .borrow_mut()
            .define(name.to_string(), info)?;
        if matches!(value, Expr::Instantiate { .. }) || self.is_class_type(&declared_type) {
            self.heap_allocated_vars.insert(name.to_string());
        }
        self.current_context = prev_context;
        Ok(())
    }

    // ----------------------------------------------------------
    // analyze_class_or_struct_decl - shared for Class and Struct
    // ----------------------------------------------------------
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn analyze_class_or_struct_decl(
        &mut self,
        visibility: Visibility,
        name: &str,
        is_class: bool,
        extends: Option<&str>,
        handles: &[HandleMethods],
        public_block: &[Decl],
        private_block: &[Decl],
        static_block: &[Decl],
        handle_block: &[Decl],
        constructor: &Option<Vec<ConstructorDecl>>,
    ) -> Result<(), String> {
        let prev_context = self.current_context.clone();
        self.current_context = Some(name.to_string());
        if let Some(parent) = extends {
            self.class_hierarchy.insert(name.to_string(), parent.to_string());
            self.record_dependency(parent.to_string());
        }
        if !is_class {
            for d in private_block.iter().chain(public_block).chain(static_block).chain(handle_block) {
                if let Decl::FnDecl { is_virtual: true, .. } = d {
                    return Err(format!(
                        "Semantic Error: 'virtual' modifier is only allowed in 'class' methods, not in struct '{}'",
                        name
                    ));
                }
            }
        }
        for d in private_block.iter().chain(public_block).chain(static_block) {
            if let Decl::VarDecl { type_node, .. } = d {
                for dep in extract_all_type_names(&type_node.as_str()) {
                    self.record_dependency(dep);
                }
            } else if let Decl::FnDecl {
                params,
                return_type,
                ..
            } = d
            {
                for p in params {
                    for dep in extract_all_type_names(&p.type_node.as_str()) {
                        self.record_dependency(dep);
                    }
                }
                for dep in extract_all_type_names(&return_type.as_str()) {
                    self.record_dependency(dep);
                }
            }
        }
        for d in handle_block {
            if let Decl::FnDecl {
                params,
                return_type,
                ..
            } = d
            {
                for p in params {
                    for dep in extract_all_type_names(&p.type_node.as_str()) {
                        self.record_dependency(dep);
                    }
                }
                for dep in extract_all_type_names(&return_type.as_str()) {
                    self.record_dependency(dep);
                }
            }
        }
        if let Some(constructors) = constructor {
            for ctor in constructors {
                for p in &ctor.params {
                    for dep in extract_all_type_names(&p.type_node.as_str()) {
                        self.record_dependency(dep);
                    }
                }
            }
        }
        let mut bp = BlueprintData::new(name);
        bp.is_class = is_class;
        if let Some(parent) = extends {
            let parent_bp = self
                .current_env
                .borrow()
                .lookup_blueprint(parent)
                .or_else(|| {
                    self.global_metadata
                        .get(parent)
                        .map(build_blueprint_from_metadata)
                });
            if let Some(parent_bp) = parent_bp {
                for (f_k, f_v) in &parent_bp.fields {
                    bp.fields.entry(f_k.clone()).or_insert_with(|| f_v.clone());
                }
                for (m_k, m_v) in &parent_bp.methods {
                    bp.methods.entry(m_k.clone()).or_insert_with(|| m_v.clone());
                }
                for h in &parent_bp.handles {
                    bp.handles.insert(*h);
                }
            }
        }
        for h in handles {
            bp.handles.insert(*h);
        }
        for d in handle_block {
            if let Decl::FnDecl {
                name: fn_name,
                params: fn_params,
                return_type,
                is_virtual,
                is_abstract,
                generics: fn_generics,
                ..
            } = d
            {
                if fn_name == "display" {
                    if !fn_params.is_empty() {
                        return Err(format!("Semantic Error: Handle method 'display' cannot take any parameters on target '{}'", name));
                    }
                    let is_valid = match return_type {
                        BaseType::Str | BaseType::Char => true,
                        BaseType::Array { base_type, .. } => {
                            matches!(base_type.as_ref(), BaseType::Char)
                        }
                        _ => false,
                    };
                    if !is_valid {
                        return Err(
                            format!(
                                "Semantic Error: Handle method 'display' must return 'str', 'char', or 'char[]', found '{}' on target '{}'",
                                return_type.as_str(),
                                name
                            )
                        );
                    }
                }

                let hk = HandleMethods::from_str(fn_name.as_str());
                if hk != HandleMethods::NotFound {
                    bp.handles.insert(hk);
                }
                bp.methods.insert(
                    fn_name.clone(),
                    FnSignature {
                        name: fn_name.clone(),
                        generics: fn_generics.clone(),
                        params: fn_params.clone(),
                        return_type: return_type.clone(),
                        is_virtual: *is_virtual,
                        is_abstract: *is_abstract,
                    },
                );
            }
        }
        for d in private_block.iter().chain(public_block).chain(static_block) {
            if let Decl::VarDecl {
                name: f_name,
                type_node,
                ..
            } = d
            {
                bp.fields.insert(f_name.clone(), type_node.clone());
            }
        }
        for d in private_block.iter().chain(public_block).chain(static_block) {
            if let Decl::FnDecl {
                name: fn_name,
                params,
                return_type,
                is_virtual,
                is_abstract,
                generics,
                ..
            } = d
            {
                bp.methods.insert(
                    fn_name.clone(),
                    FnSignature {
                        name: fn_name.clone(),
                        generics: generics.clone(),
                        params: params.clone(),
                        return_type: return_type.clone(),
                        is_virtual: *is_virtual,
                        is_abstract: *is_abstract,
                    },
                );
            }
        }
        self.current_env
            .borrow_mut()
            .define_blueprint(name.to_string(), bp);

        let info = self.make_blueprint_symbol(name, visibility);
        self.current_env
            .borrow_mut()
            .define(name.to_string(), info)?;

        self.enter_scope();
        let prev = self.in_custom_scope;
        self.in_custom_scope = true;
        let prev_type_name = self.current_type_name.clone();
        self.current_type_name = Some(name.to_string());

        for d in private_block
            .iter()
            .chain(public_block)
            .chain(static_block)
            .chain(handle_block)
        {
            self.visit_declaration(d)?;
        }

        if let Some(constructors) = constructor {
            let prev_ctor_ctx = self.current_context.clone();
            self.current_context = Some(format!("{}::init", name));
            for ctor in constructors {
                self.enter_scope();
                for param in &ctor.params {
                    let pi = SymbolInfo {
                        name: param.name.clone(),
                        kind: SymbolKind::Variable {
                            type_node: param.type_node.clone(),
                            editability: Editability::Editable,
                            is_array: false,
                        },
                        visibility: Visibility::Private,
                        dependencies: vec![],
                        is_used: false,
                        is_param: true,
                        is_uninitialized: false,
                        is_compilable: false,
                    };
                    self.current_env
                        .borrow_mut()
                        .define(param.name.clone(), pi)?;
                }
                let prev_stmt = self.in_statement_scope;
                self.in_statement_scope = true;
                for s in &ctor.body {
                    self.visit_statement(s)?;
                }
                self.in_statement_scope = prev_stmt;
                self.leave_scope();
            }
            self.current_context = prev_ctor_ctx;
        }

        self.leave_scope();
        self.in_custom_scope = prev;
        self.current_type_name = prev_type_name;
        self.current_context = prev_context;
        Ok(())
    }

    pub(crate) fn make_blueprint_symbol(&self, name: &str, visibility: Visibility) -> SymbolInfo {
        SymbolInfo {
            name: name.to_string(),
            kind: SymbolKind::Blueprint,
            visibility,
            dependencies: vec![],
            is_used: false,
            is_param: false,
            is_uninitialized: false,
            is_compilable: false,
        }
    }

}
