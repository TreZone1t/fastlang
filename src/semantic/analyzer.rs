use crate::parser::ast::{Expr, Stmt, TypeRef};
use crate::parser::parser::Parser;
use crate::semantic::environment::{Environment, SymbolInfo};
use std::cell::RefCell;
use std::rc::Rc;
pub struct SemanticAnalyzer {
    pub current_env: Rc<RefCell<Environment>>,
    pub in_class: bool,
    pub in_struct: bool,
    pub in_custom_scope: bool,
    pub in_statement_scope: bool,
    pub active_flags: Vec<String>,
    /// Declared return type of the function-like scope currently being analyzed.
    pub active_return_type: Option<crate::parser::ast::TypeNode>,
}

impl SemanticAnalyzer {
    pub fn new() -> Self {
        let mut analyzer = SemanticAnalyzer {
            current_env: Environment::new(),
            in_class: false,
            in_struct: false,
            in_custom_scope: false,
            in_statement_scope: false,
            active_flags: vec!["+is_exit".to_string()],
            active_return_type: None,
        };
        analyzer.inject_stdlib();
        analyzer
    }

    fn inject_stdlib(&mut self) {
        let std_funcs = vec![
            ("log", "void"),
            ("input", "string"),
            // Can add more here later like math_sin, etc.
        ];

        for (name, _ret_type) in std_funcs {
            let info = SymbolInfo {
                name: name.to_string(),
                type_node: Some(crate::parser::ast::TypeNode::Simple(
                    crate::parser::ast::TypeRef {
                        base_type: "fn".to_string(),
                        size: None,
                    },
                )),
                visibility: if true {
                    crate::parser::ast::Visibility::Public
                } else {
                    crate::parser::ast::Visibility::Private
                },
                editability: if true {
                    crate::parser::ast::Editability::NotEditable
                } else {
                    crate::parser::ast::Editability::Editable
                },
                settings: std::collections::HashSet::new(),
            };
            // Define standard library function in the global environment
            let _ = self.current_env.borrow_mut().define(name.to_string(), info);
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
            Stmt::VarDecl {
                visibility,
                editability,
                type_node,
                name,
                value,
            } => {
                let expr_type = self.visit_expression(value)?;

                let declared_type = type_node
                    .clone()
                    .map(|t| match t {
                        crate::parser::ast::TypeNode::Simple(r) => r.base_type.clone(),
                        crate::parser::ast::TypeNode::Generic(g) => g.base_type.clone(),
                    })
                    .unwrap_or_else(|| "unknown".to_string());

                if expr_type != "unknown" {
                    match declared_type.as_str() {
                        "name" => {
                            // المؤشرات تقبل أي شيء، لا حاجة لـ Type Checking
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
                            // التحقق الديناميكي العام (بدون أي استثناءات Hardcoded للـ Array أو الـ Str)
                            if !self.types_are_compatible(&declared_type, &expr_type) {
                                return Err(format!(
                        "Semantic Error: Type mismatch for '{}'. Declared '{}', got '{}'",
                        name, declared_type, expr_type
                    ));
                            }
                        }
                    }
                }

                let info = SymbolInfo {
                    name: name.clone(),
                    type_node: type_node.clone(),
                    visibility: visibility.clone(),
                    editability: editability.clone(),
                    settings: std::collections::HashSet::new(),
                };

                println!(
                    "DEBUG: Defining variable '{}' of type '{}' in environment",
                    name, declared_type
                );
                self.current_env.borrow_mut().define(name.clone(), info)?;
            }
            Stmt::ReassignStmt { target, value } => {
                let expr_type = self.visit_expression(value)?;
                let target_type = self.visit_expression(target)?;

                if let Expr::Identifier(name) = target {
                    if let Some(info) = self.current_env.borrow().lookup(name) {
                        if info.editability == crate::parser::ast::Editability::NotEditable {
                            return Err(format!(
                                "Semantic Error: Cannot reassign constant '{}'",
                                name
                            ));
                        }
                    }
                }

                if target_type != expr_type && target_type != "unknown" && expr_type != "unknown" {
                    if target_type == "name" && expr_type == "object" {
                        // السماح بتعيين object لـ name
                    } else if target_type == "type" || target_type.starts_with("type<") {
                        // السماح بتعيين الأنواع
                    } else {
                        // الاعتماد الكامل على نظام الأنواع دون هارد كود
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
            Stmt::CustomDecl {
                is_exported,
                name,
                settings: _,
                handles: _,
                params,
                flags,
                length: _,
                data: _,
                extends: _,
                events: _,
                fields,
                return_type: _,
                public_block,
                private_block,
                static_block,
                statements,
                variant_block: _,
                generic_block,
                handle_block,
                constructor,
            } => {
                let info = SymbolInfo {
                    name: name.clone(),
                    type_node: Some(crate::parser::ast::TypeNode::Simple(
                        crate::parser::ast::TypeRef {
                            base_type: "blueprint".to_string(),
                            size: None,
                        },
                    )),
                    visibility: if *is_exported {
                        crate::parser::ast::Visibility::Public
                    } else {
                        crate::parser::ast::Visibility::Private
                    },
                    editability: crate::parser::ast::Editability::NotEditable,
                    settings: std::collections::HashSet::new(),
                };
                self.current_env.borrow_mut().define(name.clone(), info)?;

                let prev_in_stmt = self.in_statement_scope;
                self.in_statement_scope = false;
                let prev_return_type = self.active_return_type.clone();
                let prev_flags = self.active_flags.clone();

                self.active_return_type = None;
                if let Some(ref f_vec) = flags {
                    for flg in f_vec {
                        self.active_flags.push(format!("+{}", flg.as_str()));
                    }
                }

                self.enter_scope();
                let prev_in_custom = self.in_custom_scope;
                self.in_custom_scope = true;

                if let Some(ref fields_vec) = fields {
                    for field in fields_vec {
                        let field_info = SymbolInfo {
                            name: field.name.clone(),
                            type_node: field.type_node.clone(),
                            visibility: field.visibility.clone(),
                            editability: field.editability.clone(),
                            settings: std::collections::HashSet::new(),
                        };
                        self.current_env
                            .borrow_mut()
                            .define(field.name.clone(), field_info)?;
                    }
                }

                if let Some(ref params_vec) = params {
                    for p in params_vec {
                        let param_info = SymbolInfo {
                            name: p.name.clone(),
                            type_node: p.type_node.clone(),
                            visibility: crate::parser::ast::Visibility::Public,
                            editability: crate::parser::ast::Editability::Editable,
                            settings: std::collections::HashSet::new(),
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
                if let Some(ref stmts) = private_block {
                    for s in stmts {
                        self.visit_statement(s)?;
                    }
                }
                if let Some(ref stmts) = public_block {
                    for s in stmts {
                        self.visit_statement(s)?;
                    }
                }
                if let Some(ref stmts) = static_block {
                    for s in stmts {
                        self.visit_statement(s)?;
                    }
                }
                if let Some(ref stmts) = generic_block {
                    for s in stmts {
                        self.visit_statement(s)?;
                    }
                }

                if let Some(ref constructor) = constructor {
                    self.enter_scope();
                    for param in &constructor.params {
                        let param_info = SymbolInfo {
                            name: param.name.clone(),
                            type_node: param.type_node.clone(),
                            visibility: crate::parser::ast::Visibility::Private,
                            editability: crate::parser::ast::Editability::Editable,
                            settings: std::collections::HashSet::new(),
                        };
                        self.current_env
                            .borrow_mut()
                            .define(param.name.clone(), param_info)?;
                    }
                    let prev_stmt_ctor = self.in_statement_scope;
                    self.in_statement_scope = true;
                    for statement in &constructor.body {
                        self.visit_statement(statement)?;
                    }
                    self.in_statement_scope = prev_stmt_ctor;
                    self.leave_scope();
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
                    ];
                    for s in handle_stmts {
                        if let Stmt::FnDecl { name: fn_name, .. } = s {
                            if !allowed_handle_names.contains(&fn_name.as_str()) {
                                return Err(format!(
                                    "Semantic Error: Invalid handle function name '{}'.",
                                    fn_name
                                ));
                            }
                        } else {
                            return Err("Semantic Error: Only function declarations (fn) are allowed inside a handle block.".to_string());
                        }
                        self.visit_statement(s)?;
                    }
                }

                self.leave_scope();

                self.active_flags = prev_flags;
                self.active_return_type = prev_return_type;
                self.in_statement_scope = prev_in_stmt;
                self.in_custom_scope = prev_in_custom;
            }
            Stmt::ClassDecl {
                is_exported,
                name,
                extends: _,
                handles: _,
                settings: _,
                public_block,
                private_block,
                static_block,
                generic_block,
                handle_block,
                length: _,
                constructor,
            } => {
                let info = SymbolInfo {
                    name: name.clone(),
                    type_node: Some(crate::parser::ast::TypeNode::Simple(
                        crate::parser::ast::TypeRef {
                            base_type: "blueprint".to_string(),
                            size: None,
                        },
                    )),
                    visibility: if *is_exported {
                        crate::parser::ast::Visibility::Public
                    } else {
                        crate::parser::ast::Visibility::Private
                    },
                    editability: crate::parser::ast::Editability::NotEditable,
                    settings: std::collections::HashSet::new(),
                };
                self.current_env.borrow_mut().define(name.clone(), info)?;
                self.enter_scope();
                let prev_in_custom = self.in_custom_scope;
                self.in_custom_scope = true;
                for s in private_block {
                    self.visit_statement(s)?;
                }
                for s in public_block {
                    self.visit_statement(s)?;
                }
                for s in static_block {
                    self.visit_statement(s)?;
                }
                for s in generic_block {
                    self.visit_statement(s)?;
                }
                for s in handle_block {
                    self.visit_statement(s)?;
                }
                if let Some(ref ctor) = constructor {
                    self.enter_scope();
                    for param in &ctor.params {
                        let param_info = SymbolInfo {
                            name: param.name.clone(),
                            type_node: param.type_node.clone(),
                            visibility: crate::parser::ast::Visibility::Private,
                            editability: crate::parser::ast::Editability::Editable,
                            settings: std::collections::HashSet::new(),
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
                self.leave_scope();
                self.in_custom_scope = prev_in_custom;
            }
            Stmt::ArrayDecl {
                is_exported,
                name,
                length: _,
                data: _,
                handles: _,
                settings: _,
                public_block,
                private_block,
                generic_block,
                handle_block,
                constructor,
            } => {
                let info = SymbolInfo {
                    name: name.clone(),
                    type_node: Some(crate::parser::ast::TypeNode::Simple(
                        crate::parser::ast::TypeRef {
                            base_type: "blueprint".to_string(),
                            size: None,
                        },
                    )),
                    visibility: if *is_exported {
                        crate::parser::ast::Visibility::Public
                    } else {
                        crate::parser::ast::Visibility::Private
                    },
                    editability: crate::parser::ast::Editability::NotEditable,
                    settings: std::collections::HashSet::new(),
                };
                self.current_env.borrow_mut().define(name.clone(), info)?;
                self.enter_scope();
                let prev_in_custom = self.in_custom_scope;
                self.in_custom_scope = true;
                for s in private_block {
                    self.visit_statement(s)?;
                }
                for s in public_block {
                    self.visit_statement(s)?;
                }
                for s in generic_block {
                    self.visit_statement(s)?;
                }
                for s in handle_block {
                    self.visit_statement(s)?;
                }
                if let Some(ref ctor) = constructor {
                    self.enter_scope();
                    for param in &ctor.params {
                        let param_info = SymbolInfo {
                            name: param.name.clone(),
                            type_node: param.type_node.clone(),
                            visibility: crate::parser::ast::Visibility::Private,
                            editability: crate::parser::ast::Editability::Editable,
                            settings: std::collections::HashSet::new(),
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
                self.leave_scope();
                self.in_custom_scope = prev_in_custom;
            }
            Stmt::StrDecl {
                is_exported,
                name,
                length: _,
                data: _,
                handles: _,
                settings: _,
                public_block,
                private_block,
                handle_block,
                constructor,
            } => {
                let info = SymbolInfo {
                    name: name.clone(),
                    type_node: Some(crate::parser::ast::TypeNode::Simple(
                        crate::parser::ast::TypeRef {
                            base_type: "blueprint".to_string(),
                            size: None,
                        },
                    )),
                    visibility: if *is_exported {
                        crate::parser::ast::Visibility::Public
                    } else {
                        crate::parser::ast::Visibility::Private
                    },
                    editability: crate::parser::ast::Editability::NotEditable,
                    settings: std::collections::HashSet::new(),
                };
                self.current_env.borrow_mut().define(name.clone(), info)?;
                self.enter_scope();
                let prev_in_custom = self.in_custom_scope;
                self.in_custom_scope = true;
                for s in private_block {
                    self.visit_statement(s)?;
                }
                for s in public_block {
                    self.visit_statement(s)?;
                }
                for s in handle_block {
                    self.visit_statement(s)?;
                }
                if let Some(ref ctor) = constructor {
                    self.enter_scope();
                    for param in &ctor.params {
                        let param_info = SymbolInfo {
                            name: param.name.clone(),
                            type_node: param.type_node.clone(),
                            visibility: crate::parser::ast::Visibility::Private,
                            editability: crate::parser::ast::Editability::Editable,
                            settings: std::collections::HashSet::new(),
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
                self.leave_scope();
                self.in_custom_scope = prev_in_custom;
            }
            Stmt::StructDecl {
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
                    name: name.clone(),
                    type_node: Some(crate::parser::ast::TypeNode::Simple(
                        crate::parser::ast::TypeRef {
                            base_type: "blueprint".to_string(),
                            size: None,
                        },
                    )),
                    visibility: if *is_exported {
                        crate::parser::ast::Visibility::Public
                    } else {
                        crate::parser::ast::Visibility::Private
                    },
                    editability: crate::parser::ast::Editability::NotEditable,
                    settings: std::collections::HashSet::new(),
                };
                self.current_env.borrow_mut().define(name.clone(), info)?;
                self.enter_scope();
                let prev_in_custom = self.in_custom_scope;
                self.in_custom_scope = true;
                for s in private_block {
                    self.visit_statement(s)?;
                }
                for s in public_block {
                    self.visit_statement(s)?;
                }
                for s in static_block {
                    self.visit_statement(s)?;
                }
                for s in handle_block {
                    self.visit_statement(s)?;
                }
                if let Some(ref ctor) = constructor {
                    self.enter_scope();
                    for param in &ctor.params {
                        let param_info = SymbolInfo {
                            name: param.name.clone(),
                            type_node: param.type_node.clone(),
                            visibility: crate::parser::ast::Visibility::Private,
                            editability: crate::parser::ast::Editability::Editable,
                            settings: std::collections::HashSet::new(),
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
                self.leave_scope();
                self.in_custom_scope = prev_in_custom;
            }
            Stmt::EnumDecl {
                is_exported,
                name,
                length: _,
                handles: _,
                settings: _,
                handle_block,
                variants: _,
            } => {
                let info = SymbolInfo {
                    name: name.clone(),
                    type_node: Some(crate::parser::ast::TypeNode::Simple(
                        crate::parser::ast::TypeRef {
                            base_type: "blueprint".to_string(),
                            size: None,
                        },
                    )),
                    visibility: if *is_exported {
                        crate::parser::ast::Visibility::Public
                    } else {
                        crate::parser::ast::Visibility::Private
                    },
                    editability: crate::parser::ast::Editability::NotEditable,
                    settings: std::collections::HashSet::new(),
                };
                self.current_env.borrow_mut().define(name.clone(), info)?;
                self.enter_scope();
                let prev_in_custom = self.in_custom_scope;
                self.in_custom_scope = true;
                for s in handle_block {
                    self.visit_statement(s)?;
                }
                self.leave_scope();
                self.in_custom_scope = prev_in_custom;
            }
            Stmt::BlockDecl {
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
            Stmt::CaseStmt { body, .. } => {
                self.enter_scope();
                for s in body {
                    self.visit_statement(s)?;
                }
                self.leave_scope();
            }
            Stmt::FnDecl {
                is_exported,
                name,
                params,
                return_type,
                body,
            } => {
                let info = SymbolInfo {
                    name: name.clone(),
                    type_node: Some(crate::parser::ast::TypeNode::Simple(
                        crate::parser::ast::TypeRef {
                            base_type: "blueprint".to_string(),
                            size: None,
                        },
                    )),
                    visibility: if *is_exported {
                        crate::parser::ast::Visibility::Public
                    } else {
                        crate::parser::ast::Visibility::Private
                    },
                    editability: crate::parser::ast::Editability::NotEditable,
                    settings: std::collections::HashSet::new(),
                };
                self.current_env.borrow_mut().define(name.clone(), info)?;
                let previous_flags = self.active_flags.clone();
                let previous_return_type = self.active_return_type.clone();
                self.active_flags.push("+is_return".to_string());
                self.active_flags.push("+is_throw".to_string());
                self.active_return_type = Some(return_type.clone());
                self.enter_scope();
                for p in params {
                    let param_info = SymbolInfo {
                        name: p.name.clone(),
                        type_node: p.type_node.clone(),
                        visibility: crate::parser::ast::Visibility::Public,
                        editability: crate::parser::ast::Editability::Editable,
                        settings: std::collections::HashSet::new(),
                    };
                    self.current_env
                        .borrow_mut()
                        .define(p.name.clone(), param_info)?;
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
            Stmt::WhileStmt { condition, body } => {
                let cond_type = self.visit_expression(condition)?;
                if cond_type != "bool" && cond_type != "unknown" {
                    return Err("Semantic Error: loop condition must be a boolean".to_string());
                }
                match body {
                    crate::parser::ast::EitherBlock::Inline(stmts) => {
                        self.enter_scope();
                        for s in stmts {
                            self.visit_statement(s)?;
                        }
                        self.leave_scope();
                    }
                    crate::parser::ast::EitherBlock::External(expr) => {
                        self.visit_expression(expr)?;
                    }
                }
            }
            Stmt::SwitchStmt {
                condition, cases, ..
            } => {
                self.visit_expression(condition)?;
                self.enter_scope();
                for s in cases {
                    self.visit_statement(s)?;
                }
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
                    self.visit_expression(inc)?;
                }
                match body {
                    crate::parser::ast::EitherBlock::Inline(stmts) => {
                        self.enter_scope();
                        for s in stmts {
                            self.visit_statement(s)?;
                        }
                        self.leave_scope();
                    }
                    crate::parser::ast::EitherBlock::External(expr) => {
                        self.visit_expression(expr)?;
                    }
                }
                self.leave_scope();
            }
            Stmt::ReturnStmt(expr) => {
                if !self.active_flags.contains(&"+is_return".to_string()) {
                    return Err("Semantic Error: Return statement is not allowed in this scope. 'is_return' flag is not enabled.".to_string());
                }

                let actual_type = self.visit_expression(expr)?;

                if let Some(expected_type) = &self.active_return_type {
                    let expected_type_str = match expected_type {
                        crate::parser::ast::TypeNode::Simple(r) => &r.base_type,
                        crate::parser::ast::TypeNode::Generic(g) => &g.base_type,
                    };

                    // تحقق صارم وديناميكي للمخرجات
                    if actual_type != "unknown"
                        && expected_type_str != "unknown"
                        && !self.types_are_compatible(expected_type_str, &actual_type)
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
                if !self.active_flags.contains(&"+is_break".to_string()) {
                    return Err("Semantic Error: Break statement is not allowed here. 'is_break' flag is not enabled.".to_string());
                }
            }
            Stmt::ThrowStmt(expr) => {
                if !self.active_flags.contains(&"+is_throw".to_string()) {
                    return Err("Semantic Error: Throw statement is not allowed here. 'is_throw' flag is not enabled.".to_string());
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
                    name: catch_param.clone(),
                    type_node: Some(crate::parser::ast::TypeNode::Simple(
                        crate::parser::ast::TypeRef {
                            base_type: "error".to_string(),
                            size: None,
                        },
                    )),
                    visibility: if false {
                        crate::parser::ast::Visibility::Public
                    } else {
                        crate::parser::ast::Visibility::Private
                    },
                    editability: if true {
                        crate::parser::ast::Editability::NotEditable
                    } else {
                        crate::parser::ast::Editability::Editable
                    },
                    settings: std::collections::HashSet::new(),
                };
                self.current_env
                    .borrow_mut()
                    .define(catch_param.clone(), info)?;
                for s in catch_block {
                    self.visit_statement(s)?;
                }
                self.leave_scope();
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
                for el in elements {
                    self.visit_expression(el)?;
                }
                Ok("list".to_string())
            }
            Expr::ObjectLiteral(stmts) => {
                // Should return "object"
                self.enter_scope();
                for s in stmts {
                    self.visit_statement(s)?;
                }
                self.leave_scope();
                Ok("object".to_string())
            }
            Expr::Identifier(name) => {
                if name.trim() == "__param__" || name.trim() == "None" || name.trim() == "null" {
                    return Ok("unknown".to_string());
                }
                println!("DEBUG: Checking identifier '{}', len: {}", name, name.len());
                match self.current_env.borrow().lookup(name) {
                    Some(info) => Ok(info
                        .type_node
                        .map(|t| match t {
                            crate::parser::ast::TypeNode::Simple(r) => r.base_type.clone(),
                            crate::parser::ast::TypeNode::Generic(g) => g.base_type.clone(),
                        })
                        .unwrap_or_else(|| "unknown".to_string())),
                    None => Err(format!(
                        "Semantic Error: Variable '{}' is not defined in this scope.",
                        name
                    )),
                }
            }
            Expr::Instantiate { .. } => {
                // If target is Dog, it resolves to object
                Ok("object".to_string())
            }
            Expr::BinaryOp {
                left,
                operator,
                right,
            } => {
                let l_type = self.visit_expression(left)?;
                let r_type = self.visit_expression(right)?;
                if l_type != r_type && l_type != "unknown" && r_type != "unknown" {
                    return Err(format!(
                        "Semantic Error: Type mismatch in binary operation: '{}' and '{}'",
                        l_type, r_type
                    ));
                }
                if ["<", ">", "<=", ">=", "==", "!="].contains(&operator.as_str()) {
                    Ok("bool".to_string())
                } else {
                    Ok(l_type)
                }
            }
            Expr::UnaryOp {
                operator: _,
                operand,
            } => self.visit_expression(operand),
            Expr::IndexAccess { object, index } => {
                let obj_type = self.visit_expression(object)?;
                if obj_type != "list"
                    && obj_type != "string"
                    && obj_type != "str"
                    && obj_type != "unknown"
                    && !obj_type.starts_with("array")
                {
                    return Err(format!(
                        "Semantic Error: Cannot index into type '{}'",
                        obj_type
                    ));
                }
                let idx_type = self.visit_expression(index)?;
                if idx_type != "int" && idx_type != "unknown" {
                    return Err(format!(
                        "Semantic Error: Array index must be an int, got '{}'",
                        idx_type
                    ));
                }
                // We return unknown because lists can hold anything unless generics are implemented
                Ok("unknown".to_string())
            }
            Expr::PropertyAccess { object, property } => {
                let _obj_type = self.visit_expression(object)?;
                if matches!(&**object, Expr::This) {
                    let env = self.current_env.borrow();
                    let lookup_res = env.lookup(property);
                    if lookup_res.is_none() {
                        let parent_keys = if let Some(ref p) = env.parent {
                            p.borrow().symbols.keys().cloned().collect::<Vec<_>>()
                        } else {
                            vec![]
                        };
                        println!("DEBUG: Failed to lookup '{}' in environment. Env symbols: {:?}, Parent symbols: {:?}", property, env.symbols.keys().collect::<Vec<_>>(), parent_keys);
                    }
                    return lookup_res
                        .map(|info| {
                            info.type_node
                                .map(|t| match t {
                                    crate::parser::ast::TypeNode::Simple(r) => r.base_type.clone(),
                                    crate::parser::ast::TypeNode::Generic(g) => g.base_type.clone(),
                                })
                                .unwrap_or_else(|| "unknown".to_string())
                        })
                        .ok_or_else(|| {
                            format!(
                                "Semantic Error: Field '{}' is not defined in this scope.",
                                property
                            )
                        });
                }
                // Any property access expects an object, struct, list, str, or string
                Ok("unknown".to_string())
            }
            Expr::NamespaceAccess {
                namespace: _,
                property: _,
            } => {
                // Static access or module access
                Ok("unknown".to_string())
            }
            Expr::Call { callee, args } => {
                if let Expr::Identifier(name) = &**callee {
                    println!("DEBUG: Expr::Call callee is {}", name);
                    if let Some(info) = self.current_env.borrow().lookup(name) {
                        println!("DEBUG: Found callee in env: {:?}", info.type_node);
                        if let Some(type_ref) = &info.type_node {
                            let base = match type_ref {
                                crate::parser::ast::TypeNode::Simple(r) => &r.base_type,
                                crate::parser::ast::TypeNode::Generic(g) => &g.base_type,
                            };
                            if base == "type" || base.starts_with("type<") {
                                // It's a type instantiation (e.g. T(size))
                                // Don't visit arguments as normal expressions
                                return Ok(name.clone());
                            }
                        }
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
                // Type inference on return types requires function symbol table
                Ok("unknown".to_string())
            }
            Expr::This => {
                if !self.in_class && !self.in_struct && !self.in_custom_scope {
                    return Err("Semantic Error: Cannot use 'this' outside of a class, struct, or custom scope".to_string());
                }
                Ok("object".to_string())
            }
            Expr::Global => {
                if self.current_env.borrow().parent.is_none() {
                    eprintln!("Warning: Using 'global' in the global scope is redundant and considered ugly code.");
                }
                Ok("object".to_string()) // Or perhaps "global" type depending on future needs
            }
            Expr::Super => {
                if !self.in_class {
                    return Err("Semantic Error: Cannot use 'super' outside of a class".to_string());
                }
                Ok("object".to_string())
            }
            _ => Ok("unknown".to_string()),
        }
    }

    fn validate_magic_type_assignment(&self, magic: &str, source: &str) -> Result<(), String> {
        match magic {
            "length" | "size" => {
                if !["list", "str", "string", "unknown"].contains(&source) {
                    return Err(format!(
                        "Semantic Error: '{}' can only be used with list, str, or string. Got '{}'",
                        magic, source
                    ));
                }
            }
            "param" => {
                // Param can take functions/scopes etc. For now we are lenient.
            }
            "init" => {
                // Init takes a struct/class
            }
            _ => {}
        }
        Ok(())
    }

    fn types_are_compatible(&self, expected: &str, actual: &str) -> bool {
        actual == "unknown" || expected == actual || (expected == "string" && actual == "str")
    }

    fn format_type(type_node: &crate::parser::ast::TypeNode) -> String {
        let type_ref = match type_node {
            crate::parser::ast::TypeNode::Simple(r) => r,
            crate::parser::ast::TypeNode::Generic(g) => return format!("{}<...>", g.base_type),
        };
        match type_ref.size {
            Some(size) => format!("{}({})", type_ref.base_type, size),
            None => type_ref.base_type.clone(),
        }
    }
}
