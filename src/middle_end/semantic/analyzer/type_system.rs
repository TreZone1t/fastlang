use super::*;

impl SemanticAnalyzer {
    pub(crate) fn resolve_type(&self, ty: &BaseType) -> BaseType {
        match ty {
            BaseType::Blueprint { name, .. }
            | BaseType::Struct { name, .. }
            | BaseType::Class { name, .. }
            | BaseType::Enum { name, .. } => {
                if let Some(aliased) = self.current_env.borrow().resolve_type_alias(name) {
                    self.resolve_type(&aliased)
                } else {
                    ty.clone()
                }
            }
            BaseType::Array { base_type, size } => BaseType::Array {
                base_type: Box::new(self.resolve_type(base_type)),
                size: size.clone(),
            },
            BaseType::Generic(vec) => {
                BaseType::Generic(vec.iter().map(|t| self.resolve_type(t)).collect())
            }
            _ => ty.clone(),
        }
    }

    pub(crate) fn is_primitive_stack_type(&self, type_name: &str) -> bool {
        matches!(
            type_name,
            "int"
                | "int8"
                | "int16"
                | "int32"
                | "int64"
                | "int128"
                | "uint"
                | "uint8"
                | "uint16"
                | "uint32"
                | "uint64"
                | "uint128"
                | "usize"
                | "isize"
                | "float"
                | "float32"
                | "float64"
                | "float128"
                | "char"
                | "bool"
                | "void"
                | "type"
                | "any"
                | "unknown"
        )
    }

    pub(crate) fn type_has_drop_handle(&self, type_name: &str) -> bool {
        let extracted =
            extract_blueprint_name_from_type(type_name).unwrap_or_else(|| type_name.to_string());
        let clean_name = if let Some(idx) = extracted.find('<') {
            extracted[..idx].trim().to_string()
        } else {
            extracted
        };
        if let Some(bp) = self.current_env.borrow().lookup_blueprint(&clean_name) {
            if bp.handles.contains(&HandleMethods::Drop)
                || bp.handle_signatures.contains_key("drop")
                || bp.methods.contains_key("drop")
            {
                return true;
            }
        }
        if let Some(meta) = self.global_metadata.get(&clean_name) {
            if meta.handles.contains(&HandleMethods::Drop)
                || meta.handle_signatures.contains_key("drop")
                || meta.methods.contains_key("drop")
            {
                return true;
            }
        }
        false
    }

    pub(crate) fn is_class_type(&self, type_name: &str) -> bool {
        let clean_name =
            extract_blueprint_name_from_type(type_name).unwrap_or_else(|| type_name.to_string());
        if let Some(meta) = self.global_metadata.get(&clean_name) {
            if let BaseType::Class { .. } = &meta.ty {
                return true;
            }
        }
        false
    }

    pub(crate) fn is_struct_or_blueprint_type(&self, type_name: &str) -> bool {
        let clean_name =
            extract_blueprint_name_from_type(type_name).unwrap_or_else(|| type_name.to_string());
        if let Some(meta) = self.global_metadata.get(&clean_name) {
            if matches!(
                &meta.ty,
                BaseType::Struct { .. } | BaseType::Blueprint { .. }
            ) {
                return true;
            }
        }
        if self
            .current_env
            .borrow()
            .lookup_blueprint(&clean_name)
            .is_some()
        {
            return true;
        }
        false
    }
    pub(crate) fn types_are_compatible(&self, expected: &str, actual: &str) -> bool {
        if expected == actual
            || expected == "any"
            || expected == "unknown"
            || actual == "unknown"
            || actual == "undefined"
        {
            return true;
        }
        if self.is_generic_type(expected)
            || self.is_generic_type(actual)
            || expected.starts_with("...")
            || actual.starts_with("...")
        {
            return true;
        }
        if (expected == "flag" && actual == "bool") || (expected == "bool" && actual == "flag") {
            return true;
        }
        if expected == "object" || actual == "object" {
            let other = if expected == "object" {
                actual
            } else {
                expected
            };
            if other == "object"
                || self.is_class_type(other)
                || self.is_struct_or_blueprint_type(other)
                || other.starts_with("class::")
                || other.starts_with("struct::")
                || other.starts_with("blueprint::")
                || other.starts_with("class<")
                || other.starts_with("struct<")
                || other.starts_with("blueprint<")
            {
                return true;
            }
        }
        if expected == "function"
            || expected.starts_with("function<")
            || actual == "function"
            || actual.starts_with("function<")
        {
            let other = if expected == "function" || expected.starts_with("function<") {
                actual
            } else {
                expected
            };
            let other_lower = other.to_lowercase();
            if other == "function"
                || other_lower.starts_with("fn")
                || other_lower.starts_with("lambda")
                || other_lower.starts_with("macro")
                || other_lower.starts_with("method")
                || other_lower.starts_with("micro")
                || other_lower.starts_with("machine")
            {
                return true;
            }
        }
        if (expected.to_lowercase().starts_with("lambda")
            || expected.to_lowercase().starts_with("fn")
            || expected.to_lowercase().starts_with("method"))
            && (actual.to_lowercase().starts_with("lambda")
                || actual.to_lowercase().starts_with("fn")
                || actual.to_lowercase().starts_with("method"))
        {
            return true;
        }
        let norm_exp = if expected == "char[]" {
            "array<char>".to_string()
        } else if expected.ends_with("[]") {
            format!("array<{}>", &expected[..expected.len() - 2])
        } else {
            expected.to_string()
        };
        let norm_act = if actual == "char[]" {
            "array<char>".to_string()
        } else if actual.ends_with("[]") {
            format!("array<{}>", &actual[..actual.len() - 2])
        } else {
            actual.to_string()
        };
        if norm_exp == norm_act {
            return true;
        }
        if (expected == "type" || expected.starts_with("type<"))
            && (actual == "type"
                || actual.starts_with("type<")
                || actual.starts_with("int")
                || actual.starts_with("uint")
                || actual == "bool"
                || actual.starts_with("float"))
        {
            return true;
        }
        let exp_clean =
            extract_blueprint_name_from_type(expected).unwrap_or_else(|| expected.to_string());
        let act_clean =
            extract_blueprint_name_from_type(actual).unwrap_or_else(|| actual.to_string());
        if let Some(bp) = self.current_env.borrow().lookup_blueprint(&exp_clean) {
            if bp.handle_accepts_type(HandleMethods::Equal, actual)
                || bp.handle_accepts_type(HandleMethods::EqualAssign, actual)
            {
                return true;
            }
        }
        if let Some(sym) = self.current_env.borrow().lookup(&exp_clean) {
            if let SymbolKind::Variable {
                type_node: BaseType::Type(inner),
                ..
            } = &sym.kind
            {
                if matches!(**inner, BaseType::GenericParam(_)) {
                    return true;
                }
                let inner_str = inner.as_str();
                if inner_str != exp_clean {
                    return self.types_are_compatible(&inner_str, actual);
                }
            }
        }
        if let Some(sym) = self.current_env.borrow().lookup(&act_clean) {
            if let SymbolKind::Variable {
                type_node: BaseType::Type(inner),
                ..
            } = &sym.kind
            {
                if matches!(**inner, BaseType::GenericParam(_)) {
                    return true;
                }
                let inner_str = inner.as_str();
                if inner_str != act_clean {
                    return self.types_are_compatible(expected, &inner_str);
                }
            }
        }
        if (expected.to_lowercase().starts_with("method")
            || expected.starts_with("Fn")
            || expected.starts_with("fn")
            || expected.to_lowercase().contains("<method")
            || expected.contains("<Fn<"))
            && (actual.to_lowercase().starts_with("method")
                || actual.starts_with("Fn")
                || actual.starts_with("fn")
                || actual.to_lowercase().contains("<method")
                || actual.contains("<Fn<")
                || actual == "fn")
        {
            return true;
        }
        // int and uint family
        if (expected.starts_with("int")
            || expected.starts_with("uint")
            || expected == "byte"
            || expected == "usize"
            || expected == "isize")
            && (actual.starts_with("int")
                || actual.starts_with("uint")
                || actual == "byte"
                || actual == "usize"
                || actual == "isize")
        {
            return true;
        }
        if (expected == "int" || expected == "int32" || expected == "uint" || expected == "uint32")
            && (actual == "int" || actual == "uint")
        {
            return true;
        }
        // float family
        if expected.starts_with("float") && actual.starts_with("float") {
            return true;
        }
        // array family
        if expected.starts_with("array<") && actual.starts_with("array<") {
            let exp_inner = expected.trim_start_matches("array<").trim_end_matches('>');
            let act_inner = actual.trim_start_matches("array<").trim_end_matches('>');
            let exp_base = exp_inner.split('[').next().unwrap_or(exp_inner);
            let act_base = act_inner.split('[').next().unwrap_or(act_inner);
            return self.types_are_compatible(exp_base, act_base);
        }
        // blueprint/struct/class/enum/machine/block wrappers
        for prefix in &[
            "blueprint::",
            "struct::",
            "class::",
            "enum::",
            "machine::",
            "block::",
            "object::",
        ] {
            if expected.starts_with(prefix) {
                let inner = strip_wrapper(expected, prefix);
                if inner == actual || self.types_are_compatible(inner, actual) {
                    return true;
                }
            }
            if actual.starts_with(prefix) {
                let inner = strip_wrapper(actual, prefix);
                if inner == expected || self.types_are_compatible(expected, inner) {
                    return true;
                }
            }
        }
        let exp_bp_name =
            extract_blueprint_name_from_type(expected).unwrap_or_else(|| expected.to_string());
        if let Some(bp) = self
            .current_env
            .borrow()
            .lookup_blueprint(&exp_bp_name)
            .or_else(|| {
                self.global_metadata
                    .get(&exp_bp_name)
                    .map(build_blueprint_from_metadata)
            })
        {
            if bp.handle_accepts_type(HandleMethods::ArrowAssign, actual)
                || bp.handle_accepts_type(HandleMethods::Arrow, actual)
            {
                return true;
            }
            if let Some(meta) = self.global_metadata.get(&exp_bp_name) {
                if let Some(constructors) = &meta.constructor {
                    for c in constructors {
                        if c.params.len() == 1 {
                            let param_t = c.params[0].type_node.as_str();
                            if param_t == actual
                                || (param_t != expected
                                    && self.types_are_compatible(&param_t, actual))
                            {
                                return true;
                            }
                        }
                    }
                }
            }
        }
        // address compatibility (address<T> / T&)
        if expected == "address"
            || expected == "address<unknown>"
            || actual == "address"
            || actual == "address<unknown>"
        {
            return true;
        }
        if expected.starts_with("address<") {
            let inner = strip_wrapper(expected, "address<");
            if inner == "unknown" {
                return true;
            }
            let actual_inner = if actual.starts_with("address<") {
                strip_wrapper(actual, "address<")
            } else if actual.starts_with("array<") {
                strip_wrapper(actual, "array<")
            } else if actual.starts_with("raw_ptr<") {
                strip_wrapper(actual, "raw_ptr<")
            } else {
                actual
            };
            return self.types_are_compatible(inner, actual_inner);
        }
        if actual.starts_with("address<") {
            let actual_inner = strip_wrapper(actual, "address<");
            return self.types_are_compatible(expected, actual_inner);
        }
        false
    }

    // ----------------------------------------------------------
    // Helpers
    // ----------------------------------------------------------

    pub(crate) fn is_primitive_numeric(t: &str) -> bool {
        matches!(
            t,
            "int"
                | "int8"
                | "int16"
                | "int32"
                | "int64"
                | "int128"
                | "uint"
                | "uint8"
                | "uint16"
                | "uint32"
                | "uint64"
                | "uint128"
                | "byte"
                | "usize"
                | "isize"
                | "float"
                | "float32"
                | "float64"
                | "float128"
        )
    }

    pub fn get_fast_type(&self, inner_type: &str) -> crate::middle_end::interpreter::FastType {
        let base_name = if inner_type.contains('<') {
            inner_type.split('<').next().unwrap_or(inner_type)
        } else {
            inner_type
        };
        let (
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
        ) = if let Some(bp) = self.current_env.borrow().lookup_blueprint(base_name) {
            let mut ft_map = std::collections::HashMap::new();
            for (k, v) in &bp.fields {
                ft_map.insert(k.clone(), v.as_str().to_string());
            }
            let mut mt_map = std::collections::HashMap::new();
            for (k, v) in &bp.methods {
                mt_map.insert(k.clone(), v.return_type.as_str().to_string());
            }
            let mut ht_map = std::collections::HashMap::new();
            for (k, v) in &bp.handle_signatures {
                ht_map.insert(k.clone(), v.return_type.as_str().to_string());
            }
            (
                bp.handles.iter().any(|h| h.as_str() == "display"),
                bp.handles
                    .iter()
                    .any(|h| h.as_str() == "copy" || matches!(h, HandleMethods::Copy)),
                bp.handles
                    .iter()
                    .any(|h| h.as_str() == "cast" || matches!(h, HandleMethods::Cast)),
                bp.handles.iter().any(|h| {
                    h.as_str() == "throw"
                        || h.as_str() == "$throw"
                        || matches!(h, HandleMethods::Throw)
                }),
                bp.handles
                    .iter()
                    .any(|h| h.as_str() == "default" || matches!(h, HandleMethods::Default)),
                bp.handles
                    .iter()
                    .any(|h| h.as_str() == "share" || matches!(h, HandleMethods::Share)),
                bp.handles.iter().map(|h| h.as_str().to_string()).collect(),
                bp.methods.keys().cloned().collect(),
                bp.fields.keys().cloned().collect(),
                ft_map,
                mt_map,
                ht_map,
            )
        } else {
            (
                false,
                false,
                false,
                false,
                false,
                false,
                Vec::new(),
                Vec::new(),
                Vec::new(),
                std::collections::HashMap::new(),
                std::collections::HashMap::new(),
                std::collections::HashMap::new(),
            )
        };

        crate::middle_end::interpreter::FastType::with_full_metadata(
            crate::frontend::parser::ast::BaseType::from_str(inner_type),
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
        )
    }

    pub fn is_generic_type(&self, type_str: &str) -> bool {
        let mut clean = type_str.trim().trim_start_matches("...");
        while (clean.starts_with("type<") && clean.ends_with('>'))
            || (clean.starts_with("array<") && clean.ends_with('>'))
            || clean.ends_with("[]")
        {
            if clean.ends_with("[]") {
                clean = clean.trim_end_matches("[]");
            } else {
                let open_idx = match clean.find('<') {
                    Some(i) => i,
                    None => break,
                };
                clean = &clean[open_idx + 1..clean.len() - 1];
            }
            clean = clean.trim().trim_start_matches("...");
        }

        // 1. Check if defined in current environment as a generic parameter symbol
        let check_env = |name: &str| -> bool {
            if let Some(info) = self.current_env.borrow().lookup(name) {
                match &info.kind {
                    SymbolKind::Variable { type_node, .. } => {
                        if matches!(type_node, BaseType::GenericParam(_)) {
                            return true;
                        }
                        if let BaseType::Type(inner) = type_node {
                            if matches!(**inner, BaseType::GenericParam(_)) {
                                return true;
                            }
                        }
                    }
                    _ => {}
                }
            }
            false
        };

        if check_env(clean) || check_env(&format!("...{}", clean)) {
            return true;
        }

        // 2. An unbound type (not a known primitive, metadata type, or blueprint) is generic / unresolved
        if !self.is_primitive_stack_type(clean)
            && !self.global_metadata.contains_key(clean)
            && self.current_env.borrow().lookup_blueprint(clean).is_none()
        {
            return true;
        }

        false
    }

    pub fn eval_type_reflection_bool(&self, inner_type: &str, prop: &str) -> Option<bool> {
        if self.is_generic_type(inner_type) {
            return None;
        }
        let ft = self.get_fast_type(inner_type);
        if let Some(method_name) = prop.strip_prefix("has_") {
            return Some(
                ft.has_method(method_name)
                    || ft.has_handle(method_name)
                    || (method_name == "as_str"
                        && (ft.printable() || ft.has_method("as_str") || ft.has_handle("display"))),
            );
        }
        match prop {
            "printable" | "is_printable" => Some(ft.printable()),
            "is_pointer" => Some(ft.is_pointer()),
            "is_array" => Some(ft.is_array()),
            "is_primitive" => Some(ft.is_primitive()),
            "throwable" | "is_throwable" => Some(ft.throwable()),
            "shareable" | "is_shareable" => Some(ft.shareable()),
            "copyable" | "is_copyable" => Some(ft.copyable()),
            "castable" | "is_castable" => Some(ft.castable()),
            "is_constant" | "is_const" => Some(ft.is_constant()),
            "is_runtime" => Some(ft.is_runtime()),
            "is_comptime" => Some(ft.is_comptime()),
            "is_undefined" => Some(ft.is_undefined()),
            _ => None,
        }
    }

    pub fn eval_static_bool_expr(&mut self, expr: &Expr) -> Option<bool> {
        match expr {
            Expr::LiteralBool(b) => Some(*b),
            Expr::UnaryOp { operator, operand } if operator == "!" => {
                self.eval_static_bool_expr(operand).map(|b| !b)
            }
            Expr::Call { callee, args, .. } => {
                if let Expr::PropertyAccess { object, property } = &**callee {
                    let inner_opt = if let Ok(obj_type) = self.visit_expression(object) {
                        if obj_type.starts_with("type<") {
                            Some(strip_wrapper(&obj_type, "type<").to_string())
                        } else if obj_type == "type" {
                            if let Expr::Call {
                                callee: inner_callee,
                                args: inner_args,
                                ..
                            } = &**object
                            {
                                let is_typeof = match &**inner_callee {
                                    Expr::Identifier(name) => name == "typeof",
                                    Expr::NamespaceAccess {
                                        namespace,
                                        property,
                                    } => {
                                        namespace == "@compile"
                                            && matches!(&**property, Expr::Identifier(p) if p == "typeof")
                                    }
                                    _ => false,
                                };
                                if is_typeof {
                                    if let Some(arg) = inner_args.first() {
                                        self.visit_expression(arg).ok()
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    if let Some(inner) = inner_opt {
                        if property == "castable_to" {
                            if let Some(target_expr) = args.first() {
                                if let Ok(target_type_str) = self.visit_expression(target_expr) {
                                    let target_inner = strip_wrapper(&target_type_str, "type<");
                                    if self.is_generic_type(&inner)
                                        || self.is_generic_type(target_inner)
                                    {
                                        return None;
                                    }
                                    let target_ft = self.get_fast_type(target_inner);
                                    let ft = self.get_fast_type(&inner);
                                    if target_ft.base_type == BaseType::Unknown
                                        || ft.base_type == BaseType::Unknown
                                    {
                                        return None;
                                    }
                                    return Some(ft.castable_to(&target_ft));
                                }
                            }
                        }
                        let extract_name_or_type = |e: &Expr| -> String {
                            match e {
                                Expr::LiteralString(s) => strip_wrapper(s, "type<").to_string(),
                                Expr::Identifier(id) => strip_wrapper(id, "type<").to_string(),
                                Expr::PropertyAccess { property, .. } => property.clone(),
                                _ => format!("{:?}", e),
                            }
                        };
                        if property == "has_handle" {
                            if let Some(arg) = args.first() {
                                let s1 = extract_name_or_type(arg);
                                let s2 = args.get(1).map(extract_name_or_type);
                                let ft = self.get_fast_type(&inner);
                                return Some(ft.has_handle_query(&s1, s2.as_deref()));
                            }
                        }
                        if property == "has_method" {
                            if let Some(arg) = args.first() {
                                let s1 = extract_name_or_type(arg);
                                let s2 = args.get(1).map(extract_name_or_type);
                                let ft = self.get_fast_type(&inner);
                                return Some(ft.has_method_query(&s1, s2.as_deref()));
                            }
                        }
                        if property == "has_field" {
                            if let Some(arg) = args.first() {
                                let s1 = extract_name_or_type(arg);
                                let s2 = args.get(1).map(extract_name_or_type);
                                let ft = self.get_fast_type(&inner);
                                return Some(ft.has_field_query(&s1, s2.as_deref()));
                            }
                        }
                        return self.eval_type_reflection_bool(&inner, property);
                    }
                }
                None
            }
            Expr::PropertyAccess { object, property } => {
                if let Ok(obj_type) = self.visit_expression(object) {
                    if obj_type.starts_with("type<") {
                        let inner = strip_wrapper(&obj_type, "type<");
                        return self.eval_type_reflection_bool(inner, property);
                    } else if obj_type == "type" {
                        if let Expr::Call {
                            callee: inner_callee,
                            args,
                            ..
                        } = &**object
                        {
                            let is_typeof = match &**inner_callee {
                                Expr::Identifier(name) => name == "typeof",
                                Expr::NamespaceAccess {
                                    namespace,
                                    property,
                                } => {
                                    namespace == "@compile"
                                        && matches!(&**property, Expr::Identifier(p) if p == "typeof")
                                }
                                _ => false,
                            };
                            if is_typeof {
                                if let Some(arg) = args.first() {
                                    if let Ok(t) = self.visit_expression(arg) {
                                        return self.eval_type_reflection_bool(&t, property);
                                    }
                                }
                            }
                        }
                    }
                }
                None
            }
            _ => None,
        }
    }
}

pub(crate) fn strip_wrapper<'a>(s: &'a str, prefix: &str) -> &'a str {
    if let Some(rest) = s.strip_prefix(prefix) {
        if let Some(inner) = rest.strip_suffix('>') {
            return inner;
        }
        return rest;
    }
    s
}

pub(crate) fn extract_type_args_from_str(s: &str) -> (String, Vec<BaseType>) {
    let mut trimmed = s.trim();
    for prefix in &[
        "blueprint::",
        "struct::",
        "class::",
        "enum::",
        "machine::",
        "block::",
        "object::",
    ] {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            if let Some(inner) = rest.strip_suffix('>') {
                trimmed = inner.trim();
                break;
            }
        }
    }

    if let Some(start) = trimmed.find('<') {
        if let Some(rest) = trimmed.strip_suffix('>') {
            let base = trimmed[..start].trim().to_string();
            let inside = &rest[start + 1..];
            let mut args = Vec::new();
            for arg_str in inside.split(',') {
                let clean_arg = arg_str.trim();
                if !clean_arg.is_empty() {
                    args.push(BaseType::from_str(clean_arg));
                }
            }
            return (base, args);
        }
    }
    (trimmed.to_string(), Vec::new())
}

#[allow(dead_code)]
pub(crate) fn extract_iter_payload_type(t: &BaseType) -> String {
    match t {
        BaseType::Enum { name, generics, .. } if name == "Option" && !generics.is_empty() => {
            extract_iter_payload_type(&generics[0])
        }
        BaseType::Generic(vec) if !vec.is_empty() => extract_iter_payload_type(&vec[0]),
        _ => t.as_str(),
    }
}
