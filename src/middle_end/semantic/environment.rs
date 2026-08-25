use crate::frontend::parser::ast::*;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

// ─────────────────────────────────────────────────────────────────────────────
// BlueprintData — encapsulates composite types (Custom, Class, Struct, Enum)
// Constructed from TypeMetadata or directly from the AST Declaration
// ─────────────────────────────────────────────────────────────────────────────
#[derive(Debug, Clone)]
pub struct FnSignature {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: BaseType,
    pub is_virtual: bool,
    pub is_abstract: bool,
}

#[derive(Debug, Clone)]
pub struct BlueprintData {
    pub name: String,
    pub fields: HashMap<String, BaseType>,
    pub methods: HashMap<String, FnSignature>,
    pub handles: HashSet<HandleMethods>,
    pub settings: HashSet<Setting>,
    pub generics: Vec<String>,
    pub params: Vec<Param>,
    pub variants: Vec<EnumVariant>,
    pub is_class: bool,
}

impl BlueprintData {
    pub fn new(name: impl Into<String>) -> Self {
        BlueprintData {
            name: name.into(),
            fields: HashMap::new(),
            methods: HashMap::new(),
            handles: HashSet::new(),
            settings: HashSet::new(),
            generics: Vec::new(),
            params: Vec::new(),
            variants: Vec::new(),
            is_class: false,
        }
    }

    /// Checks if this blueprint defines a specific handle method
    pub fn has_handle(&self, h: HandleMethods) -> bool {
        self.handles.contains(&h)
    }

    /// Checks if the specified handle accepts a parameter of the given type
    /// Used to verify operator overloading compatibility
    pub fn handle_accepts_type(&self, h: HandleMethods, value_type: &str) -> bool {
        let fn_name = h.as_str();
        if let Some(sig) = self.methods.get(fn_name) {
            if sig.params.is_empty() {
                // Handle without params accepts anything
                return true;
            }
            let param_type = sig.params[0].type_node.as_str();
            // If param_type is generic (T, U, ...), accept unconditionally
            if self.generics.contains(&param_type) {
                return true;
            }
            // Accept if types match or if it is an array
            if param_type == value_type {
                return true;
            }
            // If param is an array of the same generic
            if param_type.starts_with("array<") {
                return true; // array params in handles accept any array
            }
            return false;
        }
        // If no handle signature is found, accept leniently
        true
    }

    /// Monomorphize/specialize the blueprint with concrete generic type arguments
    pub fn specialize(&self, type_args: &[BaseType]) -> BlueprintData {
        let mut map = HashMap::new();
        for (g_param, g_arg) in self.generics.iter().zip(type_args.iter()) {
            map.insert(g_param.clone(), g_arg.clone());
        }
        if map.is_empty() {
            return self.clone();
        }

        let mut new_fields = HashMap::new();
        for (f_name, f_type) in &self.fields {
            new_fields.insert(f_name.clone(), f_type.substitute_generics(&map));
        }

        let mut new_methods = HashMap::new();
        for (m_name, sig) in &self.methods {
            let new_params = sig.params.iter().map(|p| Param {
                name: p.name.clone(),
                type_node: p.type_node.substitute_generics(&map),
            }).collect();
            let new_ret = sig.return_type.substitute_generics(&map);
            new_methods.insert(m_name.clone(), FnSignature {
                name: sig.name.clone(),
                params: new_params,
                return_type: new_ret,
                is_virtual: sig.is_virtual,
                is_abstract: sig.is_abstract,
            });
        }

        BlueprintData {
            name: self.name.clone(),
            fields: new_fields,
            methods: new_methods,
            handles: self.handles.clone(),
            settings: self.settings.clone(),
            generics: Vec::new(),
            params: self.params.clone(),
            variants: self.variants.clone(),
            is_class: self.is_class,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// SymbolKind — describes the type of symbol in the scope
// ─────────────────────────────────────────────────────────────────────────────
#[derive(Debug, Clone)]
pub enum SymbolKind {
    /// Standard variable (int, float, bool, custom, etc.)
    Variable {
        type_node: BaseType,
        editability: Editability,
        is_array: bool,
    },
    /// Function (fn)
    Function {
        params: Vec<Param>,
        return_type: BaseType,
    },
    /// blueprint definition (class, struct, custom, enum)
    Blueprint,
    /// label
    Label,
}

// ─────────────────────────────────────────────────────────────────────────────
#[derive(Debug, Clone)]
pub struct SymbolInfo {
    pub name: String,
    pub kind: SymbolKind,
    pub visibility: Visibility,
    pub dependencies: Vec<String>,
    pub is_used: bool,
    pub is_param: bool,
}

impl Default for SymbolInfo {
    fn default() -> Self {
        SymbolInfo {
            name: String::new(),
            kind: SymbolKind::Variable {
                type_node: BaseType::Unknown,
                editability: Editability::Editable,
                is_array: false,
            },
            visibility: Visibility::Private,
            dependencies: vec![],
            is_used: false,
            is_param: false,
        }
    }
}

impl SymbolInfo {
    /// Retrieve variable type name as string for compatibility checks
    pub fn type_str(&self) -> String {
        match &self.kind {
            SymbolKind::Variable { type_node, is_array, .. } => {
                let base = type_node.as_str();
                if *is_array {
                    format!("array<{}>", base)
                } else {
                    base
                }
            }
            SymbolKind::Function { params, return_type, .. } => {
                let p_strs: Vec<String> = params.iter().map(|p| p.type_node.as_str()).collect();
                format!("Fn<({}), {}>", p_strs.join(", "), return_type.as_str())
            }
            SymbolKind::Blueprint => "blueprint".to_string(),
            SymbolKind::Label => "label".to_string(),
        }
    }

    pub fn is_editable(&self) -> bool {
        match &self.kind {
            SymbolKind::Variable { editability, .. } => {
                *editability == Editability::Editable
            }
            _ => false,
        }
    }

    pub fn is_array(&self) -> bool {
        matches!(&self.kind, SymbolKind::Variable { is_array, .. } if *is_array)
    }

    // ── Compatibility Helpers ──────────────────────────────────────────
    pub fn type_node(&self) -> Option<&BaseType> {
        match &self.kind {
            SymbolKind::Variable { type_node, .. } => Some(type_node),
            SymbolKind::Function { return_type, .. } => Some(return_type),
            _ => None,
        }
    }

    pub fn editability(&self) -> Option<&Editability> {
        match &self.kind {
            SymbolKind::Variable { editability, .. } => Some(editability),
            _ => None,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Environment — current scope with parent chain for lexical scoping
// ─────────────────────────────────────────────────────────────────────────────
pub struct Environment {
    pub parent: Option<Rc<RefCell<Environment>>>,
    /// symbols: variables, functions, blueprints
    pub symbols: HashMap<String, SymbolInfo>,
    /// blueprints: full data for type checking (fields, methods, handles)
    /// Separate from symbols to distinguish between type definitions and instances
    pub blueprints: HashMap<String, BlueprintData>,
}

impl Environment {
    pub fn new() -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Environment {
            parent: None,
            symbols: HashMap::new(),
            blueprints: HashMap::new(),
        }))
    }

    pub fn with_parent(parent: Rc<RefCell<Environment>>) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Environment {
            parent: Some(parent),
            symbols: HashMap::new(),
            blueprints: HashMap::new(),
        }))
    }

    // ── Symbol operations ─────────────────────────────────────────────────

    pub fn define(&mut self, name: String, info: SymbolInfo) -> Result<(), String> {
        if let Some(existing) = self.symbols.get(&name) {
            // If both existing and incoming symbols are functions, allow function overloading
            if matches!(existing.kind, SymbolKind::Function { .. }) && matches!(info.kind, SymbolKind::Function { .. }) {
                self.symbols.insert(name, info);
                return Ok(());
            }
            return Err(format!(
                "Semantic Error: '{}' is already defined in this scope.",
                name
            ));
        }
        self.symbols.insert(name, info);
        Ok(())
    }

    /// Define or update symbol (used for stdlib injection)
    pub fn define_or_update(&mut self, name: String, info: SymbolInfo) {
        self.symbols.insert(name, info);
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

    pub fn mark_used(&mut self, name: &str) -> bool {
        if let Some(info) = self.symbols.get_mut(name) {
            info.is_used = true;
            return true;
        }
        if let Some(ref parent) = self.parent {
            return parent.borrow_mut().mark_used(name);
        }
        false
    }

    // ── Blueprint operations ──────────────────────────────────────────────

    pub fn define_blueprint(&mut self, name: String, data: BlueprintData) {
        self.blueprints.insert(name, data);
    }

    /// Look up blueprint in current and parent scopes
    pub fn lookup_blueprint(&self, name: &str) -> Option<BlueprintData> {
        if let Some(bp) = self.blueprints.get(name) {
            return Some(bp.clone());
        }
        if let Some(ref parent) = self.parent {
            return parent.borrow().lookup_blueprint(name);
        }
        None
    }

    /// Look up enum name containing a specific variant
    pub fn lookup_enum_for_variant(&self, variant_name: &str) -> Option<String> {
        for (name, bp) in &self.blueprints {
            if bp.variants.iter().any(|v| v.name == variant_name) {
                return Some(name.clone());
            }
        }
        if let Some(ref parent) = self.parent {
            return parent.borrow().lookup_enum_for_variant(variant_name);
        }
        None
    }

    /// Extract blueprint name from generic types like "custom<list>" -> "list"
    /// or "class<Node>" -> "Node"
    pub fn extract_blueprint_name(type_str: &str) -> Option<&str> {
        for prefix in &["custom<", "class<", "struct<", "enum<", "blueprint<"] {
            if type_str.starts_with(prefix) {
                return Some(type_str.trim_start_matches(prefix).trim_end_matches('>'));
            }
        }
        // Direct type name match (e.g. "list")
        None
    }
}
