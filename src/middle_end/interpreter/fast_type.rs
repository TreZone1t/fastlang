use crate::frontend::parser::ast::{BaseType, Size};

/// `FastType`: Compile-time Type Representation in the Interpreter.
/// Extends `BaseType` with reflection attributes, handle capabilities, and compile-time methods.
#[derive(Debug, Clone, PartialEq)]
pub struct FastType {
    pub base_type: BaseType,
    pub has_display: bool,
    pub has_copy: bool,
    pub has_cast: bool,
    pub has_throw: bool,
    pub has_default: bool,
    pub has_share: bool,
    pub handles: Vec<String>,
    pub methods: Vec<String>,
    pub fields: Vec<String>,
    pub field_types: std::collections::HashMap<String, String>,
    pub method_types: std::collections::HashMap<String, String>,
    pub handle_types: std::collections::HashMap<String, String>,
}

impl FastType {
    pub fn new(base_type: BaseType) -> Self {
        Self {
            base_type,
            has_display: false,
            has_copy: false,
            has_cast: false,
            has_throw: false,
            has_default: false,
            has_share: false,
            handles: Vec::new(),
            methods: Vec::new(),
            fields: Vec::new(),
            field_types: std::collections::HashMap::new(),
            method_types: std::collections::HashMap::new(),
            handle_types: std::collections::HashMap::new(),
        }
    }

    pub fn from_name(name: &str) -> Self {
        Self::new(BaseType::from_str(name))
    }

    pub fn with_handles(
        base_type: BaseType,
        has_display: bool,
        has_copy: bool,
        has_cast: bool,
    ) -> Self {
        Self {
            base_type,
            has_display,
            has_copy,
            has_cast,
            has_throw: false,
            has_default: false,
            has_share: false,
            handles: Vec::new(),
            methods: Vec::new(),
            fields: Vec::new(),
            field_types: std::collections::HashMap::new(),
            method_types: std::collections::HashMap::new(),
            handle_types: std::collections::HashMap::new(),
        }
    }

    pub fn with_all_handles(
        base_type: BaseType,
        has_display: bool,
        has_copy: bool,
        has_cast: bool,
        has_throw: bool,
        has_default: bool,
        has_share: bool,
    ) -> Self {
        Self {
            base_type,
            has_display,
            has_copy,
            has_cast,
            has_throw,
            has_default,
            has_share,
            handles: Vec::new(),
            methods: Vec::new(),
            fields: Vec::new(),
            field_types: std::collections::HashMap::new(),
            method_types: std::collections::HashMap::new(),
            handle_types: std::collections::HashMap::new(),
        }
    }

    pub fn with_metadata(
        base_type: BaseType,
        has_display: bool,
        has_copy: bool,
        has_cast: bool,
        has_throw: bool,
        has_default: bool,
        has_share: bool,
        handles: Vec<String>,
        methods: Vec<String>,
        fields: Vec<String>,
    ) -> Self {
        Self {
            base_type,
            has_display,
            has_copy,
            has_cast,
            has_throw,
            has_default,
            has_share,
            handles,
            methods,
            fields,
            field_types: std::collections::HashMap::new(),
            method_types: std::collections::HashMap::new(),
            handle_types: std::collections::HashMap::new(),
        }
    }

    pub fn with_full_metadata(
        base_type: BaseType,
        has_display: bool,
        has_copy: bool,
        has_cast: bool,
        has_throw: bool,
        has_default: bool,
        has_share: bool,
        handles: Vec<String>,
        methods: Vec<String>,
        fields: Vec<String>,
        field_types: std::collections::HashMap<String, String>,
        method_types: std::collections::HashMap<String, String>,
        handle_types: std::collections::HashMap<String, String>,
    ) -> Self {
        Self {
            base_type,
            has_display,
            has_copy,
            has_cast,
            has_throw,
            has_default,
            has_share,
            handles,
            methods,
            fields,
            field_types,
            method_types,
            handle_types,
        }
    }

    pub fn is_primitive(&self) -> bool {
        matches!(
            self.base_type,
            BaseType::Int(_)
                | BaseType::UInt(_)
                | BaseType::USize
                | BaseType::ISize
                | BaseType::Float(_)
                | BaseType::Bool
                | BaseType::Char
                | BaseType::UChar
                | BaseType::Void
        )
    }

    pub fn is_pointer(&self) -> bool {
        matches!(self.base_type, BaseType::RawPointer(_))
            || self.base_type.as_str().starts_with("raw_ptr<")
            || self.base_type.as_str().starts_with('&')
            || self.base_type.as_str().ends_with('*')
    }

    pub fn is_array(&self) -> bool {
        matches!(self.base_type, BaseType::Array { .. })
            || self.base_type.as_str().starts_with("array<")
            || self.base_type.as_str().ends_with("[]")
    }

    pub fn is_constant(&self) -> bool {
        self.base_type.as_str().starts_with("const ")
    }

    pub fn is_runtime(&self) -> bool {
        false
    }

    pub fn is_comptime(&self) -> bool {
        true
    }

    pub fn is_undefined(&self) -> bool {
        matches!(self.base_type, BaseType::Unknown)
            || self.base_type.as_str() == "undefined"
            || self.base_type.as_str() == "Unknown"
    }

    pub fn has_handle(&self, name: &str) -> bool {
        let clean = name.trim();
        self.handles.iter().any(|h| h == clean)
            || (clean == "display" && self.has_display)
            || (clean == "copy" && self.has_copy)
            || (clean == "cast" && self.has_cast)
            || ((clean == "throw" || clean == "$throw") && self.has_throw)
            || (clean == "default" && self.has_default)
            || (clean == "share" && self.has_share)
            || (clean == "as_str" && (self.printable() || self.as_str() == "str"))
    }

    pub fn has_method(&self, name: &str) -> bool {
        let clean = name.trim();
        self.methods.iter().any(|m| m == clean)
            || (clean == "as_str" && (self.printable() || self.as_str() == "str"))
    }

    pub fn has_field(&self, name: &str) -> bool {
        let clean = name.trim();
        self.fields.iter().any(|f| f == clean)
    }

    pub fn has_handle_query(&self, name_or_type: &str, target_type: Option<&str>) -> bool {
        let clean = name_or_type.trim();
        if let Some(t) = target_type {
            let t_clean = t.trim();
            if clean == "as_str" && (self.printable() || self.as_str() == "str") {
                return t_clean == "str" || type_matches("str", t_clean);
            }
            if let Some(actual_t) = self.handle_types.get(clean) {
                return actual_t == t_clean || type_matches(actual_t, t_clean);
            }
            return false;
        }
        if self.has_handle(clean) {
            return true;
        }
        self.handle_types.values().any(|v| v == clean || type_matches(v, clean))
    }

    pub fn has_method_query(&self, name_or_type: &str, target_type: Option<&str>) -> bool {
        let clean = name_or_type.trim();
        if let Some(t) = target_type {
            let t_clean = t.trim();
            if clean == "as_str" && (self.printable() || self.as_str() == "str") {
                return t_clean == "str" || type_matches("str", t_clean);
            }
            if let Some(actual_t) = self.method_types.get(clean) {
                return actual_t == t_clean || type_matches(actual_t, t_clean);
            }
            return false;
        }
        if self.has_method(clean) {
            return true;
        }
        self.method_types.values().any(|v| v == clean || type_matches(v, clean))
    }

    pub fn has_field_query(&self, name_or_type: &str, target_type: Option<&str>) -> bool {
        let clean = name_or_type.trim();
        if let Some(t) = target_type {
            let t_clean = t.trim();
            if let Some(actual_t) = self.field_types.get(clean) {
                return actual_t == t_clean || type_matches(actual_t, t_clean);
            }
            return false;
        }
        if self.has_field(clean) {
            return true;
        }
        self.field_types.values().any(|v| v == clean || type_matches(v, clean))
    }

    pub fn printable(&self) -> bool {
        self.is_primitive() || self.has_display
    }

    pub fn throwable(&self) -> bool {
        self.has_throw
    }

    pub fn shareable(&self) -> bool {
        self.has_share
    }

    pub fn copyable(&self) -> bool {
        self.is_primitive() || self.has_copy
    }

    pub fn castable(&self) -> bool {
        match &self.base_type {
            BaseType::Int(_)
            | BaseType::UInt(_)
            | BaseType::USize
            | BaseType::ISize
            | BaseType::Float(_)
            | BaseType::Bool
            | BaseType::Char
            | BaseType::UChar => true,
            _ if self.is_pointer() => true,
            _ => self.has_cast,
        }
    }

    pub fn castable_to(&self, target: &FastType) -> bool {
        if self.is_primitive() && target.is_primitive() {
            return true;
        }
        if self.base_type == target.base_type {
            return true;
        }
        if self.is_pointer() && target.is_pointer() {
            return true;
        }
        self.has_cast
    }

    pub fn as_str(&self) -> String {
        self.base_type.as_str()
    }

    pub fn size(&self) -> u32 {
        match &self.base_type {
            BaseType::Int(sz) | BaseType::UInt(sz) => match sz {
                Size::S8 => 1,
                Size::S16 => 2,
                Size::S32 => 4,
                Size::S64 => 8,
                Size::S128 => 16,
            },
            BaseType::USize | BaseType::ISize => 8,
            BaseType::Float(sz) => match sz {
                Size::S8 | Size::S16 | Size::S32 => 4,
                Size::S64 | Size::S128 => 8,
            },
            BaseType::Bool => 1,
            BaseType::Char => 1,
            BaseType::UChar => 4,
            BaseType::Void => 0,
            _ if self.is_pointer() => 8,
            _ => 8,
        }
    }
}

fn normalize_type_str(s: &str) -> &str {
    match s {
        "int" => "int32",
        "uint" => "uint32",
        "float" => "float32",
        _ => s,
    }
}

fn type_matches(actual: &str, expected: &str) -> bool {
    let a = actual.trim();
    let e = expected.trim();
    if a == e {
        return true;
    }
    normalize_type_str(a) == normalize_type_str(e)
}

