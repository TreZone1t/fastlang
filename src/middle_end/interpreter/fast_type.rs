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
        }
    }

    pub fn from_name(name: &str) -> Self {
        Self::new(BaseType::from_str(name))
    }

    pub fn with_handles(base_type: BaseType, has_display: bool, has_copy: bool, has_cast: bool) -> Self {
        Self {
            base_type,
            has_display,
            has_copy,
            has_cast,
            has_throw: false,
            has_default: false,
        }
    }

    pub fn with_all_handles(
        base_type: BaseType,
        has_display: bool,
        has_copy: bool,
        has_cast: bool,
        has_throw: bool,
        has_default: bool,
    ) -> Self {
        Self {
            base_type,
            has_display,
            has_copy,
            has_cast,
            has_throw,
            has_default,
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
                | BaseType::Str
                | BaseType::Void
        )
    }

    pub fn is_pointer(&self) -> bool {
        matches!(
            self.base_type,
            BaseType::Pointer(_) | BaseType::Modify(_)
        ) || self.base_type.as_str().starts_with("name<")
            || self.base_type.as_str().starts_with("pointer<")
            || self.base_type.as_str().starts_with('&')
            || self.base_type.as_str().ends_with('*')
    }

    pub fn is_array(&self) -> bool {
        matches!(self.base_type, BaseType::Array { .. })
            || self.base_type.as_str().starts_with("array<")
            || self.base_type.as_str().ends_with("[]")
    }

    pub fn printable(&self) -> bool {
        self.is_primitive() || self.has_display
    }

    pub fn throwable(&self) -> bool {
        self.has_throw
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
            | BaseType::Char => true,
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
            BaseType::Char => 4,
            BaseType::Str => 8,
            BaseType::Void => 0,
            _ if self.is_pointer() => 8,
            _ => 8,
        }
    }
}
