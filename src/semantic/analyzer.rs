use crate::parser::ast::*;
use crate::semantic::environment::{Environment, SymbolInfo};
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
    /// Declared return type of the function-like scope currently being analyzed.
    pub active_return_type: Option<TypeNode>,
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
            // Can add more here later like math_sin, etc.
        ];

        for (name, _ret_type) in std_funcs {
            let info = SymbolInfo { dependencies: Vec::new(), 
                name: name.to_string(),
                type_node: Some(TypeNode::Simple(TypeRef {
                    base_type: BaseType::from_str("fn"),
                    size: None,
                })),
                visibility: if true {
                    Visibility::Public
                } else {
                    Visibility::Private
                },
                editability: if true {
                    Editability::NotEditable
                } else {
                    crate::parser::ast::Editability::Editable
                },
                settings: std::collections::HashSet::new(),
                is_array: false,
            };
            // Define standard library function in the global environment
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
            Stmt::VarDecl {
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
                let mut declared_type = type_node
                    .clone()
                    .map(|t| match t {
                        crate::parser::ast::TypeNode::Simple(r) => r.base_type.as_str(),
                        crate::parser::ast::TypeNode::Generic(g) => g.base_type.as_str(),
                    })
                    .unwrap_or_else(|| "unknown".to_string());

                if expr_type != "unknown" {
                    let base_decl = declared_type.split('<').next().unwrap_or(&declared_type);
                    match base_decl {
                        "name" => {
                            // المؤشرات من نوع name يتم تتبع نوعها لمنع تغيير النوع عند إعادة التعيين
                            if expr_type != "unknown" && expr_type != "object" {
                                final_type_node = Some(crate::parser::ast::TypeNode::Generic(
                                    crate::parser::ast::Generic {
                                        base_type: BaseType::Name(Box::new(BaseType::Unknown)),
                                        generics: vec![crate::parser::ast::TypeNode::Simple(
                                            crate::parser::ast::TypeRef {
                                                base_type: BaseType::from_str(&expr_type),
                                                size: None,
                                            },
                                        )],
                                    },
                                ));
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

                let deps = self.dependency_graph.get(&name.clone()).cloned().unwrap_or_default().into_iter().collect();
                let mut info = SymbolInfo {
                    name: name.clone(),
                    type_node: final_type_node,
                    visibility: visibility.clone(),
                    editability: editability.clone(),
                    settings: std::collections::HashSet::new(),
                    is_array: false,
                    dependencies: deps,
                };

                println!(
                    "DEBUG: Defining variable '{}' of type '{}' in environment",
                    name, declared_type
                );
                self.current_env.borrow_mut().define(name.clone(), info)?;
                self.current_context = prev_context;
            }
            Stmt::ArrayDecl {
                visibility,
                editability,
                type_node,
                name,
                length,
                value,
            } => {
                let expr_type = self.visit_expression(value)?;
                self.visit_expression(length)?; // Verify length expression

                let mut final_type_node = type_node.clone();
                let declared_type = type_node
                    .clone()
                    .map(|t| match t {
                        crate::parser::ast::TypeNode::Simple(r) => r.base_type.as_str(),
                        crate::parser::ast::TypeNode::Generic(g) => g.base_type.as_str(),
                    })
                    .unwrap_or_else(|| expr_type.clone());


                if let Some(ref mut tn) = final_type_node {
                    if let TypeNode::Generic(ref mut g) = tn {
                        if g.base_type == BaseType::from_str("name") && g.generics.len() == 1 {
                            if let TypeNode::Simple(ref r) = g.generics[0] {
                                if r.base_type == BaseType::Unknown && expr_type != "unknown" {
                                    let inner_type = if expr_type.starts_with("array<") {
                                        BaseType::Array(expr_type.trim_start_matches("array<").trim_end_matches(">").to_string())
                                    } else {
                                        BaseType::from_str(&expr_type)
                                    };
                                    g.generics[0] = TypeNode::Simple(TypeRef {
                                        base_type: inner_type,
                                        size: None,
                                    });
                                }
                            }
                        }
                    }

                    if let TypeNode::Simple(ref mut tr) = tn {
                        if expr_type == "unknown"
                            || expr_type == "array"
                            || expr_type == format!("array<{}>", declared_type)
                            || (declared_type == "char"
                                && (expr_type == "string" || expr_type == "str"))
                        {
                            // If type could not be inferred or it's an array literal or string literal assigning to char array, it's valid
                        } else if !self.types_are_compatible(&declared_type, &expr_type) {
                            return Err(format!(
                                "Semantic Error: Type mismatch for array '{}'. Declared '{}', got '{}'",
                                name, declared_type, expr_type
                            ));
                        }
                    }
                } else {
                    final_type_node = Some(TypeNode::Simple(TypeRef {
                        base_type: BaseType::from_str(&expr_type),
                        size: None,
                    }));
                }

                let info = SymbolInfo {
                    name: name.clone(),
                    type_node: final_type_node,
                    visibility: visibility.clone(),
                    editability: editability.clone(),
                    settings: std::collections::HashSet::new(),
                    is_array: true,
                dependencies: Vec::new() };

                println!(
                    "DEBUG: Defining array '{}' of base type '{}' in environment",
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


                if target_type.starts_with("name<") {
                    let expected_type = target_type
                        .trim_start_matches("name<")
                        .trim_end_matches(">");
                    
                    if expected_type == "unknown" && expr_type != "unknown" {
                        if let Expr::Identifier(name) = target {
                            let maybe_info = self.current_env.borrow().lookup(name);
                            if let Some(mut info) = maybe_info {
                                info.type_node = Some(TypeNode::Generic(Generic {
                                    base_type: BaseType::from_str("name"),
                                    generics: vec![TypeNode::Simple(TypeRef {
                                        base_type: BaseType::from_str(&expr_type),
                                        size: None,
                                    })],
                                }));
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
                    // Smart pointers auto-delete old data when reassigned
                    // CodeGen will handle inserting the `drop` call.
                } else if target_type != expr_type
                    && target_type != "unknown"
                    && expr_type != "unknown"
                {
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
                settings,
                handles,
                params,
                flags,
                labels: _,
                length,
                data,
                extends: _,
                fields,
                return_type: _,
                public_block,
                private_block,
                static_block,
                statements,
                variant_block: _,
                generics,
                handle_block,
                constructor,
                events: _,
            } => {
                if let Some(ref s) = settings {
                    let has_public = s.contains(&crate::parser::ast::Setting::Public);
                    let has_private = s.contains(&crate::parser::ast::Setting::Private);
                    let has_stmt = s.contains(&crate::parser::ast::Setting::Statement);
                    let has_label = s.contains(&crate::parser::ast::Setting::Label);

                    if has_label && (has_public || has_private || has_stmt) {
                        return Err(format!("Semantic Error: Custom block '{}' is marked as 'label' and cannot be combined with other modifiers.", name));
                    }
                    if has_stmt && (has_public || has_private) {
                        return Err(format!("Semantic Error: Custom block '{}' is marked as 'statement' and cannot have 'public' or 'private' modifiers.", name));
                    }
                }
                let info = SymbolInfo { dependencies: Vec::new(), 
                    name: name.clone(),
                    type_node: Some(crate::parser::ast::TypeNode::Simple(
                        crate::parser::ast::TypeRef {
                            base_type: BaseType::from_str("blueprint"),
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
                    is_array: false,
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

                if let Some(ref h_vec) = handles {
                    if h_vec.contains(&crate::parser::ast::HandleMethods::Error) {
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
                        visibility: crate::parser::ast::Visibility::Public,
                        editability: crate::parser::ast::Editability::Editable,
                        settings: std::collections::HashSet::new(),
                        is_array: false,
                    dependencies: Vec::new() };
                    self.current_env
                        .borrow_mut()
                        .define("data".to_string(), data_info)?;
                }

                let length_info = SymbolInfo { dependencies: Vec::new(), 
                    name: "length".to_string(),
                    type_node: Some(crate::parser::ast::TypeNode::Simple(
                        crate::parser::ast::TypeRef {
                            base_type: BaseType::Int,
                            size: Some(32),
                        },
                    )),
                    visibility: crate::parser::ast::Visibility::Public,
                    editability: crate::parser::ast::Editability::Editable,
                    settings: std::collections::HashSet::new(),
                    is_array: false,
                };
                self.current_env
                    .borrow_mut()
                    .define("length".to_string(), length_info)?;

                if let Some(ref fields_vec) = fields {
                    for field in fields_vec {
                        let field_info = SymbolInfo {
                            name: field.name.clone(),
                            type_node: field.type_node.clone(),
                            visibility: field.visibility.clone(),
                            editability: field.editability.clone(),
                            settings: HashSet::new(),
                            is_array: false,
                        dependencies: Vec::new() };
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
                            is_array: false,
                        dependencies: Vec::new() };
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
                if let Some(ref generics) = generics {
                    let info = SymbolInfo { dependencies: Vec::new(), 
                        name: name.clone(),
                        type_node: Some(crate::parser::ast::TypeNode::Simple(
                            crate::parser::ast::TypeRef {
                                base_type: BaseType::from_str("blueprint"),
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
                        is_array: false,
                    };
                }
                // update to check a list of constructors
                if let Some(ref constructors) = constructor {
                    for ctor in constructors {
                        self.enter_scope();
                        for param in &ctor.params {
                            let param_info = SymbolInfo {
                                name: param.name.clone(),
                                type_node: param.type_node.clone(),
                                visibility: crate::parser::ast::Visibility::Private,
                                editability: crate::parser::ast::Editability::Editable,
                                settings: std::collections::HashSet::new(),
                                is_array: false,
                            dependencies: Vec::new() };
                            self.current_env
                                .borrow_mut()
                                .define(param.name.clone(), param_info)?;
                        }
                        let prev_stmt_ctor = self.in_statement_scope;
                        self.in_statement_scope = true;
                        for statement in &ctor.body {
                            self.visit_statement(statement)?;
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
                generics,
                handle_block,
                length: _,
                constructor,
            } => {
                let info = SymbolInfo { dependencies: Vec::new(), 
                    name: name.clone(),
                    type_node: Some(crate::parser::ast::TypeNode::Simple(
                        crate::parser::ast::TypeRef {
                            base_type: BaseType::from_str("blueprint"),
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
                    is_array: false,
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
                let info = SymbolInfo { dependencies: Vec::new(), 
                    name: name.clone(),
                    type_node: Some(TypeNode::Simple(TypeRef {
                        base_type: BaseType::from_str(&name),
                        size: None,
                    })), // temp fix for generics
                    visibility: if *is_exported {
                        crate::parser::ast::Visibility::Public
                    } else {
                        crate::parser::ast::Visibility::Private
                    },
                    editability: crate::parser::ast::Editability::NotEditable,
                    settings: std::collections::HashSet::new(),
                    is_array: false,
                };
                for s in handle_block {
                    self.visit_statement(s)?;
                }
                if let Some(ref constructors) = constructor {
                    for ctor in constructors {
                        self.enter_scope();
                        for param in &ctor.params {
                            let param_info = SymbolInfo {
                                name: param.name.clone(),
                                type_node: param.type_node.clone(),
                                visibility: crate::parser::ast::Visibility::Private,
                                editability: crate::parser::ast::Editability::Editable,
                                settings: std::collections::HashSet::new(),
                                is_array: false,
                            dependencies: Vec::new() };
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
                let info = SymbolInfo { dependencies: Vec::new(), 
                    name: name.clone(),
                    type_node: Some(crate::parser::ast::TypeNode::Simple(
                        crate::parser::ast::TypeRef {
                            base_type: BaseType::from_str("blueprint"),
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
                    is_array: false,
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
                if let Some(ref constructors) = constructor {
                    for ctor in constructors {
                        self.enter_scope();
                        for param in &ctor.params {
                            let param_info = SymbolInfo {
                                name: param.name.clone(),
                                type_node: param.type_node.clone(),
                                visibility: crate::parser::ast::Visibility::Private,
                                editability: crate::parser::ast::Editability::Editable,
                                settings: std::collections::HashSet::new(),
                                is_array: false,
                            dependencies: Vec::new() };
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
            Stmt::EnumDecl {
                is_exported,
                name,
                length: _,
                handles: _,
                settings: _,
                handle_block,
                variants: _,
            } => {
                let info = SymbolInfo { dependencies: Vec::new(), 
                    name: name.clone(),
                    type_node: Some(crate::parser::ast::TypeNode::Simple(
                        crate::parser::ast::TypeRef {
                            base_type: BaseType::from_str("blueprint"),
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
                    is_array: false,
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
                let info = SymbolInfo { dependencies: Vec::new(), 
                    name: name.clone(),
                    type_node: Some(crate::parser::ast::TypeNode::Simple(
                        crate::parser::ast::TypeRef {
                            base_type: BaseType::from_str("blueprint"),
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
                        type_node: p.type_node.clone(),
                        visibility: crate::parser::ast::Visibility::Public,
                        editability: crate::parser::ast::Editability::Editable,
                        settings: std::collections::HashSet::new(),
                        is_array: false,
                    dependencies: Vec::new() };
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
                
                // Iterable must be an array or string
                let item_type = if iterable_type.starts_with("array<") {
                    iterable_type.trim_start_matches("array<").trim_end_matches(">").to_string()
                } else if iterable_type == "string" {
                    "char".to_string()
                } else {
                    return Err(format!(
                        "Semantic Error: Expected array or string in for-in loop, got '{}'",
                        iterable_type
                    ));
                };

                self.enter_scope();
                
                // Evaluate the item declaration or assignment
                if let Stmt::VarDecl { type_node, .. } = &**item {
                    let declared_type = type_node.as_ref().map(|t| match t {
                        crate::parser::ast::TypeNode::Simple(r) => r.base_type.as_str(),
                        crate::parser::ast::TypeNode::Generic(g) => g.base_type.as_str(),
                    }).unwrap_or_else(|| "unknown".to_string());
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
                    crate::parser::ast::EitherBlock::Inline(stmts) => {
                        self.enter_scope();
                        self.active_flags.push("+has_break".to_string());
                        for s in stmts {
                            self.visit_statement(s)?;
                        }
                        self.active_flags.retain(|f| f != "+has_break");
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
                    crate::parser::ast::EitherBlock::Inline(stmts) => {
                        self.enter_scope();
                        self.active_flags.push("+has_break".to_string());
                        for s in stmts {
                            self.visit_statement(s)?;
                        }
                        self.active_flags.retain(|f| f != "+has_break");
                        self.leave_scope();
                    }
                    crate::parser::ast::EitherBlock::External(expr) => {
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
                    let expected_type_str = match expected_type {
                        crate::parser::ast::TypeNode::Simple(r) => r.base_type.as_str(),
                        crate::parser::ast::TypeNode::Generic(g) => g.base_type.as_str(),
                    };

                    // تحقق صارم وديناميكي للمخرجات
                    if actual_type != "unknown"
                        && expected_type_str != "unknown"
                        && !self.types_are_compatible(&expected_type_str, &actual_type)
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
                println!(
                    "DEBUG: ThrowStmt encountered! active_flags = {:?}",
                    self.active_flags
                );
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
                let info = SymbolInfo { dependencies: Vec::new(), 
                    name: catch_param.clone(),
                    type_node: Some(crate::parser::ast::TypeNode::Simple(
                        crate::parser::ast::TypeRef {
                            base_type: BaseType::Error,
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
            Stmt::LabelDecl { name, body } => {
                let info = SymbolInfo { dependencies: Vec::new(), 
                    name: name.clone(),
                    type_node: Some(TypeNode::Simple(TypeRef {
                        base_type: BaseType::from_str("label"),
                        size: None,
                    })),
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
            Stmt::GotoStmt(_) => {
                // Forward jumps mean we can't strictly resolve labels in a single pass here.
                // CodeGen or a pre-pass will enforce that the label exists within the current function/scope bounds.
            }
            Stmt::BlueprintDecl {
                name, definition, ..
            } => {
                let info = SymbolInfo { dependencies: Vec::new(), 
                    name: name.clone(),
                    type_node: Some(TypeNode::Simple(TypeRef {
                        base_type: BaseType::from_str("blueprint"),
                        size: None,
                    })),
                    visibility: Visibility::Public,
                    editability: Editability::NotEditable,
                    settings: std::collections::HashSet::new(),
                    is_array: false,
                };
                self.current_env.borrow_mut().define(name.clone(), info)?;
            }
            Stmt::ImplDecl { target, methods } => {
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
                    self.visit_statement(m)?;
                }

                self.in_struct = prev_in_struct;
                self.active_flags.retain(|x| x != "+has_return");
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
                // Should return "object"
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
                            .map(|t| match t {
                                crate::parser::ast::TypeNode::Simple(r) => r.base_type.as_str(),
                                crate::parser::ast::TypeNode::Generic(g) => {
                                    let mut params_str = Vec::new();
                                    for p in &g.generics {
                                        if let crate::parser::ast::TypeNode::Simple(r) = p {
                                            params_str.push(r.base_type.as_str());
                                        }
                                    }
                                    // Fix: g.base_type for Name is Name(Unknown), its as_str() is "name<unknown>".
                                    // We should just use "name" or extract the base name.
                                    let base_name = g.base_type.as_str();
                                    let base_name = base_name.split('<').next().unwrap_or(&base_name);
                                    if params_str.is_empty() {
                                        base_name.to_string()
                                    } else {
                                        format!("{}<{}>", base_name, params_str.join(","))
                                    }
                                }
                            })
                            .unwrap_or_else(|| "unknown".to_string());
                        if info.is_array {
                            type_str = format!("array<{}>", type_str);
                        }
                        Ok(type_str)
                    }
                    None => Err(format!(
                        "Semantic Error: Variable '{}' is not defined in this scope.",
                        name
                    )),
                }
            }
            Expr::Instantiate { target, .. } => {
                if let Expr::Identifier(name) = &**target {
                    Ok(name.clone())
                } else if let Expr::Call { callee, .. } = &**target {
                    if let Expr::Identifier(name) = &**callee {
                        Ok(name.clone())
                    } else {
                        Ok("object".to_string())
                    }
                } else {
                    Ok("object".to_string())
                }
            }
            Expr::BinaryOp {
                left,
                operator,
                right,
            } => {
                let l_type = self.visit_expression(left)?;
                let r_type = self.visit_expression(right)?;

                // Check for Operator Overloading via handles
                if let Some(metadata) = self.global_metadata.get(&l_type) {
                    let handle_name = match operator.as_str() {
                        "+" => Some("add"),
                        "-" => Some("sub"),
                        "*" => Some("mul"),
                        "/" => Some("div"),
                        "%" => Some("mod"),
                        _ => None,
                    };
                    if let Some(method_name) = handle_name {
                        if let Some(m_node) = metadata.methods.get(method_name) {
                            // The operation is valid! Return the overloaded return type.
                            return Ok(Self::format_type(&m_node.return_type));
                        }
                    }
                }

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

                let mut is_array_var = false;
                if let Expr::Identifier(ref name) = **object {
                    if let Some(info) = self.current_env.borrow().lookup(name) {
                        is_array_var = info.is_array;
                    }
                }

                let mut custom_idx_return = None;
                if let Some(metadata) = self.global_metadata.get(&obj_type) {
                    if let Some(m_node) = metadata.methods.get("index_access") {
                        custom_idx_return = Some(Self::format_type(&m_node.return_type));
                    }
                }

                if let Some(ret_type) = custom_idx_return {
                    let _idx_type = self.visit_expression(index)?;
                    return Ok(ret_type);
                }

                if !obj_type.starts_with("str")
                    && !obj_type.starts_with("array")
                    && !obj_type.starts_with("custom<")
                    && obj_type != "unknown"
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
                if is_array_var {
                    return Ok(obj_type);
                }
                // We return unknown because lists can hold anything unless generics are implemented
                Ok("unknown".to_string())
            }
            Expr::PropertyAccess { object, property } => {
                let _obj_type = self.visit_expression(object)?;
                let obj_type_clean = _obj_type.clone();

                // Check for built-ins first (length, data, size) for arrays and strings
                if (obj_type_clean.starts_with("array<") || obj_type_clean == "str")
                    && (property == "length" || property == "size" || property == "data")
                {
                    if property == "data" {
                        return Ok("unknown".to_string());
                    }
                    return Ok("int(64)".to_string());
                }

                if let Some(metadata) = self.global_metadata.get(&obj_type_clean) {
                    // Check fields
                    if let Some(t_node) = metadata.fields.get(property) {
                        let t = match t_node {
                            crate::parser::ast::TypeNode::Simple(r) => r.base_type.as_str(),
                            crate::parser::ast::TypeNode::Generic(g) => g.base_type.as_str(),
                        };
                        return Ok(t);
                    }
                    // Check vars (Custom/Structs might use vars)
                    if let Some(v_node) = metadata.vars.get(property) {
                        let t = match &v_node.type_node {
                            crate::parser::ast::TypeNode::Simple(r) => r.base_type.as_str(),
                            crate::parser::ast::TypeNode::Generic(g) => g.base_type.as_str(),
                        };
                        return Ok(t);
                    }
                    // Check methods
                    if let Some(m_node) = metadata.methods.get(property) {
                        let ret = &m_node.return_type;
                        let t = match ret {
                            crate::parser::ast::TypeNode::Simple(r) => r.base_type.as_str(),
                            crate::parser::ast::TypeNode::Generic(g) => g.base_type.as_str(),
                        };
                        return Ok(t);
                    }
                    return Err(format!(
                        "Semantic Error: Property '{}' not found on type '{}'",
                        property, obj_type_clean
                    ));
                }

                // Fallback to current env lookup if it's 'this' and we didn't find it in metadata yet
                if matches!(&**object, Expr::This) {
                    let env = self.current_env.borrow();
                    let lookup_res = env.lookup(property);
                    if let Some(info) = lookup_res {
                        return Ok(info
                            .type_node
                            .map(|t| match t {
                                crate::parser::ast::TypeNode::Simple(r) => r.base_type.as_str(),
                                crate::parser::ast::TypeNode::Generic(g) => g.base_type.as_str(),
                            })
                            .unwrap_or_else(|| "unknown".to_string()));
                    }
                    if self.in_struct || self.in_class || self.in_custom_scope {
                        return Ok("auto".to_string()); // Trust the C++ compiler for things not in metadata
                    }

                    return Err(format!(
                        "Semantic Error: Field '{}' is not defined in this scope.",
                        property
                    ));
                }

                Ok("unknown".to_string())
            }
            Expr::NamespaceAccess {
                namespace,
                property,
            } => {
                // Static access or module access
                if let Some(metadata) = self.global_metadata.get(namespace) {
                    let prop_name =
                        match &**property {
                            Expr::Identifier(prop_name) => prop_name.clone(),
                            _ => return Err(
                                "Semantic Error: Expected identifier for static property access"
                                    .to_string(),
                            ),
                        };
                    // Check fields
                    if let Some(t_node) = metadata.fields.get(&prop_name) {
                        let t = match t_node {
                            crate::parser::ast::TypeNode::Simple(r) => r.base_type.as_str(),
                            crate::parser::ast::TypeNode::Generic(g) => g.base_type.as_str(),
                        };
                        return Ok(t);
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
                            let base = match type_ref {
                                crate::parser::ast::TypeNode::Simple(r) => &r.base_type,
                                crate::parser::ast::TypeNode::Generic(g) => &g.base_type,
                            };
                            if base.as_str() == "type" || base.as_str().starts_with("type<") {
                                // It's a type instantiation (e.g. T(size))
                                // Don't visit arguments as normal expressions
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
                // Type inference on return types requires function symbol table
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
                if !["str", "array", "unknown"].contains(&source) {
                    return Err(format!(
                        "Semantic Error: '{}' can only be used with array, str. Got '{}'",
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
        if expected == actual || expected == "any" || actual == "unknown" || actual == "auto" {
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
        if (expected == "string" || expected == "custom<string>") && (actual == "str" || actual == "string") {
            return true;
        }
        false
    }

    fn format_type(type_node: &crate::parser::ast::TypeNode) -> String {
        let type_ref = match type_node {
            crate::parser::ast::TypeNode::Simple(r) => r,
            crate::parser::ast::TypeNode::Generic(g) => return format!("{}<...>", g.base_type.as_str()),
        };
        match type_ref.size {
            Some(size) => format!("{}({})", type_ref.base_type.as_str(), size),
            None => type_ref.base_type.as_str(),
        }
    }
}
