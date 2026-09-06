use crate::frontend::parser::ast::*;
use crate::middle_end::semantic::environment::{
    BlueprintData, Environment, FnSignature, SymbolInfo, SymbolKind,
};
use crate::middle_end::semantic::handle_resolver::{
    build_blueprint_from_metadata, extract_all_type_names, extract_blueprint_name_from_type,
    is_complex_type, op_to_handle, resolve_handle_for_op, HandleLookupResult,
};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

pub mod decl;
pub mod expr;
pub mod overload;
pub mod stmt;
pub mod type_system;

pub use overload::{detect_execution_mode, has_compile_directive, resolve_call_generics};
pub(crate) use type_system::{extract_iter_payload_type, extract_type_args_from_str, strip_wrapper};

// ============================================================
// SemanticAnalyzer - main orchestrator
// ============================================================
pub struct SemanticAnalyzer {
    pub current_env: Rc<RefCell<Environment>>,
    pub in_class: bool,
    pub in_struct: bool,
    pub in_custom_scope: bool,
    pub in_statement_scope: bool,
    pub active_flags: Vec<String>,
    pub active_return_type: Option<BaseType>,
    pub current_type_name: Option<String>,
    pub global_metadata: HashMap<String, TypeMetadata>,
    pub dependency_graph: HashMap<String, HashSet<String>>,
    pub current_context: Option<String>,
    pub current_file: String,
    pub source_code: Option<String>,
    pub fn_overloads: HashMap<String, Vec<FnSignature>>,
    pub heap_allocated_vars: HashSet<String>,
    pub deleted_vars: HashSet<String>,
    pub current_machine: Option<String>,
    pub machine_labels: HashMap<String, HashMap<String, HashMap<String, BaseType>>>,
    pub in_generic_template: bool,
}


impl SemanticAnalyzer {
    pub fn new(global_metadata: HashMap<String, TypeMetadata>) -> Self {
        let mut analyzer = SemanticAnalyzer {
            current_env: Environment::new(),
            in_class: false,
            in_struct: false,
            in_custom_scope: false,
            in_statement_scope: false,
            current_type_name: None,
            active_flags: vec!["+has_exit".to_string()],
            active_return_type: None,
            global_metadata,
            dependency_graph: HashMap::new(),
            current_context: None,
            current_file: String::new(),
            source_code: None,
            fn_overloads: HashMap::new(),
            heap_allocated_vars: HashSet::new(),
            deleted_vars: HashSet::new(),
            current_machine: None,
            machine_labels: HashMap::new(),
            in_generic_template: false,
        };
        analyzer.import_metadata();
        analyzer
    }

    // ----------------------------------------------------------
    // Import global TypeMetadata -> BlueprintData into env
    // ----------------------------------------------------------
    pub(crate) fn import_metadata(&mut self) {
        let names: Vec<String> = self.global_metadata.keys().cloned().collect();
        for name in names {
            let meta = self.global_metadata[&name].clone();
            let bp = build_blueprint_from_metadata(&meta);
            self.current_env.borrow_mut().define_blueprint(name, bp);
        }
    }


    pub(crate) fn enter_scope(&mut self) {
        let new_env = Environment::with_parent(Rc::clone(&self.current_env));
        self.current_env = new_env;
    }

    pub fn emit_warning(
        &self,
        message: &str,
        line: Option<usize>,
        column: Option<usize>,
        hint: Option<&str>,
    ) {
        eprintln!("\x1b[33;1mwarning:\x1b[0m {}", message);
        if let (Some(l), Some(c)) = (line, column) {
            let file_display = if self.current_file.is_empty() {
                "<input>"
            } else {
                &self.current_file
            };
            eprintln!("  --> {}:{}:{}", file_display, l, c);
            if let Some(ref src) = self.source_code {
                if let Some(line_str) = src.lines().nth(l.saturating_sub(1)) {
                    let pad = " ".repeat(format!("{}", l).len());
                    eprintln!("   {} |", pad);
                    eprintln!("{} | {}", l, line_str);
                    let caret_pad = " ".repeat(c.saturating_sub(1));
                    eprintln!("   {} | {}^{}", pad, caret_pad, "~".repeat(4));
                }
            }
        }
        if let Some(h) = hint {
            eprintln!("  = \x1b[36;1mhelp:\x1b[0m {}", h);
        }
    }

    pub(crate) fn leave_scope(&mut self) {
        if !self.in_class && !self.in_struct && !self.in_custom_scope {
            for (name, info) in &self.current_env.borrow().symbols {
                if !info.is_used
                    && !name.starts_with('_')
                    && name != "this"
                    && name != "data"
                    && name != "__this__"
                {
                    if let SymbolKind::Variable { .. } = &info.kind {
                        let kind_str = if info.is_param {
                            "parameter"
                        } else {
                            "variable"
                        };
                        let hint = format!(
                            "if this is intentional, prefix with an underscore: '_{}'",
                            name
                        );
                        self.emit_warning(
                            &format!(
                                "{} '{}' is declared but never used in scope",
                                kind_str, name
                            ),
                            None,
                            None,
                            Some(&hint),
                        );
                    }
                }
            }
        }
        let parent = self
            .current_env
            .borrow()
            .parent
            .clone()
            .expect("Cannot leave global scope");
        self.current_env = parent;
    }


    pub fn analyze(&mut self, ast: &Vec<Stmt>) -> Result<(), String> {
        // Pass 0: Pre-register @compile members, macros, and impl metadata so declaration order in file does not matter
        for stmt in ast {
            if let Stmt::Declaration(decl) = stmt {
                self.pre_register_decl(decl)?;
            }
        }

        for stmt in ast {
            self.visit_statement(stmt)?;
        }
        Ok(())
    }

    pub fn record_dependency(&mut self, dep: String) {
        let ctx = self
            .current_context
            .clone()
            .unwrap_or_else(|| "main".to_string());
        self.dependency_graph
            .entry(ctx)
            .or_insert_with(HashSet::new)
            .insert(dep);
    }

    pub fn get_reachable_symbols(&self) -> HashSet<String> {
        let mut reachable = HashSet::new();
        let mut queue = Vec::new();

        // Always start from main or top-level context
        queue.push("main".to_string());
        reachable.insert("main".to_string());

        for (name, info) in &self.current_env.borrow().symbols {
            if info.is_used {
                if reachable.insert(name.clone()) {
                    queue.push(name.clone());
                }
            }
        }

        while let Some(curr) = queue.pop() {
            if let Some(deps) = self.dependency_graph.get(&curr) {
                for dep in deps {
                    if reachable.insert(dep.clone()) {
                        queue.push(dep.clone());
                    }
                }
            }
        }

        reachable
    }

}
