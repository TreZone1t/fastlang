use std::rc::Rc;
use std::cell::RefCell;
use crate::parser::ast::{Expr, Stmt, TypeRef};
use crate::semantic::environment::{Environment, SymbolInfo};

pub struct SemanticAnalyzer {
    pub current_env: Rc<RefCell<Environment>>,
    pub in_class: bool,
    pub in_struct: bool,
    pub in_statement_scope: bool,
    pub active_flags: Vec<String>,
    /// Declared return type of the function-like scope currently being analyzed.
    pub active_return_type: Option<TypeRef>,
}

impl SemanticAnalyzer {
    pub fn new() -> Self {
        let mut analyzer = SemanticAnalyzer {
            current_env: Environment::new(),
            in_class: false,
            in_struct: false,
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

        for (name, ret_type) in std_funcs {
            let info = SymbolInfo {
                name: name.to_string(),
                base_type: "fn".to_string(),
                size: None,
                is_const: true,
                is_static: true,
                is_exported: true, // stdlib is always available
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
        let parent = self.current_env.borrow().parent.clone().expect("Cannot leave global scope");
        self.current_env = parent;
    }

    fn visit_statement(&mut self, stmt: &Stmt) -> Result<(), String> {
        match stmt {
            Stmt::VarDecl { is_static, is_const, base_type, size, name, value, is_exported } => {
                // Evaluate value first to infer type
                let expr_type = self.visit_expression(value)?;
                
                // Get the declared type
                let declared_type = base_type.clone().unwrap_or_else(|| "unknown".to_string());

                // Special handling for Magic Types (length, size, init, param, blueprint)
                if ["length", "size", "init", "param", "blueprint"].contains(&declared_type.as_str()) {
                    if declared_type == "length" && expr_type != "str" && expr_type != "string" && expr_type != "list" {
                        return Err(format!("Semantic Error: 'length' can only be applied to string or list, got '{}'", expr_type));
                    }
                    if declared_type == "size" && expr_type == "blueprint" {
                        return Err("Semantic Error: Cannot get size of a blueprint directly".to_string());
                    }
                } else if declared_type != expr_type && expr_type != "unknown" {
                    // Allow assigning 'str' to 'string'
                    if declared_type == "string" && expr_type == "str" {
                        // valid
                    } else if declared_type == "blueprint" && expr_type == "object" {
                        // valid
                    } else {
                        return Err(format!("Semantic Error: Type mismatch for '{}'. Declared '{}', got '{}'", name, declared_type, expr_type));
                    }
                }

                let info = SymbolInfo {
                    name: name.clone(),
                    base_type: declared_type.clone(),
                    size: *size,
                    is_const: *is_const,
                    is_static: *is_static,
                    is_exported: *is_exported,
                };

                self.current_env.borrow_mut().define(name.clone(), info)?;
            },
            Stmt::ReassignStmt { target, value } => {
                let expr_type = self.visit_expression(value)?;
                // Resolving target type
                let target_type = self.visit_expression(target)?;
                
                if target_type != expr_type && target_type != "unknown" && expr_type != "unknown" {
                    if target_type == "string" && expr_type == "str" {
                        // Allow assigning 'str' to 'string'
                    } else {
                        return Err(format!("Semantic Error: Cannot assign '{}' to type '{}'", expr_type, target_type));
                    }
                }
            },
            Stmt::ExpressionStmt(expr) => {
                self.visit_expression(expr)?;
            },
            Stmt::ScopeDecl { name, scope_type, is_custom, params, return_type, flags, settings: _, events: _, handles: _, statements, public_block, fields, private_block, return_value: _, is_exported, constructor } => {
                let info = SymbolInfo {
                    name: name.clone(),
                    base_type: scope_type.clone(),
                    size: None,
                    is_const: true,
                    is_static: false,
                    is_exported: *is_exported,
                };
                let _ = self.current_env.borrow_mut().define(name.clone(), info);
                
                let prev_in_stmt = self.in_statement_scope;
                if scope_type == "statement" {
                    self.in_statement_scope = true;
                }
                
                let prev_flags = self.active_flags.clone();
                let prev_return_type = self.active_return_type.clone();
                self.active_return_type = return_type.clone();
                // fn and block scopes allow return and throw
                if scope_type == "fn" || scope_type == "block" || scope_type == "Fn" {
                    self.active_flags.push("+is_return".to_string());
                    self.active_flags.push("+is_throw".to_string());
                } else if scope_type == "looped" {
                    self.active_flags.push("+is_break".to_string());
                }
                if scope_type == "custom" {
                    self.active_flags.push("+is_throw".to_string());
                }

                for f in flags {
                    if f.starts_with('+') {
                        let n = f.trim_start_matches('+');
                        if n == "all" {
                            self.active_flags.push("+is_return".to_string());
                            self.active_flags.push("+is_break".to_string());
                            self.active_flags.push("+is_throw".to_string());
                            self.active_flags.push("+is_exit".to_string());
                        } else {
                            self.active_flags.push(format!("+{}", n));
                        }
                    } else if f.starts_with('-') {
                        let n = f.trim_start_matches('-');
                        if n == "all" {
                            self.active_flags.clear();
                        } else {
                            self.active_flags.retain(|x| x != &format!("+{}", n));
                        }
                    }
                }

                self.enter_scope();
                // `add` declarations are members of a custom scope.  Define
                // them before analyzing constructors and methods so both
                // `this.width` and the unqualified `width` resolve normally.
                if *is_custom {
                    for field in fields {
                        if let Stmt::VarDecl { name: field_name, base_type, size, is_const, is_static, is_exported, .. } = field {
                            let field_info = SymbolInfo {
                                name: field_name.clone(),
                                base_type: base_type.clone().unwrap_or_else(|| "unknown".to_string()),
                                size: *size,
                                is_const: *is_const,
                                is_static: *is_static,
                                is_exported: *is_exported,
                            };
                            self.current_env.borrow_mut().define(field_name.clone(), field_info)?;
                        }
                    }
                }
                // Register params inside this scope
                for p in params {
                    if let Stmt::VarDecl { name: p_name, base_type, size, .. } = p {
                        let param_info = SymbolInfo {
                            name: p_name.clone(),
                            base_type: base_type.clone().unwrap_or_else(|| "unknown".to_string()),
                            size: *size,
                            is_const: false,
                            is_static: false,
                            is_exported: false,
                        };
                        let _ = self.current_env.borrow_mut().define(p_name.clone(), param_info);
                    }
                }
                for s in statements {
                    self.visit_statement(s)?;
                }
                for s in public_block {
                    self.visit_statement(s)?;
                }
                for s in private_block {
                    self.visit_statement(s)?;
                }
                if let Some(constructor) = constructor {
                    self.enter_scope();
                    for param in &constructor.params {
                        let param_info = SymbolInfo {
                            name: param.name.clone(),
                            base_type: param.base_type.clone(),
                            size: param.size,
                            is_const: false,
                            is_static: false,
                            is_exported: false,
                        };
                        self.current_env.borrow_mut().define(param.name.clone(), param_info)?;
                    }
                    for statement in &constructor.body {
                        self.visit_statement(statement)?;
                    }
                    self.leave_scope();
                }
                self.leave_scope();
                
                self.active_flags = prev_flags;
                self.active_return_type = prev_return_type;
                self.in_statement_scope = prev_in_stmt;
            },
            Stmt::ClassDecl { name, extends: _, public_block, private_block, static_block, constructor, is_exported } => {
                let info = SymbolInfo {
                    name: name.clone(),
                    base_type: "blueprint".to_string(), // Class is a blueprint
                    size: None,
                    is_const: true,
                    is_static: true,
                    is_exported: *is_exported,
                };
                self.current_env.borrow_mut().define(name.clone(), info)?;

                self.in_class = true;
                self.enter_scope();
                
                for s in public_block { self.visit_statement(s)?; }
                for s in private_block { self.visit_statement(s)?; }
                for s in static_block { self.visit_statement(s)?; }
                if let Some(c) = constructor {
                    self.enter_scope();
                    for p in &c.params {
                        let param_info = SymbolInfo {
                            name: p.name.clone(),
                            base_type: p.base_type.clone(),
                            size: p.size,
                            is_const: false,
                            is_static: false,
                            is_exported: false,
                        };
                        let _ = self.current_env.borrow_mut().define(p.name.clone(), param_info);
                    }
                    for s in &c.body { self.visit_statement(s)?; }
                    self.leave_scope();
                }

                self.leave_scope();
                self.in_class = false;
            },
            Stmt::StructDecl { name, public_block, private_block, static_block, constructor, is_exported } => {
                let info = SymbolInfo {
                    name: name.clone(),
                    base_type: "blueprint".to_string(),
                    size: None,
                    is_const: true,
                    is_static: true,
                    is_exported: *is_exported,
                };
                self.current_env.borrow_mut().define(name.clone(), info)?;

                self.in_struct = true;
                self.enter_scope();

                for s in public_block { self.visit_statement(s)?; }
                for s in private_block { self.visit_statement(s)?; }
                for s in static_block { self.visit_statement(s)?; }
                if let Some(c) = constructor {
                    self.enter_scope();
                    for p in &c.params {
                        let param_info = SymbolInfo {
                            name: p.name.clone(),
                            base_type: p.base_type.clone(),
                            size: p.size,
                            is_const: false,
                            is_static: false,
                            is_exported: false,
                        };
                        let _ = self.current_env.borrow_mut().define(p.name.clone(), param_info);
                    }
                    for s in &c.body { self.visit_statement(s)?; }
                    self.leave_scope();
                }

                self.leave_scope();
                self.in_struct = false;
            },
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
            },
            Stmt::WhileStmt { condition, body } => {
                let cond_type = self.visit_expression(condition)?;
                if cond_type != "bool" && cond_type != "unknown" {
                    return Err("Semantic Error: loop condition must be a boolean".to_string());
                }
                match body {
                    crate::parser::ast::LoopBody::Inline(stmts) => {
                        self.enter_scope();
                        for s in stmts { self.visit_statement(s)?; }
                        self.leave_scope();
                    },
                    crate::parser::ast::LoopBody::ScopeCall(expr) => {
                        self.visit_expression(expr)?;
                    }
                }
            },
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
                    self.visit_expression(inc)?;
                }
                match body {
                    crate::parser::ast::LoopBody::Inline(stmts) => {
                        self.enter_scope();
                        for s in stmts { self.visit_statement(s)?; }
                        self.leave_scope();
                    },
                    crate::parser::ast::LoopBody::ScopeCall(expr) => {
                        self.visit_expression(expr)?;
                    }
                }
                self.leave_scope();
            },
            Stmt::ReturnStmt(expr) => {
                if !self.active_flags.contains(&"+is_return".to_string()) {
                    return Err("Semantic Error: Return statement is not allowed in this scope. 'is_return' flag is not enabled.".to_string());
                }
                let actual_type = self.visit_expression(expr)?;
                if let Some(expected_type) = &self.active_return_type {
                    if !self.types_are_compatible(&expected_type.base_type, &actual_type) {
                        return Err(format!(
                            "Semantic Error: Return type mismatch. Declared '{}', got '{}'",
                            Self::format_type(expected_type),
                            actual_type
                        ));
                    }
                }
            },
            Stmt::BreakStmt => {
                if !self.active_flags.contains(&"+is_break".to_string()) {
                    return Err("Semantic Error: Break statement is not allowed here. 'is_break' flag is not enabled.".to_string());
                }
            },
            Stmt::ThrowStmt(expr) => {
                if !self.active_flags.contains(&"+is_throw".to_string()) {
                    return Err("Semantic Error: Throw statement is not allowed here. 'is_throw' flag is not enabled.".to_string());
                }
                self.visit_expression(expr)?;
            },
            Stmt::TryCatchStmt { try_block, catch_param, catch_block } => {
                self.enter_scope();
                for s in try_block { self.visit_statement(s)?; }
                self.leave_scope();

                self.enter_scope();
                let info = SymbolInfo {
                    name: catch_param.clone(),
                    base_type: "error".to_string(),
                    size: None,
                    is_const: true,
                    is_static: false,
                    is_exported: false,
                };
                self.current_env.borrow_mut().define(catch_param.clone(), info)?;
                for s in catch_block { self.visit_statement(s)?; }
                self.leave_scope();
            },
            _ => {}
        }
        Ok(())
    }

    fn visit_expression(&mut self, expr: &Expr) -> Result<String, String> {
        match expr {
            Expr::LiteralInt(_) => Ok("int".to_string()),
            Expr::LiteralFloat(_) => Ok("float".to_string()),
            Expr::LiteralString(_) => Ok("str".to_string()), // string or str based on usage
            Expr::LiteralBool(_) => Ok("bool".to_string()),
            Expr::ListLiteral(elements) => {
                for el in elements {
                    self.visit_expression(el)?;
                }
                Ok("list".to_string())
            },
            Expr::ObjectLiteral(stmts) => {
                // Should return "object"
                self.enter_scope();
                for s in stmts {
                    self.visit_statement(s)?;
                }
                self.leave_scope();
                Ok("object".to_string())
            },
            Expr::Identifier(name) => {
                if name == "__param__" {
                    return Ok("unknown".to_string());
                }
                match self.current_env.borrow().lookup(name) {
                    Some(info) => Ok(info.base_type),
                    None => Err(format!("Semantic Error: Variable '{}' is not defined in this scope.", name)),
                }
            },
            Expr::Instantiate { op: _, target, args: _ } => {
                // If target is Dog, it resolves to object
                Ok("object".to_string())
            },
            Expr::BinaryOp { left, operator, right } => {
                let l_type = self.visit_expression(left)?;
                let r_type = self.visit_expression(right)?;
                if l_type != r_type && l_type != "unknown" && r_type != "unknown" {
                    return Err(format!("Semantic Error: Type mismatch in binary operation: '{}' and '{}'", l_type, r_type));
                }
                if ["<", ">", "<=", ">=", "==", "!="].contains(&operator.as_str()) {
                    Ok("bool".to_string())
                } else {
                    Ok(l_type)
                }
            },
            Expr::UnaryOp { operator: _, operand } => {
                self.visit_expression(operand)
            },
            Expr::IndexAccess { object, index } => {
                let obj_type = self.visit_expression(object)?;
                if obj_type != "list" && obj_type != "string" && obj_type != "str" && obj_type != "unknown" {
                    return Err(format!("Semantic Error: Cannot index into type '{}'", obj_type));
                }
                let idx_type = self.visit_expression(index)?;
                if idx_type != "int" && idx_type != "unknown" {
                    return Err(format!("Semantic Error: Array index must be an int, got '{}'", idx_type));
                }
                // We return unknown because lists can hold anything unless generics are implemented
                Ok("unknown".to_string())
            },
            Expr::PropertyAccess { object, property } => {
                let _obj_type = self.visit_expression(object)?;
                if matches!(&**object, Expr::This) {
                    return self.current_env.borrow().lookup(property)
                        .map(|info| info.base_type)
                        .ok_or_else(|| format!(
                            "Semantic Error: Field '{}' is not defined in this scope.",
                            property
                        ));
                }
                // Any property access expects an object, struct, list, str, or string
                Ok("unknown".to_string())
            },
            Expr::NamespaceAccess { namespace: _, property: _ } => {
                // Static access or module access
                Ok("unknown".to_string())
            },
            Expr::Call { callee, args } => {
                if let Expr::Identifier(name) = &**callee {
                    if self.current_env.borrow().lookup(name).is_none() {
                        return Err(format!("Semantic Error: Function '{}' is not defined in this scope.", name));
                    }
                }
                for arg in args {
                    self.visit_expression(arg)?;
                }
                // Type inference on return types requires function symbol table
                Ok("unknown".to_string())
            },
            Expr::This => {
                if self.in_statement_scope {
                    return Err("Semantic Error: Cannot use 'this' inside a statement scope".to_string());
                }
                if self.current_env.borrow().parent.is_none() {
                    return Err("Semantic Error: Cannot use 'this' in the global scope".to_string());
                }
                Ok("object".to_string())
            },
            Expr::Global => {
                if self.current_env.borrow().parent.is_none() {
                    eprintln!("Warning: Using 'global' in the global scope is redundant and considered ugly code.");
                }
                Ok("object".to_string()) // Or perhaps "global" type depending on future needs
            },
            Expr::Super => {
                if !self.in_class {
                    return Err("Semantic Error: Cannot use 'super' outside of a class".to_string());
                }
                Ok("object".to_string())
            },
            _ => Ok("unknown".to_string()),
        }
    }

    fn validate_magic_type_assignment(&self, magic: &str, source: &str) -> Result<(), String> {
        match magic {
            "length" | "size" => {
                if !["list", "str", "string", "unknown"].contains(&source) {
                    return Err(format!("Semantic Error: '{}' can only be used with list, str, or string. Got '{}'", magic, source));
                }
            },
            "param" => {
                // Param can take functions/scopes etc. For now we are lenient.
            },
            "init" => {
                // Init takes a struct/class
            },
            _ => {}
        }
        Ok(())
    }

    fn types_are_compatible(&self, expected: &str, actual: &str) -> bool {
        actual == "unknown"
            || expected == actual
            || (expected == "string" && actual == "str")
    }

    fn format_type(type_ref: &TypeRef) -> String {
        match type_ref.size {
            Some(size) => format!("{}({})", type_ref.base_type, size),
            None => type_ref.base_type.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SemanticAnalyzer;
    use crate::parser::ast::{ConstructorDecl, Expr, Stmt, TypeRef};

    fn int_field(name: &str) -> Stmt {
        Stmt::VarDecl {
            is_exported: false,
            is_static: false,
            is_const: false,
            base_type: Some("int".to_string()),
            size: Some(32),
            name: name.to_string(),
            value: Expr::Identifier("__param__".to_string()),
        }
    }

    #[test]
    fn rejects_a_return_value_with_the_wrong_type() {
        let function = Stmt::ScopeDecl {
            is_exported: false,
            name: "number".to_string(),
            scope_type: "fn".to_string(),
            is_custom: false,
            params: Vec::new(),
            return_type: Some(TypeRef {
                base_type: "int".to_string(),
                size: Some(32),
            }),
            flags: Vec::new(),
            settings: Vec::new(),
            events: Vec::new(),
            handles: Vec::new(),
            statements: vec![Stmt::ReturnStmt(Expr::LiteralString("wrong".to_string()))],
            public_block: Vec::new(),
            fields: Vec::new(),
            private_block: Vec::new(),
            return_value: None,
            constructor: None,
        };

        let error = SemanticAnalyzer::new().analyze(&vec![function]).unwrap_err();
        assert!(error.contains("Return type mismatch"));
    }

    #[test]
    fn resolves_custom_scope_fields_through_this() {
        let custom_scope = Stmt::ScopeDecl {
            is_exported: false,
            name: "Box".to_string(),
            scope_type: "custom".to_string(),
            is_custom: true,
            params: Vec::new(),
            return_type: None,
            flags: Vec::new(),
            settings: Vec::new(),
            events: Vec::new(),
            handles: Vec::new(),
            statements: Vec::new(),
            public_block: Vec::new(),
            fields: vec![int_field("width")],
            private_block: Vec::new(),
            return_value: None,
            constructor: Some(ConstructorDecl {
                params: Vec::new(),
                expected_types: Vec::new(),
                body: vec![Stmt::ReassignStmt {
                    target: Expr::PropertyAccess {
                        object: Box::new(Expr::This),
                        property: "width".to_string(),
                    },
                    value: Expr::LiteralInt(10),
                }],
            }),
        };

        assert!(SemanticAnalyzer::new().analyze(&vec![custom_scope]).is_ok());
    }
}
