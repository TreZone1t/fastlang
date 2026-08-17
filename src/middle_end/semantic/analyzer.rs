use crate::frontend::parser::ast::*;
use crate::middle_end::semantic::environment::{Environment, SymbolInfo};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

pub struct SemanticAnalyzer {
    pub current_env: Rc<RefCell<Environment>>,
    pub in_class: bool,
    pub in_struct: bool,
    pub in_custom_scope: bool,
    pub in_statement_scope: bool,
    pub active_flags: Vec<String>,
    pub active_return_type: Option<crate::frontend::parser::ast::BaseType>,
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
        analyzer.inject_stdlib();
        analyzer
    }

    fn inject_stdlib(&mut self) {
        let std_funcs = vec![
            ("log", "void"),
            ("input", "string"),
            ("to_string", "string"),
        ];

        for (name, _ret_type) in std_funcs {
            let info = SymbolInfo {
                dependencies: Vec::new(),
                name: name.to_string(),
                type_node: Some(BaseType::from_str("fn")),
                visibility: Visibility::Public,
                editability: Editability::NotEditable,
                settings: std::collections::HashSet::new(),
                is_array: false,
            };
            let _ = self.current_env.borrow_mut().define(name.to_string(), info);
        }
    }

    pub fn record_dependency(&mut self, dep: String) {
        if let Some(ctx) = &self.current_context {
            self.dependency_graph
                .entry(ctx.clone())
                .or_insert_with(HashSet::new)
                .insert(dep);
        }
    }

    pub fn analyze(&mut self, ast: &Vec<Stmt>) -> Result<(), String> {
        for stmt in ast {
            self.visit_statement(stmt)?;
        }
        Ok(())
    }

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

    fn visit_statement(&mut self, stmt: &Stmt) -> Result<(), String> {
        match stmt {
            Stmt::Declaration(decl) => {
                self.visit_declaration(decl)?;
            }
            Stmt::ReassignStmt { target, value } => {
                let expr_type = self.visit_expression(value)?;
                let target_type = self.visit_expression(target)?;

                if let Expr::Identifier(name) = target {
                    if let Some(info) = self.current_env.borrow().lookup(name) {
                        if info.editability
                            == crate::frontend::parser::ast::Editability::NotEditable
                        {
                            return Err(format!(
                                "Semantic Error: Cannot reassign constant '{}'",
                                name
                            ));
                        }
                    }
                }

                if target_type.starts_with("name<") {
                    let expected_type = target_type
                        .trim_start_matches("name<")
                        .trim_end_matches(">");

                    if expected_type == "unknown" && expr_type != "unknown" {
                        if let Expr::Identifier(name) = target {
                            let maybe_info = self.current_env.borrow().lookup(name);
                            if let Some(mut info) = maybe_info {
                                info.type_node = Some(BaseType::Generic(Box::new(HashMap::from(
                                    [("T".to_string(), BaseType::from_str(&expr_type))],
                                ))));
                                self.current_env.borrow_mut().update(name, info);
                            }
                        }
                    } else if expected_type != expr_type
                        && expected_type != "unknown"
                        && expr_type != "unknown"
                        && expr_type != "object"
                    {
                        return Err(format!(
                            "Semantic Error: Cannot reassign smart pointer 'name<{}>' to type '{}'",
                            expected_type, expr_type
                        ));
                    }
                } else if target_type != expr_type
                    && target_type != "unknown"
                    && expr_type != "unknown"
                {
                    if target_type == "name" && expr_type == "object" {
                        // السماح بتعيين object لـ name
                    } else if target_type == "type" || target_type.starts_with("type<") {
                        // السماح بتعيين الأنواع
                    } else {
                        if !self.types_are_compatible(&target_type, &expr_type) {
                            return Err(format!(
                                "Semantic Error: Cannot assign '{}' to type '{}'",
                                expr_type, target_type
                            ));
                        }
                    }
                }
            }
            Stmt::ExpressionStmt(expr) => {
                self.visit_expression(expr)?;
            }
            Stmt::CaseStmt {
                option: _,
                set: _,
                body,
            } => {
                self.enter_scope();
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
            Stmt::ForInStmt {
                item,
                iterable,
                body,
            } => {
                let iterable_type = self.visit_expression(iterable)?;

                let item_type = if iterable_type.starts_with("array<") {
                    iterable_type
                        .trim_start_matches("array<")
                        .trim_end_matches(">")
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

                if let Stmt::Declaration(crate::frontend::parser::ast::Decl::VarDecl {
                    type_node,
                    ..
                }) = &**item
                {
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
                        for stmt in stmts {
                            self.visit_statement(stmt)?;
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
                    crate::frontend::parser::ast::EitherBlock::Inline(stmts) => {
                        self.enter_scope();
                        self.active_flags.push("+has_break".to_string());
                        for s in stmts {
                            self.visit_statement(s)?;
                        }
                        self.active_flags.retain(|f| f != "+has_break");
                        self.leave_scope();
                    }
                    crate::frontend::parser::ast::EitherBlock::External(expr) => {
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
            Stmt::DelStmt(expr) => {
                self.visit_expression(expr)?;
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
                    crate::frontend::parser::ast::EitherBlock::Inline(stmts) => {
                        self.enter_scope();
                        self.active_flags.push("+has_break".to_string());
                        for s in stmts {
                            self.visit_statement(s)?;
                        }
                        self.active_flags.retain(|f| f != "+has_break");
                        self.leave_scope();
                    }
                    crate::frontend::parser::ast::EitherBlock::External(expr) => {
                        self.visit_expression(expr)?;
                    }
                }
                self.leave_scope();
            }
            Stmt::ReturnStmt(expr) => {
                if !self.active_flags.contains(&"+has_return".to_string()) {
                    return Err("Semantic Error: Return statement is not allowed in this scope. 'has_return' flag is not enabled.".to_string());
                }

                let actual_type = self.visit_expression(expr)?;

                if let Some(expected_type) = &self.active_return_type {
                    let expected_type_str: &str = &expected_type.as_str();

                    if actual_type != "unknown"
                        && expected_type_str != "unknown"
                        && !self.types_are_compatible(&expected_type_str.to_string(), &actual_type)
                    {
                        return Err(format!(
                            "Semantic Error: Return type mismatch. Expected '{}', got '{}'",
                            Self::format_type(expected_type),
                            actual_type
                        ));
                    }
                }
            }
            Stmt::BreakStmt => {
                if !self.active_flags.contains(&"+has_break".to_string()) {
                    return Err("Semantic Error: Break statement is not allowed outside loops or switch statements. 'has_break' flag is not enabled.".to_string());
                }
            }
            Stmt::ThrowStmt(expr) => {
                if !self.active_flags.contains(&"+has_throw".to_string()) {
                    return Err("Semantic Error: Throw statement is not allowed here. 'has_throw' flag is not enabled.".to_string());
                }
                self.visit_expression(expr)?;
            }
            Stmt::TryCatchStmt {
                try_block,
                catch_param,
                catch_block,
            } => {
                self.enter_scope();
                for s in try_block {
                    self.visit_statement(s)?;
                }
                self.leave_scope();

                self.enter_scope();
                let info = SymbolInfo {
                    dependencies: Vec::new(),
                    name: catch_param.clone(),
                    type_node: Some(BaseType::Error),
                    visibility: Visibility::Private,
                    editability: Editability::NotEditable,
                    settings: std::collections::HashSet::new(),
                    is_array: false,
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
            _ => {}
        }
        Ok(())
    }

    fn visit_declaration(&mut self, decl: &Decl) -> Result<(), String> {
        match decl {
            Decl::VarDecl {
                visibility,
                editability,
                type_node,
                name,
                value,
            } => {
                let prev_context = self.current_context.clone();
                self.current_context = Some(name.clone());
                let expr_type = self.visit_expression(value)?;

                let mut final_type_node = type_node.clone();
                let mut declared_type = type_node.as_str();

                if expr_type != "unknown" {
                    let base_decl = declared_type.split('<').next().unwrap_or(&declared_type);
                    match base_decl {
                        "name" => {
                            if expr_type != "unknown" && expr_type != "object" {
                                final_type_node =
                                    BaseType::Name(Box::new(BaseType::from_str(&expr_type)));
                                declared_type = format!("name<{}>", expr_type);
                            }
                        }
                        "blueprint" => {
                            if expr_type != "object" {
                                return Err(format!(
                                    "Semantic Error: 'blueprint' must be initialized with an object literal, got '{}'",
                                    expr_type
                                ));
                            }
                        }
                        "length" | "size" => {
                            if !expr_type.starts_with("int") {
                                return Err(format!(
                                    "Semantic Error: '{}' expects an integer value, got '{}'",
                                    declared_type, expr_type
                                ));
                            }
                        }
                        _ => {
                            if !self.types_are_compatible(&declared_type, &expr_type) {
                                return Err(format!(
                                    "Semantic Error: Type mismatch for '{}'. Declared '{}', got '{}'",
                                    name, declared_type, expr_type
                                ));
                            }
                        }
                    }
                }

                let deps = self
                    .dependency_graph
                    .get(&name.clone())
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .collect();
                let info = SymbolInfo {
                    name: name.clone(),
                    type_node: Some(final_type_node),
                    visibility: visibility.clone(),
                    editability: editability.clone(),
                    settings: std::collections::HashSet::new(),
                    is_array: false,
                    dependencies: deps,
                };

                self.current_env.borrow_mut().define(name.clone(), info)?;
                self.current_context = prev_context;
            }
            Decl::ArrayDecl {
                visibility,
                editability,
                type_node,
                name,
                length,
                value,
            } => {
                let expr_type = self.visit_expression(value)?;
                self.visit_expression(length)?;

                let mut final_type_node = type_node.clone();
                let mut declared_type = type_node.as_str();
                if declared_type == "unknown" {
                    declared_type = expr_type.clone();
                }

                if true {
                    let tn = &mut final_type_node;
                    if let BaseType::Name(ref mut inner) = tn {
                        if **inner == BaseType::Unknown && expr_type != "unknown" {
                            let inner_type = if expr_type.starts_with("array<") {
                                BaseType::Array(Box::new(BaseType::from_str(
                                    expr_type.trim_start_matches("array<").trim_end_matches(">"),
                                )))
                            } else {
                                BaseType::from_str(&expr_type)
                            };
                            **inner = inner_type;
                        }
                    }

                    if let BaseType::Name(_) = tn {
                        if expr_type == "unknown"
                            || expr_type == "array"
                            || expr_type == format!("array<{}>", declared_type)
                            || (declared_type == "char"
                                && (expr_type == "string" || expr_type == "str"))
                        {
                            // Valid
                        } else if !self.types_are_compatible(&declared_type, &expr_type) {
                            return Err(format!(
                                "Semantic Error: Type mismatch for array '{}'. Declared '{}', got '{}'",
                                name, declared_type, expr_type
                            ));
                        }
                    }
                } else {
                    final_type_node = BaseType::from_str(&expr_type);
                }

                let info = SymbolInfo {
                    name: name.clone(),
                    type_node: Some(final_type_node),
                    visibility: visibility.clone(),
                    editability: editability.clone(),
                    settings: std::collections::HashSet::new(),
                    is_array: true,
                    dependencies: Vec::new(),
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
                labels: _,
                length: _,
                data,
                extends: _,
                return_type: _,
                public_block,
                private_block,
                static_block,
                statements,
                label_blocks: _,
                variant_block: _,
                generics: _,
                handle_block,
                constructor,
            } => {
                if let Some(s) = settings as &Option<Vec<crate::frontend::parser::ast::Setting>> {
                    let has_public = s.contains(&crate::frontend::parser::ast::Setting::Public);
                    let has_private = s.contains(&crate::frontend::parser::ast::Setting::Private);
                    let has_stmt = s.contains(&crate::frontend::parser::ast::Setting::Statement);
                    let has_label = s.contains(&crate::frontend::parser::ast::Setting::Label);

                    if has_label && (has_public || has_private || has_stmt) {
                        return Err(format!("Semantic Error: Custom block '{}' is marked as 'label' and cannot be combined with other modifiers.", name));
                    }
                    if has_stmt && (has_public || has_private) {
                        return Err(format!("Semantic Error: Custom block '{}' is marked as 'statement' and cannot have 'public' or 'private' modifiers.", name));
                    }
                }

                let info = SymbolInfo {
                    dependencies: Vec::new(),
                    name: name.clone(),
                    type_node: Some(BaseType::from_str("blueprint")),
                    visibility: if *is_exported {
                        Visibility::Public
                    } else {
                        Visibility::Private
                    },
                    editability: Editability::NotEditable,
                    settings: std::collections::HashSet::new(),
                    is_array: false,
                };
                self.current_env.borrow_mut().define(name.clone(), info)?;

                let prev_in_stmt = self.in_statement_scope;
                self.in_statement_scope = false;
                let prev_return_type = self.active_return_type.clone();
                let prev_flags = self.active_flags.clone();

                self.active_return_type = None;
                if let Some(f_vec) = flags as &Option<Vec<crate::frontend::parser::ast::Flag>> {
                    for flg in (f_vec as &Vec<crate::frontend::parser::ast::Flag>).iter() {
                        let fs: &str = &flg.as_str();
                        self.active_flags.push(format!("+{}", fs));
                    }
                }

                if let Some(h_vec) =
                    handles as &Option<Vec<crate::frontend::parser::ast::HandleMethods>>
                {
                    if h_vec.contains(&crate::frontend::parser::ast::HandleMethods::Error) {
                        self.active_flags.push("+has_throw".to_string());
                        self.active_flags.push("+has_error".to_string());
                    }
                }

                self.enter_scope();
                let prev_in_custom = self.in_custom_scope;
                self.in_custom_scope = true;

                if let Some(_) = data {
                    let data_info = SymbolInfo {
                        name: "data".to_string(),
                        type_node: None,
                        visibility: Visibility::Public,
                        editability: Editability::Editable,
                        settings: std::collections::HashSet::new(),
                        is_array: false,
                        dependencies: Vec::new(),
                    };
                    self.current_env
                        .borrow_mut()
                        .define("data".to_string(), data_info)?;
                }

                let length_info = SymbolInfo {
                    dependencies: Vec::new(),
                    name: "length".to_string(),
                    type_node: Some(BaseType::Int32),
                    visibility: Visibility::Public,
                    editability: Editability::Editable,
                    settings: std::collections::HashSet::new(),
                    is_array: false,
                };
                self.current_env
                    .borrow_mut()
                    .define("length".to_string(), length_info)?;

                if let Some(ref params_vec) = params {
                    for p in params_vec {
                        let param_info = SymbolInfo {
                            name: p.name.clone(),
                            type_node: Some(p.type_node.clone()),
                            visibility: Visibility::Public,
                            editability: Editability::Editable,
                            settings: std::collections::HashSet::new(),
                            is_array: false,
                            dependencies: Vec::new(),
                        };
                        self.current_env
                            .borrow_mut()
                            .define(p.name.clone(), param_info)?;
                    }
                }

                if let Some(ref stmts) = statements {
                    for s in stmts {
                        self.visit_statement(s)?;
                    }
                }

                if let Some(ref decls) = private_block {
                    for d in decls {
                        self.visit_declaration(d)?;
                    }
                }
                if let Some(ref decls) = public_block {
                    for d in decls {
                        self.visit_declaration(d)?;
                    }
                }
                if let Some(ref decls) = static_block {
                    for d in decls {
                        self.visit_declaration(d)?;
                    }
                }

                if let Some(ref constructors) = constructor {
                    for ctor in constructors {
                        self.enter_scope();
                        for param in &ctor.params {
                            let param_info = SymbolInfo {
                                name: param.name.clone(),
                                type_node: Some(param.type_node.clone()),
                                visibility: Visibility::Private,
                                editability: Editability::Editable,
                                settings: std::collections::HashSet::new(),
                                is_array: false,
                                dependencies: Vec::new(),
                            };
                            self.current_env
                                .borrow_mut()
                                .define(param.name.clone(), param_info)?;
                        }
                        let prev_stmt_ctor = self.in_statement_scope;
                        self.in_statement_scope = true;
                        for stmt in &ctor.body {
                            self.visit_statement(stmt)?;
                        }
                        self.in_statement_scope = prev_stmt_ctor;
                        self.leave_scope();
                    }
                }

                if let Some(ref handle_stmts) = handle_block {
                    let allowed_handle_names = [
                        "index_access",
                        "display",
                        "add",
                        "sub",
                        "mul",
                        "div",
                        "mod",
                        "iterator",
                        "next",
                        "length",
                        "size",
                        "call",
                        "label",
                        "goto",
                        "leave",
                        "data",
                        "yield",
                        "has_error",
                    ];
                    for d in handle_stmts {
                        if let Decl::FnDecl { name: fn_name, .. } = d {
                            if !allowed_handle_names.contains(&fn_name.as_str()) {
                                return Err(format!(
                                    "Semantic Error: Invalid handle function name '{}'.",
                                    fn_name
                                ));
                            }
                        } else {
                            return Err("Semantic Error: Only function declarations (fn) are allowed inside a handle block.".to_string());
                        }
                        self.visit_declaration(d)?;
                    }
                }

                self.leave_scope();
                self.active_flags = prev_flags;
                self.active_return_type = prev_return_type;
                self.in_statement_scope = prev_in_stmt;
                self.in_custom_scope = prev_in_custom;
            }
            Decl::ClassDecl {
                is_exported,
                name,
                extends: _,
                handles: _,
                settings: _,
                public_block,
                private_block,
                static_block,
                generics: _,
                handle_block,
                length: _,
                constructor,
            } => {
                let info = SymbolInfo {
                    dependencies: Vec::new(),
                    name: name.clone(),
                    type_node: Some(BaseType::from_str("blueprint")),
                    visibility: if *is_exported {
                        Visibility::Public
                    } else {
                        Visibility::Private
                    },
                    editability: Editability::NotEditable,
                    settings: std::collections::HashSet::new(),
                    is_array: false,
                };
                self.current_env.borrow_mut().define(name.clone(), info)?;
                self.enter_scope();
                let prev_in_custom = self.in_custom_scope;
                self.in_custom_scope = true;

                for d in private_block {
                    self.visit_declaration(d)?;
                }
                for d in public_block {
                    self.visit_declaration(d)?;
                }
                for d in static_block {
                    self.visit_declaration(d)?;
                }
                for d in handle_block {
                    self.visit_declaration(d)?;
                }

                if let Some(ref constructors) = constructor {
                    for ctor in constructors {
                        self.enter_scope();
                        for param in &ctor.params {
                            let param_info = SymbolInfo {
                                name: param.name.clone(),
                                type_node: Some(param.type_node.clone()),
                                visibility: Visibility::Private,
                                editability: Editability::Editable,
                                settings: std::collections::HashSet::new(),
                                is_array: false,
                                dependencies: Vec::new(),
                            };
                            self.current_env
                                .borrow_mut()
                                .define(param.name.clone(), param_info)?;
                        }
                        let prev_stmt_ctor = self.in_statement_scope;
                        self.in_statement_scope = true;

                        for stmt in &ctor.body {
                            self.visit_statement(stmt)?;
                        }
                        self.in_statement_scope = prev_stmt_ctor;
                        self.leave_scope();
                    }
                }
                self.leave_scope();
                self.in_custom_scope = prev_in_custom;
            }
            Decl::StructDecl {
                is_exported,
                name,
                handles: _,
                settings: _,
                public_block,
                private_block,
                handle_block,
                static_block,
                constructor,
            } => {
                let info = SymbolInfo {
                    dependencies: Vec::new(),
                    name: name.clone(),
                    type_node: Some(BaseType::from_str("blueprint")),
                    visibility: if *is_exported {
                        Visibility::Public
                    } else {
                        Visibility::Private
                    },
                    editability: Editability::NotEditable,
                    settings: std::collections::HashSet::new(),
                    is_array: false,
                };
                self.current_env.borrow_mut().define(name.clone(), info)?;
                self.enter_scope();
                let prev_in_custom = self.in_custom_scope;
                self.in_custom_scope = true;

                for d in private_block {
                    self.visit_declaration(d)?;
                }
                for d in public_block {
                    self.visit_declaration(d)?;
                }
                for d in static_block {
                    self.visit_declaration(d)?;
                }
                for d in handle_block {
                    self.visit_declaration(d)?;
                }
                if let Some(ref constructors) = constructor {
                    for ctor in constructors {
                        self.enter_scope();
                        for param in &ctor.params {
                            let param_info = SymbolInfo {
                                name: param.name.clone(),
                                type_node: Some(param.type_node.clone()),
                                visibility: Visibility::Private,
                                editability: Editability::Editable,
                                settings: std::collections::HashSet::new(),
                                is_array: false,
                                dependencies: Vec::new(),
                            };
                            self.current_env
                                .borrow_mut()
                                .define(param.name.clone(), param_info)?;
                        }
                        for stmt in &ctor.body {
                            self.visit_statement(stmt)?;
                        }
                        self.leave_scope();
                    }
                }
                self.leave_scope();
                self.in_custom_scope = prev_in_custom;
            }
            Decl::EnumDecl {
                is_exported,
                name,
                length: _,
                handles: _,
                settings: _,
                handle_block,
                variants: _,
                generics: _,
            } => {
                let info = SymbolInfo {
                    dependencies: Vec::new(),
                    name: name.clone(),
                    type_node: Some(BaseType::from_str("blueprint")),
                    visibility: if *is_exported {
                        Visibility::Public
                    } else {
                        Visibility::Private
                    },
                    editability: Editability::NotEditable,
                    settings: std::collections::HashSet::new(),
                    is_array: false,
                };
                self.current_env.borrow_mut().define(name.clone(), info)?;
                self.enter_scope();
                let prev_in_custom = self.in_custom_scope;
                self.in_custom_scope = true;

                for d in handle_block {
                    self.visit_declaration(d)?;
                }

                self.leave_scope();
                self.in_custom_scope = prev_in_custom;
            }
            Decl::BlockDecl {
                is_exported: _,
                name: _,
                statements,
            } => {
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
                let info = SymbolInfo {
                    dependencies: Vec::new(),
                    name: name.clone(),
                    type_node: Some(BaseType::from_str("blueprint")),
                    visibility: if *is_exported {
                        Visibility::Public
                    } else {
                        Visibility::Private
                    },
                    editability: Editability::NotEditable,
                    settings: std::collections::HashSet::new(),
                    is_array: false,
                };
                self.current_env.borrow_mut().define(name.clone(), info)?;
                let previous_flags = self.active_flags.clone();
                let previous_return_type = self.active_return_type.clone();
                self.active_flags.push("+has_return".to_string());
                self.active_flags.push("+has_throw".to_string());
                self.active_return_type = Some(return_type.clone());
                self.enter_scope();
                for p in params {
                    let param_info = SymbolInfo {
                        name: p.name.clone(),
                        type_node: Some(p.type_node.clone()),
                        visibility: Visibility::Public,
                        editability: Editability::Editable,
                        settings: std::collections::HashSet::new(),
                        is_array: false,
                        dependencies: Vec::new(),
                    };
                    self.current_env
                        .borrow_mut()
                        .define(p.name.clone(), param_info)?;
                }
                for s in body {
                    self.visit_statement(s)?;
                }
                self.leave_scope();
                self.active_flags = previous_flags;
                self.active_return_type = previous_return_type;
            }
            Decl::LabelDecl { name, body } => {
                let info = SymbolInfo {
                    dependencies: Vec::new(),
                    name: name.clone(),
                    type_node: Some(BaseType::from_str("label")),
                    visibility: Visibility::Private,
                    editability: Editability::NotEditable,
                    settings: std::collections::HashSet::new(),
                    is_array: false,
                };
                self.current_env.borrow_mut().define(name.clone(), info)?;

                self.enter_scope();
                for s in body {
                    self.visit_statement(s)?;
                }
                self.leave_scope();
            }
            Decl::BlueprintDecl {
                name,
                definition: _,
                ..
            } => {
                let info = SymbolInfo {
                    dependencies: Vec::new(),
                    name: name.clone(),
                    type_node: Some(BaseType::from_str("blueprint")),
                    visibility: Visibility::Public,
                    editability: Editability::NotEditable,
                    settings: std::collections::HashSet::new(),
                    is_array: false,
                };
                self.current_env.borrow_mut().define(name.clone(), info)?;
            }
            Decl::ImplDecl { target, methods } => {
                let lookup = self.current_env.borrow().lookup(target);
                if lookup.is_none() {
                    return Err(format!(
                        "Semantic Error: Blueprint '{}' not found for impl block",
                        target
                    ));
                }

                self.enter_scope();
                self.active_flags.push("+has_return".to_string());
                let prev_in_struct = self.in_struct;
                self.in_struct = true;

                for m in methods {
                    self.visit_declaration(m)?;
                }

                self.in_struct = prev_in_struct;
                self.active_flags.retain(|x| x != "+has_return");
                self.leave_scope();
            }
            Decl::NameDecl {
                name,
                inner_type,
                target,
                access_mode,
                is_heap: _,
            } => {
                let prev_context = self.current_context.clone();
                self.current_context = Some(name.clone());
                let target_type = self.visit_expression(target)?;

                let mut final_inner = inner_type.clone();
                if final_inner == BaseType::Unknown && target_type != "unknown" {
                    final_inner = BaseType::from_str(&target_type);
                }

                let info = SymbolInfo {
                    name: name.clone(),
                    type_node: Some(BaseType::Name(Box::new(final_inner))),
                    visibility: Visibility::Private,
                    editability: Editability::Editable,
                    settings: std::collections::HashSet::new(),
                    is_array: false,
                    dependencies: Vec::new(),
                };
                self.current_env.borrow_mut().define(name.clone(), info)?;
                self.current_context = prev_context;
            }
            Decl::PointerDecl {
                name,
                inner_type,
                length,
                value,
            } => {
                let prev_context = self.current_context.clone();
                self.current_context = Some(name.clone());
                let _val_type = self.visit_expression(value)?;
                if let Some(l) = length {
                    self.visit_expression(l)?;
                }

                let info = SymbolInfo {
                    name: name.clone(),
                    type_node: Some(BaseType::Pointer(Box::new(inner_type.clone()))),
                    visibility: Visibility::Private,
                    editability: Editability::Editable,
                    settings: std::collections::HashSet::new(),
                    is_array: length.is_some(),
                    dependencies: Vec::new(),
                };
                self.current_env.borrow_mut().define(name.clone(), info)?;
                self.current_context = prev_context;
            }
            _ => {}
        }
        Ok(())
    }

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

                let mut element_type = None;
                for el in elements {
                    let inferred = self.visit_expression(el)?;
                    match &element_type {
                        None => element_type = Some(inferred),
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
                if name.trim() == "__default__" || name.trim() == "None" || name.trim() == "null" {
                    return Ok("unknown".to_string());
                }
                self.record_dependency(name.clone());
                println!("DEBUG: Checking identifier '{}', len: {}", name, name.len());
                match self.current_env.borrow().lookup(name) {
                    Some(info) => {
                        let mut type_str = info
                            .type_node
                            .as_ref()
                            .map(|t: &crate::frontend::parser::ast::BaseType| t.as_str())
                            .unwrap_or_else(|| "unknown".to_string());
                        if info.is_array {
                            type_str = format!("array<{}>", type_str);
                        }
                        return Ok(type_str);
                    }
                    None => {
                        if self.in_struct || self.in_class || self.in_custom_scope {
                            return Ok("auto".to_string());
                        }
                        return Err(format!(
                            "Semantic Error: Identifier '{}' is not defined in this scope.",
                            name
                        ));
                    }
                }
            }
            Expr::NamespaceAccess {
                namespace,
                property,
            } => {
                if let Some(metadata) = self.global_metadata.get(namespace) {
                    let prop_name =
                        match &**property {
                            Expr::Identifier(prop_name) => prop_name.clone(),
                            _ => return Err(
                                "Semantic Error: Expected identifier for static property access"
                                    .to_string(),
                            ),
                        };
                    if let Some(t_node) = metadata.fields.get(&prop_name) {
                        return Ok(t_node.as_str());
                    }
                }
                Ok("unknown".to_string())
            }
            Expr::Call { callee, args } => {
                if let Expr::Identifier(name) = &**callee {
                    println!("DEBUG: Expr::Call callee is {}", name);
                    if let Some(info) = self.current_env.borrow().lookup(name) {
                        println!("DEBUG: Found callee in env: {:?}", info.type_node);
                        if let Some(type_ref) = &info.type_node {
                            let base_str = type_ref.as_str();
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
                    return Err("Semantic Error: Cannot use 'this' outside of a class, struct, or custom scope".to_string());
                }
                if let Some(name) = &self.current_type_name {
                    Ok(name.clone())
                } else {
                    Ok("object".to_string())
                }
            }
            Expr::Global => {
                if self.current_env.borrow().parent.is_none() {
                    eprintln!("Warning: Using 'global' in the global scope is redundant and considered ugly code.");
                }
                Ok("object".to_string())
            }
            Expr::Super => {
                if !self.in_class {
                    return Err("Semantic Error: Cannot use 'super' outside of a class".to_string());
                }
                Ok("object".to_string())
            }
            Expr::Modify { target } => {
                let t = self.visit_expression(target)?;
                Ok(t)
            }
            Expr::Copy { target } => {
                let t = self.visit_expression(target)?;
                Ok(t)
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
            _ => Ok("unknown".to_string()),
        }
    }

    fn validate_magic_type_assignment(&self, magic: &str, source: &str) -> Result<(), String> {
        match magic {
            "length" | "size" => {
                if !["str", "array", "unknown"].contains(&source) {
                    return Err(format!(
                        "Semantic Error: '{}' can only be used with array, str. Got '{}'",
                        magic, source
                    ));
                }
            }
            "param" => {}
            "init" => {}
            _ => {}
        }
        Ok(())
    }

    fn types_are_compatible(&self, expected: &str, actual: &str) -> bool {
        if expected == actual || expected == "any" || actual == "unknown" || actual == "auto" {
            return true;
        }
        if expected.starts_with("int") && actual.starts_with("int") {
            return true;
        }
        if expected.starts_with("float") && actual.starts_with("float") {
            return true;
        }
        if expected.starts_with("custom<") {
            let inner = expected.trim_start_matches("custom<").trim_end_matches(">");
            if inner == actual {
                return true;
            }
        }

        if expected.starts_with("object<") {
            let inner = expected.trim_start_matches("object<").trim_end_matches(">");
            if inner == actual {
                return true;
            }
        }
        if expected == "name" || expected == "name<unknown>" {
            return true;
        }
        if expected.starts_with("name<") {
            let inner = expected.trim_start_matches("name<").trim_end_matches(">");
            return inner == actual;
        }
        if (expected == "string" || expected == "custom<string>")
            && (actual == "str" || actual == "string")
        {
            return true;
        }
        false
    }

    fn format_type(type_node: &crate::frontend::parser::ast::BaseType) -> String {
        format!("{}", type_node.as_str())
    }
}
