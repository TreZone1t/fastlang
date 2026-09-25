use super::*;

pub fn resolve_call_generics(
    fn_generics: &[BaseType],
    params: &[Param],
    call_generics: &[BaseType],
    arg_types: &[String],
) -> HashMap<String, BaseType> {
    let mut resolved = HashMap::new();

    // 1. Identify which fn_generics are present in the params (inferred)
    // and which ones are not present in any param (must come from explicit call_generics).
    let mut param_generic_names = HashSet::new();
    for p in params {
        for name in p.type_node.extract_generic_params() {
            param_generic_names.insert(name);
        }
    }

    let mut non_inferred_generics = Vec::new();
    for g in fn_generics {
        let g_name = match g {
            BaseType::GenericParam(name) => name.clone(),
            BaseType::New(name) => name.clone(),
            _ => g.as_str(),
        };
        if !param_generic_names.contains(&g_name) {
            non_inferred_generics.push(g_name);
        }
    }

    // 2. If call_generics length matches total fn_generics, map 1-to-1:
    if !call_generics.is_empty() && call_generics.len() == fn_generics.len() {
        for (i, g) in fn_generics.iter().enumerate() {
            let g_name = match g {
                BaseType::GenericParam(name) => name.clone(),
                BaseType::New(name) => name.clone(),
                _ => g.as_str(),
            };
            resolved.insert(g_name, call_generics[i].clone());
        }
    } else {
        // Otherwise, map call_generics to non_inferred_generics in order:
        for (i, target_g) in non_inferred_generics.iter().enumerate() {
            if let Some(concrete) = call_generics.get(i) {
                resolved.insert(target_g.clone(), concrete.clone());
            }
        }
    }

    // 3. Infer remaining generics from arg_types:
    for (i, p) in params.iter().enumerate() {
        if let Some(arg_ty_str) = arg_types.get(i) {
            let arg_ty = BaseType::from_str(arg_ty_str);
            p.type_node.infer_generics(&arg_ty, &mut resolved);
        }
    }

    resolved
}

fn has_compile_directive_expr(expr: &Expr) -> bool {
    match expr {
        Expr::MacroCall { callee, .. } => {
            if let Expr::NamespaceAccess { namespace, .. } = &**callee {
                if namespace == "@compile" {
                    println!("Found compile directive in MacroCall!");
                    return true;
                }
            }
            false
        }
        Expr::Call { callee, args, .. } => {
            has_compile_directive_expr(callee) || args.iter().any(has_compile_directive_expr)
        }
        Expr::BinaryOp { left, right, .. } => {
            has_compile_directive_expr(left) || has_compile_directive_expr(right)
        }
        Expr::PropertyAccess { object, .. } => has_compile_directive_expr(object),
        Expr::ArrayLiteral(elements) => elements.iter().any(has_compile_directive_expr),
        Expr::ObjectLiteral(stmts) => has_compile_directive(stmts),
        Expr::Instantiate { args, .. } => args.iter().any(has_compile_directive_expr),
        Expr::NamespaceAccess {
            namespace,
            property,
            ..
        } => {
            if namespace == "@compile" {
                return true;
            }
            has_compile_directive_expr(property)
        }
        Expr::IfExpr { condition, then_branch, else_branch } => {
            has_compile_directive_expr(condition) || has_compile_directive_expr(then_branch) || has_compile_directive_expr(else_branch)
        }
        Expr::BlockExpr { statements, final_expr } => {
            has_compile_directive(statements) || final_expr.as_ref().map(|e| has_compile_directive_expr(e)).unwrap_or(false)
        }
        _ => false,
    }
}

pub fn has_compile_directive(stmts: &[Stmt]) -> bool {
    for stmt in stmts {
        match stmt {
            Stmt::ExpressionStmt(expr) | Stmt::CallStmt(expr) => {
                if has_compile_directive_expr(expr) {
                    return true;
                }
            }
            Stmt::Declaration(Decl::VarDecl { value, .. }) => {
                if has_compile_directive_expr(value) {
                    return true;
                }
            }
            Stmt::Block(inner) | Stmt::ThisBlock(inner) => {
                if has_compile_directive(inner) {
                    return true;
                }
            }
            Stmt::IfStmt {
                condition,
                then_block,
                else_block,
            } => {
                if has_compile_directive_expr(condition) {
                    return true;
                }
                if has_compile_directive(then_block) {
                    return true;
                }
                if let Some(e) = else_block {
                    if has_compile_directive(e) {
                        return true;
                    }
                }
            }
            Stmt::WhileStmt { condition, body } => {
                if has_compile_directive_expr(condition) {
                    return true;
                }
                match body {
                    crate::frontend::parser::ast::EitherBlock::Inline(b) => {
                        if has_compile_directive(b) {
                            return true;
                        }
                    }
                    crate::frontend::parser::ast::EitherBlock::External(e) => {
                        if has_compile_directive_expr(e) {
                            return true;
                        }
                    }
                }
            }
            Stmt::LoopStmt { count, body } => {
                if let Some(c) = count {
                    if has_compile_directive_expr(c) {
                        return true;
                    }
                }
                match body {
                    crate::frontend::parser::ast::EitherBlock::Inline(b) => {
                        if has_compile_directive(b) {
                            return true;
                        }
                    }
                    crate::frontend::parser::ast::EitherBlock::External(e) => {
                        if has_compile_directive_expr(e) {
                            return true;
                        }
                    }
                }
            }
            Stmt::ForStmt {
                init,
                condition,
                increment,
                body,
            } => {
                if let Some(i) = init {
                    if has_compile_directive(&[*(i.clone())]) {
                        return true;
                    }
                }
                if let Some(c) = condition {
                    if has_compile_directive_expr(c) {
                        return true;
                    }
                }
                if let Some(inc) = increment {
                    if has_compile_directive(&[*(inc.clone())]) {
                        return true;
                    }
                }
                match body {
                    crate::frontend::parser::ast::EitherBlock::Inline(b) => {
                        if has_compile_directive(b) {
                            return true;
                        }
                    }
                    crate::frontend::parser::ast::EitherBlock::External(e) => {
                        if has_compile_directive_expr(e) {
                            return true;
                        }
                    }
                }
            }
            Stmt::ReturnStmt(Some(e)) => {
                if has_compile_directive_expr(e) {
                    return true;
                }
            }
            Stmt::ThrowStmt(e) => {
                if has_compile_directive_expr(e) {
                    return true;
                }
            }
            Stmt::ReassignStmt { target, value, .. } => {
                if has_compile_directive_expr(target) || has_compile_directive_expr(value) {
                    return true;
                }
            }
            Stmt::Declaration(Decl::CompileDecl { .. }) => {
                return true;
            }
            Stmt::CompileValidation { .. } => {
                return true;
            }
            _ => {}
        }
    }
    false
}

pub fn detect_execution_mode(body: &[Stmt]) -> ExecutionMode {
    if body.is_empty() {
        return ExecutionMode::Runtime;
    }

    let mut has_compile = false;
    let mut has_runtime = false;

    fn check_stmt(s: &Stmt, has_c: &mut bool, has_r: &mut bool) {
        match s {
            Stmt::CompileValidation { .. } => {
                *has_c = true;
            }
            Stmt::CallStmt(Expr::Call { callee, .. }) | Stmt::ExpressionStmt(Expr::Call { callee, .. }) => {
                if let Expr::NamespaceAccess { namespace, .. } = &**callee {
                    if namespace == "@compile" {
                        *has_c = true;
                        return;
                    }
                }
                *has_r = true;
            }
            Stmt::ReturnStmt(Some(Expr::Call { callee, .. })) => {
                if let Expr::NamespaceAccess { namespace, .. } = &**callee {
                    if namespace == "@compile" {
                        *has_c = true;
                        return;
                    }
                }
                *has_r = true;
            }
            Stmt::Declaration(Decl::CompileDecl { .. }) => {
                *has_c = true;
            }
            Stmt::Declaration(Decl::VarDecl { value, .. }) => {
                if has_compile_directive_expr(value) {
                    *has_c = true;
                } else {
                    *has_r = true;
                }
            }
            Stmt::Block(inner) | Stmt::ThisBlock(inner) => {
                for stmt in inner {
                    check_stmt(stmt, has_c, has_r);
                }
            }
            Stmt::IfStmt { condition, then_block, else_block } => {
                if has_compile_directive_expr(condition) {
                    *has_c = true;
                }
                for stmt in then_block {
                    check_stmt(stmt, has_c, has_r);
                }
                if let Some(eb) = else_block {
                    for stmt in eb {
                        check_stmt(stmt, has_c, has_r);
                    }
                }
            }
            Stmt::Declaration(_) => {
                *has_r = true;
            }
            _ => {
                *has_r = true;
            }
        }
    }

    for s in body {
        check_stmt(s, &mut has_compile, &mut has_runtime);
    }

    if has_compile && !has_runtime {
        ExecutionMode::FullyCompilable
    } else if has_compile && has_runtime {
        ExecutionMode::CompileRuntimeMix
    } else {
        ExecutionMode::Runtime
    }
}

