use super::*;
use crate::middle_end::interpreter::eval::strip_type_wrapper;

impl SemanticAnalyzer {
    pub(crate) fn visit_expression(&mut self, expr: &Expr) -> Result<String, String> {
        match expr {
            Expr::LiteralInt(_) => Ok("int".to_string()),
            Expr::LiteralUInt(_) => Ok("uint".to_string()),
            Expr::LiteralFloat(_) => Ok("float".to_string()),
            Expr::LiteralString(_) => Ok("array<char>".to_string()),
            Expr::LiteralChar(_) => Ok("char".to_string()),
            Expr::LiteralUChar(_) => Ok("uchar".to_string()),
            Expr::LiteralBool(_) => Ok("bool".to_string()),
            Expr::LiteralVoid => Ok("void".to_string()),
            Expr::LiteralUndefined => Ok("undefined".to_string()),

            Expr::ArrayLiteral(elements) => {
                if elements.is_empty() {
                    return Ok("array<unknown>".to_string());
                }
                let mut element_type: Option<String> = None;
                for el in elements {
                    let inferred = if let Expr::Spread(inner) = el {
                        let inner_t = self.visit_expression(inner)?;
                        if inner_t.starts_with("array<") {
                            inner_t
                                .trim_start_matches("array<")
                                .trim_end_matches('>')
                                .to_string()
                        } else {
                            inner_t
                        }
                    } else {
                        self.visit_expression(el)?
                    };
                    match &element_type {
                        None => {
                            element_type = Some(inferred);
                        }
                        Some(prev) if prev == &inferred => {}
                        Some(_) => {
                            element_type = Some("unknown".to_string());
                            break;
                        }
                    }
                }
                Ok(format!(
                    "array<{}>",
                    element_type.unwrap_or_else(|| "unknown".to_string())
                ))
            }

            Expr::Spread(inner) => {
                let inner_t = self.visit_expression(inner)?;
                if inner_t.starts_with("array<") {
                    Ok(inner_t
                        .trim_start_matches("array<")
                        .trim_end_matches('>')
                        .to_string())
                } else {
                    Ok(inner_t)
                }
            }

            Expr::ObjectLiteral(stmts) => {
                let prev_in_struct = self.in_struct;
                self.in_struct = true;
                self.enter_scope();
                for s in stmts {
                    match s {
                        Stmt::ReassignStmt { value, .. } => {
                            self.visit_expression(value)?;
                        }
                        Stmt::Declaration(Decl::VarDecl { value, .. }) => {
                            self.visit_expression(value)?;
                        }
                        _ => {
                            self.visit_statement(s)?;
                        }
                    }
                }
                self.leave_scope();
                self.in_struct = prev_in_struct;
                Ok("object".to_string())
            }

            Expr::Default(type_arg) => {
                if let Some(t) = type_arg {
                    Ok(t.as_str())
                } else {
                    Ok("default".to_string())
                }
            }

            Expr::Identifier(name) => {
                if matches!(
                    name.trim(),
                    "broken"
                        | "is_done"
                        | "yielded"
                        | "returned"
                        | "leaved"
                        | "continued"
                        | "has_break"
                        | "has_yield"
                        | "has_leave"
                        | "has_return"
                        | "has_call"
                        | "has_error"
                        | "compilable"
                        | "is_compilable"
                        | "printable"
                        | "is_printable"
                        | "throwable"
                        | "is_throwable"
                        | "shareable"
                        | "is_shareable"
                ) {
                    return Ok("bool".to_string());
                }
                if matches!(
                    name.as_str(),
                    "bool"
                        | "int8"
                        | "int16"
                        | "int32"
                        | "int64"
                        | "int"
                        | "uint8"
                        | "uint16"
                        | "uint32"
                        | "uint64"
                        | "uint"
                        | "float32"
                        | "float64"
                        | "float"
                        | "char"
                        | "byte"
                        | "usize"
                        | "isize"
                        | "type"
                        | "void"
                ) {
                    return Ok(format!("type<{}>", name));
                }
                if self.deleted_vars.contains(name) {
                    return Err(format!("Semantic Error: Cannot use variable '{}' because it has already been deleted.", name));
                }
                self.record_dependency(name.clone());
                self.current_env.borrow_mut().mark_used(name);
                if let Some(info) = self.current_env.borrow().lookup(name) {
                    if info.is_uninitialized {
                        return Err(format!("Semantic Error: Variable '{}' is used before being initialized (undefined value).", name));
                    }
                    return Ok(info.type_str());
                }
                if self.current_env.borrow().lookup_blueprint(name).is_some()
                    || self.global_metadata.contains_key(name)
                {
                    return Ok(format!("type<{}>", name));
                }
                if name.contains('<') && name.ends_with('>') {
                    let base_name = name.split('<').next().unwrap_or(name);
                    if self
                        .current_env
                        .borrow()
                        .lookup_blueprint(base_name)
                        .is_some()
                        || self.global_metadata.contains_key(base_name)
                        || matches!(base_name, "array")
                    {
                        return Ok(format!("type<{}>", name));
                    }
                }
                for (enum_name, meta) in &self.global_metadata {
                    if let Some(ref variants) = meta.variants {
                        if variants.iter().any(|v| v.name == *name) {
                            return Ok(enum_name.clone());
                        }
                    }
                }
                if let Some(enum_name) = self.current_env.borrow().lookup_enum_for_variant(name) {
                    return Ok(enum_name);
                }
                if let Some(ref type_name) = self.current_type_name {
                    if let Some(bp) = self.current_env.borrow().lookup_blueprint(type_name) {
                        if let Some(field_type) = bp.fields.get(name) {
                            return Ok(field_type.as_str());
                        }
                    }
                    if let Some(meta) = self.global_metadata.get(type_name) {
                        if let Some(field_type) = meta.fields.get(name) {
                            return Ok(field_type.as_str());
                        }
                    }
                }
                if self.in_struct || self.in_class || self.in_custom_scope {
                    return Ok("unknown".to_string());
                }
                Err(format!(
                    "Semantic Error: Identifier '{}' is not defined in this scope.",
                    name
                ))
            }

            Expr::NamespaceAccess {
                namespace,
                property,
            } => {
                if namespace.starts_with('@') {
                    if let Some(ref m_name) = self.current_machine {
                        let clean_lbl = namespace.replace("@", "");
                        if let Some(labels_map) = self.machine_labels.get(m_name) {
                            if let Some(vars_map) = labels_map
                                .get(&clean_lbl)
                                .or_else(|| labels_map.get(namespace))
                            {
                                if let Expr::Identifier(prop_name) = &**property {
                                    if let Some(t_node) = vars_map.get(prop_name) {
                                        return Ok(t_node.as_str());
                                    }
                                }
                            }
                        }
                    }
                    if let Expr::Identifier(prop_name) = &**property {
                        if let Some(info) = self.current_env.borrow().lookup(prop_name) {
                            return Ok(info.type_str());
                        }
                    }
                    return Ok("unknown".to_string());
                }

                // Check global metadata first
                if let Some(metadata) = self.global_metadata.get(namespace) {
                    if let Expr::Identifier(prop_name) = &**property {
                        if let Some(t_node) = metadata.fields.get(prop_name) {
                            return Ok(t_node.as_str());
                        }
                    }
                }
                // Then check blueprints
                if let Some(bp) = self.current_env.borrow().lookup_blueprint(namespace) {
                    if let Expr::Identifier(prop_name) = &**property {
                        if let Some(field_type) = bp.fields.get(prop_name) {
                            return Ok(field_type.as_str());
                        }
                    }
                }
                Ok("unknown".to_string())
            }

            Expr::Call { callee, args, .. } | Expr::MacroCall { callee, args } => {
                let call_generics = if let Expr::Call { generics, .. } = expr {
                    generics.clone()
                } else {
                    Vec::new()
                };
                // Generic Macro lookup and type checking
                let macro_info = match &**callee {
                    Expr::Identifier(fn_name) => {
                        self.current_env.borrow().lookup(fn_name).and_then(|info| {
                            if let SymbolKind::Macro {
                                params,
                                return_type,
                                body,
                            } = &info.kind
                            {
                                Some((
                                    fn_name.clone(),
                                    params.clone(),
                                    return_type.clone(),
                                    body.clone(),
                                ))
                            } else {
                                None
                            }
                        })
                    }
                    Expr::NamespaceAccess {
                        namespace,
                        property,
                    } => {
                        let prop_str = if let Expr::Identifier(p) = &**property {
                            p.clone()
                        } else {
                            "".to_string()
                        };
                        let candidates = vec![
                            format!("{}::{}", namespace, prop_str),
                            format!("{}::${}", namespace, prop_str),
                            prop_str.clone(),
                            format!("${}", prop_str),
                        ];
                        let mut found = None;
                        for cand in candidates {
                            if let Some(info) = self.current_env.borrow().lookup(&cand) {
                                if let SymbolKind::Macro {
                                    params,
                                    return_type,
                                    body,
                                } = &info.kind
                                {
                                    found = Some((
                                        cand,
                                        params.clone(),
                                        return_type.clone(),
                                        body.clone(),
                                    ));
                                    break;
                                }
                            }
                        }
                        found
                    }
                    _ => None,
                };

                if let Some((macro_name, params, return_type, _macro_body)) = macro_info {
                    if args.len() != params.len() {
                        return Err(format!(
                            "Semantic Error: Macro '{}' expects {} arguments, but got {}.",
                            macro_name,
                            params.len(),
                            args.len()
                        ));
                    }
                    for (arg, param) in args.iter().zip(params.iter()) {
                        let arg_type = self.visit_expression(arg)?;
                        let param_type_str = param.type_node.as_str();
                        if arg_type != "unknown"
                            && param_type_str != "unknown"
                            && !self.types_are_compatible(&param_type_str, &arg_type)
                        {
                            return Err(format!(
                                "Semantic Error: Macro '{}' parameter '{}' type mismatch. Expected '{}', got '{}'",
                                macro_name,
                                param.name,
                                param_type_str,
                                arg_type
                            ));
                        }
                    }

                    return Ok(return_type.as_str());
                }
                if let Expr::PropertyAccess { object, property } = &**callee {
                    let obj_type = self.visit_expression(object)?;
                    if property == "default"
                        && (obj_type.starts_with("type<") || obj_type == "type")
                    {
                        for arg in args {
                            self.visit_expression(arg)?;
                        }
                        if obj_type.starts_with("type<") {
                            let inner = strip_type_wrapper(&obj_type).to_string();
                            return Ok(inner);
                        } else {
                            return Ok("unknown".to_string());
                        }
                    }
                    let unwrap_address = if obj_type.starts_with("address<") {
                        strip_wrapper(&obj_type, "address<").to_string()
                    } else if obj_type.ends_with('&') {
                        obj_type[..obj_type.len() - 1].trim().to_string()
                    } else {
                        obj_type.clone()
                    };
                    let normalized_type = if unwrap_address == "char[]" {
                        "array<char>".to_string()
                    } else if unwrap_address.ends_with("[]") {
                        format!("array<{}>", &unwrap_address[..unwrap_address.len() - 2])
                    } else {
                        unwrap_address
                    };

                    let (specialized_type_name, primary_type_name) =
                        if let Some(bp) = extract_blueprint_name_from_type(&normalized_type) {
                            (None, bp)
                        } else if let Some(idx) = normalized_type.find('<') {
                            let base = normalized_type[..idx].trim().to_string();
                            (Some(normalized_type.clone()), base)
                        } else {
                            (None, normalized_type.clone())
                        };

                    let type_name = primary_type_name.clone();

                    let has_handle = if let Some(spec_name) = &specialized_type_name {
                        if let Some(bp) = self.current_env.borrow().lookup_blueprint(spec_name) {
                            bp.handles.iter().any(|h| h.as_str() == property)
                        } else if let Some(meta) = self.global_metadata.get(spec_name) {
                            meta.handles.iter().any(|h| h.as_str() == property)
                        } else {
                            false
                        }
                    } else {
                        false
                    } || if let Some(bp) = self
                        .current_env
                        .borrow()
                        .lookup_blueprint(&primary_type_name)
                    {
                        bp.handles.iter().any(|h| h.as_str() == property)
                    } else if let Some(meta) = self.global_metadata.get(&primary_type_name) {
                        meta.handles.iter().any(|h| h.as_str() == property)
                    } else {
                        false
                    };

                    if has_handle || property == "display" {
                        return Err(
                            format!(
                                "Semantic Error: Handle '{}' cannot be called directly as a method on target '{}'.",
                                property,
                                type_name
                            )
                        );
                    }

                    // Two-tier method lookup:
                    // 1. Check specialized type (e.g. array<char>)
                    // 2. Fall back to primary type (e.g. array)
                    let (chosen_bp_name, bp_method_info) = {
                        let env = self.current_env.borrow();
                        let mut res = None;
                        let mut name = primary_type_name.clone();
                        if let Some(spec_name) = &specialized_type_name {
                            if let Some(bp) = env.lookup_blueprint(spec_name) {
                                if let Some(sig) = bp
                                    .methods
                                    .get(property)
                                    .or_else(|| bp.handle_signatures.get(property))
                                {
                                    res = Some(sig.clone());
                                    name = spec_name.clone();
                                }
                            }
                        }
                        if res.is_none() {
                            if let Some(bp) = env.lookup_blueprint(&primary_type_name) {
                                if let Some(sig) = bp
                                    .methods
                                    .get(property)
                                    .or_else(|| bp.handle_signatures.get(property))
                                {
                                    res = Some(sig.clone());
                                    name = primary_type_name.clone();
                                }
                            }
                        }
                        (name, res)
                    };

                    if let Some(method_sig) = bp_method_info {
                        self.record_dependency(format!("{}::{}", chosen_bp_name, property));
                        self.record_dependency(property.clone());
                        self.record_dependency(chosen_bp_name.clone());
                        for dep in extract_all_type_names(&method_sig.return_type.as_str()) {
                            self.record_dependency(dep);
                        }
                        for p in &method_sig.params {
                            for dep in extract_all_type_names(&p.type_node.as_str()) {
                                self.record_dependency(dep);
                            }
                        }
                        for arg in args {
                            self.visit_expression(arg)?;
                        }
                        let ret_type =
                            if chosen_bp_name == "array" && obj_type.starts_with("array<") {
                                let elem_t =
                                    obj_type.trim_start_matches("array<").trim_end_matches('>');
                                let elem_base = BaseType::from_str(elem_t);
                                let mut gen_map = std::collections::HashMap::new();
                                for g_param in method_sig.return_type.extract_generic_params() {
                                    gen_map.insert(g_param, elem_base.clone());
                                }
                                for p in &method_sig.params {
                                    for g_param in p.type_node.extract_generic_params() {
                                        gen_map.insert(g_param, elem_base.clone());
                                    }
                                }
                                method_sig.return_type.substitute_generics(&gen_map)
                            } else {
                                method_sig.return_type.clone()
                            };
                        return Ok(ret_type.as_str());
                    }

                    let (chosen_meta_name, method_deps_and_ret) = {
                        let find_in_meta = |t_name: &str| {
                            if let Some(meta) = self.global_metadata.get(t_name) {
                                if let Some(fn_type) = meta
                                    .methods
                                    .get(property)
                                    .or_else(|| meta.handle_signatures.get(property))
                                {
                                    let mut deps = vec![
                                        format!("{}::{}", t_name, property),
                                        property.clone(),
                                        t_name.to_string(),
                                    ];
                                    for dep in extract_all_type_names(&fn_type.return_type.as_str())
                                    {
                                        deps.push(dep);
                                    }
                                    for p in &fn_type.params {
                                        for dep in extract_all_type_names(&p.type_node.as_str()) {
                                            deps.push(dep);
                                        }
                                    }
                                    Some((deps, fn_type.return_type.clone()))
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        };

                        let mut res = None;
                        let mut name = primary_type_name.clone();
                        if let Some(spec_name) = &specialized_type_name {
                            if let Some(found) = find_in_meta(spec_name) {
                                res = Some(found);
                                name = spec_name.clone();
                            }
                        }
                        if res.is_none() {
                            if let Some(found) = find_in_meta(&primary_type_name) {
                                res = Some(found);
                                name = primary_type_name.clone();
                            }
                        }
                        (name, res)
                    };

                    if let Some((deps, return_type)) = method_deps_and_ret {
                        for dep in deps {
                            self.record_dependency(dep);
                        }
                        for arg in args {
                            self.visit_expression(arg)?;
                        }
                        let ret_type =
                            if chosen_meta_name == "array" && obj_type.starts_with("array<") {
                                let elem_t =
                                    obj_type.trim_start_matches("array<").trim_end_matches('>');
                                let elem_base = BaseType::from_str(elem_t);
                                let mut gen_map = std::collections::HashMap::new();
                                for g_param in return_type.extract_generic_params() {
                                    gen_map.insert(g_param, elem_base.clone());
                                }
                                return_type.substitute_generics(&gen_map)
                            } else {
                                return_type
                            };
                        return Ok(ret_type.as_str());
                    }
                    if property == "as_str" && args.is_empty() {
                        return Ok("array<char>".to_string());
                    }

                    // Forward method call via `property_access` handle if defined on wrapper
                    if let Some(target_type) = self.resolve_property_access_target_type(&obj_type) {
                        let target_bp_opt = extract_blueprint_name_from_type(&target_type)
                            .or_else(|| Some(target_type.clone()));
                        if let Some(t_bp_name) = target_bp_opt {
                            let mut found_ret: Option<(String, Vec<String>)> = None;
                            if let Some(t_bp) =
                                self.current_env.borrow().lookup_blueprint(&t_bp_name)
                            {
                                if let Some(sig) = t_bp.methods.get(property) {
                                    let mut deps = vec![
                                        format!("{}::{}", t_bp_name, property),
                                        property.clone(),
                                        t_bp_name.clone(),
                                    ];
                                    for dep in extract_all_type_names(&sig.return_type.as_str()) {
                                        deps.push(dep);
                                    }
                                    for p in &sig.params {
                                        for dep in extract_all_type_names(&p.type_node.as_str()) {
                                            deps.push(dep);
                                        }
                                    }
                                    found_ret = Some((sig.return_type.as_str(), deps));
                                }
                            }
                            if found_ret.is_none() {
                                if let Some(t_meta) = self.global_metadata.get(&t_bp_name) {
                                    if let Some(fn_type) = t_meta.methods.get(property) {
                                        let mut deps = vec![
                                            format!("{}::{}", t_bp_name, property),
                                            property.clone(),
                                            t_bp_name.clone(),
                                        ];
                                        for dep in
                                            extract_all_type_names(&fn_type.return_type.as_str())
                                        {
                                            deps.push(dep);
                                        }
                                        for p in &fn_type.params {
                                            for dep in extract_all_type_names(&p.type_node.as_str())
                                            {
                                                deps.push(dep);
                                            }
                                        }
                                        found_ret = Some((fn_type.return_type.as_str(), deps));
                                    }
                                }
                            }
                            if let Some((ret_str, deps)) = found_ret {
                                for dep in deps {
                                    self.record_dependency(dep);
                                }
                                for arg in args {
                                    self.visit_expression(arg)?;
                                }
                                return Ok(ret_str);
                            }
                        }
                    }
                }
                let mut arg_types = Vec::new();
                for arg in args {
                    arg_types.push(self.visit_expression(arg)?);
                }

                let mut ret_type_from_ns = None;
                let name_opt: Option<String> = match &**callee {
                    Expr::Identifier(name) => Some(name.clone()),
                    Expr::NamespaceAccess {
                        namespace,
                        property,
                    } => {
                        if let Expr::Identifier(prop_name) = &**property {
                            let qualified = format!("{}::{}", namespace, prop_name);
                            if self.current_env.borrow().lookup(&qualified).is_some() {
                                Some(qualified)
                            } else if let Some(bp) =
                                self.current_env.borrow().lookup_blueprint(namespace)
                            {
                                if let Some(sig) = bp
                                    .methods
                                    .get(prop_name)
                                    .or_else(|| bp.handle_signatures.get(prop_name))
                                {
                                    ret_type_from_ns = Some(sig.return_type.as_str());
                                    Some(prop_name.clone())
                                } else {
                                    Some(qualified)
                                }
                            } else if let Some(meta) = self.global_metadata.get(namespace) {
                                if let Some(sig) = meta
                                    .methods
                                    .get(prop_name)
                                    .or_else(|| meta.handle_signatures.get(prop_name))
                                {
                                    ret_type_from_ns = Some(sig.return_type.as_str());
                                    Some(prop_name.clone())
                                } else {
                                    Some(qualified)
                                }
                            } else {
                                Some(qualified)
                            }
                        } else {
                            None
                        }
                    }
                    _ => None,
                };

                if let Some(ret) = ret_type_from_ns {
                    return Ok(ret);
                }

                if let Some(ref name) = name_opt {
                    self.record_dependency(name.clone());
                    self.current_env.borrow_mut().mark_used(name);
                    if let Some(info) = self.current_env.borrow().lookup(name) {
                        if let SymbolKind::Variable { type_node, .. } = &info.kind {
                            let base_str = type_node.as_str();
                            if base_str == "type" || base_str.starts_with("type<") {
                                return Ok(name.clone());
                            }
                        }
                    }
                    if name.starts_with("@compile::") {
                        let member = name
                            .strip_prefix("@compile::")
                            .unwrap_or(name)
                            .trim_start_matches('$');
                        if let Some(intrinsic) =
                            crate::middle_end::interpreter::intrinsics::CompilerIntrinsics::find_compile_member(member)
                        {
                            match intrinsic.name {
                                "typeof" => {
                                    let arg_t = if let Some(t) = arg_types.first() {
                                        t.clone()
                                    } else {
                                        "unknown".to_string()
                                    };
                                    return Ok(format!("type<{}>", arg_t));
                                }
                                "sizeof" => return Ok("uint32".to_string()),
                                "rand" => {
                                    let target_t = call_generics
                                        .first()
                                        .map(|t| t.as_str())
                                        .unwrap_or_else(|| {
                                            arg_types
                                                .first()
                                                .cloned()
                                                .unwrap_or_else(|| "int32".to_string())
                                        });
                                    return Ok(target_t);
                                }
                                "print" | "throw" => return Ok("void".to_string()),
                                "cast" => {
                                    let target_t = call_generics
                                        .first()
                                        .map(|t| t.as_str())
                                        .unwrap_or_else(|| "void".to_string());
                                    return Ok(target_t);
                                }
                                "access" => return Ok("unknown".to_string()),
                                _ => {}
                            }
                        }

                        if let Some(info) = self.current_env.borrow().lookup(name) {
                            if let SymbolKind::Function {
                                params,
                                return_type,
                                generics: fn_generics,
                                body: _,
                            } = &info.kind
                            {
                                let generic_map = resolve_call_generics(
                                    fn_generics,
                                    params,
                                    &call_generics,
                                    &arg_types,
                                );
                                let resolved_ret = return_type.substitute_generics(&generic_map);

                                return Ok(resolved_ret.as_str());
                            }
                        }
                    }
                    if let Some(intrinsic) =
                        crate::middle_end::interpreter::intrinsics::CompilerIntrinsics::find_compile_member(name)
                    {
                        match intrinsic.name {
                            "typeof" => {
                                let arg_t = if let Some(t) = arg_types.first() {
                                    t.clone()
                                } else {
                                    "unknown".to_string()
                                };
                                return Ok(format!("type<{}>", arg_t));
                            }
                            "sizeof" => return Ok("uint32".to_string()),
                            "rand" => {
                                let target_t = call_generics
                                    .first()
                                    .map(|t| t.as_str())
                                    .unwrap_or_else(|| {
                                        arg_types
                                            .first()
                                            .cloned()
                                            .unwrap_or_else(|| "int32".to_string())
                                    });
                                return Ok(target_t);
                            }
                            _ => {}
                        }
                    }

                    let bp_call = if let Some(bp) = self.current_env.borrow().lookup_blueprint(name)
                    {
                        if bp.is_class {
                            return Err(
                                format!(
                                    "Semantic Error: Class '{}' must be instantiated on the heap using 'new' (e.g. 'new {}(...)').",
                                    name,
                                    name
                                )
                            );
                        }
                        bp.handle_signatures
                            .get("call")
                            .or_else(|| bp.methods.get("call"))
                            .map(|s| s.return_type.as_str())
                    } else {
                        None
                    };
                    if let Some(ret) = bp_call {
                        return Ok(ret);
                    }

                    if let Some(meta) = self.global_metadata.get(name) {
                        if let Some(call_sig) = meta
                            .handle_signatures
                            .get("call")
                            .or_else(|| meta.methods.get("call"))
                        {
                            return Ok(call_sig.return_type.as_str());
                        }
                    }
                    if let Some(sigs) = self.fn_overloads.get(name).cloned() {
                        let mut best_match = None;
                        // Pass 1: Prioritize exact concrete overloads
                        for sig in &sigs {
                            let is_variadic =
                                sig.params.last().map(|p| p.is_variadic).unwrap_or(false);
                            let min_args = sig
                                .params
                                .iter()
                                .take_while(|p| p.default_value.is_none() && !p.is_variadic)
                                .count();
                            let max_args = if is_variadic {
                                usize::MAX
                            } else {
                                sig.params.len()
                            };
                            if arg_types.len() >= min_args && arg_types.len() <= max_args {
                                let get_param = |idx: usize| -> &Param {
                                    if idx < sig.params.len() {
                                        &sig.params[idx]
                                    } else {
                                        sig.params.last().unwrap()
                                    }
                                };
                                let has_generic = (0..arg_types.len()).any(|i| {
                                    matches!(get_param(i).type_node, BaseType::GenericParam(_))
                                });
                                if !has_generic {
                                    let matches = (0..arg_types.len()).all(|i| {
                                        let p = get_param(i);
                                        let a_ty = &arg_types[i];
                                        self.types_are_compatible(&p.type_node.as_str(), a_ty)
                                    });
                                    if matches {
                                        let sig_key = crate::middle_end::semantic::analyzer::decl::make_sig_key(name, &sig.params);
                                        self.record_dependency(sig_key);
                                        best_match = Some(sig.return_type.as_str());
                                        break;
                                    }
                                }
                            }
                        }

                        // Pass 2: Generic fallback/catch-all overloads
                        if best_match.is_none() {
                            for sig in &sigs {
                                let is_variadic =
                                    sig.params.last().map(|p| p.is_variadic).unwrap_or(false);
                                let min_args = sig
                                    .params
                                    .iter()
                                    .take_while(|p| p.default_value.is_none() && !p.is_variadic)
                                    .count();
                                let max_args = if is_variadic {
                                    usize::MAX
                                } else {
                                    sig.params.len()
                                };
                                if arg_types.len() >= min_args && arg_types.len() <= max_args {
                                    let get_param = |idx: usize| -> &Param {
                                        if idx < sig.params.len() {
                                            &sig.params[idx]
                                        } else {
                                            sig.params.last().unwrap()
                                        }
                                    };
                                    let mut gen_bindings: std::collections::HashMap<
                                        String,
                                        String,
                                    > = std::collections::HashMap::new();
                                    let mut matches = true;
                                    for i in 0..arg_types.len() {
                                        let p = get_param(i);
                                        let a_ty = &arg_types[i];
                                        if let BaseType::GenericParam(g_name) = &p.type_node {
                                            if g_name.starts_with("...") {
                                                // Heterogeneous variadic pack: accepts any type!
                                            } else {
                                                // Homogeneous single-type generic:
                                                if let Some(existing) = gen_bindings.get(g_name) {
                                                    if !self.types_are_compatible(existing, a_ty) {
                                                        matches = false;
                                                        break;
                                                    }
                                                } else {
                                                    gen_bindings
                                                        .insert(g_name.clone(), a_ty.clone());
                                                }
                                            }
                                        } else if !self
                                            .types_are_compatible(&p.type_node.as_str(), a_ty)
                                        {
                                            matches = false;
                                            break;
                                        }
                                    }
                                    if matches {
                                        let sig_key = crate::middle_end::semantic::analyzer::decl::make_sig_key(name, &sig.params);
                                        self.record_dependency(sig_key);
                                        let generic_map = resolve_call_generics(
                                            &sig.generics,
                                            &sig.params,
                                            &call_generics,
                                            &arg_types,
                                        );
                                        let resolved_ret =
                                            sig.return_type.substitute_generics(&generic_map);
                                        if let BaseType::GenericParam(ret_g) = &resolved_ret {
                                            if let Some(inferred_ret) = gen_bindings.get(ret_g) {
                                                best_match = Some(inferred_ret.clone());
                                                break;
                                            }
                                        }
                                        best_match = Some(resolved_ret.as_str());
                                        break;
                                    }
                                }
                            }
                        }

                        if let Some(ret) = best_match {
                            return Ok(ret);
                        }
                        if let Some(last) = sigs.last() {
                            return Ok(last.return_type.as_str());
                        }
                    }

                    if let Some(info) = self.current_env.borrow().lookup(name) {
                        if let SymbolKind::Function {
                            return_type,
                            generics: fn_generics,
                            params,
                            ..
                        } = &info.kind
                        {
                            let generic_map = resolve_call_generics(
                                fn_generics,
                                params,
                                &call_generics,
                                &arg_types,
                            );
                            let resolved_ret = return_type.substitute_generics(&generic_map);
                            return Ok(resolved_ret.as_str());
                        }
                    }
                    if crate::middle_end::interpreter::intrinsics::CompilerIntrinsics::is_compile_member(name) {
                        return Ok("void".to_string());
                    }
                    if name != "Some" && self.current_env.borrow().lookup(name).is_none() {
                        let entity = if name.starts_with('@')
                            || name.starts_with('$')
                            || name.contains("::")
                        {
                            "Macro"
                        } else {
                            "Function"
                        };
                        return Err(format!(
                            "Semantic Error: {} '{}' is not defined in this scope.",
                            entity, name
                        ));
                    }
                } else if let Expr::Lambda { return_type, .. } = &**callee {
                    if let Some(rt) = return_type {
                        return Ok(rt.as_str());
                    } else {
                        return Ok("void".to_string());
                    }
                }
                Ok("unknown".to_string())
            }

            Expr::This => {
                if !self.in_class && !self.in_struct && !self.in_custom_scope {
                    return Err(
                        "Semantic Error: Cannot use 'this' outside of a class, struct, or custom scope".to_string()
                    );
                }
                Ok(self
                    .current_type_name
                    .clone()
                    .unwrap_or_else(|| "object".to_string()))
            }

            Expr::Global => Ok("object".to_string()),

            Expr::Super => {
                if !self.in_class {
                    return Err("Semantic Error: Cannot use 'super' outside of a class".to_string());
                }
                Ok("object".to_string())
            }

            Expr::ArrayAllocate {
                type_node,
                size,
                length,
            } => {
                self.visit_expression(size)?;
                if let Some(l) = length {
                    self.visit_expression(l)?;
                }
                Ok(format!("array<{}>", type_node.as_str()))
            }

            Expr::BinaryOp {
                left,
                operator,
                right,
            } => {
                let left_type = self.visit_expression(left)?;
                let right_type = self.visit_expression(right)?;
                if left_type == "undefined" || right_type == "undefined" {
                    return Err(
                        "Semantic Error: Cannot use undefined value in operation".to_string()
                    );
                }
                let op_handle = match operator.as_str() {
                    "+" => Some("add"),
                    "-" => Some("sub"),
                    "*" => Some("mul"),
                    "/" => Some("div"),
                    "%" => Some("mod"),
                    "==" => Some("equal"),
                    "!=" => Some("not_equal"),
                    "<" => Some("less_than"),
                    ">" => Some("greater_than"),
                    "<=" => Some("less_than_equal"),
                    ">=" => Some("greater_than_equal"),
                    "->" => Some("arrow"),
                    "=>" => Some("fat_arrow"),
                    _ => None,
                };
                if let Some(h) = op_handle {
                    self.record_dependency(format!("{}::{}", left_type, h));
                }
                if operator == "->" || operator == "=>" {
                    let handle_name = if operator == "->" { "arrow" } else { "fat_arrow" };
                    let clean_left = left_type.trim_end_matches('*').trim();
                    let bp_opt = self.current_env.borrow().lookup_blueprint(clean_left);
                    if let Some(bp) = bp_opt {
                        if let Some(h_sig) = bp.handle_signatures.get(handle_name).or_else(|| bp.methods.get(handle_name)) {
                            return Ok(h_sig.return_type.as_str().to_string());
                        }
                    }
                    return Ok(left_type);
                }
                match operator.as_str() {
                    "==" | "!=" | ">" | "<" | ">=" | "<=" | "&&" | "||" => Ok("bool".to_string()),
                    _ => {
                        if left_type != "unknown" {
                            Ok(left_type)
                        } else {
                            Ok(right_type)
                        }
                    }
                }
            }

            Expr::UnaryOp { operand, operator } => {
                let op_type = self.visit_expression(operand)?;
                if op_type == "undefined" {
                    return Err(
                        "Semantic Error: Cannot use undefined value in operation".to_string()
                    );
                }
                if operator == "!" {
                    return Ok("bool".to_string());
                } else if operator == "*" {
                    for prefix in &["raw_ptr<", "array<", "address<"] {
                        if op_type.starts_with(prefix) {
                            let inner = strip_wrapper(&op_type, prefix);
                            return Ok(inner.to_string());
                        }
                    }
                } else if operator == "&" {
                    return Ok(format!("address<{}>", op_type));
                }
                Ok(op_type)
            }

            Expr::PrefixUpdate { right, .. } => self.visit_expression(right),
            Expr::PostfixUpdate { left, .. } => self.visit_expression(left),
            // TODO: add check if the type of the object contains a index_access handle if it not a name or array or pointer
            Expr::IndexAccess { object, indices } => {
                for idx in indices {
                    self.visit_expression(idx)?; //todo: check if the index is the same type as the parameter of the index_access handle
                }

                let obj_type = self.visit_expression(object)?;

                let mut current_type = obj_type.clone();

                while current_type.starts_with("raw_ptr<") {
                    current_type = strip_wrapper(&current_type, "raw_ptr<").to_string();
                }

                // Compile-time bounds checking for constant indices
                if indices.len() == 1 {
                    if let Expr::LiteralInt(val) = &indices[0] {
                        if *val < 0 {
                            return Err(format!(
                                "Semantic Error: Array index cannot be negative (found {}).",
                                val
                            ));
                        }
                        if let Some(start_bracket) = current_type.rfind('[') {
                            if let Some(end_bracket) = current_type.rfind(']') {
                                if start_bracket < end_bracket {
                                    let size_str = &current_type[start_bracket + 1..end_bracket];
                                    if let Ok(size) = size_str.parse::<i128>() {
                                        if *val >= size {
                                            return Err(format!(
                                                "Semantic Error: Array index {} is out of bounds for array of size {}.",
                                                val, size
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                        if let Expr::ArrayLiteral(elems) = object.as_ref() {
                            if (*val as usize) >= elems.len() {
                                return Err(format!(
                                    "Semantic Error: Array index {} is out of bounds for array literal of size {}.",
                                    val, elems.len()
                                ));
                            }
                        }
                    }
                }
                if current_type.starts_with("array<") || current_type == "array" {
                    if indices.len() > 1 {
                        return Err(
                            format!(
                                "Semantic Error: Multi-index access [a, b] is only supported for custom types with an 'index_access' handle. Built-in arrays must use [a][b]."
                            )
                        );
                    }
                    if current_type == "array" {
                        return Ok("unknown".to_string());
                    }
                    return Ok(strip_wrapper(&current_type, "array<").to_string());
                }
                if obj_type.starts_with("raw_ptr<") {
                    return Ok(strip_wrapper(&obj_type, "raw_ptr<").to_string());
                }
                if current_type != "unknown" {
                    let bp_name = extract_blueprint_name_from_type(&obj_type)
                        .unwrap_or_else(|| obj_type.clone());
                    let bp_match = {
                        let env = self.current_env.borrow();
                        if let Some(bp) = env.lookup_blueprint(&bp_name) {
                            if bp.handles.contains(&HandleMethods::IndexAccess)
                                || bp.methods.contains_key("index_access")
                            {
                                let ret = bp
                                    .methods
                                    .get("index_access")
                                    .map(|s| s.return_type.as_str())
                                    .unwrap_or_else(|| "int32".to_string());
                                Some(ret)
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    };
                    if let Some(ret_sig) = bp_match {
                        self.record_dependency(format!("{}::index_access", bp_name));
                        return Ok(ret_sig);
                    }

                    let meta_match = if let Some(meta) = self.global_metadata.get(&bp_name) {
                        if meta.handles.contains(&HandleMethods::IndexAccess)
                            || meta.methods.contains_key("index_access")
                        {
                            let ret = meta
                                .methods
                                .get("index_access")
                                .map(|s| s.return_type.as_str())
                                .unwrap_or_else(|| "int32".to_string());
                            Some(ret)
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                    if let Some(ret_ty) = meta_match {
                        self.record_dependency(format!("{}::index_access", bp_name));
                        return Ok(ret_ty);
                    }
                    Err(
                        format!("Semantic Error: Type '{}' does not support index access (make a index_access handle for it).", obj_type)
                    )
                } else {
                    Ok("unknown".to_string())
                }
            }

            Expr::PropertyAccess { object, property } => {
                let obj_type = self.visit_expression(object)?;

                if property == "default" && (obj_type.starts_with("type<") || obj_type == "type") {
                    if obj_type.starts_with("type<") {
                        let inner = strip_type_wrapper(&obj_type).to_string();
                        return Ok(inner);
                    } else {
                        return Ok("unknown".to_string());
                    }
                }

                if matches!(
                    property.as_str(),
                    "broken"
                        | "is_done"
                        | "yielded"
                        | "returned"
                        | "leaved"
                        | "continued"
                        | "has_break"
                        | "has_yield"
                        | "has_leave"
                        | "has_return"
                        | "has_call"
                        | "has_error"
                        | "compilable"
                        | "is_compilable"
                ) {
                    return Ok("bool".to_string());
                }

                // If object is an address<T> (or T&), unwrap to T for direct property access (C++ reference style)
                let actual_obj_type = if obj_type.starts_with("address<") {
                    strip_wrapper(&obj_type, "address<").to_string()
                } else if obj_type.ends_with('&') {
                    obj_type[..obj_type.len() - 1].trim().to_string()
                } else {
                    obj_type.clone()
                };

                // Try to find in blueprint
                let bp_name_opt =
                    if actual_obj_type.starts_with("type<") || actual_obj_type == "type" {
                        Some("type".to_string())
                    } else {
                        extract_blueprint_name_from_type(&actual_obj_type)
                            .or_else(|| Some(actual_obj_type.clone()))
                    };
                if let Some(bp_name) = bp_name_opt {
                    if let Some(bp) = self.current_env.borrow().lookup_blueprint(&bp_name) {
                        if let Some(field_type) = bp.fields.get(property) {
                            return Ok(field_type.as_str());
                        }
                        if let Some(sig) = bp
                            .methods
                            .get(property)
                            .or_else(|| bp.handle_signatures.get(property))
                        {
                            return Ok(sig.return_type.as_str());
                        }
                    }
                    // fallback to global_metadata
                    if let Some(meta) = self.global_metadata.get(&bp_name) {
                        if let Some(field_type) = meta.fields.get(property) {
                            return Ok(field_type.as_str());
                        }
                        if let Some(fn_type) = meta
                            .methods
                            .get(property)
                            .or_else(|| meta.handle_signatures.get(property))
                        {
                            return Ok(fn_type.return_type.as_str());
                        }
                    }

                    // Forward access via `property_access` handle if defined on wrapper
                    if let Some(target_type) =
                        self.resolve_property_access_target_type(&actual_obj_type)
                    {
                        let target_bp_opt = extract_blueprint_name_from_type(&target_type)
                            .or_else(|| Some(target_type.clone()));
                        if let Some(t_bp_name) = target_bp_opt {
                            if let Some(t_bp) =
                                self.current_env.borrow().lookup_blueprint(&t_bp_name)
                            {
                                if let Some(field_type) = t_bp.fields.get(property) {
                                    return Ok(field_type.as_str());
                                }
                                if let Some(sig) = t_bp
                                    .methods
                                    .get(property)
                                    .or_else(|| t_bp.handle_signatures.get(property))
                                {
                                    return Ok(sig.return_type.as_str());
                                }
                            }
                            if let Some(t_meta) = self.global_metadata.get(&t_bp_name) {
                                if let Some(field_type) = t_meta.fields.get(property) {
                                    return Ok(field_type.as_str());
                                }
                                if let Some(fn_type) = t_meta
                                    .methods
                                    .get(property)
                                    .or_else(|| t_meta.handle_signatures.get(property))
                                {
                                    return Ok(fn_type.return_type.as_str());
                                }
                            }
                        }
                    }
                }
                Ok("unknown".to_string())
            }

            Expr::Instantiate { target, args } => {
                for arg in args {
                    self.visit_expression(arg)?;
                }
                if let Expr::Identifier(n) = &**target {
                    self.record_dependency(n.clone());
                    self.record_dependency(format!("{}::init", n));
                    return Ok(n.clone());
                }
                if let Expr::NamespaceAccess { namespace, .. } = &**target {
                    self.record_dependency(namespace.clone());
                    self.record_dependency(format!("{}::init", namespace));
                    if self.global_metadata.contains_key(namespace)
                        || self
                            .current_env
                            .borrow()
                            .lookup_blueprint(namespace)
                            .is_some()
                    {
                        return Ok(namespace.clone());
                    }
                }
                Ok("unknown".to_string())
            }

            Expr::New { type_node, target } => {
                self.visit_expression(target)?;
                let type_str = type_node.as_str();
                for dep in extract_all_type_names(&type_str) {
                    self.record_dependency(dep.clone());
                    self.record_dependency(format!("{}::init", dep));
                }
                Ok(type_str)
            }

            Expr::Lambda { params, body, .. } => {
                self.enter_scope();
                for p in params {
                    let p_info = SymbolInfo {
                        name: p.name.clone(),
                        kind: SymbolKind::Variable {
                            type_node: p.type_node.clone(),
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
                        .define(p.name.clone(), p_info)?;
                }
                let mut inferred_return: Option<String> = None;
                for s in body {
                    if let Stmt::ReturnStmt(Some(ret_expr)) = s {
                        inferred_return = Some(self.visit_expression(ret_expr)?);
                    } else {
                        self.visit_statement(s)?;
                    }
                }
                self.leave_scope();
                let ret_t = inferred_return.unwrap_or_else(|| "void".to_string());
                let p_types: Vec<String> = params.iter().map(|p| p.type_node.as_str()).collect();
                Ok(format!("lambda<({}), {}>", p_types.join(", "), ret_t))
            }

            Expr::Cast { expr, target_type } => {
                let source_type_str = self.visit_expression(expr)?;
                let target_type_str = target_type.as_str();

                let source_base = extract_blueprint_name_from_type(&source_type_str)
                    .unwrap_or_else(|| source_type_str.clone());

                let source_ft = self.get_fast_type(&source_base);
                let target_ft = self.get_fast_type(&target_type_str);

                if source_ft.castable_to(&target_ft) {
                    return Ok(target_type_str);
                }

                // Check handle cast on source type (struct, class, blueprint, etc.)
                let has_cast_handle =
                    if let Some(bp) = self.current_env.borrow().lookup_blueprint(&source_base) {
                        bp.has_handle(HandleMethods::Cast)
                            || bp.handles.iter().any(|h| h.as_str() == "cast")
                    } else if let Some(meta) = self.global_metadata.get(&source_base) {
                        meta.handles
                            .iter()
                            .any(|h| h.as_str() == "cast" || matches!(h, HandleMethods::Cast))
                    } else {
                        false
                    };

                if has_cast_handle {
                    return Ok(target_type_str);
                }

                // If source and target are the same type or generic/type parameter (e.g. T or ty in std.fast)
                if source_type_str == target_type_str
                    || self.is_generic_type(&source_type_str)
                    || self.is_generic_type(&target_type_str)
                    || source_type_str == "type"
                    || target_type_str == "type"
                    || target_type_str == "unknown"
                {
                    return Ok(target_type_str);
                }

                return Err(format!(
                    "Semantic Error: Type '{}' cannot be casted to '{}' using 'as'.",
                    source_type_str, target_type_str
                ));
            }
            Expr::HandleCall {
                object,
                handle_name,
                args,
            } => {
                let obj_type = self.visit_expression(object)?;
                for arg in args {
                    self.visit_expression(arg)?;
                }
                if obj_type.starts_with("array<") || obj_type.ends_with("[]") {
                    let elem = if obj_type.starts_with("array<") {
                        strip_wrapper(&obj_type, "array<").to_string()
                    } else {
                        obj_type.trim_end_matches("[]").to_string()
                    };
                    match handle_name.as_str() {
                        "iter" => return Ok(format!("array_iterator<{}>", elem)),
                        "is_done_iter" => return Ok("bool".to_string()),
                        "next" | "index_access" => return Ok(elem),
                        "display" => return Ok("array<char>".to_string()),
                        "drop" => return Ok("void".to_string()),
                        _ => {}
                    }
                }
                let bp_name_opt =
                    extract_blueprint_name_from_type(&obj_type).or_else(|| Some(obj_type.clone()));
                if let Some(bp_name) = bp_name_opt {
                    if let Some(bp) = self.current_env.borrow().lookup_blueprint(&bp_name) {
                        if let Some(sig) = bp
                            .handle_signatures
                            .get(handle_name)
                            .or_else(|| bp.methods.get(handle_name))
                        {
                            return Ok(sig.return_type.as_str());
                        }
                    }
                    if let Some(meta) = self.global_metadata.get(&bp_name) {
                        if let Some(fn_type) = meta
                            .handle_signatures
                            .get(handle_name)
                            .or_else(|| meta.methods.get(handle_name))
                        {
                            return Ok(fn_type.return_type.as_str());
                        }
                    }
                }
                Ok("unknown".to_string())
            }
        }
    }

    pub(crate) fn resolve_property_access_target_type(&self, raw_obj_type: &str) -> Option<String> {
        let (base_name, type_args) = extract_type_args_from_str(raw_obj_type);
        let has_prop_access = {
            let env = self.current_env.borrow();
            if let Some(bp) = env.lookup_blueprint(&base_name) {
                bp.handles.iter().any(|h| h.as_str() == "property_access")
                    || bp.handle_signatures.contains_key("property_access")
            } else if let Some(meta) = self.global_metadata.get(&base_name) {
                meta.handles.iter().any(|h| h.as_str() == "property_access")
                    || meta.handle_signatures.contains_key("property_access")
            } else {
                false
            }
        };
        if !has_prop_access {
            return None;
        }

        // If the wrapper has type arguments (e.g. ref<Point>, mutRef<Point>, pointer<Point>),
        // the inner target type is the primary type argument.
        if let Some(first_arg) = type_args.first() {
            return Some(first_arg.as_str());
        }

        // If no explicit generic args in string, check if the blueprint defines a field 'ptr'
        let ptr_type = {
            let env = self.current_env.borrow();
            if let Some(bp) = env.lookup_blueprint(&base_name) {
                bp.fields.get("ptr").cloned()
            } else if let Some(meta) = self.global_metadata.get(&base_name) {
                meta.fields.get("ptr").map(|bt| bt.clone())
            } else {
                None
            }
        };

        if let Some(p) = ptr_type {
            let p_str = p.as_str();
            if p_str.starts_with("raw_ptr<") {
                return Some(strip_wrapper(&p_str, "raw_ptr<").to_string());
            } else if p_str.ends_with('*') {
                return Some(p_str[..p_str.len() - 1].trim().to_string());
            }
        }

        None
    }
}
