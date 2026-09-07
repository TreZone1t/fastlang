use crate::frontend::parser::ast::HandleMethods;
use crate::middle_end::semantic::environment::{BlueprintData, Environment, FnSignature};
use std::cell::RefCell;
use std::rc::Rc;

// ─────────────────────────────────────────────────────────────────────────────
// op_to_handle — maps an operator string to its corresponding HandleMethods
// ─────────────────────────────────────────────────────────────────────────────
pub fn op_to_handle(op: &str) -> HandleMethods {
    match op {
        "->"  => HandleMethods::Arrow,
        "arrow_assign" => HandleMethods::ArrowAssign,
        "=>"  => HandleMethods::FatArrow,
        "="   => HandleMethods::Equal,
        "+="  => HandleMethods::Add,
        "-="  => HandleMethods::Sub,
        "*="  => HandleMethods::Mul,
        "/="  => HandleMethods::Div,
        "%="  => HandleMethods::Mod,
        "+"   => HandleMethods::Add,
        "-"   => HandleMethods::Sub,
        "*"   => HandleMethods::Mul,
        "/"   => HandleMethods::Div,
        "%"   => HandleMethods::Mod,
        "=="  => HandleMethods::PartialEqual,
        "!="  => HandleMethods::NotEqual,
        ">"   => HandleMethods::GreaterThan,
        "<"   => HandleMethods::LessThan,
        ">="  => HandleMethods::GreaterThanEqual,
        "<="  => HandleMethods::LessThanEqual,
        "&&"  => HandleMethods::And,
        "||"  => HandleMethods::Or,
        "++"  => HandleMethods::Increment,
        "--"  => HandleMethods::Decrement,
        _     => HandleMethods::NotFound,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// resolve_handle_for_op — looks up a blueprint defining a handle for the given operator
// Used in VarDecl and ReassignStmt to verify operator overloading
// ─────────────────────────────────────────────────────────────────────────────
pub fn resolve_handle_for_op(
    env: &Rc<RefCell<Environment>>,
    blueprint_name: &str,
    op: &str,
) -> HandleLookupResult {
    let handle = op_to_handle(op);

    if matches!(handle, HandleMethods::NotFound) {
        return HandleLookupResult::UnknownOp;
    }

    let bp = env.borrow().lookup_blueprint(blueprint_name);
    match bp {
        None => HandleLookupResult::BlueprintNotFound,
        Some(data) => {
            if data.has_handle(handle) {
                HandleLookupResult::Found(data)
            } else {
                HandleLookupResult::HandleMissing { handle }
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// HandleLookupResult — outcome of handle resolution
// ─────────────────────────────────────────────────────────────────────────────
#[derive(Debug)]
pub enum HandleLookupResult {
    /// Handle was found in the blueprint
    Found(BlueprintData),
    /// Blueprint was not found in the scope (not yet defined)
    BlueprintNotFound,
    /// Blueprint exists but does not implement this handle
    HandleMissing { handle: HandleMethods },
    /// Operator is unknown
    UnknownOp,
}

// ─────────────────────────────────────────────────────────────────────────────
// extract_blueprint_name_from_type
// Extracts blueprint name from types like:
//   "custom<list>"   -> Some("list")
//   "class<Node>"    -> Some("Node")
//   "struct<Point>"  -> Some("Point")
//   "int32"          -> None
// ─────────────────────────────────────────────────────────────────────────────
pub fn extract_blueprint_name_from_type(type_str: &str) -> Option<String> {
    const PREFIXES: &[&str] = &["class<", "struct<", "enum<", "blueprint<", "machine<", "block<", "name<", "pointer<", "modify<", "copy<"];
    for prefix in PREFIXES {
        if let Some(rest) = type_str.strip_prefix(prefix) {
            let base_name = rest.split('<').next().unwrap_or(rest).trim_end_matches('>').to_string();
            return Some(base_name);
        }
    }
    None
}

pub fn extract_all_type_names(type_str: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut current = String::new();
    for ch in type_str.chars() {
        if ch.is_alphanumeric() || ch == '_' {
            current.push(ch);
        } else {
            if !current.is_empty() {
                if !is_primitive_type(&current) {
                    names.push(current.clone());
                }
                current.clear();
            }
        }
    }
    if !current.is_empty() && !is_primitive_type(&current) {
        names.push(current);
    }
    names
}
fn is_primitive_type(s: &str) -> bool {
    matches!(
        s,
        "int8" | "int16" | "int32" | "int64" | "int128" | "int"
        | "uint8" | "uint16" | "uint32" | "uint64" | "uint128" | "uint"
        | "float32" | "float64" | "float128" | "float"
        | "char" | "bool" | "void" | "type" | "unknown" | "any"
        | "name" | "modify" | "copy" | "pointer" | "array" | "class" | "struct" | "enum" | "blueprint" | "machine" | "block"
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// is_complex_type — checks if type is composite/user-defined (can define handles)
// ─────────────────────────────────────────────────────────────────────────────
pub fn is_complex_type(type_str: &str) -> bool {
    type_str.starts_with("class<")
        || type_str.starts_with("struct<")
        || type_str.starts_with("enum<")
        || type_str.starts_with("blueprint<")
        || type_str.starts_with("machine<")
        || type_str.starts_with("block<")
}

// ─────────────────────────────────────────────────────────────────────────────
// build_blueprint_from_metadata
// Converts TypeMetadata (from parser) into BlueprintData
// ─────────────────────────────────────────────────────────────────────────────
// build_blueprint_from_base_type — creates a BlueprintData from BaseType
// ─────────────────────────────────────────────────────────────────────────────
pub fn build_blueprint_from_base_type(
    ty: &crate::frontend::parser::ast::BaseType,
) -> BlueprintData {
    use crate::middle_end::semantic::environment::FnSignature;
    use crate::frontend::parser::ast::BaseType;

    let name = ty.get_name();
    let mut bp = BlueprintData::new(name);

    match ty {
        BaseType::Struct { fields, methods, generics, .. }
        | BaseType::Blueprint { fields, methods, generics, .. } => {
            for (f_name, f_type) in fields.as_ref() {
                bp.fields.insert(f_name.clone(), f_type.clone());
            }
            for (m_name, fn_type) in methods.as_ref() {
                let hk = HandleMethods::from_str(m_name.as_str());
                if hk != HandleMethods::NotFound {
                    bp.handles.insert(hk);
                }
                bp.methods.insert(
                    m_name.clone(),
                    FnSignature {
                        name: fn_type.name.clone(),
                        generics: fn_type.generics.clone(),
                        params: fn_type.params.clone(),
                        return_type: fn_type.return_type.clone(),
                        is_virtual: false,
                        is_abstract: false,
                    },
                );
            }
            for g in generics {
                if let BaseType::New(name) = g {
                    bp.generics.push(name.clone());
                } else {
                    let s = g.as_str();
                    if !s.is_empty() && s != "unknown" {
                        bp.generics.push(s);
                    }
                }
            }
        }
        BaseType::Class { fields, methods, generics, .. } => {
            bp.is_class = true;
            for (f_name, f_type) in fields.as_ref() {
                bp.fields.insert(f_name.clone(), f_type.clone());
            }
            for (m_name, fn_type) in methods.as_ref() {
                let hk = HandleMethods::from_str(m_name.as_str());
                if hk != HandleMethods::NotFound {
                    bp.handles.insert(hk);
                }
                bp.methods.insert(
                    m_name.clone(),
                    FnSignature {
                        name: fn_type.name.clone(),
                        generics: fn_type.generics.clone(),
                        params: fn_type.params.clone(),
                        return_type: fn_type.return_type.clone(),
                        is_virtual: true,
                        is_abstract: false,
                    },
                );
            }
            for g in generics {
                if let BaseType::New(name) = g {
                    bp.generics.push(name.clone());
                } else {
                    let s = g.as_str();
                    if !s.is_empty() && s != "unknown" {
                        bp.generics.push(s);
                    }
                }
            }
        }
        BaseType::Enum { variants, methods, generics, .. } => {
            for v in variants {
                bp.fields.insert(v.name.clone(), BaseType::from_str(&ty.get_name()));
            }
            for (m_name, fn_type) in methods.as_ref() {
                let hk = HandleMethods::from_str(m_name.as_str());
                if hk != HandleMethods::NotFound {
                    bp.handles.insert(hk);
                }
                bp.methods.insert(
                    m_name.clone(),
                    FnSignature {
                        name: fn_type.name.clone(),
                        generics: fn_type.generics.clone(),
                        params: fn_type.params.clone(),
                        return_type: fn_type.return_type.clone(),
                        is_virtual: false,
                        is_abstract: false,
                    },
                );
            }
            for g in generics {
                if let BaseType::New(name) = g {
                    bp.generics.push(name.clone());
                } else {
                    let s = g.as_str();
                    if !s.is_empty() && s != "unknown" {
                        bp.generics.push(s);
                    }
                }
            }
        }
        BaseType::Machine { fields, methods, .. } => {
            for (f_name, f_type) in fields.as_ref() {
                bp.fields.insert(f_name.clone(), f_type.clone());
            }
            for (m_name, fn_type) in methods.as_ref() {
                let hk = HandleMethods::from_str(m_name.as_str());
                if hk != HandleMethods::NotFound {
                    bp.handles.insert(hk);
                }
                bp.methods.insert(
                    m_name.clone(),
                    FnSignature {
                        name: fn_type.name.clone(),
                        generics: fn_type.generics.clone(),
                        params: fn_type.params.clone(),
                        return_type: fn_type.return_type.clone(),
                        is_virtual: false,
                        is_abstract: false,
                    },
                );
            }
        }
        BaseType::Block { methods, .. } => {
            for (m_name, fn_type) in methods.as_ref() {
                let hk = HandleMethods::from_str(m_name.as_str());
                if hk != HandleMethods::NotFound {
                    bp.handles.insert(hk);
                }
                bp.methods.insert(
                    m_name.clone(),
                    FnSignature {
                        name: fn_type.name.clone(),
                        generics: fn_type.generics.clone(),
                        params: fn_type.params.clone(),
                        return_type: fn_type.return_type.clone(),
                        is_virtual: false,
                        is_abstract: false,
                    },
                );
            }
        }
        _ => {}
    }

    bp
}

pub fn build_blueprint_from_metadata(
    meta: &crate::frontend::parser::ast::TypeMetadata,
) -> BlueprintData {
    let mut bp = build_blueprint_from_base_type(&meta.ty);
    for h in &meta.handles {
        bp.handles.insert(*h);
    }
    for (f_name, f_type) in &meta.fields {
        bp.fields.entry(f_name.clone()).or_insert_with(|| f_type.clone());
    }
    let is_class = bp.is_class;
    for (m_name, fn_type) in &meta.methods {
        let hk = HandleMethods::from_str(m_name.as_str());
        if hk != HandleMethods::NotFound {
            bp.handles.insert(hk);
        }
        bp.methods.entry(m_name.clone()).or_insert_with(|| FnSignature {
            name: fn_type.name.clone(),
            generics: fn_type.generics.clone(),
            params: fn_type.params.clone(),
            return_type: fn_type.return_type.clone(),
            is_virtual: is_class,
            is_abstract: false,
        });
    }
    bp
}
