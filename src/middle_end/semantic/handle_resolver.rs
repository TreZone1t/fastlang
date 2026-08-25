use crate::frontend::parser::ast::HandleMethods;
use crate::middle_end::semantic::environment::{BlueprintData, Environment};
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
        "+="  => HandleMethods::IndexAdd,
        "-="  => HandleMethods::IndexSub,
        "*="  => HandleMethods::IndexMul,
        "/="  => HandleMethods::IndexDiv,
        "%="  => HandleMethods::IndexMod,
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
    const PREFIXES: &[&str] = &["custom<", "class<", "struct<", "enum<", "blueprint<", "name<", "pointer<"];
    for prefix in PREFIXES {
        if let Some(rest) = type_str.strip_prefix(prefix) {
            let base_name = rest.split('<').next().unwrap_or(rest).trim_end_matches('>').to_string();
            return Some(base_name);
        }
    }
    None
}

// ─────────────────────────────────────────────────────────────────────────────
// is_complex_type — checks if type is composite/user-defined (can define handles)
// ─────────────────────────────────────────────────────────────────────────────
pub fn is_complex_type(type_str: &str) -> bool {
    type_str.starts_with("custom<")
        || type_str.starts_with("class<")
        || type_str.starts_with("struct<")
        || type_str.starts_with("enum<")
        || type_str.starts_with("blueprint<")
}

// ─────────────────────────────────────────────────────────────────────────────
// build_blueprint_from_metadata
// Converts TypeMetadata (from parser) into BlueprintData
// ─────────────────────────────────────────────────────────────────────────────
pub fn build_blueprint_from_metadata(
    meta: &crate::frontend::parser::ast::TypeMetadata,
) -> BlueprintData {
    use crate::middle_end::semantic::environment::FnSignature;

    let mut bp = BlueprintData::new(meta.name.clone());

    // fields
    for (field_name, field_type) in &meta.fields {
        bp.fields.insert(field_name.clone(), field_type.clone());
    }

    // methods
    for (method_name, fn_type) in &meta.methods {
        bp.methods.insert(
            method_name.clone(),
            FnSignature {
                name: fn_type.name.clone(),
                params: fn_type.params.clone(),
                return_type: fn_type.return_type.clone(),
                is_virtual: true,
                is_abstract: false,
            },
        );
    }

    // handles
    for h in &meta.handles {
        bp.handles.insert(*h);
    }

    // generics — extract generic parameter names from type
    for g in &meta.generics {
        if let crate::frontend::parser::ast::BaseType::New(name) = g {
            bp.generics.push(name.clone());
        } else {
            // Generic parameters stored as identifiers
            let s = g.as_str();
            if !s.is_empty() && s != "unknown" {
                bp.generics.push(s);
            }
        }
    }

    // params
    bp.params = meta.params.clone();

    bp
}
