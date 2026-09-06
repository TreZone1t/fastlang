use std::collections::HashMap;
use crate::frontend::parser::ast::Expr;

#[derive(Debug, Clone)]
pub struct InterpreterEnv {
    variables: HashMap<String, Expr>,
    parent: Option<Box<InterpreterEnv>>,
}

impl InterpreterEnv {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            parent: None,
        }
    }

    pub fn with_parent(parent: InterpreterEnv) -> Self {
        Self {
            variables: HashMap::new(),
            parent: Some(Box::new(parent)),
        }
    }

    pub fn push_scope(&mut self) {
        let old = std::mem::replace(self, InterpreterEnv::new());
        *self = InterpreterEnv::with_parent(old);
    }

    pub fn pop_scope(&mut self) {
        if let Some(parent) = self.parent.take() {
            *self = *parent;
        }
    }

    pub fn define(&mut self, name: String, value: Expr) {
        self.variables.insert(name, value);
    }

    pub fn assign(&mut self, name: &str, value: Expr) -> Result<(), String> {
        if self.variables.contains_key(name) {
            self.variables.insert(name.to_string(), value);
            Ok(())
        } else if let Some(ref mut parent) = self.parent {
            parent.assign(name, value)
        } else {
            Err(format!("Undefined variable '{}' at compile time.", name))
        }
    }

    pub fn get(&self, name: &str) -> Result<Expr, String> {
        if let Some(val) = self.variables.get(name) {
            Ok(val.clone())
        } else if let Some(ref parent) = self.parent {
            parent.get(name)
        } else {
            Err(format!("Undefined variable '{}' at compile time.", name))
        }
    }

    pub fn remove(&mut self, name: &str) {
        self.variables.remove(name);
        if let Some(ref mut parent) = self.parent {
            parent.remove(name);
        }
    }
}
