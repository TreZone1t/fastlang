use crate::frontend::parser::ast::*;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

// ─────────────────────────────────────────────────────────────────────────────
// BlueprintData — كل ما يتعلق بنوع مُركّب (Custom, Class, Struct, Enum)
// يُبنى مرة واحدة من TypeMetadata أو من الـ AST Declaration
// ─────────────────────────────────────────────────────────────────────────────
#[derive(Debug, Clone)]
pub struct FnSignature {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: BaseType,
}

#[derive(Debug, Clone)]
pub struct BlueprintData {
    /// اسم الـ blueprint (مثل "list", "Node", "Option")
    pub name: String,
    /// الحقول (fields) مع أنواعها
    pub fields: HashMap<String, BaseType>,
    /// الـ methods مع signatures كاملة
    pub methods: HashMap<String, FnSignature>,
    /// الـ handles المُسجَّلة في هذا الـ blueprint
    /// هذا ما يُمكّن الـ operator overloading
    pub handles: HashSet<HandleMethods>,
    /// الـ settings (oop, function, etc.)
    pub settings: HashSet<Setting>,
    /// أسماء الـ generic params (مثل ["T", "U"])
    pub generics: Vec<String>,
    /// الـ params الخاصة بالـ custom scope
    pub params: Vec<Param>,
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
        }
    }

    /// هل يملك هذا الـ blueprint handle معيّن؟
    pub fn has_handle(&self, h: HandleMethods) -> bool {
        self.handles.contains(&h)
    }

    /// هل يقبل الـ handle المحدد معامل (parameter) من نوع معيّن؟
    /// يُستخدم للتحقق من صحة الـ operator overloading
    pub fn handle_accepts_type(&self, h: HandleMethods, value_type: &str) -> bool {
        let fn_name = h.as_str();
        if let Some(sig) = self.methods.get(fn_name) {
            if sig.params.is_empty() {
                // handle بدون params — يقبل أي شيء
                return true;
            }
            let param_type = sig.params[0].type_node.as_str();
            // لو param_type هو generic (T, U, ...) نقبل دائماً
            if self.generics.contains(&param_type) {
                return true;
            }
            // نقبل لو النوع متطابق أو مصفوفة
            if param_type == value_type {
                return true;
            }
            // لو الـ param هو array من نفس الـ generic
            if param_type.starts_with("array<") {
                return true; // array params في handles تقبل أي array
            }
            return false;
        }
        // لو ما وجدنا signature للـ handle — نقبل (تساهل في التحقق)
        true
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// SymbolKind — يصف نوع الـ symbol في الـ scope
// ─────────────────────────────────────────────────────────────────────────────
#[derive(Debug, Clone)]
pub enum SymbolKind {
    /// متغير عادي (int, float, bool, custom, etc.)
    Variable {
        type_node: BaseType,
        editability: Editability,
        is_array: bool,
    },
    /// دالة (fn)
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
// SymbolInfo — المعلومات المخزّنة لكل اسم في الـ scope
// ─────────────────────────────────────────────────────────────────────────────
#[derive(Debug, Clone)]
pub struct SymbolInfo {
    pub name: String,
    pub kind: SymbolKind,
    pub visibility: Visibility,
    pub dependencies: Vec<String>,
}

impl SymbolInfo {
    /// استرجاع نوع المتغير كـ String (للتحقق من التوافق)
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
            SymbolKind::Function { return_type, .. } => return_type.as_str(),
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

    // ── helpers للتوافق مع الكود القديم ──────────────────────────────────
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
// Environment — الـ scope الحالي مع parent chain
// ─────────────────────────────────────────────────────────────────────────────
pub struct Environment {
    pub parent: Option<Rc<RefCell<Environment>>>,
    /// symbols: variables, functions, blueprints
    pub symbols: HashMap<String, SymbolInfo>,
    /// blueprints: full data for type checking (fields, methods, handles)
    /// مستقل عن symbols عشان نفرق بين "تعريف النوع" و"متغير من هذا النوع"
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
        if self.symbols.contains_key(&name) {
            return Err(format!(
                "Semantic Error: '{}' is already defined in this scope.",
                name
            ));
        }
        self.symbols.insert(name, info);
        Ok(())
    }

    /// تعريف أو تحديث (للـ inject operations مثل stdlib)
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

    // ── Blueprint operations ──────────────────────────────────────────────

    pub fn define_blueprint(&mut self, name: String, data: BlueprintData) {
        self.blueprints.insert(name, data);
    }

    /// ابحث عن blueprint في الـ scope الحالي والـ parent scopes
    pub fn lookup_blueprint(&self, name: &str) -> Option<BlueprintData> {
        if let Some(bp) = self.blueprints.get(name) {
            return Some(bp.clone());
        }
        if let Some(ref parent) = self.parent {
            return parent.borrow().lookup_blueprint(name);
        }
        None
    }

    /// استخرج اسم الـ blueprint من نوع مثل "custom<list>" -> "list"
    /// أو "class<Node>" -> "Node"
    pub fn extract_blueprint_name(type_str: &str) -> Option<&str> {
        for prefix in &["custom<", "class<", "struct<", "enum<", "blueprint<"] {
            if type_str.starts_with(prefix) {
                return Some(type_str.trim_start_matches(prefix).trim_end_matches('>'));
            }
        }
        // لو النوع نفسه هو الاسم (مثل "list" مباشرة)
        None
    }
}
