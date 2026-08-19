use crate::frontend::parser::ast::*;
use crate::middle_end::semantic::environment::{
    BlueprintData, Environment, FnSignature, SymbolInfo, SymbolKind,
};
use crate::middle_end::semantic::handle_resolver::{
    build_blueprint_from_metadata, extract_blueprint_name_from_type, is_complex_type,
    op_to_handle, resolve_handle_for_op, HandleLookupResult,
};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

// ============================================================
// SemanticAnalyzer - main orchestrator
// ============================================================
pub struct SemanticAnalyzer {
    pub current_env: Rc<RefCell<Environment>>,
    pub in_class: bool,
    pub in_struct: bool,
    pub in_custom_scope: bool,
    pub in_statement_scope: bool,
    pub active_flags: Vec<String>,
    pub active_return_type: Option<BaseType>,
    pub current_type_name: Option<String>,
    pub global_metadata: HashMap<String, TypeMetadata>,
    pub dependency_graph: HashMap<String, HashSet<String>>,
    pub current_context: Option<String>,
}

impl SemanticAnalyzer {
    pub fn new(global_metadata: HashMap<String, TypeMetadata>) -> Self {
        let mut analyzer = SemanticAnalyzer {
            current_env: Environment::new(),
            in_class: false,
            in_struct: false,
            in_custom_scope: false,
            in_statement_scope: false,
            current_type_name: None,
            active_flags: vec!["+has_exit".to_string()],
            active_return_type: None,
            global_metadata,
            dependency_graph: HashMap::new(),
            current_context: None,
        };
        analyzer.import_metadata();
        analyzer.inject_stdlib();
        analyzer
    }

    // ----------------------------------------------------------
    // Import global TypeMetadata -> BlueprintData into env
    // ----------------------------------------------------------
    fn import_metadata(&mut self) {
        let names: Vec<String> = self.global_metadata.keys().cloned().collect();
        for name in names {
            let meta = self.global_metadata[&name].clone();
            let bp = build_blueprint_from_metadata(&meta);
            self.current_env.borrow_mut().define_blueprint(name, bp);
        }
    }

    fn inject_stdlib(&mut self) {
        let std_funcs: &[(&str, BaseType)] = &[
            ("log", BaseType::Void),
            ("input", BaseType::from_str("string")),
            ("to_string", BaseType::from_str("string")),
        ];
        for (name, ret) in std_funcs {
            let info = SymbolInfo {
                name: name.to_string(),
                kind: SymbolKind::Function {
                    params: vec![],
                    return_type: ret.clone(),
                },
                visibility: Visibility::Public,
                dependencies: vec![],
            };
            self.current_env.borrow_mut().define_or_update(name.to_string(), info);
        }
    }

    // ----------------------------------------------------------
    // Scope management
    // ----------------------------------------------------------
    fn enter_scope(&mut self) {
        let new_env = Environment::with_parent(Rc::clone(&self.current_env));
        self.current_env = new_env;
    }

    fn leave_scope(&mut self) {
        let parent = self
            .current_env
            .borrow()
            .parent
            .clone()
            .expect("Cannot leave global scope");
        self.current_env = parent;
    }

    pub fn analyze(&mut self, ast: &Vec<Stmt>) -> Result<(), String> {
        for stmt in ast {
            self.visit_statement(stmt)?;
        }
        Ok(())
    }

    pub fn record_dependency(&mut self, dep: String) {
        if let Some(ctx) = &self.current_context {
            self.dependency_graph
                .entry(ctx.clone())
                .or_insert_with(HashSet::new)
                .insert(dep);
        }
    }

    // ----------------------------------------------------------
    // visit_statement
    // ----------------------------------------------------------
    fn visit_statement(&mut self, stmt: &Stmt) -> Result<(), String> {
        match stmt {
            Stmt::Declaration(decl) => {
                self.visit_declaration(decl)?;
            }

            Stmt::ReassignStmt { target, value, op } => {
                self.analyze_reassign(target, value, op)?;
            }

            Stmt::ExpressionStmt(expr) => {
                self.visit_expression(expr)?;
            }

            Stmt::CaseStmt { body, .. } => {
                self.enter_scope();
                for s in body {
                    self.visit_statement(s)?;
                }
                self.leave_scope();
            }

            Stmt::IfStmt { condition, then_block, else_block } => {
                let cond_type = self.visit_expression(condition)?;
                if cond_type != "bool" && cond_type != "unknown" {
                    return Err("Semantic Error: if condition must be a boolean".to_string());
                }
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
            }

            Stmt::ForInStmt { item, iterable, body } => {
                let iterable_type = self.visit_expression(iterable)?;
                let item_type = if iterable_type.starts_with("array<") {
                    iterable_type
                        .trim_start_matches("array<")
                        .trim_end_matches('>')
                        .to_string()
                } else if iterable_type == "string" {
                    "char".to_string()
                } else {
                    return Err(format!(
                        "Semantic Error: Expected array or string in for-in loop, got '{}'",
                        iterable_type
                    ));
                };

                self.enter_scope();

                if let Stmt::Declaration(Decl::VarDecl { type_node, .. }) = &**item {
                    let declared_type = type_node.as_str();
                    if !self.types_are_compatible(&declared_type, &item_type) {
                        return Err(format!(
                            "Semantic Error: Type mismatch in for-in loop. Iterable elements are '{}', but item is declared as '{}'",
                            item_type, declared_type
                        ));
                    }
                }

                self.visit_statement(item)?;

                match body {
                    EitherBlock::Inline(stmts) => {
                        for s in stmts {
                            self.visit_statement(s)?;
                        }
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

            Stmt::SwitchStmt { condition, cases, .. } => {
                self.visit_expression(condition)?;
                self.enter_scope();
                self.active_flags.push("+has_break".to_string());
                for s in cases {
                    self.visit_statement(s)?;
                }
                self.active_flags.retain(|f| f != "+has_break");
                self.leave_scope();
            }

            Stmt::DelStmt { target, .. } => {
                self.visit_expression(target)?;
            }

            Stmt::ForStmt { init, condition, increment, body } => {
                self.enter_scope();
                if let Some(i) = init {
                    self.visit_statement(i)?;
                }
                if let Some(c) = condition {
                    let cond_type = self.visit_expression(c)?;
                    if cond_type != "bool" && cond_type != "unknown" {
                        return Err(
                            "Semantic Error: for condition must be a boolean".to_string(),
                        );
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
                let actual_type = self.visit_expression(expr)?;
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
                if !self.active_flags.contains(&"+has_throw".to_string()) {
                    return Err(
                        "Semantic Error: Throw statement is not allowed here. 'has_throw' flag is not enabled.".to_string()
                    );
                }
                self.visit_expression(expr)?;
            }

            Stmt::TryCatchStmt { try_block, catch_param, catch_block } => {
                self.enter_scope();
                for s in try_block {
                    self.visit_statement(s)?;
                }
                self.leave_scope();

                self.enter_scope();
                let info = SymbolInfo {
                    name: catch_param.clone(),
                    kind: SymbolKind::Variable {
                        type_node: BaseType::Error,
                        editability: Editability::NotEditable,
                        is_array: false,
                    },
                    visibility: Visibility::Private,
                    dependencies: vec![],
                };
                self.current_env.borrow_mut().define(catch_param.clone(), info)?;
                for s in catch_block {
                    self.visit_statement(s)?;
                }
                self.leave_scope();
            }

            Stmt::GotoStmt(_) => {}
            _ => {}
        }
        Ok(())
    }

    // ----------------------------------------------------------
    // analyze_reassign - ReassignStmt with full operator awareness
    // ----------------------------------------------------------
    fn analyze_reassign(
        &mut self,
        target: &Expr,
        value: &Expr,
        op: &str,
    ) -> Result<(), String> {
        // Check mutability first
        if let Expr::Identifier(name) = target {
            if let Some(info) = self.current_env.borrow().lookup(name) {
                if !info.is_editable() {
                    return Err(format!(
                        "Semantic Error: Cannot reassign constant '{}'",
                        name
                    ));
                }
            }
        }

        let expr_type = self.visit_expression(value)?;
        let target_type = self.visit_expression(target)?;

        if op != "=" {
            return self.verify_operator_overload(&target_type, &expr_type, op, target);
        }

        self.verify_type_assignment(&target_type, &expr_type, target)
    }

    // ----------------------------------------------------------
    // verify_operator_overload
    // ----------------------------------------------------------
    fn verify_operator_overload(
        &mut self,
        target_type: &str,
        expr_type: &str,
        op: &str,
        _target: &Expr,
    ) -> Result<(), String> {
        // Compound operators on primitive numerics are always allowed
        // e.g. counter += 1; x -= 2;
        let is_compound =
            matches!(op, "+=" | "-=" | "*=" | "/=" | "%=");
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
                        Err(format!(
                            "Semantic Error: Handle '{}' in '{}' does not accept type '{}'. Check the handle's parameter type.",
                            handle.as_str(),
                            bp_name,
                            expr_type
                        ))
                    } else {
                        Ok(())
                    }
                }
                HandleLookupResult::BlueprintNotFound => Ok(()),
                HandleLookupResult::HandleMissing { handle } => Err(format!(
                    "Semantic Error: Type '{}' does not support the '{}' operator (missing handle '{}').",
                    bp_name,
                    op,
                    handle.as_str()
                )),
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
    fn verify_type_assignment(
        &mut self,
        target_type: &str,
        expr_type: &str,
        target: &Expr,
    ) -> Result<(), String> {
        if target_type.starts_with("name<") {
            let inner = target_type
                .trim_start_matches("name<")
                .trim_end_matches('>');

            if inner == "unknown" && expr_type != "unknown" {
                if let Expr::Identifier(name) = target {
                    let maybe_info = self.current_env.borrow().lookup(name);
                    if let Some(mut info) = maybe_info {
                        info.kind = SymbolKind::Variable {
                            type_node: BaseType::Name(Box::new(BaseType::from_str(
                                expr_type,
                            ))),
                            editability: Editability::Editable,
                            is_array: false,
                        };
                        self.current_env.borrow_mut().update(name, info);
                    }
                }
            } else if inner != expr_type
                && inner != "unknown"
                && expr_type != "unknown"
                && expr_type != "object"
            {
                return Err(format!(
                    "Semantic Error: Cannot reassign smart pointer 'name<{}>' to type '{}'",
                    inner, expr_type
                ));
            }
            return Ok(());
        }

        if target_type == expr_type
            || target_type == "unknown"
            || expr_type == "unknown"
            || expr_type == "auto"
        {
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

    // ----------------------------------------------------------
    // visit_declaration
    // ----------------------------------------------------------
    fn visit_declaration(&mut self, decl: &Decl) -> Result<(), String> {
        match decl {
            Decl::VarDecl {
                visibility,
                editability,
                type_node,
                name,
                value,
                assign_op,
            } => {
                self.analyze_var_decl(
                    visibility, editability, type_node, name, value, assign_op,
                )?;
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
                // Accept when:
                //   - types match directly
                //   - expr_type is unknown
                //   - expr_type is array<T> where T compatible with declared
                //   - char array initialized with string literal
                let array_inner = if expr_type.starts_with("array<") {
                    expr_type
                        .trim_start_matches("array<")
                        .trim_end_matches('>')
                        .to_string()
                } else {
                    expr_type.clone()
                };

                if expr_type != "unknown"
                    && !self.types_are_compatible(&declared_type, &expr_type)
                    && !self.types_are_compatible(&declared_type, &array_inner)
                    && !(declared_type == "char"
                        && (expr_type == "string" || expr_type == "str"))
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
                };
                self.current_env.borrow_mut().define(name.clone(), info)?;
            }

            Decl::CustomDecl {
                is_exported,
                name,
                settings,
                handles,
                params,
                flags,
                data,
                public_block,
                private_block,
                static_block,
                statements,
                handle_block,
                constructor,
                ..
            } => {
                self.analyze_custom_decl(
                    *is_exported,
                    name,
                    settings,
                    handles,
                    params,
                    flags,
                    data,
                    public_block,
                    private_block,
                    static_block,
                    statements,
                    handle_block,
                    constructor,
                )?;
            }

            Decl::ClassDecl {
                is_exported,
                name,
                handles,
                public_block,
                private_block,
                static_block,
                handle_block,
                constructor,
                ..
            } => {
                self.analyze_class_or_struct_decl(
                    *is_exported,
                    name,
                    handles,
                    public_block,
                    private_block,
                    static_block,
                    handle_block,
                    constructor,
                )?;
            }

            Decl::StructDecl {
                is_exported,
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
                    *is_exported,
                    name,
                    handles,
                    public_block,
                    private_block,
                    static_block,
                    handle_block,
                    constructor,
                )?;
            }

            Decl::EnumDecl {
                is_exported,
                name,
                handle_block,
                ..
            } => {
                let info = self.make_blueprint_symbol(name, *is_exported);
                self.current_env.borrow_mut().define(name.clone(), info)?;
                self.enter_scope();
                let prev = self.in_custom_scope;
                self.in_custom_scope = true;
                for d in handle_block {
                    self.visit_declaration(d)?;
                }
                self.leave_scope();
                self.in_custom_scope = prev;
            }

            Decl::BlockDecl { statements, .. } => {
                self.enter_scope();
                for s in statements {
                    self.visit_statement(s)?;
                }
                self.leave_scope();
            }

            Decl::FnDecl {
                is_exported,
                name,
                params,
                return_type,
                body,
            } => {
                let fn_info = SymbolInfo {
                    name: name.clone(),
                    kind: SymbolKind::Function {
                        params: params.clone(),
                        return_type: return_type.clone(),
                    },
                    visibility: if *is_exported {
                        Visibility::Public
                    } else {
                        Visibility::Private
                    },
                    dependencies: vec![],
                };
                self.current_env.borrow_mut().define(name.clone(), fn_info)?;

                let prev_flags = self.active_flags.clone();
                let prev_return = self.active_return_type.clone();
                self.active_flags.push("+has_return".to_string());
                self.active_flags.push("+has_throw".to_string());
                self.active_return_type = Some(return_type.clone());

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
            }

            Decl::LabelDecl { name, body } => {
                let info = SymbolInfo {
                    name: name.clone(),
                    kind: SymbolKind::Label,
                    visibility: Visibility::Private,
                    dependencies: vec![],
                };
                self.current_env.borrow_mut().define(name.clone(), info)?;
                self.enter_scope();
                for s in body {
                    self.visit_statement(s)?;
                }
                self.leave_scope();
            }

            Decl::BlueprintDecl { name, .. } => {
                let info = self.make_blueprint_symbol(name, true);
                self.current_env.borrow_mut().define(name.clone(), info)?;
            }

            Decl::ImplDecl { target, methods } => {
                let exists = self.current_env.borrow().lookup(target).is_some();
                if !exists {
                    return Err(format!(
                        "Semantic Error: Blueprint '{}' not found for impl block",
                        target
                    ));
                }
                self.enter_scope();
                self.active_flags.push("+has_return".to_string());
                let prev = self.in_struct;
                self.in_struct = true;
                for m in methods {
                    self.visit_declaration(m)?;
                }
                self.in_struct = prev;
                self.active_flags.retain(|x| x != "+has_return");
                self.leave_scope();
            }

            Decl::NameDecl {
                name,
                inner_type,
                target,
                ..
            } => {
                let prev = self.current_context.clone();
                self.current_context = Some(name.clone());
                let target_type = self.visit_expression(target)?;

                let final_inner =
                    if *inner_type == BaseType::Unknown && target_type != "unknown" {
                        BaseType::from_str(&target_type)
                    } else {
                        inner_type.clone()
                    };

                let info = SymbolInfo {
                    name: name.clone(),
                    kind: SymbolKind::Variable {
                        type_node: BaseType::Name(Box::new(final_inner)),
                        editability: Editability::Editable,
                        is_array: false,
                    },
                    visibility: Visibility::Private,
                    dependencies: vec![],
                };
                self.current_env.borrow_mut().define(name.clone(), info)?;
                self.current_context = prev;
            }

            Decl::PointerDecl {
                name,
                inner_type,
                length,
                value,
            } => {
                let prev = self.current_context.clone();
                self.current_context = Some(name.clone());
                self.visit_expression(value)?;
                if let Some(l) = length {
                    self.visit_expression(l)?;
                }
                let info = SymbolInfo {
                    name: name.clone(),
                    kind: SymbolKind::Variable {
                        type_node: BaseType::Pointer(Box::new(inner_type.clone())),
                        editability: Editability::Editable,
                        is_array: length.is_some(),
                    },
                    visibility: Visibility::Private,
                    dependencies: vec![],
                };
                self.current_env.borrow_mut().define(name.clone(), info)?;
                self.current_context = prev;
            }

            Decl::Import { .. } => {}
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
    fn analyze_var_decl(
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

        let expr_type = self.visit_expression(value)?;
        let declared_type = type_node.as_str();

        if expr_type != "unknown" {
            let base_decl = declared_type.split('<').next().unwrap_or(&declared_type);

            match base_decl {
                // smart pointer - accept anything
                "name" => {}

                // complex types with possible handle overloading
                "custom" | "class" | "struct" | "enum" => {
                    let bp_name = extract_blueprint_name_from_type(&declared_type)
                        .unwrap_or_else(|| declared_type.clone());

                    // Direct compatibility: custom<X> = custom<X> or custom<X> = X
                    let directly_compatible = self
                        .types_are_compatible(&declared_type, &expr_type)
                        || expr_type == bp_name
                        || expr_type == format!("custom<{}>", bp_name);

                    // If types are directly compatible, always accept regardless of op
                    // e.g. MathScope a -> new MathScope() is fine even with ->
                    if directly_compatible {
                        // accepted
                    } else if assign_op == "=" {
                        return Err(format!(
                            "Semantic Error: Type mismatch for '{}'. Declared '{}', got '{}'",
                            name, declared_type, expr_type
                        ));
                    } else {
                        // operator overloading via handle
                        match resolve_handle_for_op(
                            &self.current_env,
                            &bp_name,
                            assign_op,
                        ) {
                            HandleLookupResult::Found(bp) => {
                                let handle = op_to_handle(assign_op);
                                if !bp.handle_accepts_type(handle, &expr_type) {
                                    return Err(format!(
                                        "Semantic Error: Handle '{}' in '{}' does not accept type '{}'. Expected a compatible type for the '{}' operator.",
                                        handle.as_str(),
                                        bp_name,
                                        expr_type,
                                        assign_op
                                    ));
                                }
                                // OK - handle overloading accepts this
                            }
                            HandleLookupResult::BlueprintNotFound => {
                                // Not in env yet (generic or not-yet-defined) - allow
                            }
                            HandleLookupResult::HandleMissing { handle } => {
                                return Err(format!(
                                    "Semantic Error: Type '{}' does not support the '{}' operator. Handle '{}' is not defined in its handle block.",
                                    bp_name,
                                    assign_op,
                                    handle.as_str()
                                ));
                            }
                            HandleLookupResult::UnknownOp => {
                                // Unknown operator - allow
                            }
                        }
                    }
                }

                // bare type name (e.g. "MathScope" instead of "custom<MathScope>")
                // could be a user-defined type - look up in blueprints
                name_key if self
                    .current_env
                    .borrow()
                    .lookup_blueprint(name_key)
                    .is_some() =>
                {
                    // It is a known blueprint type used without "custom<>" prefix
                    if assign_op != "=" {
                        match resolve_handle_for_op(
                            &self.current_env,
                            name_key,
                            assign_op,
                        ) {
                            HandleLookupResult::Found(bp) => {
                                let handle = op_to_handle(assign_op);
                                if !bp.handle_accepts_type(handle, &expr_type) {
                                    return Err(format!(
                                        "Semantic Error: Handle '{}' in '{}' does not accept type '{}'.",
                                        handle.as_str(), name_key, expr_type
                                    ));
                                }
                            }
                            HandleLookupResult::BlueprintNotFound => {}
                            HandleLookupResult::HandleMissing { handle } => {
                                return Err(format!(
                                    "Semantic Error: Type '{}' does not support '{}' (missing handle '{}').",
                                    name_key, assign_op, handle.as_str()
                                ));
                            }
                            HandleLookupResult::UnknownOp => {}
                        }
                    }
                    // For "=", just accept - they're using the type by its raw name
                }

                // simple primitive types
                _ => {
                    // Allow -> operator for any type (it's used as "default init" syntax)
                    if assign_op != "->"
                        && !self.types_are_compatible(&declared_type, &expr_type)
                    {
                        return Err(format!(
                            "Semantic Error: Type mismatch for '{}'. Declared '{}', got '{}'",
                            name, declared_type, expr_type
                        ));
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

        let info = SymbolInfo {
            name: name.to_string(),
            kind: SymbolKind::Variable {
                type_node: type_node.clone(),
                editability: editability.clone(),
                is_array: false,
            },
            visibility: visibility.clone(),
            dependencies: deps,
        };
        self.current_env.borrow_mut().define(name.to_string(), info)?;
        self.current_context = prev_context;
        Ok(())
    }

    // ----------------------------------------------------------
    // analyze_custom_decl
    // ----------------------------------------------------------
    #[allow(clippy::too_many_arguments)]
    fn analyze_custom_decl(
        &mut self,
        is_exported: bool,
        name: &str,
        settings: &Option<Vec<Setting>>,
        handles: &Option<Vec<HandleMethods>>,
        params: &Option<Vec<Param>>,
        flags: &Option<Vec<Flag>>,
        data: &Option<Expr>,
        public_block: &Option<Vec<Decl>>,
        private_block: &Option<Vec<Decl>>,
        static_block: &Option<Vec<Decl>>,
        statements: &Option<Vec<Stmt>>,
        handle_block: &Option<Vec<Decl>>,
        constructor: &Option<Vec<ConstructorDecl>>,
    ) -> Result<(), String> {
        // Validate settings combinations
        if let Some(s) = settings {
            let has_public = s.contains(&Setting::Public);
            let has_private = s.contains(&Setting::Private);
            let has_stmt = s.contains(&Setting::Statement);
            let has_label = s.contains(&Setting::Label);

            if has_label && (has_public || has_private || has_stmt) {
                return Err(format!(
                    "Semantic Error: Custom block '{}' is marked as 'label' and cannot be combined with other modifiers.",
                    name
                ));
            }
            if has_stmt && (has_public || has_private) {
                return Err(format!(
                    "Semantic Error: Custom block '{}' is marked as 'statement' and cannot have 'public' or 'private' modifiers.",
                    name
                ));
            }
        }

        // Build BlueprintData for this custom scope
        let mut bp = BlueprintData::new(name);
        if let Some(h_vec) = handles {
            for h in h_vec {
                bp.handles.insert(*h);
            }
        }
        if let Some(p_vec) = params {
            bp.params = p_vec.clone();
        }
        // Register handle method signatures
        if let Some(hb) = handle_block {
            for d in hb {
                if let Decl::FnDecl {
                    name: fn_name,
                    params: fn_params,
                    return_type,
                    ..
                } = d
                {
                    bp.methods.insert(
                        fn_name.clone(),
                        FnSignature {
                            name: fn_name.clone(),
                            params: fn_params.clone(),
                            return_type: return_type.clone(),
                        },
                    );
                }
            }
        }

        self.current_env.borrow_mut().define_blueprint(name.to_string(), bp);

        let info = self.make_blueprint_symbol(name, is_exported);
        self.current_env.borrow_mut().define(name.to_string(), info)?;

        let prev_in_stmt = self.in_statement_scope;
        self.in_statement_scope = false;
        let prev_return = self.active_return_type.clone();
        let prev_flags = self.active_flags.clone();

        self.active_return_type = None;

        if let Some(f_vec) = flags {
            for flg in f_vec {
                self.active_flags.push(format!("+{}", flg.as_str()));
            }
        }
        if let Some(h_vec) = handles {
            if h_vec.contains(&HandleMethods::Error) {
                self.active_flags.push("+has_throw".to_string());
                self.active_flags.push("+has_error".to_string());
            }
        }

        self.enter_scope();
        let prev_in_custom = self.in_custom_scope;
        self.in_custom_scope = true;

        if data.is_some() {
            let data_info = SymbolInfo {
                name: "data".to_string(),
                kind: SymbolKind::Variable {
                    type_node: BaseType::Unknown,
                    editability: Editability::Editable,
                    is_array: false,
                },
                visibility: Visibility::Public,
                dependencies: vec![],
            };
            self.current_env
                .borrow_mut()
                .define("data".to_string(), data_info)?;
        }

        if let Some(p_vec) = params {
            for p in p_vec {
                let param_info = SymbolInfo {
                    name: p.name.clone(),
                    kind: SymbolKind::Variable {
                        type_node: p.type_node.clone(),
                        editability: Editability::Editable,
                        is_array: false,
                    },
                    visibility: Visibility::Public,
                    dependencies: vec![],
                };
                self.current_env
                    .borrow_mut()
                    .define(p.name.clone(), param_info)?;
            }
        }

        if let Some(stmts) = statements {
            for s in stmts {
                self.visit_statement(s)?;
            }
        }

        for block in [private_block, public_block, static_block] {
            if let Some(decls) = block {
                for d in decls {
                    self.visit_declaration(d)?;
                }
            }
        }

        if let Some(constructors) = constructor {
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
                    };
                    self.current_env.borrow_mut().define(param.name.clone(), pi)?;
                }
                let prev_stmt = self.in_statement_scope;
                self.in_statement_scope = true;
                for s in &ctor.body {
                    self.visit_statement(s)?;
                }
                self.in_statement_scope = prev_stmt;
                self.leave_scope();
            }
        }

        if let Some(hb) = handle_block {
            for d in hb {
                if let Decl::FnDecl { name: fn_name, .. } = d {
                    if handles.is_some()
                        && !handles
                            .as_ref()
                            .unwrap()
                            .contains(&HandleMethods::from_str(fn_name.as_str()))
                    {
                        return Err(format!(
                            "Semantic Error: Invalid handle function name '{}'. It is not listed in the handles declaration for '{}'.",
                            fn_name, name
                        ));
                    }
                } else {
                    return Err(
                        "Semantic Error: Only function declarations (fn) are allowed inside a handle block.".to_string(),
                    );
                }
                self.visit_declaration(d)?;
            }
        }

        self.leave_scope();
        self.active_flags = prev_flags;
        self.active_return_type = prev_return;
        self.in_statement_scope = prev_in_stmt;
        self.in_custom_scope = prev_in_custom;

        Ok(())
    }

    // ----------------------------------------------------------
    // analyze_class_or_struct_decl - shared for Class and Struct
    // ----------------------------------------------------------
    #[allow(clippy::too_many_arguments)]
    fn analyze_class_or_struct_decl(
        &mut self,
        is_exported: bool,
        name: &str,
        handles: &[HandleMethods],
        public_block: &[Decl],
        private_block: &[Decl],
        static_block: &[Decl],
        handle_block: &[Decl],
        constructor: &Option<Vec<ConstructorDecl>>,
    ) -> Result<(), String> {
        let mut bp = BlueprintData::new(name);
        for h in handles {
            bp.handles.insert(*h);
        }
        for d in handle_block {
            if let Decl::FnDecl {
                name: fn_name,
                params,
                return_type,
                ..
            } = d
            {
                bp.methods.insert(
                    fn_name.clone(),
                    FnSignature {
                        name: fn_name.clone(),
                        params: params.clone(),
                        return_type: return_type.clone(),
                    },
                );
            }
        }
        self.current_env
            .borrow_mut()
            .define_blueprint(name.to_string(), bp);

        let info = self.make_blueprint_symbol(name, is_exported);
        self.current_env.borrow_mut().define(name.to_string(), info)?;

        self.enter_scope();
        let prev = self.in_custom_scope;
        self.in_custom_scope = true;

        for d in private_block
            .iter()
            .chain(public_block)
            .chain(static_block)
            .chain(handle_block)
        {
            self.visit_declaration(d)?;
        }

        if let Some(constructors) = constructor {
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
        }

        self.leave_scope();
        self.in_custom_scope = prev;
        Ok(())
    }

    // ----------------------------------------------------------
    // visit_expression - returns type string
    // ----------------------------------------------------------
    fn visit_expression(&mut self, expr: &Expr) -> Result<String, String> {
        match expr {
            Expr::LiteralInt(_) => Ok("int".to_string()),
            Expr::LiteralFloat(_) => Ok("float".to_string()),
            Expr::LiteralString(_) => Ok("str".to_string()),
            Expr::LiteralChar(_) => Ok("char".to_string()),
            Expr::LiteralBool(_) => Ok("bool".to_string()),

            Expr::ArrayLiteral(elements) => {
                if elements.is_empty() {
                    return Ok("array<unknown>".to_string());
                }
                let mut element_type: Option<String> = None;
                for el in elements {
                    let inferred = self.visit_expression(el)?;
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

            Expr::ObjectLiteral(stmts) => {
                self.enter_scope();
                for s in stmts {
                    self.visit_statement(s)?;
                }
                self.leave_scope();
                Ok("object".to_string())
            }

            Expr::Identifier(name) => {
                if matches!(name.trim(), "__default__" | "None" | "null") {
                    return Ok("unknown".to_string());
                }
                self.record_dependency(name.clone());
                match self.current_env.borrow().lookup(name) {
                    Some(info) => Ok(info.type_str()),
                    None => {
                        if self.in_struct || self.in_class || self.in_custom_scope {
                            return Ok("auto".to_string());
                        }
                        Err(format!(
                            "Semantic Error: Identifier '{}' is not defined in this scope.",
                            name
                        ))
                    }
                }
            }

            Expr::NamespaceAccess { namespace, property } => {
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

            Expr::Call { callee, args } => {
                if let Expr::Identifier(name) = &**callee {
                    if let Some(info) = self.current_env.borrow().lookup(name) {
                        if let SymbolKind::Variable { type_node, .. } = &info.kind {
                            let base_str = type_node.as_str();
                            if base_str == "type" || base_str.starts_with("type<") {
                                return Ok(name.clone());
                            }
                        }
                    }
                    if name == "error" {
                        for arg in args {
                            self.visit_expression(arg)?;
                        }
                        return Ok("error".to_string());
                    }
                    if name != "Some" && self.current_env.borrow().lookup(name).is_none() {
                        return Err(format!(
                            "Semantic Error: Function '{}' is not defined in this scope.",
                            name
                        ));
                    }
                }
                for arg in args {
                    self.visit_expression(arg)?;
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
                    return Err(
                        "Semantic Error: Cannot use 'super' outside of a class".to_string()
                    );
                }
                Ok("object".to_string())
            }

            Expr::Modify { target } | Expr::Copy { target } => {
                self.visit_expression(target)
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

            Expr::BinaryOp { left, operator, right } => {
                let left_type = self.visit_expression(left)?;
                let _right_type = self.visit_expression(right)?;
                match operator.as_str() {
                    "==" | "!=" | ">" | "<" | ">=" | "<=" | "&&" | "||" => {
                        Ok("bool".to_string())
                    }
                    _ => {
                        if left_type != "unknown" {
                            Ok(left_type)
                        } else {
                            Ok(_right_type)
                        }
                    }
                }
            }

            Expr::UnaryOp { operand, .. } => self.visit_expression(operand),

            Expr::PrefixUpdate { right, .. } => self.visit_expression(right),
            Expr::PostfixUpdate { left, .. } => self.visit_expression(left),

            Expr::IndexAccess { object, index } => {
                self.visit_expression(index)?;
                let obj_type = self.visit_expression(object)?;
                if obj_type.starts_with("array<") {
                    let inner = obj_type
                        .trim_start_matches("array<")
                        .trim_end_matches('>');
                    Ok(inner.to_string())
                } else {
                    Ok("unknown".to_string())
                }
            }

            Expr::PropertyAccess { object, property } => {
                let obj_type = self.visit_expression(object)?;

                // Try to find in blueprint
                if let Some(bp_name) = extract_blueprint_name_from_type(&obj_type) {
                    if let Some(bp) =
                        self.current_env.borrow().lookup_blueprint(&bp_name)
                    {
                        if let Some(field_type) = bp.fields.get(property) {
                            return Ok(field_type.as_str());
                        }
                        if let Some(sig) = bp.methods.get(property) {
                            return Ok(sig.return_type.as_str());
                        }
                    }
                    // fallback to global_metadata
                    if let Some(meta) = self.global_metadata.get(&bp_name) {
                        if let Some(field_type) = meta.fields.get(property) {
                            return Ok(field_type.as_str());
                        }
                        if let Some(fn_type) = meta.methods.get(property) {
                            return Ok(fn_type.return_type.as_str());
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
                    return Ok(format!("custom<{}>", n));
                }
                Ok("unknown".to_string())
            }

            Expr::New { type_node, target } => {
                self.visit_expression(target)?;
                Ok(type_node.as_str())
            }

            Expr::TypeOf { target } => {
                self.visit_expression(target)?;
                Ok("type".to_string())
            }

            Expr::SizeOf { target } => {
                self.visit_expression(target)?;
                Ok("int".to_string())
            }

            Expr::ToString { target } => {
                self.visit_expression(target)?;
                Ok("str".to_string())
            }

            _ => Ok("unknown".to_string()),
        }
    }

    // ----------------------------------------------------------
    // types_are_compatible
    // ----------------------------------------------------------
    fn types_are_compatible(&self, expected: &str, actual: &str) -> bool {
        if expected == actual
            || expected == "any"
            || actual == "unknown"
            || actual == "auto"
        {
            return true;
        }
        // int family
        if expected.starts_with("int") && actual.starts_with("int") {
            return true;
        }
        if (expected == "int" || expected == "int32") && actual == "int" {
            return true;
        }
        // float family
        if expected.starts_with("float") && actual.starts_with("float") {
            return true;
        }
        // string
        if (expected == "string" || expected == "custom<string>")
            && (actual == "str" || actual == "string")
        {
            return true;
        }
        if expected == "str" && (actual == "string" || actual == "str") {
            return true;
        }
        // custom<X> compatible with X
        if expected.starts_with("custom<") {
            let inner = expected
                .trim_start_matches("custom<")
                .trim_end_matches('>');
            if inner == actual {
                return true;
            }
        }
        if expected.starts_with("object<") {
            let inner = expected
                .trim_start_matches("object<")
                .trim_end_matches('>');
            if inner == actual {
                return true;
            }
        }
        // name pointer
        if expected == "name" || expected == "name<unknown>" {
            return true;
        }
        if expected.starts_with("name<") {
            let inner = expected
                .trim_start_matches("name<")
                .trim_end_matches('>');
            return inner == actual || inner == "unknown";
        }
        false
    }

    // ----------------------------------------------------------
    // Helpers
    // ----------------------------------------------------------

    fn is_primitive_numeric(t: &str) -> bool {
        matches!(
            t,
            "int"
                | "int8"
                | "int16"
                | "int32"
                | "int64"
                | "int128"
                | "float"
                | "float32"
                | "float64"
        )
    }

    fn make_blueprint_symbol(&self, name: &str, is_exported: bool) -> SymbolInfo {
        SymbolInfo {
            name: name.to_string(),
            kind: SymbolKind::Blueprint,
            visibility: if is_exported {
                Visibility::Public
            } else {
                Visibility::Private
            },
            dependencies: vec![],
        }
    }
}
