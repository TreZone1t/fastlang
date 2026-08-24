use serde_json::{json, Value};
use crate::frontend::parser::ast::*;

pub fn ast_to_json_string(ast: &[Stmt]) -> String {
    let val = stmts_to_json(ast);
    serde_json::to_string_pretty(&val).unwrap_or_else(|_| "[]".to_string())
}

pub fn stmts_to_json(stmts: &[Stmt]) -> Value {
    let items: Vec<Value> = stmts.iter().map(stmt_to_json).collect();
    json!(items)
}

pub fn either_block_to_json(eb: &EitherBlock) -> Value {
    match eb {
        EitherBlock::Inline(stmts) => json!({
            "type": "Inline",
            "statements": stmts_to_json(stmts)
        }),
        EitherBlock::External(expr) => json!({
            "type": "External",
            "expr": expr_to_json(expr)
        }),
    }
}

pub fn stmt_to_json(stmt: &Stmt) -> Value {
    match stmt {
        Stmt::Declaration(decl) => json!({
            "type": "Declaration",
            "decl": decl_to_json(decl)
        }),
        Stmt::ExpressionStmt(expr) => json!({
            "type": "ExpressionStmt",
            "expr": expr_to_json(expr)
        }),
        Stmt::CallStmt(expr) => json!({
            "type": "CallStmt",
            "expr": expr_to_json(expr)
        }),
        Stmt::ReassignStmt { target, value, op } => json!({
            "type": "ReassignStmt",
            "target": expr_to_json(target),
            "op": op,
            "value": expr_to_json(value)
        }),
        Stmt::ReturnStmt(expr) => json!({
            "type": "ReturnStmt",
            "value": expr_to_json(expr)
        }),
        Stmt::YieldStmt(expr) => json!({
            "type": "YieldStmt",
            "value": expr.as_ref().map(expr_to_json)
        }),
        Stmt::LeaveStmt => json!({
            "type": "LeaveStmt"
        }),
        Stmt::BreakStmt => json!({
            "type": "BreakStmt"
        }),
        Stmt::ContinueStmt => json!({
            "type": "ContinueStmt"
        }),
        Stmt::ThrowStmt(expr) => json!({
            "type": "ThrowStmt",
            "value": expr_to_json(expr)
        }),
        Stmt::IfStmt { condition, then_block, else_block } => json!({
            "type": "IfStmt",
            "condition": expr_to_json(condition),
            "then": stmts_to_json(then_block),
            "else": else_block.as_ref().map(|b| stmts_to_json(b))
        }),
        Stmt::WhileStmt { condition, body } => json!({
            "type": "WhileStmt",
            "condition": expr_to_json(condition),
            "body": either_block_to_json(body)
        }),
        Stmt::DoWhileStmt { body, condition } => json!({
            "type": "DoWhileStmt",
            "body": either_block_to_json(body),
            "condition": expr_to_json(condition)
        }),
        Stmt::ForInStmt { item, iterable, body } => json!({
            "type": "ForInStmt",
            "item": stmt_to_json(item),
            "iterable": expr_to_json(iterable),
            "body": either_block_to_json(body)
        }),
        Stmt::ForStmt { init, condition, increment, body } => json!({
            "type": "ForStmt",
            "init": init.as_ref().map(|s| stmt_to_json(s)),
            "condition": condition.as_ref().map(expr_to_json),
            "increment": increment.as_ref().map(|s| stmt_to_json(s)),
            "body": either_block_to_json(body)
        }),
        Stmt::LoopStmt { count, body } => json!({
            "type": "LoopStmt",
            "count": count.as_ref().map(expr_to_json),
            "body": either_block_to_json(body)
        }),
        Stmt::TryCatchStmt { try_block, catch_param, catch_block } => json!({
            "type": "TryCatchStmt",
            "try": stmts_to_json(try_block),
            "catch_param": catch_param,
            "catch": stmts_to_json(catch_block)
        }),
        Stmt::SwitchStmt { name, condition, cases } => json!({
            "type": "SwitchStmt",
            "name": name,
            "condition": expr_to_json(condition),
            "cases": stmts_to_json(cases)
        }),
        Stmt::CaseStmt { option, set, body } => json!({
            "type": "CaseStmt",
            "option": expr_to_json(option),
            "set": expr_to_json(set),
            "body": stmts_to_json(body)
        }),
        Stmt::GotoStmt(expr) => json!({
            "type": "GotoStmt",
            "target": expr_to_json(expr)
        }),
        Stmt::DelStmt { target, is_array } => json!({
            "type": "DelStmt",
            "target": expr_to_json(target),
            "is_array": is_array
        }),
        Stmt::EnableStmt(setting) => json!({
            "type": "EnableStmt",
            "setting": setting
        }),
        Stmt::AddPropertyStmt { kind_name, value } => json!({
            "type": "AddPropertyStmt",
            "kind": kind_name,
            "value": expr_to_json(value)
        }),
        _ => json!({
            "type": "OtherStmt"
        })
    }
}

pub fn decl_to_json(decl: &Decl) -> Value {
    match decl {
        Decl::VarDecl { visibility, editability, type_node, name, value, .. } => json!({
            "type": "VarDecl",
            "name": name,
            "type_node": type_node.as_str(),
            "visibility": format!("{:?}", visibility),
            "editability": format!("{:?}", editability),
            "value": expr_to_json(value)
        }),
        Decl::ArrayDecl { visibility, editability, type_node, name, length, value, .. } => json!({
            "type": "ArrayDecl",
            "name": name,
            "type_node": type_node.as_str(),
            "length": expr_to_json(length),
            "visibility": format!("{:?}", visibility),
            "editability": format!("{:?}", editability),
            "value": expr_to_json(value)
        }),
        Decl::FnDecl { is_exported, name, params, return_type, body } => json!({
            "type": "FnDecl",
            "name": name,
            "is_exported": is_exported,
            "params": params.iter().map(|p| json!({
                "name": p.name,
                "type": p.type_node.as_str()
            })).collect::<Vec<_>>(),
            "return_type": return_type.as_str(),
            "body": stmts_to_json(body)
        }),
        Decl::CustomDecl { is_exported, name, public_block, private_block, static_block, handle_block, .. } => json!({
            "type": "CustomDecl",
            "name": name,
            "is_exported": is_exported,
            "public": public_block.as_ref().map(|d| d.iter().map(decl_to_json).collect::<Vec<_>>()),
            "private": private_block.as_ref().map(|d| d.iter().map(decl_to_json).collect::<Vec<_>>()),
            "static": static_block.as_ref().map(|d| d.iter().map(decl_to_json).collect::<Vec<_>>()),
            "handles": handle_block.as_ref().map(|d| d.iter().map(decl_to_json).collect::<Vec<_>>()),
        }),
        Decl::ClassDecl { is_exported, name, public_block, private_block, static_block, handle_block, .. } => json!({
            "type": "ClassDecl",
            "name": name,
            "is_exported": is_exported,
            "public": public_block.iter().map(decl_to_json).collect::<Vec<_>>(),
            "private": private_block.iter().map(decl_to_json).collect::<Vec<_>>(),
            "static": static_block.iter().map(decl_to_json).collect::<Vec<_>>(),
            "handles": handle_block.iter().map(decl_to_json).collect::<Vec<_>>(),
        }),
        Decl::StructDecl { is_exported, name, public_block, private_block, static_block, handle_block, .. } => json!({
            "type": "StructDecl",
            "name": name,
            "is_exported": is_exported,
            "public": public_block.iter().map(decl_to_json).collect::<Vec<_>>(),
            "private": private_block.iter().map(decl_to_json).collect::<Vec<_>>(),
            "static": static_block.iter().map(decl_to_json).collect::<Vec<_>>(),
            "handles": handle_block.iter().map(decl_to_json).collect::<Vec<_>>(),
        }),
        Decl::BlockDecl { is_exported, name, return_type, statements } => json!({
            "type": "BlockDecl",
            "name": name,
            "is_exported": is_exported,
            "return_type": return_type.as_ref().map(|t| t.as_str()),
            "body": stmts_to_json(statements)
        }),
        Decl::MicroDecl { is_exported, name, params, return_type, body, generics } => json!({
            "type": "MicroDecl",
            "name": name,
            "is_exported": is_exported,
            "generics": generics,
            "params": params.iter().map(|p| json!({ "name": p.name, "type": p.type_node.as_str() })).collect::<Vec<_>>(),
            "return_type": return_type.as_ref().map(|t| t.as_str()),
            "body": stmts_to_json(body)
        }),
        Decl::LabelDecl { name, body } => json!({
            "type": "LabelDecl",
            "name": name,
            "body": stmts_to_json(body)
        }),
        Decl::Import { module_path, imports } => json!({
            "type": "Import",
            "module": module_path.join("/"),
            "imports": imports
        }),
        _ => json!({
            "type": "OtherDecl"
        })
    }
}

pub fn expr_to_json(expr: &Expr) -> Value {
    match expr {
        Expr::LiteralInt(v) => json!({ "type": "LiteralInt", "value": v }),
        Expr::LiteralFloat(v) => json!({ "type": "LiteralFloat", "value": v }),
        Expr::LiteralString(v) => json!({ "type": "LiteralString", "value": v }),
        Expr::LiteralChar(v) => json!({ "type": "LiteralChar", "value": v }),
        Expr::LiteralBool(v) => json!({ "type": "LiteralBool", "value": v }),
        Expr::Identifier(v) => json!({ "type": "Identifier", "name": v }),
        Expr::BinaryOp { left, operator, right } => json!({
            "type": "BinaryOp",
            "operator": operator,
            "left": expr_to_json(left),
            "right": expr_to_json(right)
        }),
        Expr::UnaryOp { operator, operand } => json!({
            "type": "UnaryOp",
            "operator": operator,
            "operand": expr_to_json(operand)
        }),
        Expr::Call { callee, args } => json!({
            "type": "Call",
            "callee": expr_to_json(callee),
            "args": args.iter().map(expr_to_json).collect::<Vec<_>>()
        }),
        Expr::IndexAccess { object, indices } => json!({
            "type": "IndexAccess",
            "object": expr_to_json(object),
            "indices": indices.iter().map(expr_to_json).collect::<Vec<_>>()
        }),
        Expr::PropertyAccess { object, property } => json!({
            "type": "PropertyAccess",
            "object": expr_to_json(object),
            "property": property
        }),
        Expr::ArrayLiteral(elems) => json!({
            "type": "ArrayLiteral",
            "elements": elems.iter().map(expr_to_json).collect::<Vec<_>>()
        }),
        Expr::This => json!({ "type": "This" }),
        Expr::Super => json!({ "type": "Super" }),
        _ => json!({ "type": "OtherExpr" })
    }
}
