use crate::parser::ast::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct SymbolInfo {
    pub name: String,
    pub type_node: Option<TypeNode>,
    pub visibility: Visibility,
    pub editability: Editability,
    pub settings: std::collections::HashSet<Setting>,
    pub is_array: bool,
    pub dependencies: Vec<String>,
}

pub struct Environment {
    pub parent: Option<Rc<RefCell<Environment>>>,
    pub symbols: HashMap<String, SymbolInfo>,
}

impl Environment {
    pub fn new() -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Environment {
            parent: None,
            symbols: HashMap::new(),
        }))
    }

    pub fn with_parent(parent: Rc<RefCell<Environment>>) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Environment {
            parent: Some(parent),
            symbols: HashMap::new(),
        }))
    }

    pub fn define(&mut self, name: String, info: SymbolInfo) -> Result<(), String> {
        if self.symbols.contains_key(&name) {
            return Err(format!(
                "Semantic Error: Variable '{}' is already defined in this scope.",
                name
            ));
        }
        self.symbols.insert(name, info);
        Ok(())
    }

    pub fn lookup(&self, name: &str) -> Option<SymbolInfo> {
        if let Some(info) = self.symbols.get(name) {
            return Some(info.clone());
        }
        if let Some(ref parent) = self.parent {
            return parent.borrow().lookup(name);
        }
        None
    }

    pub fn update(&mut self, name: &str, info: SymbolInfo) -> bool {
        if self.symbols.contains_key(name) {
            self.symbols.insert(name.to_string(), info);
            return true;
        }
        if let Some(ref parent) = self.parent {
            return parent.borrow_mut().update(name, info);
        }
        false
    }
}

