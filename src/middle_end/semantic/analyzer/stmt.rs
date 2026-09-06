use super::*;

impl SemanticAnalyzer {
    pub(crate) fn visit_statement(&mut self, stmt: &Stmt) -> Result<(), String> {
        match stmt {
            Stmt::Declaration(decl) => {
                self.visit_declaration(decl)?;
            }

            Stmt::ReassignStmt { target, value, op } => {
                self.analyze_reassign(target, value, op)?;
            }

            Stmt::ExpressionStmt(expr) | Stmt::CallStmt(expr) => {
                self.visit_expression(expr)?;
            }

            Stmt::CompileValidation { body, args } => {
                for a in args {
                    let _ = self.visit_expression(a);
                }
                self.enter_scope();
                for s in body {
                    self.visit_statement(s)?;
                }
                self.leave_scope();
            }

            Stmt::CaseStmt { option, body, .. } => {
                self.enter_scope();
                if let Expr::Call { args, .. } = option {
                    for arg in args {
                        if let Expr::Identifier(var_name) = arg {
                            let info = SymbolInfo {
                                name: var_name.clone(),
                                kind: SymbolKind::Variable {
                                    type_node: BaseType::Unknown,
                                    editability: Editability::Editable,
                                    is_array: false,
                                },
                                visibility: Visibility::Private,
                                dependencies: vec![],
                                is_used: true,
                                is_param: false,
                                is_uninitialized: false,
                                is_compilable: false,
                            };
                            let _ = self
                                .current_env
                                .borrow_mut()
                                .define_or_update(var_name.clone(), info);
                        }
                    }
                } else if let Expr::Instantiate { args, .. } = option {
                    if args.len() == 1 {
                        if let Expr::ObjectLiteral(stmts) = &args[0] {
                            for s in stmts {
                                let var_name = match s {
                                    Stmt::Declaration(Decl::VarDecl { name, .. }) => {
                                        Some(name.clone())
                                    }
                                    Stmt::ReassignStmt { target, .. } => {
                                        if let Expr::Identifier(n) = target {
                                            Some(n.clone())
                                        } else {
                                            None
                                        }
                                    }
                                    Stmt::ExpressionStmt(Expr::Identifier(n)) => Some(n.clone()),
                                    _ => None,
                                };
                                if let Some(name) = var_name {
                                    let info = SymbolInfo {
                                        name: name.clone(),
                                        kind: SymbolKind::Variable {
                                            type_node: BaseType::Unknown,
                                            editability: Editability::Editable,
                                            is_array: false,
                                        },
                                        visibility: Visibility::Private,
                                        dependencies: vec![],
                                        is_used: true,
                                        is_param: false,
                                        is_uninitialized: false,
                                        is_compilable: false,
                                    };
                                    let _ = self
                                        .current_env
                                        .borrow_mut()
                                        .define_or_update(name.clone(), info);
                                }
                            }
                        }
                    }
                }
                for s in body {
                    self.visit_statement(s)?;
                }
                self.leave_scope();
            }

            Stmt::IfStmt {
                condition,
                then_block,
                else_block,
            } => {
                let cond_type = self.visit_expression(condition)?;
                if cond_type != "bool" && cond_type != "unknown" {
                    return Err("Semantic Error: if condition must be a boolean".to_string());
                }

                let static_val = self.eval_static_bool_expr(condition);
                if let Some(true) = static_val {
                    self.enter_scope();
                    for s in then_block {
                        self.visit_statement(s)?;
                    }
                    self.leave_scope();
                    // Dead branch elimination: else branch is eliminated, do not analyze or throw!
                    return Ok(());
                } else if let Some(false) = static_val {
                    if let Some(eb) = else_block {
                        self.enter_scope();
                        for s in eb {
                            self.visit_statement(s)?;
                        }
                        self.leave_scope();
                    }
                    return Ok(());
                }

                let prev_in_generic = self.in_generic_template;
                self.in_generic_template = true;
                self.enter_scope();
                for s in then_block {
                    self.visit_statement(s)?;
                }
                self.leave_scope();
                if let Some(eb) = else_block {
                    self.enter_scope();
                    for s in eb {
                        self.visit_statement(s)?;
                    }
                    self.leave_scope();
                }
                self.in_generic_template = prev_in_generic;
            }

            Stmt::ForInStmt {
                item,
                iterable,
                body,
            } => {
                let iterable_type = self.visit_expression(iterable)?;
                let (base_name, type_args) = extract_type_args_from_str(&iterable_type);
                let item_type = if iterable_type.starts_with("array<")
                    || iterable_type.ends_with("[]")
                {
                    iterable_type
                        .trim_start_matches("array<")
                        .trim_end_matches('>')
                        .trim_end_matches("[]")
                        .to_string()
                } else if iterable_type == "str" {
                    "char".to_string()
                } else if let Some(bp) = self.current_env.borrow().lookup_blueprint(&base_name) {
                    let specialized_bp = bp.specialize(&type_args);
                    if let Some(sig) = specialized_bp.methods.get("next") {
                        extract_iter_payload_type(&sig.return_type)
                    } else if specialized_bp.handles.contains(&HandleMethods::Next) {
                        if !type_args.is_empty() {
                            type_args[0].as_str()
                        } else {
                            "int32".to_string()
                        }
                    } else {
                        return Err(
                            format!("Semantic Error: Type '{}' does not implement 'next' handle for for-in iteration", iterable_type)
                        );
                    }
                } else if let Some(meta) = self
                    .global_metadata
                    .get(&base_name)
                    .or_else(|| self.global_metadata.get(&iterable_type))
                {
                    if let Some(next_fn) = meta.methods.get("next") {
                        let mut map = HashMap::new();
                        let generics = match &meta.ty {
                            BaseType::Struct { generics, .. }
                            | BaseType::Class { generics, .. }
                            | BaseType::Enum { generics, .. }
                            | BaseType::Blueprint { generics, .. } => generics.clone(),
                            _ => vec![],
                        };
                        for (g_param, g_arg) in generics.iter().zip(type_args.iter()) {
                            map.insert(g_param.as_str(), g_arg.clone());
                        }
                        let specialized_ret = next_fn.return_type.substitute_generics(&map);
                        extract_iter_payload_type(&specialized_ret)
                    } else if meta.handles.contains(&HandleMethods::Next) {
                        if !type_args.is_empty() {
                            type_args[0].as_str()
                        } else {
                            "int32".to_string()
                        }
                    } else {
                        return Err(
                            format!("Semantic Error: Type '{}' does not implement 'next' handle for for-in iteration", iterable_type)
                        );
                    }
                } else {
                    return Err(
                        format!("Semantic Error: Expected iterable, array, or string in for-in loop, got '{}'", iterable_type)
                    );
                };

                self.enter_scope();

                if let Stmt::Declaration(Decl::VarDecl {
                    type_node, name, ..
                }) = &**item
                {
                    let declared_type = type_node.as_str();
                    if item_type != "unknown"
                        && !self.types_are_compatible(&declared_type, &item_type)
                    {
                        return Err(
                            format!(
                                "Semantic Error: Type mismatch in for-in loop. Iterable elements are '{}', but item is declared as '{}'",
                                item_type,
                                declared_type
                            )
                        );
                    }
                    self.visit_statement(item)?;
                    self.current_env.borrow_mut().mark_initialized(name);
                } else if let Stmt::ExpressionStmt(Expr::Identifier(var_name)) = &**item {
                    let info = SymbolInfo {
                        name: var_name.clone(),
                        kind: SymbolKind::Variable {
                            type_node: BaseType::from_str(&item_type),
                            editability: Editability::Editable,
                            is_array: false,
                        },
                        visibility: Visibility::Private,
                        dependencies: vec![],
                        is_used: false,
                        is_param: false,
                        is_uninitialized: false,
                        is_compilable: false,
                    };
                    self.current_env
                        .borrow_mut()
                        .define(var_name.clone(), info)?;
                } else {
                    self.visit_statement(item)?;
                }

                match body {
                    EitherBlock::Inline(stmts) => {
                        self.active_flags.push("+has_break".to_string());
                        for s in stmts {
                            self.visit_statement(s)?;
                        }
                        self.active_flags.retain(|f| f != "+has_break");
                    }
                    EitherBlock::External(expr) => {
                        self.visit_expression(expr)?;
                    }
                }
                self.leave_scope();
            }

            Stmt::WhileStmt { condition, body } => {
                let cond_type = self.visit_expression(condition)?;
                if cond_type != "bool" && cond_type != "unknown" {
                    return Err("Semantic Error: loop condition must be a boolean".to_string());
                }
                match body {
                    EitherBlock::Inline(stmts) => {
                        self.enter_scope();
                        self.active_flags.push("+has_break".to_string());
                        for s in stmts {
                            self.visit_statement(s)?;
                        }
                        self.active_flags.retain(|f| f != "+has_break");
                        self.leave_scope();
                    }
                    EitherBlock::External(expr) => {
                        self.visit_expression(expr)?;
                    }
                }
            }

            Stmt::DoWhileStmt { body, condition } => {
                let cond_type = self.visit_expression(condition)?;
                if cond_type != "bool" && cond_type != "unknown" {
                    return Err("Semantic Error: do-while condition must be a boolean".to_string());
                }
                match body {
                    EitherBlock::Inline(stmts) => {
                        self.enter_scope();
                        self.active_flags.push("+has_break".to_string());
                        for s in stmts {
                            self.visit_statement(s)?;
                        }
                        self.active_flags.retain(|f| f != "+has_break");
                        self.leave_scope();
                    }
                    EitherBlock::External(expr) => {
                        self.visit_expression(expr)?;
                    }
                }
            }

            Stmt::LoopStmt { count, body } => {
                if let Some(c) = count {
                    let c_type = self.visit_expression(c)?;
                    if !c_type.starts_with("int")
                        && !c_type.starts_with("uint")
                        && c_type != "unknown"
                    {
                        return Err("Semantic Error: loop count must be an integer".to_string());
                    }
                }
                match body {
                    EitherBlock::Inline(stmts) => {
                        self.enter_scope();
                        self.active_flags.push("+has_break".to_string());
                        for s in stmts {
                            self.visit_statement(s)?;
                        }
                        self.active_flags.retain(|f| f != "+has_break");
                        self.leave_scope();
                    }
                    EitherBlock::External(expr) => {
                        self.visit_expression(expr)?;
                    }
                }
            }

            Stmt::SwitchStmt {
                condition, cases, ..
            } => {
                self.visit_expression(condition)?;
                self.enter_scope();
                self.active_flags.push("+has_break".to_string());
                for s in cases {
                    self.visit_statement(s)?;
                }
                self.active_flags.retain(|f| f != "+has_break");
                self.leave_scope();
            }

            Stmt::DelStmt { target, is_array } => {
                if let Expr::Identifier(name) = target {
                    let var_info = if let Some(info) = self.current_env.borrow().lookup(name) {
                        info
                    } else {
                        return Err(format!(
                            "Semantic Error: Cannot delete non-existent variable '{}'",
                            name
                        ));
                    };
                    self.current_env.borrow_mut().mark_used(name);

                    if let Some(type_node) = var_info.type_node() {
                        if matches!(type_node, BaseType::Name(_)) {
                            return Err(
                                format!("Semantic Error: Cannot delete name '{}'\n (name is a read-only ref delete it through a modify instead)", name)
                            );
                        }
                    }

                    // Stage 1: Double-Delete Check
                    if self.deleted_vars.contains(name) {
                        return Err(format!(
                            "Semantic Error: Variable '{}' has already been deleted.",
                            name
                        ));
                    }

                    let type_str = var_info.type_str();
                    let is_pointer_or_ref = type_str.ends_with('*')
                        || type_str.starts_with("pointer<")
                        || type_str.starts_with("modify<")
                        || type_str.starts_with("name<")
                        || type_str == "name"
                        || type_str.starts_with("array<")
                        || type_str.ends_with("[]")
                        || *is_array;

                    let is_on_heap = self.heap_allocated_vars.contains(name)
                        || is_pointer_or_ref
                        || self.is_class_type(&type_str);

                    // Stage 2: Non-heap primitive check
                    if self.is_primitive_stack_type(&type_str) && !is_pointer_or_ref && !is_on_heap
                    {
                        return Err(format!("Semantic Error: Cannot delete variable '{}' because it is not allocated on the heap.", name));
                    }

                    // Stage 3: Class, Struct, Blueprint check for drop handle
                    if self.is_class_type(&type_str) {
                        if !self.type_has_drop_handle(&type_str) {
                            return Err(format!(
                                "Semantic Error: Cannot delete object '{}' of class '{}' because it does not implement a 'drop' handle.",
                                name, type_str
                            ));
                        }
                    } else if self.is_struct_or_blueprint_type(&type_str) && !is_pointer_or_ref {
                        if !self.type_has_drop_handle(&type_str) {
                            return Err(format!(
                                "Semantic Error: Cannot delete object '{}' of type '{}' because it does not implement a 'drop' handle.",
                                name, type_str
                            ));
                        }
                    }

                    // Mark as deleted and remove from active heap variables
                    self.deleted_vars.insert(name.clone());
                    self.heap_allocated_vars.remove(name);
                } else {
                    self.visit_expression(target)?;
                }
            }

            Stmt::ForStmt {
                init,
                condition,
                increment,
                body,
            } => {
                self.enter_scope();
                if let Some(i) = init {
                    self.visit_statement(i)?;
                }
                if let Some(c) = condition {
                    let cond_type = self.visit_expression(c)?;
                    if cond_type != "bool" && cond_type != "unknown" {
                        return Err("Semantic Error: for condition must be a boolean".to_string());
                    }
                }
                if let Some(inc) = increment {
                    self.visit_statement(inc)?;
                }
                match body {
                    EitherBlock::Inline(stmts) => {
                        self.enter_scope();
                        self.active_flags.push("+has_break".to_string());
                        for s in stmts {
                            self.visit_statement(s)?;
                        }
                        self.active_flags.retain(|f| f != "+has_break");
                        self.leave_scope();
                    }
                    EitherBlock::External(expr) => {
                        self.visit_expression(expr)?;
                    }
                }
                self.leave_scope();
            }

            Stmt::ReturnStmt(expr) => {
                if !self.active_flags.contains(&"+has_return".to_string()) {
                    return Err(
                        "Semantic Error: Return statement is not allowed in this scope. 'has_return' flag is not enabled.".to_string()
                    );
                }
                let actual_type;
                if expr.is_none() {
                    actual_type = "void".to_string();
                } else {
                    actual_type = self.visit_expression(&expr.clone().unwrap())?;
                }
                if let Some(expected_type) = &self.active_return_type.clone() {
                    let expected_str = expected_type.as_str();
                    if actual_type != "unknown"
                        && expected_str != "unknown"
                        && !self.types_are_compatible(&expected_str, &actual_type)
                    {
                        return Err(format!(
                            "Semantic Error: Return type mismatch. Expected '{}', got '{}'",
                            expected_str, actual_type
                        ));
                    }
                }
            }

            Stmt::BreakStmt => {
                if !self.active_flags.contains(&"+has_break".to_string()) {
                    return Err(
                        "Semantic Error: Break statement is not allowed outside loops or switch statements.".to_string()
                    );
                }
            }

            Stmt::ThrowStmt(expr) => {
                self.record_dependency("Error".to_string());
                if !self.active_flags.contains(&"+has_throw".to_string()) {
                    return Err(
                        "Semantic Error: Throw statement is not allowed here. The scope must have 'has_throw' enabled (e.g. inside try block or custom scope with error handle).".to_string()
                    );
                }
                let thrown_type = self.visit_expression(expr)?;
                if thrown_type != "unknown" {
                    let bp_name = extract_blueprint_name_from_type(&thrown_type)
                        .unwrap_or_else(|| thrown_type.clone());
                    let is_throwable = if bp_name == "Error" || bp_name == "std::Error" {
                        true
                    } else if let Some(bp) = self.current_env.borrow().lookup_blueprint(&bp_name) {
                        bp.handles.contains(&HandleMethods::Throw)
                            || bp.methods.contains_key("throw")
                            || bp.name == "Error"
                    } else if let Some(meta) = self.global_metadata.get(&bp_name) {
                        meta.handles.contains(&HandleMethods::Throw)
                            || meta.methods.contains_key("throw")
                            || meta.name == "Error"
                    } else {
                        false
                    };
                    if !is_throwable {
                        return Err(
                            format!("Semantic Error: Type '{}' is not throwable. Only 'Error' or types implementing a 'throw' handle can be thrown.", thrown_type)
                        );
                    }
                }
            }

            Stmt::TryCatchStmt {
                try_block,
                catch_param,
                catch_block,
            } => {
                self.record_dependency("Error".to_string());
                self.enter_scope();
                self.active_flags.push("+has_throw".to_string());
                for s in try_block {
                    self.visit_statement(s)?;
                }
                self.active_flags.retain(|f| f != "+has_throw");
                self.leave_scope();

                self.enter_scope();
                let info = SymbolInfo {
                    name: catch_param.clone(),
                    kind: SymbolKind::Variable {
                        type_node: BaseType::from_str("Error"),
                        editability: Editability::NotEditable,
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
                    .define(catch_param.clone(), info)?;
                for s in catch_block {
                    self.visit_statement(s)?;
                }
                self.leave_scope();
            }

            Stmt::GotoStmt(_) => {}

            Stmt::YieldStmt(expr) => {
                if !self.active_flags.contains(&"+has_yield".to_string()) {
                    return Err(
                        "Semantic Error: Yield statement is not allowed in this scope (forbidden in global scope).".to_string()
                    );
                }
                if let Some(e) = expr {
                    self.visit_expression(e)?;
                }
            }

            Stmt::LeaveStmt => {
                if !self.active_flags.contains(&"+has_leave".to_string()) {
                    return Err(
                        "Semantic Error: Leave statement is not allowed in this scope (forbidden in global scope).".to_string()
                    );
                }
            }

            Stmt::UsingStmt(name) => {
                if let Some(meta) = self.global_metadata.get(name) {
                    if let Some(ref variants) = meta.variants {
                        for v in variants {
                            let var_symbol = SymbolInfo {
                                name: v.name.clone(),
                                kind: SymbolKind::Variable {
                                    type_node: BaseType::from_str(name),
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
                            let _ = self
                                .current_env
                                .borrow_mut()
                                .define(v.name.clone(), var_symbol);
                        }
                    }
                }
                if let Some(bp) = self.current_env.borrow().lookup_blueprint(name) {
                    if !bp.variants.is_empty() {
                        for v in &bp.variants {
                            let var_symbol = SymbolInfo {
                                name: v.name.clone(),
                                kind: SymbolKind::Variable {
                                    type_node: BaseType::from_str(name),
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
                            let _ = self
                                .current_env
                                .borrow_mut()
                                .define(v.name.clone(), var_symbol);
                        }
                    }
                }
            }

            _ => {}
        }
        Ok(())
    }

    // ----------------------------------------------------------
    // analyze_reassign - ReassignStmt with full operator awareness
    // ----------------------------------------------------------
    pub(crate) fn analyze_reassign(&mut self, target: &Expr, value: &Expr, op: &str) -> Result<(), String> {
        // Scope flags are strictly read-only
        if let Expr::Identifier(name) = target {
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
            ) {
                return Err(format!(
                    "Semantic Error: Scope flag '{}' is read-only and cannot be manually modified",
                    name
                ));
            }
        }
        // Check mutability and pointer lifecycle
        let mut is_modify_target = false;
        if let Expr::Identifier(name) = target {
            if let Some(info) = self.current_env.borrow().lookup(name) {
                if !info.is_editable() {
                    return Err(format!(
                        "Semantic Error: Cannot reassign constant '{}'",
                        name
                    ));
                }
                if let SymbolKind::Variable { type_node, .. } = &info.kind {
                    if matches!(type_node, BaseType::Modify(_)) {
                        is_modify_target = true;
                    }
                }
            }
        }

        let expr_type = self.visit_expression(value)?;
        if let Expr::Identifier(name) = target {
            if self.deleted_vars.contains(name) {
                if matches!(value, Expr::Instantiate { .. }) {
                    self.deleted_vars.remove(name);
                    self.heap_allocated_vars.insert(name.clone());
                } else {
                    return Err(format!("Semantic Error: Cannot assign to variable '{}' because it has been deleted.", name));
                }
            } else if matches!(value, Expr::Instantiate { .. }) {
                self.heap_allocated_vars.insert(name.clone());
            }
            if expr_type != "undefined" {
                self.current_env.borrow_mut().mark_initialized(name);
            }
        }
        let target_type = self.visit_expression(target)?;

        if op == "=" && is_modify_target {
            let is_addr = matches!(value, Expr::UnaryOp { operator, .. } if operator == "&");
            if expr_type.starts_with("modify<")
                || expr_type.starts_with("name<")
                || expr_type.starts_with("pointer")
                || is_addr
            {
                if let Expr::Identifier(name) = target {
                    return Err(
                        format!("Semantic Error: Cannot reassign a modify pointer '{}' directly. 'modify' manages owned memory; use 'name' for rebindable weak references.", name)
                    );
                }
            }
        }

        if op != "=" {
            return self.verify_operator_overload(&target_type, &expr_type, op, target);
        }

        self.verify_type_assignment(&target_type, &expr_type, target)
    }

    // ----------------------------------------------------------
    // verify_operator_overload
    // ----------------------------------------------------------
    pub(crate) fn verify_operator_overload(
        &mut self,
        target_type: &str,
        expr_type: &str,
        op: &str,
        _target: &Expr,
    ) -> Result<(), String> {
        // Compound operators on primitive numerics are always allowed
        // e.g. counter += 1; x -= 2;
        let is_compound = matches!(op, "+=" | "-=" | "*=" | "/=" | "%=");
        if is_compound && Self::is_primitive_numeric(target_type) {
            return Ok(());
        }

        // Complex types: look up handle
        if is_complex_type(target_type) {
            let bp_name = extract_blueprint_name_from_type(target_type)
                .unwrap_or_else(|| target_type.to_string());

            return match resolve_handle_for_op(&self.current_env, &bp_name, op) {
                HandleLookupResult::Found(bp) => {
                    let handle = op_to_handle(op);
                    if !bp.handle_accepts_type(handle, expr_type) {
                        Err(
                            format!(
                                "Semantic Error: Handle '{}' in '{}' does not accept type '{}'. Check the handle's parameter type.",
                                handle.as_str(),
                                bp_name,
                                expr_type
                            )
                        )
                    } else {
                        Ok(())
                    }
                }
                HandleLookupResult::BlueprintNotFound => Ok(()),
                HandleLookupResult::HandleMissing { handle } =>
                    Err(
                        format!(
                            "Semantic Error: Type '{}' does not support the '{}' operator (missing handle '{}').",
                            bp_name,
                            op,
                            handle.as_str()
                        )
                    ),
                HandleLookupResult::UnknownOp => Ok(()),
            };
        }

        // Primitive with unknown operator -> allow if types unknown
        if target_type == "unknown" || expr_type == "unknown" {
            return Ok(());
        }

        Ok(())
    }

    // ----------------------------------------------------------
    // verify_type_assignment - normal "=" assignment check
    // ----------------------------------------------------------
    pub(crate) fn verify_type_assignment(
        &mut self,
        target_type: &str,
        expr_type: &str,
        target: &Expr,
    ) -> Result<(), String> {
        if expr_type == "default" || expr_type == "unknown" {
            return Ok(());
        }

        // ----------------------------------------------------------
        // Smart Pointers: name, modify, copy
        // ----------------------------------------------------------
        if target_type.starts_with("name<")
            || target_type.starts_with("modify<name<")
            || target_type.starts_with("copy<name<")
        {
            let is_modify = target_type.starts_with("modify");
            let is_copy = target_type.starts_with("copy");

            let inner = if is_modify {
                strip_wrapper(strip_wrapper(target_type, "modify<"), "name<")
            } else if is_copy {
                strip_wrapper(strip_wrapper(target_type, "copy<"), "name<")
            } else {
                strip_wrapper(target_type, "name<")
            };

            let expr_inner = if expr_type.starts_with("array<") {
                strip_wrapper(expr_type, "array<")
            } else if expr_type.starts_with("modify<name<") {
                strip_wrapper(strip_wrapper(expr_type, "modify<"), "name<")
            } else if expr_type.starts_with("copy<name<") {
                strip_wrapper(strip_wrapper(expr_type, "copy<"), "name<")
            } else if expr_type.starts_with("name<") {
                strip_wrapper(expr_type, "name<")
            } else {
                expr_type
            };
            if inner == "unknown" && expr_inner != "unknown" {
                if let Expr::Identifier(name) = target {
                    let maybe_info = self.current_env.borrow().lookup(name);
                    if let Some(mut info) = maybe_info {
                        let is_arr = expr_type.starts_with("array<");

                        let base_inner = BaseType::Name(Box::new(BaseType::from_str(expr_inner)));
                        let final_type_node = if is_modify {
                            BaseType::Modify(Box::new(base_inner))
                        } else if is_copy {
                            BaseType::Copy(Box::new(base_inner))
                        } else {
                            base_inner
                        };

                        info.kind = SymbolKind::Variable {
                            type_node: final_type_node,
                            editability: Editability::Editable,
                            is_array: is_arr,
                        };
                        self.current_env.borrow_mut().update(name, info);
                    }
                }
            } else if !self.types_are_compatible(inner, expr_inner)
                && inner != "unknown"
                && expr_type != "unknown"
                && expr_type != "object"
            {
                let prefix = if is_modify {
                    "modify"
                } else if is_copy {
                    "copy"
                } else {
                    "name"
                };
                return Err(format!(
                    "Semantic Error: Cannot reassign smart pointer '{}<{}>' to type '{}'",
                    prefix, inner, expr_type
                ));
            }
            return Ok(());
        }

        if target_type.starts_with("pointer<") {
            let inner = strip_wrapper(target_type, "pointer<");

            let expr_inner = if expr_type.starts_with("array<") {
                strip_wrapper(expr_type, "array<")
            } else if expr_type.starts_with("pointer<") {
                strip_wrapper(expr_type, "pointer<")
            } else if expr_type.starts_with("name<") {
                strip_wrapper(expr_type, "name<")
            } else {
                expr_type
            };

            if !self.types_are_compatible(inner, expr_inner)
                && inner != "unknown"
                && expr_type != "unknown"
                && expr_type != "object"
            {
                return Err(format!(
                    "Semantic Error: Cannot reassign pointer 'pointer<{}>' to type '{}'",
                    inner, expr_type
                ));
            }
            return Ok(());
        }

        if target_type == expr_type || target_type == "unknown" || expr_type == "unknown" {
            return Ok(());
        }

        if target_type == "name" && expr_type == "object" {
            return Ok(());
        }
        if target_type == "type" || target_type.starts_with("type<") {
            return Ok(());
        }

        if !self.types_are_compatible(target_type, expr_type) {
            return Err(format!(
                "Semantic Error: Cannot assign '{}' to type '{}'",
                expr_type, target_type
            ));
        }
        Ok(())
    }

    pub(crate) fn block_always_terminates(&self, stmts: &[Stmt]) -> bool {
        for s in stmts {
            if self.stmt_always_terminates(s) {
                return true;
            }
        }
        false
    }

    pub(crate) fn stmt_always_terminates(&self, stmt: &Stmt) -> bool {
        match stmt {
            Stmt::ReturnStmt(_)
            | Stmt::ThrowStmt(_)
            | Stmt::LeaveStmt
            | Stmt::YieldStmt(_)
            | Stmt::GotoStmt(_)
            | Stmt::SwitchStmt { .. }
            | Stmt::CaseStmt { .. } => true,
            Stmt::IfStmt {
                then_block,
                else_block,
                ..
            } => {
                if let Some(eb) = else_block {
                    self.block_always_terminates(then_block) && self.block_always_terminates(eb)
                } else {
                    false
                }
            }
            Stmt::TryCatchStmt {
                try_block,
                catch_block,
                ..
            } => {
                self.block_always_terminates(try_block) && self.block_always_terminates(catch_block)
            }
            Stmt::DoWhileStmt { body, .. } => match body {
                EitherBlock::Inline(stmts) => self.block_always_terminates(stmts),
                EitherBlock::External(_) => false,
            },
            _ => false,
        }
    }

}
