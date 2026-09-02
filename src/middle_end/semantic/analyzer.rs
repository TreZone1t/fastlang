use crate::frontend::parser::ast::*;
use crate::middle_end::semantic::environment::{
    BlueprintData,
    Environment,
    FnSignature,
    SymbolInfo,
    SymbolKind,
};
use crate::middle_end::semantic::handle_resolver::{
    build_blueprint_from_metadata,
    extract_all_type_names,
    extract_blueprint_name_from_type,
    is_complex_type,
    op_to_handle,
    resolve_handle_for_op,
    HandleLookupResult,
};
use std::cell::RefCell;
use std::collections::{ HashMap, HashSet };
use std::rc::Rc;
// use crate::middle_end::interpreter::eval::Interpreter;

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
    pub current_file: String,
    pub source_code: Option<String>,
    pub fn_overloads: HashMap<String, Vec<FnSignature>>,
    pub heap_allocated_vars: HashSet<String>,
    pub deleted_vars: HashSet<String>,
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
            current_file: String::new(),
            source_code: None,
            fn_overloads: HashMap::new(),
            heap_allocated_vars: HashSet::new(),
            deleted_vars: HashSet::new(),
        };
        analyzer.import_metadata();
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

    fn is_primitive_stack_type(&self, type_name: &str) -> bool {
        matches!(
            type_name,
            "int8" | "int16" | "int32" | "int64" |
            "uint8" | "uint16" | "uint32" | "uint64" |
            "usize" | "isize" | "float32" | "float64" |
            "char" | "bool" | "str" | "void"
        )
    }

    fn type_has_drop_handle(&self, type_name: &str) -> bool {
        let clean_name = extract_blueprint_name_from_type(type_name).unwrap_or_else(|| type_name.to_string());
        if let Some(bp) = self.current_env.borrow().lookup_blueprint(&clean_name) {
            if bp.handles.contains(&HandleMethods::Drop) || bp.methods.contains_key("drop") {
                return true;
            }
        }
        if let Some(meta) = self.global_metadata.get(&clean_name) {
            if meta.handles.contains(&HandleMethods::Drop) || meta.methods.contains_key("drop") {
                return true;
            }
        }
        false
    }

    fn is_class_type(&self, type_name: &str) -> bool {
        let clean_name = extract_blueprint_name_from_type(type_name).unwrap_or_else(|| type_name.to_string());
        if let Some(meta) = self.global_metadata.get(&clean_name) {
            if let BaseType::Class { .. } = &meta.ty {
                return true;
            }
        }
        false
    }

    fn is_struct_or_blueprint_type(&self, type_name: &str) -> bool {
        let clean_name = extract_blueprint_name_from_type(type_name).unwrap_or_else(|| type_name.to_string());
        if let Some(meta) = self.global_metadata.get(&clean_name) {
            if matches!(&meta.ty, BaseType::Struct { .. } | BaseType::Blueprint { .. }) {
                return true;
            }
        }
        if self.current_env.borrow().lookup_blueprint(&clean_name).is_some() {
            return true;
        }
        false
    }

    // ----------------------------------------------------------
    // Scope management
    // ----------------------------------------------------------
    fn enter_scope(&mut self) {
        let new_env = Environment::with_parent(Rc::clone(&self.current_env));
        self.current_env = new_env;
    }

    pub fn emit_warning(
        &self,
        message: &str,
        line: Option<usize>,
        column: Option<usize>,
        hint: Option<&str>
    ) {
        eprintln!("\x1b[33;1mwarning:\x1b[0m {}", message);
        if let (Some(l), Some(c)) = (line, column) {
            let file_display = if self.current_file.is_empty() {
                "<input>"
            } else {
                &self.current_file
            };
            eprintln!("  --> {}:{}:{}", file_display, l, c);
            if let Some(ref src) = self.source_code {
                if let Some(line_str) = src.lines().nth(l.saturating_sub(1)) {
                    let pad = " ".repeat(format!("{}", l).len());
                    eprintln!("   {} |", pad);
                    eprintln!("{} | {}", l, line_str);
                    let caret_pad = " ".repeat(c.saturating_sub(1));
                    eprintln!("   {} | {}^{}", pad, caret_pad, "~".repeat(4));
                }
            }
        }
        if let Some(h) = hint {
            eprintln!("  = \x1b[36;1mhelp:\x1b[0m {}", h);
        }
    }

    fn leave_scope(&mut self) {
        if !self.in_class && !self.in_struct && !self.in_custom_scope {
            for (name, info) in &self.current_env.borrow().symbols {
                if
                    !info.is_used &&
                    !name.starts_with('_') &&
                    name != "this" &&
                    name != "data" &&
                    name != "__this__"
                {
                    if let SymbolKind::Variable { .. } = &info.kind {
                        let kind_str = if info.is_param { "parameter" } else { "variable" };
                        let hint =
                            format!("if this is intentional, prefix with an underscore: '_{}'", name);
                        self.emit_warning(
                            &format!("{} '{}' is declared but never used in scope", kind_str, name),
                            None,
                            None,
                            Some(&hint)
                        );
                    }
                }
            }
        }
        let parent = self.current_env.borrow().parent.clone().expect("Cannot leave global scope");
        self.current_env = parent;
    }

    pub fn analyze(&mut self, ast: &Vec<Stmt>) -> Result<(), String> {
        for stmt in ast {
            self.visit_statement(stmt)?;
        }
        Ok(())
    }

    pub fn record_dependency(&mut self, dep: String) {
        let ctx = self.current_context.clone().unwrap_or_else(|| "main".to_string());
        self.dependency_graph.entry(ctx).or_insert_with(HashSet::new).insert(dep);
    }

    pub fn get_reachable_symbols(&self) -> HashSet<String> {
        let mut reachable = HashSet::new();
        let mut queue = Vec::new();

        // Always start from main or top-level context
        queue.push("main".to_string());
        reachable.insert("main".to_string());

        for (name, info) in &self.current_env.borrow().symbols {
            if info.is_used {
                if reachable.insert(name.clone()) {
                    queue.push(name.clone());
                }
            }
        }

        while let Some(curr) = queue.pop() {
            if let Some(deps) = self.dependency_graph.get(&curr) {
                for dep in deps {
                    if reachable.insert(dep.clone()) {
                        queue.push(dep.clone());
                    }
                }
            }
        }

        reachable
    }
    fn is_valid_pointer_rhs(&self, value: &Expr, expr_type: &str) -> bool {
        if matches!(value, Expr::Default(_)) || expr_type == "default" || expr_type == "unknown" {
            return true;
        }
        if let Expr::UnaryOp { operator, .. } = value {
            if operator == "&" {
                return true;
            }
        }
        if matches!(value, Expr::New { .. }) {
            return true;
        }

        if
            expr_type.starts_with("name<") ||
            expr_type.starts_with("modify<") ||
            expr_type.starts_with("copy<") ||
            expr_type.starts_with("pointer<") ||
            expr_type.starts_with("scope") ||
            expr_type.starts_with("Fn<") ||
            expr_type.starts_with("method") ||
            expr_type == "fn"
        {
            return true;
        }

        false
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

            Stmt::ExpressionStmt(expr) | Stmt::CallStmt(expr) => {
                self.visit_expression(expr)?;
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
                            let _ = self.current_env
                                .borrow_mut()
                                .define_or_update(var_name.clone(), info);
                        }
                    }
                } else if let Expr::Instantiate { args, .. } = option {
                    if args.len() == 1 {
                        if let Expr::ObjectLiteral(stmts) = &args[0] {
                            for s in stmts {
                                let var_name = match s {
                                    Stmt::Declaration(Decl::VarDecl { name, .. }) => Some(name.clone()),
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
                                    let _ = self.current_env
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
                let (base_name, type_args) = extract_type_args_from_str(&iterable_type);
                let item_type = if
                    iterable_type.starts_with("array<") ||
                    iterable_type.ends_with("[]")
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
                } else if
                    let Some(meta) = self.global_metadata
                        .get(&base_name)
                        .or_else(|| self.global_metadata.get(&iterable_type))
                {
                    if let Some(next_fn) = meta.methods.get("next") {
                        let mut map = HashMap::new();
                        let generics = match &meta.ty {
                            | BaseType::Struct { generics, .. }
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

                if let Stmt::Declaration(Decl::VarDecl { type_node, name, .. }) = &**item {
                    let declared_type = type_node.as_str();
                    if
                        item_type != "unknown" &&
                        !self.types_are_compatible(&declared_type, &item_type)
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
                    self.current_env.borrow_mut().define(var_name.clone(), info)?;
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

            Stmt::DelStmt { target, is_array } => {
                if let Expr::Identifier(name) = target {
                    let var_info = if let Some(info) = self.current_env.borrow().lookup(name) {
                        info
                    } else {
                        return Err(
                            format!("Semantic Error: Cannot delete non-existent variable '{}'", name)
                        );
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
                        return Err(format!("Semantic Error: Variable '{}' has already been deleted.", name));
                    }

                    let type_str = var_info.type_str();
                    let is_pointer_or_ref = type_str.ends_with('*') ||
                        type_str.starts_with("pointer<") ||
                        type_str.starts_with("modify<") ||
                        type_str.starts_with("name<") ||
                        type_str == "name" ||
                        type_str.starts_with("array<") ||
                        type_str.ends_with("[]") ||
                        *is_array;

                    let is_on_heap = self.heap_allocated_vars.contains(name) ||
                        is_pointer_or_ref ||
                        self.is_class_type(&type_str);

                    // Stage 2: Non-heap primitive check
                    if self.is_primitive_stack_type(&type_str) && !is_pointer_or_ref && !is_on_heap {
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

            Stmt::ForStmt { init, condition, increment, body } => {
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
                    if
                        actual_type != "unknown" &&
                        expected_str != "unknown" &&
                        !self.types_are_compatible(&expected_str, &actual_type)
                    {
                        return Err(
                            format!(
                                "Semantic Error: Return type mismatch. Expected '{}', got '{}'",
                                expected_str,
                                actual_type
                            )
                        );
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
                    let bp_name = extract_blueprint_name_from_type(&thrown_type).unwrap_or_else(||
                        thrown_type.clone()
                    );
                    let is_throwable = if bp_name == "Error" || bp_name == "std::Error" {
                        true
                    } else if let Some(bp) = self.current_env.borrow().lookup_blueprint(&bp_name) {
                        bp.handles.contains(&HandleMethods::Throw) ||
                            bp.methods.contains_key("throw") ||
                            bp.name == "Error"
                    } else if let Some(meta) = self.global_metadata.get(&bp_name) {
                        meta.handles.contains(&HandleMethods::Throw) ||
                            meta.methods.contains_key("throw") ||
                            meta.name == "Error"
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

            Stmt::TryCatchStmt { try_block, catch_param, catch_block } => {
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
                self.current_env.borrow_mut().define(catch_param.clone(), info)?;
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
                            let _ = self.current_env
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
                            let _ = self.current_env
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
    fn analyze_reassign(&mut self, target: &Expr, value: &Expr, op: &str) -> Result<(), String> {
        // Scope flags are strictly read-only
        if let Expr::Identifier(name) = target {
            if
                matches!(
                    name.trim(),
                    "broken" |
                        "is_done" |
                        "yielded" |
                        "returned" |
                        "leaved" |
                        "continued" |
                        "has_break" |
                        "has_yield" |
                        "has_leave" |
                        "has_return" |
                        "has_call" |
                        "has_error" |
                        "compilable" |
                        "is_compilable" |
                        "printable" |
                        "is_printable" |
                        "throwable" |
                        "is_throwable"
                )
            {
                return Err(
                    format!("Semantic Error: Scope flag '{}' is read-only and cannot be manually modified", name)
                );
            }
        }
        // Check mutability and pointer lifecycle
        let mut is_modify_target = false;
        if let Expr::Identifier(name) = target {
            if let Some(info) = self.current_env.borrow().lookup(name) {
                if !info.is_editable() {
                    return Err(format!("Semantic Error: Cannot reassign constant '{}'", name));
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
            if
                expr_type.starts_with("modify<") ||
                expr_type.starts_with("name<") ||
                expr_type.starts_with("pointer") ||
                is_addr
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
    fn verify_operator_overload(
        &mut self,
        target_type: &str,
        expr_type: &str,
        op: &str,
        _target: &Expr
    ) -> Result<(), String> {
        // Compound operators on primitive numerics are always allowed
        // e.g. counter += 1; x -= 2;
        let is_compound = matches!(op, "+=" | "-=" | "*=" | "/=" | "%=");
        if is_compound && Self::is_primitive_numeric(target_type) {
            return Ok(());
        }

        // Complex types: look up handle
        if is_complex_type(target_type) {
            let bp_name = extract_blueprint_name_from_type(target_type).unwrap_or_else(||
                target_type.to_string()
            );

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
    fn verify_type_assignment(
        &mut self,
        target_type: &str,
        expr_type: &str,
        target: &Expr
    ) -> Result<(), String> {
        if expr_type == "default" || expr_type == "unknown" {
            return Ok(());
        }

        // ----------------------------------------------------------
        // Smart Pointers: name, modify, copy
        // ----------------------------------------------------------
        if
            target_type.starts_with("name<") ||
            target_type.starts_with("modify<name<") ||
            target_type.starts_with("copy<name<")
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
            } else if
                !self.types_are_compatible(inner, expr_inner) &&
                inner != "unknown" &&
                expr_type != "unknown" &&
                expr_type != "object"
            {
                let prefix = if is_modify { "modify" } else if is_copy { "copy" } else { "name" };
                return Err(
                    format!(
                        "Semantic Error: Cannot reassign smart pointer '{}<{}>' to type '{}'",
                        prefix,
                        inner,
                        expr_type
                    )
                );
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

            if
                !self.types_are_compatible(inner, expr_inner) &&
                inner != "unknown" &&
                expr_type != "unknown" &&
                expr_type != "object"
            {
                return Err(
                    format!(
                        "Semantic Error: Cannot reassign pointer 'pointer<{}>' to type '{}'",
                        inner,
                        expr_type
                    )
                );
            }
            return Ok(());
        }

        if
            target_type == expr_type ||
            target_type == "unknown" ||
            expr_type == "unknown"
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
            return Err(
                format!("Semantic Error: Cannot assign '{}' to type '{}'", expr_type, target_type)
            );
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
                        assign_op
                    )?;
                }
            }

            Decl::ObjectDestructureDecl { visibility, editability, type_name, fields, rhs, .. } => {
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
                                let params = meta.constructor
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

            Decl::ArrayDecl { visibility, editability, type_node, name, length, value, .. } => {
                let expr_type = self.visit_expression(value)?;
                self.visit_expression(length)?;

                let declared_type = type_node.as_str();
                let array_inner = if expr_type.starts_with("array<") {
                    expr_type.trim_start_matches("array<").trim_end_matches('>').to_string()
                } else {
                    expr_type.clone()
                };
                let array_inner_base = array_inner.split('[').next().unwrap_or(&array_inner);

                if
                    expr_type != "unknown" &&
                    expr_type != "default" &&
                    !self.types_are_compatible(&declared_type, &expr_type) &&
                    !self.types_are_compatible(&declared_type, &array_inner) &&
                    !self.types_are_compatible(&declared_type, array_inner_base) &&
                    !(
                        declared_type.contains("char") &&
                        expr_type == "str"
                    )
                {
                    return Err(
                        format!(
                            "Semantic Error: Type mismatch for array '{}'. Declared '{}', got '{}'",
                            name,
                            declared_type,
                            expr_type
                        )
                    );
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
                    constructor
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
                    constructor
                )?;
            }

            Decl::EnumDecl { visibility, name, handle_block, variants, .. } => {
                let info = self.make_blueprint_symbol(name, visibility.clone());
                self.current_env.borrow_mut().define(name.clone(), info)?;
                for v in variants {
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
                    let _ = self.current_env.borrow_mut().define(v.name.clone(), var_symbol);
                }
                self.enter_scope();
                let prev = self.in_custom_scope;
                self.in_custom_scope = true;
                for d in handle_block {
                    self.visit_declaration(d)?;
                }
                self.leave_scope();
                self.in_custom_scope = prev;
            }

            Decl::BlockDecl { visibility, name, return_type, statements } => {
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
                    if let Stmt::Declaration(Decl::VarDecl { name: f_name, type_node, .. }) = s {
                        bp.fields.insert(f_name.clone(), type_node.clone());
                    }
                    self.visit_statement(s)?;
                }
                self.leave_scope();
                self.active_flags = prev_flags;
                self.active_return_type = prev_return;

                self.current_env.borrow_mut().define_blueprint(name.to_string(), bp);

                let info = SymbolInfo {
                    name: name.clone(),
                    kind: SymbolKind::Variable {
                        type_node: BaseType::Block {
                            name: name.clone(),
                            methods: Box::new(std::collections::HashMap::new()),
                            return_type: Box::new(BaseType::Unknown),
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
                self.current_env.borrow_mut().define(name.to_string(), info)?;
            }

            Decl::MachineDecl { visibility, name, return_type: _, labels, handle_block } => {
                // Register the machine as a custom blueprint type
                let mut bp = BlueprintData::new(name);

                let info = SymbolInfo {
                    name: name.clone(),
                    kind: SymbolKind::Variable {
                        type_node: BaseType::Machine {
                            name: name.clone(),
                            fields: Box::new(std::collections::HashMap::new()),
                            methods: Box::new(std::collections::HashMap::new()),
                            labels: labels.iter().filter_map(|l| if let Decl::LabelDecl { name, .. } = l { Some(name.clone()) } else { None }).collect(),
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
                self.current_env.borrow_mut().define(name.to_string(), info)?;

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
                    if
                        let Decl::FnDecl {
                            name: f_name,
                            params,
                            return_type,
                            is_virtual,
                            is_abstract,
                            ..
                        } = d
                    {
                        bp.methods.insert(
                            f_name.clone(),
                            crate::middle_end::semantic::environment::FnSignature {
                                name: f_name.clone(),
                                params: params.clone(),
                                return_type: return_type.clone(),
                                is_virtual: *is_virtual,
                                is_abstract: *is_abstract,
                            }
                        );
                    }
                    let _ = self.visit_declaration(d);
                }

                self.leave_scope();
                self.in_custom_scope = prev;
                self.current_env.borrow_mut().define_blueprint(name.to_string(), bp);
            }

            Decl::FnDecl {
                visibility,
                name,
                params,
                return_type,
                body,
                is_virtual,
                is_abstract,
                ..
            } => {
                self.fn_overloads.entry(name.clone()).or_default().push(FnSignature {
                    name: name.clone(),
                    params: params.clone(),
                    return_type: return_type.clone(),
                    is_virtual: *is_virtual,
                    is_abstract: *is_abstract,
                });
                let is_compilable_fn = has_compile_directive(body);

                let fn_info = SymbolInfo {
                    name: name.clone(),
                    kind: SymbolKind::Function {
                        params: params.clone(),
                        return_type: return_type.clone(),
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
                    self.current_env.borrow_mut().define_or_update(name.clone(), fn_info);
                } else {
                    self.current_env.borrow_mut().define(name.clone(), fn_info)?;
                }

                let prev_flags = self.active_flags.clone();
                let prev_return = self.active_return_type.clone();
                let prev_context = self.current_context.clone();
                let prev_deleted = self.deleted_vars.clone();
                let prev_heap = self.heap_allocated_vars.clone();
                self.deleted_vars.clear();
                self.heap_allocated_vars.clear();
                self.current_context = Some(name.clone());
                self.active_flags.push("+has_return".to_string());
                self.active_flags.push("+has_throw".to_string());
                self.active_flags.push("+has_yield".to_string());
                self.active_flags.push("+has_leave".to_string());
                self.active_return_type = Some(return_type.clone());

                self.enter_scope();
                for p in params {
                    for dep in extract_all_type_names(&p.type_node.as_str()) {
                        self.record_dependency(dep);
                    }
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
                    self.current_env.borrow_mut().define(p.name.clone(), param_info)?;
                }
                for dep in extract_all_type_names(&return_type.as_str()) {
                    self.record_dependency(dep);
                }
                for s in body {
                    self.visit_statement(s)?;
                }

                if
                    !self.in_custom_scope &&
                    *return_type != BaseType::Void &&
                    !self.block_always_terminates(body)
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
            }

            Decl::MicroDecl { visibility, name, params, return_type, body, generics: _ } => {
                let micro_info = SymbolInfo {
                    name: name.clone(),
                    kind: SymbolKind::Function {
                        params: params.clone(),
                        return_type: return_type.clone().unwrap_or(BaseType::Void),
                        body: None,
                    },
                    visibility: visibility.clone(),
                    dependencies: vec![],
                    is_used: false,
                    is_param: false,
                    is_uninitialized: false,
            is_compilable: false,
                };
                self.current_env.borrow_mut().define(name.clone(), micro_info)?;

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
                    self.current_env.borrow_mut().define(p.name.clone(), param_info)?;
                }
                for s in body {
                    self.visit_statement(s)?;
                }
                self.leave_scope();

                self.active_flags = prev_flags;
                self.active_return_type = prev_return;
                self.current_context = prev_context;
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

            Decl::BlueprintDecl { name, visibility, .. } => {
                let info = self.make_blueprint_symbol(name, visibility.clone());
                self.current_env.borrow_mut().define(name.clone(), info)?;
            }

            Decl::ImplDecl { target, target_generics, is_handle_impl: _, methods, handle_block } => {
                let is_builtin = Self::is_primitive_numeric(target) || matches!(target.as_str(), "char" | "str" | "bool" | "flag" | "array" | "byte" | "usize" | "isize");
                let symbol_opt = self.current_env.borrow().lookup(target);
                let meta_opt = self.global_metadata.get(target).cloned();

                if !is_builtin && symbol_opt.is_none() && meta_opt.is_none() {
                    return Err(
                        format!("Semantic Error: Target '{}' not found for impl block", target)
                    );
                }

                if self.global_metadata.get(target).is_none() {
                    self.global_metadata.insert(target.clone(), TypeMetadata {
                        name: target.clone(),
                        ty: BaseType::from_str(target),
                        fields: std::collections::HashMap::new(),
                        methods: std::collections::HashMap::new(),
                        constructor: None,
                        handles: vec![],
                        vars: std::collections::HashMap::new(),
                        variants: None,
                    });
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
                    if let Decl::FnDecl { name: h_name, params, return_type, .. } = h {
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

                        if
                            matches!(
                                h_name.as_str(),
                                "display" | "iterator" | "iter" | "copy" | "next" | "break" | "continue"
                            )
                        {
                            if !params.is_empty() {
                                return Err(
                                    format!(
                                        "Semantic Error: Handle method '{}' cannot take any parameters on target '{}'",
                                        h_name,
                                        target
                                    )
                                );
                            }
                        }
                    }
                }

                self.enter_scope();
                self.active_flags.push("+has_return".to_string());
                let prev = self.in_struct;
                self.in_struct = true;

                let this_type = if target == "array" {
                    BaseType::Array {
                        base_type: Box::new(if target_generics.is_empty() { BaseType::GenericParam("T".to_string()) } else { target_generics[0].clone() }),
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
                let _ = self.current_env.borrow_mut().define("this".to_string(), this_info);

                for m in methods {
                    self.visit_declaration(m)?;
                    if let Decl::FnDecl { name, params, return_type, .. } = m {
                        if let Some(meta) = self.global_metadata.get_mut(target) {
                            meta.methods.insert(name.clone(), FnType {
                                name: name.clone(),
                                params: params.clone(),
                                return_type: return_type.clone(),
                            });
                        }
                    }
                }

                for h in handle_block {
                    self.visit_declaration(h)?;
                    if let Decl::FnDecl { name, params, return_type, .. } = h {
                        let hk = HandleMethods::from_str(name.as_str());
                        if let Some(meta) = self.global_metadata.get_mut(target) {
                            if hk != HandleMethods::NotFound && !meta.handles.contains(&hk) {
                                meta.handles.push(hk);
                            }
                            meta.methods.insert(name.clone(), FnType {
                                name: name.clone(),
                                params: params.clone(),
                                return_type: return_type.clone(),
                            });
                        }
                    }
                }

                self.current_type_name = prev_type;
                self.in_struct = prev;
                self.active_flags.retain(|x| x != "+has_return");
                self.leave_scope();
            }

            Decl::ExternFnDecl { name, params, return_type, alias, .. } => {
                let sym_name = alias.as_ref().unwrap_or(name);
                let info = SymbolInfo {
                    name: sym_name.clone(),
                    kind: SymbolKind::Function {
                        return_type: return_type.clone(),
                        params: params.clone(),
                        body: None,
                    },
                    visibility: Visibility::Public,
                    dependencies: vec![],
                    is_used: false,
                    is_param: false,
                    is_uninitialized: false,
            is_compilable: false,
                };
                self.current_env.borrow_mut().define(sym_name.clone(), info)?;
            }

            Decl::ExternBlockDecl { decls, .. } => {
                for d in decls {
                    self.visit_declaration(d)?;
                }
            }

            Decl::Import { .. } => {}
            Decl::DefineDecl { name, type_alias, value, .. } => {
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
            Decl::CompileDecl { decls, .. } => {
                for d in decls {
                    self.visit_declaration(d)?;
                }
            }
            Decl::MacroDecl { name, params, body, visibility } => {
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
    fn analyze_var_decl(
        &mut self,
        visibility: &Visibility,
        editability: &Editability,
        type_node: &BaseType,
        name: &str,
        value: &Expr,
        assign_op: &str
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
                            declared_type.trim_start_matches("modify<name<").trim_end_matches(">>")
                        } else {
                            declared_type.trim_start_matches("copy<name<").trim_end_matches(">>")
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

                    let deps = self.dependency_graph
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
                    self.current_env.borrow_mut().define(name.to_string(), info)?;
                    self.heap_allocated_vars.insert(name.to_string());
                    self.current_context = prev_context;
                    return Ok(());
                }
                // pointer type
                "pointer" => {
                    let is_arr = expr_type.starts_with("array<");
                    let inner_type_str = if declared_type.starts_with("pointer<") {
                        declared_type.trim_start_matches("pointer<").trim_end_matches('>')
                    } else {
                        "unknown"
                    };
                    let expr_inner = if is_arr {
                        expr_type.trim_start_matches("array<").trim_end_matches('>')
                    } else if expr_type.starts_with("pointer<") {
                        expr_type.trim_start_matches("pointer<").trim_end_matches('>')
                    } else {
                        &expr_type
                    };

                    if
                        expr_type != "default" &&
                        inner_type_str != "unknown" &&
                        !self.types_are_compatible(inner_type_str, expr_inner)
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

                    let deps = self.dependency_graph
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
                            type_node: BaseType::Pointer(
                                Box::new(BaseType::from_str(inner_type_str))
                            ),
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
                    self.current_env.borrow_mut().define(name.to_string(), info)?;
                    self.heap_allocated_vars.insert(name.to_string());
                    self.current_context = prev_context;
                    return Ok(());
                }

                // complex types with possible handle overloading
                "custom" | "class" | "struct" | "enum" => {
                    let bp_name = extract_blueprint_name_from_type(&declared_type).unwrap_or_else(||
                        declared_type.clone()
                    );

                    // Check if bp_name is a type alias defined via define
                    let is_alias_compatible = if
                        let Some(info) = self.current_env.borrow().lookup(&bp_name)
                    {
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

                    // Direct compatibility: custom<X> = custom<X> or custom<X> = X
                    let directly_compatible =
                        is_alias_compatible ||
                        self.types_are_compatible(&declared_type, &expr_type) ||
                        expr_type == bp_name ||
                        expr_type == format!("custom<{}>", bp_name);

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
                name_key if self.current_env.borrow().lookup_blueprint(name_key).is_some() => {
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
                    // Allow -> operator for any type (it's used as "default init" syntax)
                    // Allow := operator (inferred type)
                    if assign_op != "->" && assign_op != ":=" && declared_type != "unknown" && expr_type != "default" && !self.types_are_compatible(&declared_type, &expr_type) {
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
        let deps = self.dependency_graph
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
                BaseType::Array { base_type: Box::new(BaseType::Char), size: Box::new(None) }
            } else {
                BaseType::from_str(&expr_type)
            }
        } else {
            type_node.clone()
        };

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
                ) || self.current_env.borrow().lookup_blueprint(&inner.as_str()).is_some()
            }
            _ => self.current_env.borrow().lookup_blueprint(&final_type_node.as_str()).is_some(),
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
        self.current_env.borrow_mut().define(name.to_string(), info)?;
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
    fn analyze_class_or_struct_decl(
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
        constructor: &Option<Vec<ConstructorDecl>>
    ) -> Result<(), String> {
        let prev_context = self.current_context.clone();
        self.current_context = Some(name.to_string());
        if let Some(parent) = extends {
            self.record_dependency(parent.to_string());
        }
        for d in private_block.iter().chain(public_block).chain(static_block) {
            if let Decl::VarDecl { type_node, .. } = d {
                for dep in extract_all_type_names(&type_node.as_str()) {
                    self.record_dependency(dep);
                }
            } else if let Decl::FnDecl { params, return_type, .. } = d {
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
            if let Decl::FnDecl { params, return_type, .. } = d {
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
            let parent_bp = self.current_env
                .borrow()
                .lookup_blueprint(parent)
                .or_else(|| self.global_metadata.get(parent).map(build_blueprint_from_metadata));
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
            if
                let Decl::FnDecl {
                    name: fn_name,
                    params: fn_params,
                    return_type,
                    is_virtual,
                    is_abstract,
                    ..
                } = d
            {
                let hk = HandleMethods::from_str(fn_name.as_str());
                if hk != HandleMethods::NotFound {
                    bp.handles.insert(hk);
                }
                bp.methods.insert(fn_name.clone(), FnSignature {
                    name: fn_name.clone(),
                    params: fn_params.clone(),
                    return_type: return_type.clone(),
                    is_virtual: *is_virtual,
                    is_abstract: *is_abstract,
                });
            }
        }
        for d in private_block.iter().chain(public_block).chain(static_block) {
            if let Decl::VarDecl { name: f_name, type_node, .. } = d {
                bp.fields.insert(f_name.clone(), type_node.clone());
            }
        }
        for d in private_block.iter().chain(public_block).chain(static_block) {
            if
                let Decl::FnDecl {
                    name: fn_name,
                    params,
                    return_type,
                    is_virtual,
                    is_abstract,
                    ..
                } = d
            {
                bp.methods.insert(fn_name.clone(), FnSignature {
                    name: fn_name.clone(),
                    params: params.clone(),
                    return_type: return_type.clone(),
                    is_virtual: *is_virtual,
                    is_abstract: *is_abstract,
                });
            }
        }
        self.current_env.borrow_mut().define_blueprint(name.to_string(), bp);

        let info = self.make_blueprint_symbol(name, visibility);
        self.current_env.borrow_mut().define(name.to_string(), info)?;

        self.enter_scope();
        let prev = self.in_custom_scope;
        self.in_custom_scope = true;
        let prev_type_name = self.current_type_name.clone();
        self.current_type_name = Some(name.to_string());

        for d in private_block.iter().chain(public_block).chain(static_block).chain(handle_block) {
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
                        is_used: false,
                        is_param: true,
                        is_uninitialized: false,
            is_compilable: false,
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

        self.leave_scope();
        self.in_custom_scope = prev;
        self.current_type_name = prev_type_name;
        self.current_context = prev_context;
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
                            inner_t.trim_start_matches("array<").trim_end_matches('>').to_string()
                        } else if inner_t == "str" {
                            "char".to_string()
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
                Ok(
                    format!(
                        "array<{}>",
                        element_type.unwrap_or_else(|| "unknown".to_string())
                    )
                )
            }

            Expr::Spread(inner) => {
                let inner_t = self.visit_expression(inner)?;
                if inner_t.starts_with("array<") {
                    Ok(inner_t.trim_start_matches("array<").trim_end_matches('>').to_string())
                } else if inner_t == "str" {
                    Ok("char".to_string())
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
                if let Some(t) = type_arg { Ok(t.as_str()) } else { Ok("default".to_string()) }
            }

            Expr::Identifier(name) => {
                if
                    matches!(
                        name.trim(),
                        "broken" |
                            "is_done" |
                            "yielded" |
                            "returned" |
                            "leaved" |
                            "continued" |
                            "has_break" |
                            "has_yield" |
                            "has_leave" |
                            "has_return" |
                            "has_call" |
                            "has_error" |
                            "compilable" |
                            "is_compilable" |
                            "printable" |
                            "is_printable" |
                            "throwable" |
                            "is_throwable"
                    )
                {
                    return Ok("bool".to_string());
                }
                if
                    matches!(
                        name.as_str(),
                        "bool" |
                            "int8" |
                            "int16" |
                            "int32" |
                            "int64" |
                            "int" |
                            "uint8" |
                            "uint16" |
                            "uint32" |
                            "uint64" |
                            "uint" |
                            "float32" |
                            "float64" |
                            "float" |
                            "char" |
                            "str" |
                            "byte" |
                            "usize" |
                            "isize" |
                            "type" |
                            "void"
                    )
                {
                    return Ok("type".to_string());
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
                if
                    self.current_env.borrow().lookup_blueprint(name).is_some() ||
                    self.global_metadata.contains_key(name)
                {
                    return Ok("type".to_string());
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
                Err(format!("Semantic Error: Identifier '{}' is not defined in this scope.", name))
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

            Expr::Call { callee, args } | Expr::MacroCall { callee, args } => {

                // Generic Macro lookup and type checking
                let macro_info = match &**callee {
                    Expr::Identifier(fn_name) => {
                        self.current_env.borrow().lookup(fn_name).and_then(|info| {
                            if let SymbolKind::Macro { params, return_type, body } = &info.kind {
                                Some((fn_name.clone(), params.clone(), return_type.clone(), body.clone()))
                            } else {
                                None
                            }
                        })
                    }
                    Expr::NamespaceAccess { namespace: _, property } => {
                        if let Expr::Identifier(prop_name) = &**property {
                            self.current_env.borrow().lookup(prop_name).and_then(|info| {
                                if let SymbolKind::Macro { params, return_type, body } = &info.kind {
                                    Some((prop_name.clone(), params.clone(), return_type.clone(), body.clone()))
                                } else {
                                    None
                                }
                            })
                        } else {
                            None
                        }
                    }
                    _ => None,
                };

                if let Some((macro_name, params, return_type, macro_body)) = macro_info {
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
                        if arg_type != "unknown" && param_type_str != "unknown" && !self.types_are_compatible(&param_type_str, &arg_type) {
                            return Err(format!(
                                "Semantic Error: Macro '{}' parameter '{}' type mismatch. Expected '{}', got '{}'",
                                macro_name,
                                param.name,
                                param_type_str,
                                arg_type
                            ));
                        }
                    }

                    // Evaluate the macro at compile time using the interpreter
                    let mut interp = crate::middle_end::interpreter::eval::Interpreter::new();
                    if let Err(e) = interp.eval_macro(&macro_body, &params, args) {
                        return Err(e);
                    }

                    return Ok(return_type.as_str());
                }
                if let Expr::PropertyAccess { object, property } = &**callee {
                    let obj_type = self.visit_expression(object)?;
                    let type_name = if obj_type.starts_with("array<") {
                        "array".to_string()
                    } else {
                        extract_blueprint_name_from_type(&obj_type).unwrap_or_else(||
                            obj_type.clone()
                        )
                    };

                    let has_handle = if
                        let Some(bp) = self.current_env.borrow().lookup_blueprint(&type_name)
                    {
                        bp.handles.iter().any(|h| h.as_str() == property)
                    } else if let Some(meta) = self.global_metadata.get(&type_name) {
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

                    let bp_method_info = {
                        let env = self.current_env.borrow();
                        if let Some(bp) = env.lookup_blueprint(&type_name) {
                            bp.methods.get(property).cloned()
                        } else {
                            None
                        }
                    };

                    if let Some(method_sig) = bp_method_info {
                        self.record_dependency(property.clone());
                        self.record_dependency(type_name.clone());
                        for dep in extract_all_type_names(&method_sig.return_type.as_str()) {
                            self.record_dependency(dep);
                        }
                        for p in &method_sig.params {
                            for dep in extract_all_type_names(&p.type_node.as_str()) {
                                self.record_dependency(dep);
                            }
                        }
                        let ret_type = if type_name == "array" && obj_type.starts_with("array<") {
                            let elem_t = obj_type.trim_start_matches("array<").trim_end_matches('>');
                            let mut gen_map = std::collections::HashMap::new();
                            gen_map.insert("T".to_string(), BaseType::from_str(elem_t));
                            method_sig.return_type.substitute_generics(&gen_map)
                        } else {
                            method_sig.return_type.clone()
                        };
                        return Ok(ret_type.as_str());
                    }
                    let method_deps_and_ret = if let Some(meta) = self.global_metadata.get(&type_name) {
                        if let Some(fn_type) = meta.methods.get(property) {
                            let mut deps = vec![property.clone(), type_name.clone()];
                            for dep in extract_all_type_names(&fn_type.return_type.as_str()) {
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
                    };

                    if let Some((deps, return_type)) = method_deps_and_ret {
                        for dep in deps {
                            self.record_dependency(dep);
                        }
                        let ret_type = if type_name == "array" && obj_type.starts_with("array<") {
                            let elem_t = obj_type.trim_start_matches("array<").trim_end_matches('>');
                            let mut gen_map = std::collections::HashMap::new();
                            gen_map.insert("T".to_string(), BaseType::from_str(elem_t));
                            return_type.substitute_generics(&gen_map)
                        } else {
                            return_type
                        };
                        return Ok(ret_type.as_str());
                    }
                }
                let mut arg_types = Vec::new();
                for arg in args {
                    arg_types.push(self.visit_expression(arg)?);
                }

                if let Expr::Identifier(name) = &**callee {
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
                    if name == "error" {
                        return Ok("error".to_string());
                    }
                    if name == "typeof" {
                        return Ok("type".to_string());
                    }
                    if name == "sizeof" {
                        return Ok("int32".to_string());
                    }

                    let bp_call = if
                        let Some(bp) = self.current_env.borrow().lookup_blueprint(name)
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
                        bp.methods.get("call").map(|s| s.return_type.as_str())
                    } else {
                        None
                    };
                    if let Some(ret) = bp_call {
                        return Ok(ret);
                    }

                    if let Some(meta) = self.global_metadata.get(name) {
                        if let Some(call_sig) = meta.methods.get("call") {
                            return Ok(call_sig.return_type.as_str());
                        }
                    }

                    if let Some(info) = self.current_env.borrow().lookup(name) {
                        if info.is_compilable {
                            if let SymbolKind::Function { params, return_type: _, body } = &info.kind {
                                if let Some(b) = body {
                                    let mut interp = crate::middle_end::interpreter::eval::Interpreter::new();
                                    if let Err(e) = interp.eval_macro(b, params, args) {
                                        return Err(e);
                                    }
                                }
                            }
                        }
                    }

                    if let Some(sigs) = self.fn_overloads.get(name) {
                        let mut best_match = None;
                        for sig in sigs {
                            if sig.params.len() == arg_types.len() {
                                let matches = sig.params
                                    .iter()
                                    .zip(arg_types.iter())
                                    .all(|(p, a_ty)| {
                                        self.types_are_compatible(&p.type_node.as_str(), a_ty)
                                    });
                                if matches {
                                    best_match = Some(sig.return_type.as_str());
                                    break;
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
                        if let SymbolKind::Function { return_type, .. } = &info.kind {
                            return Ok(return_type.as_str());
                        }
                    }
                    if name != "Some" && self.current_env.borrow().lookup(name).is_none() {
                        return Err(
                            format!("Semantic Error: Function '{}' is not defined in this scope.", name)
                        );
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
                Ok(self.current_type_name.clone().unwrap_or_else(|| "object".to_string()))
            }

            Expr::Global => Ok("object".to_string()),

            Expr::Super => {
                if !self.in_class {
                    return Err("Semantic Error: Cannot use 'super' outside of a class".to_string());
                }
                Ok("object".to_string())
            }

            Expr::ArrayAllocate { type_node, size, length } => {
                self.visit_expression(size)?;
                if let Some(l) = length {
                    self.visit_expression(l)?;
                }
                Ok(format!("array<{}>", type_node.as_str()))
            }

            Expr::BinaryOp { left, operator, right } => {
                let left_type = self.visit_expression(left)?;
                let right_type = self.visit_expression(right)?;
                if left_type == "undefined" || right_type == "undefined" {
                    return Err("Semantic Error: Cannot use undefined value in operation".to_string());
                }
                if (left_type == "str" || left_type == "char" || left_type == "array<char>")
                    && (right_type == "str" || right_type == "char" || right_type == "array<char>")
                    && operator == "+"
                {
                    return Ok("str".to_string());
                }
                match operator.as_str() {
                    "==" | "!=" | ">" | "<" | ">=" | "<=" | "&&" | "||" => {
                        Ok("bool".to_string())
                    }
                    _ => {
                        if left_type != "unknown" { Ok(left_type) } else { Ok(right_type) }
                    }
                }
            }

            Expr::UnaryOp { operand, operator } => {
                let op_type = self.visit_expression(operand)?;
                if op_type == "undefined" {
                    return Err("Semantic Error: Cannot use undefined value in operation".to_string());
                }
                if operator == "*" {
                    for prefix in &["name<", "modify<", "copy<", "pointer<", "array<"] {
                        if op_type.starts_with(prefix) {
                            let inner = strip_wrapper(&op_type, prefix);
                            return Ok(inner.to_string());
                        }
                    }
                } else if operator == "&" {
                    return Ok(format!("pointer<{}>", op_type));
                }
                Ok(op_type)
            }

            Expr::PrefixUpdate { right, .. } => self.visit_expression(right),
            Expr::PostfixUpdate { left, .. } => self.visit_expression(left),
            // TODO: add check if the type of the object contains a index_access handle if it not a name or array or pointer
            // TODO: add modify and copy
            Expr::IndexAccess { object, indices } => {
                for idx in indices {
                    self.visit_expression(idx)?; //todo: check if the index is the same type as the parameter of the index_access handle
                }

                let obj_type = self.visit_expression(object)?;

                let mut current_type = obj_type.clone();

                while
                    current_type.starts_with("modify<") ||
                    current_type.starts_with("copy<") ||
                    current_type.starts_with("name<") ||
                    current_type.starts_with("pointer<")
                {
                    if current_type.starts_with("modify<") {
                        current_type = strip_wrapper(&current_type, "modify<").to_string();
                    } else if current_type.starts_with("copy<") {
                        current_type = strip_wrapper(&current_type, "copy<").to_string();
                    } else if current_type.starts_with("name<") {
                        current_type = strip_wrapper(&current_type, "name<").to_string();
                    } else if current_type.starts_with("pointer<") {
                        current_type = strip_wrapper(&current_type, "pointer<").to_string();
                    }
                }

                if current_type == "str" {
                    return Ok("char".to_string());
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
                if obj_type.starts_with("pointer<") {
                    return Ok(strip_wrapper(&obj_type, "pointer<").to_string());
                }
                if current_type != "unknown" {
                    let bp_name = extract_blueprint_name_from_type(&obj_type).unwrap_or_else(||
                        obj_type.clone()
                    );
                    if let Some(bp) = self.current_env.borrow().lookup_blueprint(&bp_name) {
                        if
                            bp.handles.contains(&HandleMethods::IndexAccess) ||
                            bp.methods.contains_key("index_access")
                        {
                            if let Some(ret_sig) = bp.methods.get("index_access") {
                                return Ok(ret_sig.return_type.as_str());
                            }
                            return Ok("int32".to_string());
                        }
                    }
                    if let Some(meta) = self.global_metadata.get(&bp_name) {
                        if
                            meta.handles.contains(&HandleMethods::IndexAccess) ||
                            meta.methods.contains_key("index_access")
                        {
                            if let Some(ret_ty) = meta.methods.get("index_access") {
                                return Ok(ret_ty.return_type.as_str());
                            }
                            return Ok("int32".to_string());
                        }
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

                if obj_type.starts_with("array<") || obj_type.ends_with("[]") {
                    if property == "length" || property == "len" || property == "size" {
                        return Ok("int32".to_string());
                    }
                }

                if
                    matches!(
                        property.as_str(),
                        "broken" |
                            "is_done" |
                            "yielded" |
                            "returned" |
                            "leaved" |
                            "continued" |
                            "has_break" |
                            "has_yield" |
                            "has_leave" |
                            "has_return" |
                            "has_call" |
                            "has_error" |
                            "compilable" |
                            "is_compilable" |
                            "printable" |
                            "is_printable" |
                            "throwable" |
                            "is_throwable"
                    )
                {
                    return Ok("bool".to_string());
                }

                if obj_type.starts_with("array<") || obj_type == "array" {
                    if property == "len" || property == "length" || property == "size" {
                        return Ok("int32".to_string());
                    }
                }

                if obj_type.starts_with("scope") {
                    match property.as_str() {
                        "has_yield" | "has_leave" | "has_return" | "is_done" => {
                            return Ok("bool".to_string());
                        }
                        "yield_value" | "leave_value" | "return_value" | "result" => {
                            return Ok("any".to_string());
                        }
                        _ => {}
                    }
                }

                // Try to find in blueprint
                let bp_name_opt = extract_blueprint_name_from_type(&obj_type).or_else(||
                    Some(obj_type.clone())
                );
                if let Some(bp_name) = bp_name_opt {
                    if let Some(bp) = self.current_env.borrow().lookup_blueprint(&bp_name) {
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
                // todo : we removed custom<> support
                if let Expr::Identifier(n) = &**target {
                    self.record_dependency(n.clone());
                    return Ok(format!("custom<{}>", n));
                }
                if let Expr::NamespaceAccess { namespace, .. } = &**target {
                    self.record_dependency(namespace.clone());
                    if self.global_metadata.contains_key(namespace) || self.current_env.borrow().lookup_blueprint(namespace).is_some() {
                        return Ok(namespace.clone());
                    }
                }
                Ok("unknown".to_string())
            }

            Expr::New { type_node, target } => {
                self.visit_expression(target)?;
                let type_str = type_node.as_str();
                for dep in extract_all_type_names(&type_str) {
                    self.record_dependency(dep);
                }
                Ok(type_str)
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
                    self.current_env.borrow_mut().define(p.name.clone(), p_info)?;
                }
                for s in body {
                    self.visit_statement(s)?;
                }
                self.leave_scope();
                Ok("method".to_string())
            }
        }
    }

    // ----------------------------------------------------------
    // types_are_compatible
    // ----------------------------------------------------------
    fn types_are_compatible(&self, expected: &str, actual: &str) -> bool {
        if
            expected == actual ||
            expected == "any" ||
            expected == "unknown" ||
            actual == "unknown" ||
            actual == "undefined"
        {
            return true;
        }
        if actual == "Fn<(), void>" || actual == "stop" || actual.contains("Micro::stop") || actual.contains("stop") {
            if expected.starts_with("modify<") || expected.starts_with("Option<") || expected.starts_with("pointer<") || expected.starts_with("name<") || expected.contains("Option") || expected.contains("stop") {
                return true;
            }
        }
        if expected == "Fn<(), void>" || expected == "stop" || expected.contains("Micro::stop") || expected.contains("stop") {
            if actual.starts_with("modify<") || actual.starts_with("Option<") || actual.starts_with("pointer<") || actual.starts_with("name<") || actual.contains("Option") || actual.contains("stop") {
                return true;
            }
        }
        if (expected == "flag" && actual == "bool") || (expected == "bool" && actual == "flag") {
            return true;
        }
        if (expected == "str" || expected == "array<char>" || expected == "char[]") &&
           (actual == "str" || actual == "array<char>" || actual == "char[]") {
            return true;
        }
        if
            (expected == "type" || expected.starts_with("type<")) &&
            (actual == "type" ||
                actual.starts_with("type<") ||
                actual.starts_with("int") ||
                actual.starts_with("uint") ||
                actual == "str" ||
                actual == "bool" ||
                actual.starts_with("float"))
        {
            return true;
        }
        let exp_clean = extract_blueprint_name_from_type(expected).unwrap_or_else(||
            expected.to_string()
        );
        let act_clean = extract_blueprint_name_from_type(actual).unwrap_or_else(||
            actual.to_string()
        );
        if let Some(sym) = self.current_env.borrow().lookup(&exp_clean) {
            if let SymbolKind::Variable { type_node: BaseType::Type(inner), .. } = &sym.kind {
                return self.types_are_compatible(&inner.as_str(), actual);
            }
        }
        if let Some(sym) = self.current_env.borrow().lookup(&act_clean) {
            if let SymbolKind::Variable { type_node: BaseType::Type(inner), .. } = &sym.kind {
                return self.types_are_compatible(expected, &inner.as_str());
            }
        }
        if
            (expected.to_lowercase().starts_with("method") ||
                expected.starts_with("Fn") ||
                expected.starts_with("fn") ||
                expected.to_lowercase().contains("<method") ||
                expected.contains("<Fn<")) &&
            (actual.to_lowercase().starts_with("method") ||
                actual.starts_with("Fn") ||
                actual.starts_with("fn") ||
                actual.to_lowercase().contains("<method") ||
                actual.contains("<Fn<") ||
                actual == "fn")
        {
            return true;
        }
        // int and uint family
        if
            (expected.starts_with("int") ||
                expected.starts_with("uint") ||
                expected == "byte" ||
                expected == "usize" ||
                expected == "isize") &&
            (actual.starts_with("int") ||
                actual.starts_with("uint") ||
                actual == "byte" ||
                actual == "usize" ||
                actual == "isize")
        {
            return true;
        }
        if
            (expected == "int" ||
                expected == "int32" ||
                expected == "uint" ||
                expected == "uint32") &&
            (actual == "int" || actual == "uint")
        {
            return true;
        }
        // float family
        if expected.starts_with("float") && actual.starts_with("float") {
            return true;
        }
        // array family
        if expected.starts_with("array<") && actual.starts_with("array<") {
            let exp_inner = expected.trim_start_matches("array<").trim_end_matches('>');
            let act_inner = actual.trim_start_matches("array<").trim_end_matches('>');
            let exp_base = exp_inner.split('[').next().unwrap_or(exp_inner);
            let act_base = act_inner.split('[').next().unwrap_or(act_inner);
            return self.types_are_compatible(exp_base, act_base);
        }
        if expected.len() == 1 && expected.chars().next().unwrap().is_ascii_uppercase() {
            return true;
        }
        if actual.len() == 1 && actual.chars().next().unwrap().is_ascii_uppercase() {
            return true;
        }
        if expected == "custom<T>" || expected == "T" || actual == "custom<T>" || actual == "T" {
            return true;
        }
        // blueprint/struct/class/enum/machine/block wrappers
        for prefix in &[
            "blueprint<",
            "struct<",
            "class<",
            "enum<",
            "machine<",
            "block<",
            "custom<",
            "object<",
        ] {
            if expected.starts_with(prefix) {
                let inner = strip_wrapper(expected, prefix);
                if inner == actual || self.types_are_compatible(inner, actual) {
                    return true;
                }
            }
            if actual.starts_with(prefix) {
                let inner = strip_wrapper(actual, prefix);
                if inner == expected || self.types_are_compatible(expected, inner) {
                    return true;
                }
            }
        }
        let exp_bp_name = extract_blueprint_name_from_type(expected).unwrap_or_else(||
            expected.to_string()
        );
        if
            let Some(bp) = self.current_env
                .borrow()
                .lookup_blueprint(&exp_bp_name)
                .or_else(||
                    self.global_metadata.get(&exp_bp_name).map(build_blueprint_from_metadata)
                )
        {
            if
                bp.handle_accepts_type(HandleMethods::ArrowAssign, actual) ||
                bp.handle_accepts_type(HandleMethods::Arrow, actual)
            {
                return true;
            }
            if let Some(meta) = self.global_metadata.get(&exp_bp_name) {
                if let Some(constructors) = &meta.constructor {
                    for c in constructors {
                        if c.params.len() == 1 {
                            let param_t = c.params[0].type_node.as_str();
                            if
                                param_t == actual ||
                                (param_t != expected && self.types_are_compatible(&param_t, actual))
                            {
                                return true;
                            }
                        }
                    }
                }
            }
        }
        // name pointer compatibility (holds T, array<T>, name<T>, pointer<T>)
        if
            expected == "name" ||
            expected == "name<unknown>" ||
            actual == "name" ||
            actual == "name<unknown>"
        {
            return true;
        }
        if expected.starts_with("name<") {
            let inner = strip_wrapper(expected, "name<");
            if inner == "unknown" {
                return true;
            }
            let actual_inner = if actual.starts_with("array<") {
                strip_wrapper(actual, "array<")
            } else if actual.starts_with("name<") {
                strip_wrapper(actual, "name<")
            } else if actual.starts_with("pointer<") {
                strip_wrapper(actual, "pointer<")
            } else {
                actual
            };
            return self.types_are_compatible(inner, actual_inner);
        }
        if actual.starts_with("name<") {
            let actual_inner = strip_wrapper(actual, "name<");
            return self.types_are_compatible(expected, actual_inner);
        }

        // raw pointer compatibility (points to T, array<T>, pointer<T>, name<T>)
        if
            expected == "pointer" ||
            expected == "pointer<unknown>" ||
            actual == "pointer" ||
            actual == "pointer<unknown>"
        {
            return true;
        }
        if expected.starts_with("pointer<") {
            let inner = strip_wrapper(expected, "pointer<");
            if inner == "unknown" {
                return true;
            }
            let actual_inner = if actual.starts_with("array<") {
                strip_wrapper(actual, "array<")
            } else if actual.starts_with("pointer<") {
                strip_wrapper(actual, "pointer<")
            } else if actual.starts_with("name<") {
                strip_wrapper(actual, "name<")
            } else {
                actual
            };
            return self.types_are_compatible(inner, actual_inner);
        }
        if actual.starts_with("pointer<") {
            let actual_inner = strip_wrapper(actual, "pointer<");
            return self.types_are_compatible(expected, actual_inner);
        }

        // modify pointer compatibility (holds T, array<T>, pointer<T>, name<T>, modify<T>)
        if
            expected == "modify" ||
            expected == "modify<unknown>" ||
            actual == "modify" ||
            actual == "modify<unknown>"
        {
            return true;
        }
        if expected.starts_with("modify<") {
            let inner = strip_wrapper(expected, "modify<");
            if inner == "unknown" {
                return true;
            }
            let actual_inner = if actual.starts_with("array<") {
                strip_wrapper(actual, "array<")
            } else if actual.starts_with("pointer<") {
                strip_wrapper(actual, "pointer<")
            } else if actual.starts_with("modify<") {
                strip_wrapper(actual, "modify<")
            } else if actual.starts_with("name<") {
                strip_wrapper(actual, "name<")
            } else {
                actual
            };
            if inner.contains(',') {
                for part in inner.split(',') {
                    let clean = part.trim();
                    if self.types_are_compatible(clean, actual_inner) {
                        return true;
                    }
                }
            }
            return self.types_are_compatible(inner, actual_inner);
        }
        if actual.starts_with("modify<") {
            let actual_inner = strip_wrapper(actual, "modify<");
            if actual_inner.contains(',') {
                for part in actual_inner.split(',') {
                    let clean = part.trim();
                    if self.types_are_compatible(expected, clean) {
                        return true;
                    }
                }
            }
            return self.types_are_compatible(expected, actual_inner);
        }

        // copy intermediate compatibility (copies from T, name<T>, modify<T>, pointer<T>)
        if expected.starts_with("copy<") || actual.starts_with("copy<") {
            let exp_inner = if expected.starts_with("copy<") {
                strip_wrapper(expected, "copy<")
            } else {
                expected
            };
            let act_inner = if actual.starts_with("copy<") {
                strip_wrapper(actual, "copy<")
            } else {
                actual
            };
            return self.types_are_compatible(exp_inner, act_inner);
        }

        false
    }

    // ----------------------------------------------------------
    // Helpers
    // ----------------------------------------------------------

    fn is_primitive_numeric(t: &str) -> bool {
        matches!(
            t,
            "int" |
                "int8" |
                "int16" |
                "int32" |
                "int64" |
                "int128" |
                "uint" |
                "uint8" |
                "uint16" |
                "uint32" |
                "uint64" |
                "uint128" |
                "byte" |
                "usize" |
                "isize" |
                "float" |
                "float32" |
                "float64" |
                "float128"
        )
    }

    fn make_blueprint_symbol(&self, name: &str, visibility: Visibility) -> SymbolInfo {
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

    fn block_always_terminates(&self, stmts: &[Stmt]) -> bool {
        for s in stmts {
            if self.stmt_always_terminates(s) {
                return true;
            }
        }
        false
    }

    fn stmt_always_terminates(&self, stmt: &Stmt) -> bool {
        match stmt {
            | Stmt::ReturnStmt(_)
            | Stmt::ThrowStmt(_)
            | Stmt::LeaveStmt
            | Stmt::YieldStmt(_)
            | Stmt::GotoStmt(_)
            | Stmt::SwitchStmt { .. }
            | Stmt::CaseStmt { .. } => true,
            Stmt::IfStmt { then_block, else_block, .. } => {
                if let Some(eb) = else_block {
                    self.block_always_terminates(then_block) && self.block_always_terminates(eb)
                } else {
                    false
                }
            }
            Stmt::TryCatchStmt { try_block, catch_block, .. } => {
                self.block_always_terminates(try_block) && self.block_always_terminates(catch_block)
            }
            Stmt::DoWhileStmt { body, .. } => {
                match body {
                    EitherBlock::Inline(stmts) => self.block_always_terminates(stmts),
                    EitherBlock::External(_) => false,
                }
            }
            _ => false,
        }
    }
}

fn strip_wrapper<'a>(s: &'a str, prefix: &str) -> &'a str {
    if let Some(rest) = s.strip_prefix(prefix) {
        if let Some(inner) = rest.strip_suffix('>') {
            return inner;
        }
        return rest;
    }
    s
}

fn extract_type_args_from_str(s: &str) -> (String, Vec<BaseType>) {
    let mut trimmed = s.trim();
    for prefix in &[
        "blueprint<",
        "struct<",
        "class<",
        "enum<",
        "machine<",
        "block<",
        "custom<",
        "object<",
    ] {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            if let Some(inner) = rest.strip_suffix('>') {
                trimmed = inner.trim();
                break;
            }
        }
    }

    if let Some(start) = trimmed.find('<') {
        if let Some(rest) = trimmed.strip_suffix('>') {
            let base = trimmed[..start].trim().to_string();
            let inside = &rest[start + 1..];
            let mut args = Vec::new();
            for arg_str in inside.split(',') {
                let clean_arg = arg_str.trim();
                if !clean_arg.is_empty() {
                    args.push(BaseType::from_str(clean_arg));
                }
            }
            return (base, args);
        }
    }
    (trimmed.to_string(), Vec::new())
}

fn extract_iter_payload_type(t: &BaseType) -> String {
    match t {
        BaseType::Enum { name, generics, .. } if name == "Option" && !generics.is_empty() => {
            extract_iter_payload_type(&generics[0])
        }
        BaseType::Copy(inner) | BaseType::Name(inner) | BaseType::Modify(inner) => {
            match &**inner {
                BaseType::Enum { name, generics, .. } if
                    name == "Option" &&
                    !generics.is_empty()
                => {
                    extract_iter_payload_type(&generics[0])
                }
                BaseType::Generic(vec) if !vec.is_empty() => extract_iter_payload_type(&vec[0]),
                _ => inner.as_str(),
            }
        }
        BaseType::Generic(vec) if !vec.is_empty() => extract_iter_payload_type(&vec[0]),
        _ => t.as_str(),
    }
}

fn has_compile_directive_expr(expr: &Expr) -> bool {
    match expr {
        Expr::MacroCall { callee, .. } => {
            if let Expr::NamespaceAccess { namespace, .. } = &**callee {
                if namespace == "@compile" {
                    println!("Found compile directive in MacroCall!");
                    return true;
                }
            }
            false
        }
        Expr::Call { callee, args } => {
            has_compile_directive_expr(callee) || args.iter().any(has_compile_directive_expr)
        }
        Expr::BinaryOp { left, right, .. } => has_compile_directive_expr(left) || has_compile_directive_expr(right),
        Expr::PropertyAccess { object, .. } => has_compile_directive_expr(object),
        Expr::ArrayLiteral(elements) => elements.iter().any(has_compile_directive_expr),
        Expr::ObjectLiteral(stmts) => has_compile_directive(stmts),
        Expr::Instantiate { args, .. } => args.iter().any(has_compile_directive_expr),
        Expr::NamespaceAccess { namespace, property, .. } => {
            if namespace == "@compile" {
                return true;
            }
            has_compile_directive_expr(property)
        }
        _ => false,
    }
}

pub fn has_compile_directive(stmts: &[Stmt]) -> bool {
    for stmt in stmts {
        match stmt {
            Stmt::ExpressionStmt(expr) | Stmt::CallStmt(expr) => {
                if has_compile_directive_expr(expr) { return true; }
            }
            Stmt::Declaration(Decl::VarDecl { value, .. }) => {
                if has_compile_directive_expr(value) { return true; }
            }
            Stmt::Block(inner) | Stmt::ThisBlock(inner) => {
                if has_compile_directive(inner) { return true; }
            }
            Stmt::IfStmt { condition, then_block, else_block } => {
                if has_compile_directive_expr(condition) { return true; }
                if has_compile_directive(then_block) { return true; }
                if let Some(e) = else_block {
                    if has_compile_directive(e) { return true; }
                }
            }
            Stmt::WhileStmt { condition, body } => {
                if has_compile_directive_expr(condition) { return true; }
                match body {
                    crate::frontend::parser::ast::EitherBlock::Inline(b) => {
                        if has_compile_directive(b) { return true; }
                    }
                    crate::frontend::parser::ast::EitherBlock::External(e) => {
                        if has_compile_directive_expr(e) { return true; }
                    }
                }
            }
            Stmt::ForStmt { init, condition, increment, body } => {
                if let Some(i) = init {
                    if has_compile_directive(&[*(i.clone())]) { return true; }
                }
                if let Some(c) = condition {
                    if has_compile_directive_expr(c) { return true; }
                }
                if let Some(inc) = increment {
                    if has_compile_directive(&[*(inc.clone())]) { return true; }
                }
                match body {
                    crate::frontend::parser::ast::EitherBlock::Inline(b) => {
                        if has_compile_directive(b) { return true; }
                    }
                    crate::frontend::parser::ast::EitherBlock::External(e) => {
                        if has_compile_directive_expr(e) { return true; }
                    }
                }
            }
            Stmt::ReturnStmt(Some(e)) => {
                if has_compile_directive_expr(e) { return true; }
            }
            Stmt::ThrowStmt(e) => {
                if has_compile_directive_expr(e) { return true; }
            }
            Stmt::ReassignStmt { target, value, .. } => {
                if has_compile_directive_expr(target) || has_compile_directive_expr(value) { return true; }
            }
            Stmt::Declaration(Decl::CompileDecl { .. }) => {
                return true;
            }
            _ => {}
        }
    }
    false
}