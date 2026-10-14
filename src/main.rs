use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::rc::Rc;

use crate::backend::*;
use crate::frontend::parser::ast::*;
use crate::middle_end::semantic::analyzer::SemanticAnalyzer;
use crate::middle_end::semantic::environment::Environment;
pub mod backend;
pub mod frontend;
pub mod loader;
pub mod middle_end;
pub mod project;

pub fn report_visual_error(source: &str, line: usize, column: usize, err_msg: &str) {
    let lines: Vec<&str> = source.lines().collect();

    let context_lines = 2;
    let start_line = if line > context_lines {
        line - context_lines
    } else {
        1
    };
    let end_line = std::cmp::min(line + context_lines, lines.len());

    eprintln!("\n\x1b[31;1merror:\x1b[0m {}", err_msg);
    eprintln!("  \x1b[34;1m-->\x1b[0m line {}:{}", line, column);
    eprintln!("   \x1b[34;1m|\x1b[0m");

    for i in start_line..=end_line {
        let i_minus_1 = i - 1;
        if i_minus_1 < lines.len() {
            let l_text = lines[i_minus_1];
            if i == line {
                eprintln!("{:3}\x1b[34;1m |\x1b[0m {}", i, l_text);
                let padding = " ".repeat(if column > 0 { column - 1 } else { 0 });
                eprintln!("   \x1b[34;1m|\x1b[0m {}\x1b[31;1m^\x1b[0m", padding);
            } else {
                eprintln!("{:3}\x1b[34;1m |\x1b[0m {}", i, l_text);
            }
        }
    }
    eprintln!("   \x1b[34;1m|\x1b[0m\n");
}

pub fn extract_line_col_from_msg(msg: &str) -> (Option<usize>, Option<usize>) {
    if let Some(idx) = msg.find("at line ") {
        let sub = &msg[idx + 8..];
        let mut parts = sub.split(|c: char| !c.is_ascii_digit()).filter(|s| !s.is_empty());
        let line = parts.next().and_then(|s| s.parse::<usize>().ok());
        let col = parts.next().and_then(|s| s.parse::<usize>().ok());
        (line, col)
    } else {
        (None, None)
    }
}

fn module_import_deps(ast: &[Stmt]) -> Vec<(String, Option<Vec<String>>)> {
    ast.iter()
        .filter_map(|stmt| {
            if let Stmt::Declaration(Decl::Import {
                module_path,
                imports,
                abi,
                ..
            }) = stmt
            {
                if abi.is_some()
                    || module_path
                        .first()
                        .map_or(false, |p| p.ends_with(".h") || p.ends_with(".hpp"))
                {
                    None
                } else {
                    Some((module_path.join("/"), imports.clone()))
                }
            } else {
                None
            }
        })
        .collect()
}

fn inject_module_exports(
    analyzer: &mut SemanticAnalyzer,
    dep_name: &str,
    imports: &Option<Vec<String>>,
    envs: &HashMap<String, Rc<RefCell<Environment>>>,
    all_fn_overloads: &HashMap<
        String,
        HashMap<String, Vec<crate::middle_end::semantic::environment::FnSignature>>,
    >,
    debug: bool,
) -> Result<(), String> {
    let env = envs
        .get(dep_name)
        .ok_or_else(|| format!("Error: Module {} not found.", dep_name))?;
    let full_path = dep_name.replace('/', "::");
    let base_name = dep_name.split('/').last().unwrap_or(dep_name);
    let mut prefixes = vec![base_name.to_string()];
    if full_path != base_name {
        prefixes.push(full_path);
    }

    let symbols = env.borrow().symbols.clone();
    for (sym_name, info) in symbols {
        if info.visibility == Visibility::Public {
            let should_inject = match imports {
                Some(selected) => selected.contains(&sym_name),
                None => true,
            };
            if should_inject {
                if debug {
                    println!("Module {} injected symbol: {}", dep_name, sym_name);
                }
                analyzer
                    .current_env
                    .borrow_mut()
                    .define(sym_name.clone(), info.clone())
                    .ok();
                if !sym_name.contains("::") {
                    for prefix in &prefixes {
                        let qual = format!("{}::{}", prefix, sym_name);
                        analyzer
                            .current_env
                            .borrow_mut()
                            .define(qual, info.clone())
                            .ok();
                    }
                }
            }
        }
    }
    if let Some(mod_overloads) = all_fn_overloads.get(dep_name) {
        for (fn_name, sigs) in mod_overloads {
            let should_inject = match imports {
                Some(selected) => selected.contains(fn_name),
                None => true,
            };
            if should_inject {
                analyzer
                    .fn_overloads
                    .entry(fn_name.clone())
                    .or_default()
                    .extend(sigs.clone());
                if !fn_name.contains("::") {
                    for prefix in &prefixes {
                        let qual = format!("{}::{}", prefix, fn_name);
                        analyzer
                            .fn_overloads
                            .entry(qual)
                            .or_default()
                            .extend(sigs.clone());
                    }
                }
            }
        }
    }
    let blueprints = env.borrow().blueprints.clone();
    for (bp_name, bp_data) in blueprints {
        let should_inject = match imports {
            Some(selected) => selected.contains(&bp_name),
            None => true,
        };
        if should_inject {
            if debug {
                println!("Module {} injected blueprint: {}", dep_name, bp_name);
            }
            let mut cur_opt = analyzer.current_env.borrow().lookup_blueprint(&bp_name);
            if let Some(ref mut cur_bp) = cur_opt {
                if cur_bp.fields.is_empty() && !bp_data.fields.is_empty() {
                    cur_bp.fields = bp_data.fields.clone();
                } else {
                    for (k, v) in &bp_data.fields {
                        cur_bp.fields.entry(k.clone()).or_insert(v.clone());
                    }
                }
                if cur_bp.methods.is_empty() && !bp_data.methods.is_empty() {
                    cur_bp.methods = bp_data.methods.clone();
                } else {
                    for (k, v) in &bp_data.methods {
                        cur_bp.methods.entry(k.clone()).or_insert(v.clone());
                    }
                }
                for h in &bp_data.handles {
                    cur_bp.handles.insert(h.clone());
                }
                for (k, v) in &bp_data.handle_signatures {
                    cur_bp.handle_signatures.entry(k.clone()).or_insert(v.clone());
                }
                for f in &bp_data.private_fields {
                    cur_bp.private_fields.insert(f.clone());
                }
                for m in &bp_data.private_methods {
                    cur_bp.private_methods.insert(m.clone());
                }
                if !bp_data.generics.is_empty() {
                    cur_bp.generics = bp_data.generics.clone();
                }
                analyzer
                    .current_env
                    .borrow_mut()
                    .define_blueprint(bp_name.clone(), cur_bp.clone());
                if !bp_name.contains("::") {
                    for prefix in &prefixes {
                        let qual = format!("{}::{}", prefix, bp_name);
                        analyzer
                            .current_env
                            .borrow_mut()
                            .define_blueprint(qual, cur_bp.clone());
                    }
                }
            } else {
                analyzer
                    .current_env
                    .borrow_mut()
                    .define_blueprint(bp_name.clone(), bp_data.clone());
                if !bp_name.contains("::") {
                    for prefix in &prefixes {
                        let qual = format!("{}::{}", prefix, bp_name);
                        analyzer
                            .current_env
                            .borrow_mut()
                            .define_blueprint(qual, bp_data.clone());
                    }
                }
            }
        }
    }
    let namespaces = env.borrow().namespaces.clone();
    for (ns_name, ns_env) in namespaces {
        analyzer
            .current_env
            .borrow_mut()
            .define_namespace(ns_name.clone(), Rc::clone(&ns_env));
        if !ns_name.contains("::") {
            for prefix in &prefixes {
                if prefix != &ns_name {
                    let qual = format!("{}::{}", prefix, ns_name);
                    analyzer
                        .current_env
                        .borrow_mut()
                        .define_namespace(qual, Rc::clone(&ns_env));
                }
            }
        }
        let ns_symbols = ns_env.borrow().symbols.clone();
        for (sym_name, info) in ns_symbols {
            if info.visibility == Visibility::Public {
                let qual_sym = format!("{}::{}", ns_name, sym_name);
                analyzer
                    .current_env
                    .borrow_mut()
                    .define_or_update(qual_sym.clone(), info.clone());
                for prefix in &prefixes {
                    let full_qual = format!("{}::{}", prefix, sym_name);
                    analyzer
                        .current_env
                        .borrow_mut()
                        .define_or_update(full_qual, info.clone());
                }
            }
        }
        let ns_blueprints = ns_env.borrow().blueprints.clone();
        for (bp_name, bp_data) in ns_blueprints {
            let qual_bp = format!("{}::{}", ns_name, bp_name);
            analyzer
                .current_env
                .borrow_mut()
                .define_blueprint(qual_bp, bp_data.clone());
            for prefix in &prefixes {
                let full_qual = format!("{}::{}", prefix, bp_name);
                analyzer
                    .current_env
                    .borrow_mut()
                    .define_blueprint(full_qual, bp_data.clone());
            }
        }
    }
    Ok(())
}

fn prune_ast_for_reachability(
    ast: &[Stmt],
    reachable: &HashSet<String>,
    fn_overloads: &HashMap<String, Vec<crate::middle_end::semantic::environment::FnSignature>>,
    is_main_file: bool,
) -> Vec<Stmt> {
    let mut pruned = Vec::new();
    for stmt in ast {
        match stmt {
            Stmt::Declaration(decl) => {
                if let Some(pruned_decl) =
                    prune_decl_for_reachability(decl, reachable, fn_overloads, is_main_file)
                {
                    pruned.push(Stmt::Declaration(pruned_decl));
                }
            }
            other => {
                pruned.push(other.clone());
            }
        }
    }
    pruned
}

fn prune_decl_for_reachability(
    decl: &Decl,
    reachable: &HashSet<String>,
    fn_overloads: &HashMap<String, Vec<crate::middle_end::semantic::environment::FnSignature>>,
    is_main_file: bool,
) -> Option<Decl> {
    match decl {
        Decl::FnDecl { name, params, .. } => {
            if name == "main" {
                return Some(decl.clone());
            }
            let sig_key = crate::middle_end::semantic::analyzer::decl::make_sig_key(name, params);
            let has_multiple_overloads = fn_overloads.get(name).map_or(0, |sigs| {
                let mut distinct = HashSet::new();
                for s in sigs {
                    distinct.insert(crate::middle_end::semantic::analyzer::decl::make_sig_key(
                        &s.name, &s.params,
                    ));
                }
                distinct.len()
            }) > 1;

            let is_kept = if has_multiple_overloads {
                reachable.contains(&sig_key)
            } else {
                reachable.contains(name)
                    || (name.contains("_br") && {
                        let base = name.split("_br").next().unwrap();
                        reachable.contains(base)
                    })
            };

            if is_kept {
                Some(decl.clone())
            } else {
                None
            }
        }

        Decl::ClassDecl {
            visibility,
            name,
            extends,
            handles,
            public_block,
            private_block,
            static_block,
            generics,
            handle_block,
            constructor,
        } => {
            if !reachable.contains(name) {
                return None;
            }

            // For module files (stdlib), keep ALL methods of reachable types.
            // We can't guarantee tracking all internal method-to-method calls,
            // so we only prune methods for user-defined types in the main file.
            if !is_main_file {
                return Some(decl.clone());
            }

            let filter_methods = |block: &[Decl]| -> Vec<Decl> {
                block
                    .iter()
                    .filter(|d| match d {
                        Decl::FnDecl { name: m_name, .. } => {
                            let full_m = format!("{}::{}", name, m_name);
                            reachable.contains(&full_m)
                        }
                        _ => true,
                    })
                    .cloned()
                    .collect()
            };

            Some(Decl::ClassDecl {
                visibility: visibility.clone(),
                name: name.clone(),
                extends: extends.clone(),
                handles: handles.clone(),
                public_block: filter_methods(public_block),
                private_block: filter_methods(private_block),
                static_block: filter_methods(static_block),
                generics: generics.clone(),
                handle_block: handle_block.clone(),
                constructor: constructor.clone(),
            })
        }

        Decl::StructDecl {
            visibility,
            name,
            handles,
            public_block,
            private_block,
            handle_block,
            static_block,
            constructor,
        } => {
            if !reachable.contains(name) {
                return None;
            }

            // For module files, keep all methods of reachable types.
            if !is_main_file {
                return Some(decl.clone());
            }

            let filter_methods = |block: &[Decl]| -> Vec<Decl> {
                block
                    .iter()
                    .filter(|d| match d {
                        Decl::FnDecl { name: m_name, .. } => {
                            let full_m = format!("{}::{}", name, m_name);
                            reachable.contains(&full_m)
                        }
                        _ => true,
                    })
                    .cloned()
                    .collect()
            };

            Some(Decl::StructDecl {
                visibility: visibility.clone(),
                name: name.clone(),
                handles: handles.clone(),
                public_block: filter_methods(public_block),
                private_block: filter_methods(private_block),
                handle_block: handle_block.clone(),
                static_block: filter_methods(static_block),
                constructor: constructor.clone(),
            })
        }

        Decl::ImplDecl {
            target,
            target_generics,
            is_handle_impl,
            methods,
            handle_block,
        } => {
            // If the target type is reachable, keep its impl methods
            if reachable.contains(target) {
                return Some(decl.clone());
            }

            let filter_methods: Vec<Decl> = methods
                .iter()
                .filter(|d| match d {
                    Decl::FnDecl { name: m_name, .. } => {
                        let full_m = format!("{}::{}", target, m_name);
                        reachable.contains(&full_m) || reachable.contains(m_name)
                    }
                    _ => true,
                })
                .cloned()
                .collect();

            if filter_methods.is_empty() && handle_block.is_empty() {
                None
            } else {
                Some(Decl::ImplDecl {
                    target: target.clone(),
                    target_generics: target_generics.clone(),
                    is_handle_impl: *is_handle_impl,
                    methods: filter_methods,
                    handle_block: handle_block.clone(),
                })
            }
        }

        Decl::EnumDecl {
            visibility,
            name,
            generics,
            handles,
            handle_block,
            variants,
        } => {
            if !reachable.contains(name) {
                return None;
            }

            // For module files, keep all handles of reachable enum types.
            if !is_main_file {
                return Some(decl.clone());
            }

            Some(Decl::EnumDecl {
                visibility: visibility.clone(),
                name: name.clone(),
                generics: generics.clone(),
                handles: handles.clone(),
                handle_block: handle_block.clone(),
                variants: variants.clone(),
            })
        }

        Decl::BlueprintDecl { name, .. } => {
            if !reachable.contains(name) {
                return None;
            }
            Some(decl.clone())
        }

        Decl::MachineDecl { name, .. } => {
            if !reachable.contains(name) {
                return None;
            }
            Some(decl.clone())
        }

        Decl::ObjectDecl { name, .. } => {
            if !reachable.contains(name) {
                return None;
            }
            Some(decl.clone())
        }

        Decl::ExternFnDecl { name, alias, .. } => {
            let is_kept =
                reachable.contains(name) || alias.as_ref().map_or(false, |a| reachable.contains(a));
            if is_kept {
                Some(decl.clone())
            } else {
                None
            }
        }

        Decl::ExternBlockDecl { abi, decls } => {
            let filtered_decls: Vec<Decl> = decls
                .iter()
                .filter_map(|d| {
                    prune_decl_for_reachability(d, reachable, fn_overloads, is_main_file)
                })
                .collect();
            if filtered_decls.is_empty() {
                None
            } else {
                Some(Decl::ExternBlockDecl {
                    abi: abi.clone(),
                    decls: filtered_decls,
                })
            }
        }

        Decl::VarDecl { name, .. } => {
            if is_main_file {
                Some(decl.clone())
            } else {
                if reachable.contains(name) {
                    Some(decl.clone())
                } else {
                    None
                }
            }
        }

        _ => Some(decl.clone()),
    }
}

fn print_help() {
    println!(
        r#"FastLang Compiler (fast / fast_lang) v0.1.5-stable
High-performance compiled language with fine-grained memory management and explicit scopes.

USAGE:
    fast [SUBCOMMAND] [OPTIONS]
    fast [OPTIONS] <SOURCE_FILE> [-- <ARGS>...]

SUBCOMMANDS:
    new <NAME> [--type bin|lib] [-y]   Create a new FastLang project package
    init [--type bin|lib] [-y]         Initialize a FastLang package in current directory
    run [SCRIPT|FILE] [ARGS...]        Run project entry point, named script, or specific source file
    build [FILE] [OPTIONS]             Compile project or source file without executing
    test                               Discover and run all project tests in tests/
    clean [PATH]                       Delete build/ directory and cached build artifacts
    lib <list|add|remove> [PATH]       Manage compiler-wide global library search paths
    registry <list|add|import> [...]   Manage known package registry and dependency URLs
    pkg <add|install> [NAME]           Install or add dependencies to current project

ARGUMENTS:
    <SOURCE_FILE>                  Path to the main entry source file (.fast or .fs)
    -- <ARGS>...                   Pass arguments directly to the executed program

OPTIONS:
    -h, --help, -help              Print this help information and exit
    -d, --debug                    Enable verbose debug and compilation trace output
    --debug-error, --debug-errors  Accumulate and report all semantic errors without generating code
    -o, --output <PATH>            Specify output binary or build directory path
    -I, --include <DIR>            Add module search directory
    -b, --backend <BACKEND>        Set code generator backend: 'cpp' (C++20, default) or 'cranelift'
    --ast, --emit-ast [FILE]       Dump parsed AST as formatted JSON (to stdout or FILE)
    --ast-out, --ast-file, --json-ast <FILE>
                                   Export parsed AST as formatted JSON directly to FILE
    --emit-cpp, --cpp-only         Generate and emit C++ source file without compiling binary
    --emit-ir, --print-ir          Print intermediate representation (IR) output
    --ir-out, --ir-file <FILE>     Write IR output directly to specified file
    --aot                          Enable Ahead-Of-Time (AOT) compilation via Cranelift backend
    --target <TARGET>              Specify target architecture/platform
    --clean                        Remove temporary C++ and library files after successful build

PROJECT MANIFEST (Fast.toml):
    [package]
    name = "my_app"
    version = "0.1.0"
    type = "bin"                   # "bin" (executable) or "module" / "lib" (library)
    main = "src/main.fast"         # Entry source file
    backend = "cpp"                # Preferred compiler backend ("cpp" or "cranelift")

    [scripts]
    dev = "fast run"               # Custom task runner scripts

    [paths]
    packages = "packages"          # Local package dependencies folder
    libraries = []                 # Additional library search paths

EXAMPLES:
    fast new my_app
    fast new my_lib --type lib -y
    fast run
    fast run my_script
    fast run -- arg1 arg2
    fast build
    fast test
    fast lib add C:/libs/my_shared_lib
    fast lib list
    fast main.fast -o build/main.exe
    fast clean
"#
    );
}

fn run_project_tests(project_root: &std::path::Path) -> Result<(), ()> {
    let tests_dir = project_root.join("tests");
    if !tests_dir.exists() {
        println!("No tests directory found at '{}'.", tests_dir.display());
        return Ok(());
    }

    let mut entries: Vec<std::path::PathBuf> = match std::fs::read_dir(&tests_dir) {
        Ok(read_dir) => read_dir
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| {
                if let Some(ext) = p.extension().and_then(|s| s.to_str()) {
                    (ext == "fast" || ext == "fs") && p.is_file()
                } else {
                    false
                }
            })
            .collect(),
        Err(e) => {
            eprintln!("Failed to read tests directory: {}", e);
            return Err(());
        }
    };

    if entries.is_empty() {
        println!("No test files found in '{}'.", tests_dir.display());
        return Ok(());
    }

    entries.sort();

    println!("\n=== Running FastLang Project Tests ===");
    println!("Found {} test(s) in {}\n", entries.len(), tests_dir.display());

    let self_exe = std::env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from("fast_lang"));
    let mut passed = 0;
    let mut failed = 0;

    for test_path in entries {
        let file_name = test_path.file_name().unwrap_or_default().to_string_lossy();
        let stem = test_path.file_stem().unwrap_or_default().to_string_lossy();
        print!("--> Running test: {} ... ", file_name);
        let _ = std::io::Write::flush(&mut std::io::stdout());

        let build_dir = project_root.join("build");
        let _ = std::fs::create_dir_all(&build_dir);
        let test_bin = build_dir.join(format!("{}.exe", stem));

        let compile_output = std::process::Command::new(&self_exe)
            .arg(&test_path)
            .arg("-o")
            .arg(&test_bin)
            .output();

        match compile_output {
            Ok(out) if out.status.success() => {
                let run_output = std::process::Command::new(&test_bin).output();
                match run_output {
                    Ok(r_out) if r_out.status.success() => {
                        println!("PASSED");
                        passed += 1;
                    }
                    Ok(r_out) => {
                        println!("FAILED (runtime exit code: {:?})", r_out.status.code());
                        let stderr = String::from_utf8_lossy(&r_out.stderr);
                        if !stderr.trim().is_empty() {
                            eprintln!("    Stderr: {}", stderr.trim());
                        }
                        failed += 1;
                    }
                    Err(e) => {
                        println!("FAILED to execute binary: {}", e);
                        failed += 1;
                    }
                }
            }
            Ok(out) => {
                println!("FAILED to compile");
                let stderr = String::from_utf8_lossy(&out.stderr);
                if !stderr.trim().is_empty() {
                    eprintln!("    Compiler error: {}", stderr.trim());
                }
                failed += 1;
            }
            Err(e) => {
                println!("FAILED to run compiler: {}", e);
                failed += 1;
            }
        }
    }

    println!("\nTest Results: {} passed, {} failed\n", passed, failed);
    if failed > 0 {
        Err(())
    } else {
        Ok(())
    }
}


fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 1
        || args.contains(&"--help".to_string())
        || args.contains(&"-h".to_string())
        || args.contains(&"-help".to_string())
    {
        print_help();
        return;
    }

    // 1. Subcommand: clean
    if args.len() > 1 && args[1] == "clean" {
        let target_dir = if args.len() > 2 {
            std::path::PathBuf::from(&args[2])
        } else {
            std::path::PathBuf::from(".")
        };
        let build_dir = if target_dir.file_name().and_then(|s| s.to_str()) == Some("build") {
            target_dir
        } else {
            target_dir.join("build")
        };
        if build_dir.exists() {
            if let Err(e) = std::fs::remove_dir_all(&build_dir) {
                eprintln!("Failed to clean {}: {}", build_dir.display(), e);
                std::process::exit(1);
            }
            println!("Cleaned build directory: {}", build_dir.display());
        } else {
            println!(
                "Nothing to clean. ({}) does not exist.",
                build_dir.display()
            );
        }
        return;
    }

    // 2. Subcommand: new <name> [--type bin|lib] [-y]
    if args.len() > 1 && args[1] == "new" {
        let mut proj_name: Option<String> = None;
        let mut p_type: Option<crate::project::ProjectType> = None;
        let mut non_interactive = false;

        let mut idx = 2;
        while idx < args.len() {
            if (args[idx] == "--type" || args[idx] == "-t") && idx + 1 < args.len() {
                p_type = crate::project::ProjectType::from_str_opt(&args[idx + 1]);
                idx += 2;
            } else if args[idx] == "-y" || args[idx] == "--yes" {
                non_interactive = true;
                idx += 1;
            } else if !args[idx].starts_with('-') && proj_name.is_none() {
                proj_name = Some(args[idx].clone());
                idx += 1;
            } else {
                idx += 1;
            }
        }

        let name = match proj_name {
            Some(n) => n,
            None => {
                if non_interactive {
                    eprintln!("Error: Project name is required. Usage: fast new <name> [--type bin|lib] [-y]");
                    std::process::exit(1);
                }
                print!("Enter project name: ");
                let _ = std::io::Write::flush(&mut std::io::stdout());
                let mut input = String::new();
                std::io::stdin().read_line(&mut input).unwrap_or_default();
                let trimmed = input.trim();
                if trimmed.is_empty() {
                    eprintln!("Project name cannot be empty.");
                    std::process::exit(1);
                }
                trimmed.to_string()
            }
        };

        let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        match crate::project::scaffold::create_new_project(&name, &cwd, p_type, non_interactive) {
            Ok(dir) => {
                println!("Created new FastLang project '{}' at {}", name, dir.display());
                println!("Get started with:");
                if name != "." {
                    println!("  cd {}", name);
                }
                println!("  fast run");
            }
            Err(e) => {
                eprintln!("Error creating project: {}", e);
                std::process::exit(1);
            }
        }
        return;
    }

    // 3. Subcommand: init [--type bin|lib] [-y]
    if args.len() > 1 && args[1] == "init" {
        let mut p_type: Option<crate::project::ProjectType> = None;
        let mut non_interactive = false;

        let mut idx = 2;
        while idx < args.len() {
            if (args[idx] == "--type" || args[idx] == "-t") && idx + 1 < args.len() {
                p_type = crate::project::ProjectType::from_str_opt(&args[idx + 1]);
                idx += 2;
            } else if args[idx] == "-y" || args[idx] == "--yes" {
                non_interactive = true;
                idx += 1;
            } else {
                idx += 1;
            }
        }

        let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        let proj_name = cwd.file_name().and_then(|n| n.to_str()).unwrap_or("my_project").to_string();
        let target_type = if let Some(t) = p_type {
            t
        } else if non_interactive {
            crate::project::ProjectType::Bin
        } else {
            crate::project::ProjectType::Bin
        };

        match crate::project::scaffold::init_project_in_dir(&cwd, proj_name, target_type) {
            Ok(_) => {
                println!("Initialized FastLang project in '{}'", cwd.display());
                println!("Run project with: fast run");
            }
            Err(e) => {
                eprintln!("Error initializing project: {}", e);
                std::process::exit(1);
            }
        }
        return;
    }

    // 4. Subcommand: lib <list | add <path> | remove <path>>
    if args.len() > 1 && args[1] == "lib" {
        let sub = args.get(2).map(|s| s.as_str()).unwrap_or("list");
        match sub {
            "list" => {
                let config = crate::project::GlobalConfig::load();
                if config.library_paths.is_empty() {
                    println!("No global library paths registered. Add one using 'fast lib add <path>'.");
                } else {
                    println!("Registered Global Library Paths:");
                    for (i, p) in config.library_paths.iter().enumerate() {
                        println!("  [{}] {}", i + 1, p);
                    }
                }
            }
            "add" => {
                let lib_path_arg = match args.get(3) {
                    Some(p) => p,
                    None => {
                        eprintln!("Usage: fast lib add <path>");
                        std::process::exit(1);
                    }
                };
                let lib_path = std::path::Path::new(lib_path_arg);
                if !lib_path.exists() {
                    eprintln!("Warning: path '{}' does not currently exist on disk.", lib_path.display());
                }
                let mut config = crate::project::GlobalConfig::load();
                match config.add_library_path(lib_path) {
                    Ok(_) => {
                        println!("Added global library path: {}", lib_path.display());
                    }
                    Err(e) => {
                        eprintln!("Failed to save global config: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            "remove" | "rm" => {
                let lib_path_arg = match args.get(3) {
                    Some(p) => p,
                    None => {
                        eprintln!("Usage: fast lib remove <path>");
                        std::process::exit(1);
                    }
                };
                let mut config = crate::project::GlobalConfig::load();
                match config.remove_library_path(lib_path_arg) {
                    Ok(true) => {
                        println!("Removed global library path: {}", lib_path_arg);
                    }
                    Ok(false) => {
                        println!("Path '{}' was not found in registered libraries.", lib_path_arg);
                    }
                    Err(e) => {
                        eprintln!("Failed to save global config: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            other => {
                eprintln!("Unknown lib subcommand '{}'. Use 'add', 'list', or 'remove'.", other);
                std::process::exit(1);
            }
        }
        return;
    }

    // 4b. Subcommand: registry <list | add <name> <url> [version] [desc] | import <file>>
    if args.len() > 1 && args[1] == "registry" {
        let sub = args.get(2).map(|s| s.as_str()).unwrap_or("list");
        match sub {
            "list" => {
                let reg = crate::project::DependencyRegistry::load();
                let list = reg.list();
                if list.is_empty() {
                    println!("No dependencies registered in known registry.");
                } else {
                    println!("Known Dependency Registry ({} packages):", list.len());
                    println!("{:<15} {:<10} {:<45} {}", "NAME", "VERSION", "URL", "DESCRIPTION");
                    println!("{:-<15} {:-<10} {:-<45} {:-<20}", "", "", "", "");
                    for dep in list {
                        let ver = dep.version.as_deref().unwrap_or("-");
                        let desc = dep.description.as_deref().unwrap_or("");
                        println!("{:<15} {:<10} {:<45} {}", dep.name, ver, dep.url, desc);
                    }
                }
            }
            "add" => {
                let name = match args.get(3) {
                    Some(n) => n.clone(),
                    None => {
                        eprintln!("Usage: fast registry add <name> <url> [version] [description]");
                        std::process::exit(1);
                    }
                };
                let url = match args.get(4) {
                    Some(u) => u.clone(),
                    None => {
                        eprintln!("Usage: fast registry add <name> <url> [version] [description]");
                        std::process::exit(1);
                    }
                };
                let version = args.get(5).cloned();
                let description = args.get(6).cloned();

                let mut reg = crate::project::DependencyRegistry::load();
                let dep = crate::project::KnownDependency {
                    name: name.clone(),
                    version,
                    url,
                    description,
                };
                match reg.add(dep) {
                    Ok(_) => println!("Successfully registered package '{}' in known registry.", name),
                    Err(e) => {
                        eprintln!("Failed to save registry: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            "import" => {
                let file_path = match args.get(3) {
                    Some(f) => std::path::Path::new(f),
                    None => {
                        eprintln!("Usage: fast registry import <path-to-dependencies.toml>");
                        std::process::exit(1);
                    }
                };
                let mut reg = crate::project::DependencyRegistry::load();
                match reg.import_file(file_path) {
                    Ok(count) => {
                        println!("Successfully imported {} package(s) into known dependency registry.", count);
                    }
                    Err(e) => {
                        eprintln!("Import failed: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            other => {
                eprintln!("Unknown registry subcommand '{}'. Use 'list', 'add', or 'import'.", other);
                std::process::exit(1);
            }
        }
        return;
    }

    // 4c. Subcommand: pkg <add <name> | install> (or fast add <name>)
    if (args.len() > 1 && args[1] == "pkg") || (args.len() > 1 && args[1] == "add") {
        let is_direct_add = args[1] == "add";
        let sub = if is_direct_add {
            "add"
        } else {
            args.get(2).map(|s| s.as_str()).unwrap_or("install")
        };
        let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        let (manifest_opt, project_root) = match crate::project::FastManifest::find_in_dir_or_parents(&cwd) {
            Some((m, root)) => (Some(m), root),
            None => {
                eprintln!("Error: No Fast.toml manifest found in current directory or parents.");
                std::process::exit(1);
            }
        };
        let mut manifest = manifest_opt.unwrap();
        let packages_dir = manifest.get_packages_dir(&project_root);

        match sub {
            "add" => {
                let pkg_name_idx = if is_direct_add { 2 } else { 3 };
                let pkg_name = match args.get(pkg_name_idx) {
                    Some(n) => n.clone(),
                    None => {
                        eprintln!("Usage: fast pkg add <name> (or fast add <name>)");
                        std::process::exit(1);
                    }
                };
                let req_version = args.get(pkg_name_idx + 1).cloned();

                match crate::project::resolve_and_fetch_dependency(&pkg_name, req_version.as_deref(), &packages_dir) {
                    Ok(_) => {
                        let deps = manifest.dependencies.get_or_insert_with(HashMap::new);
                        deps.insert(pkg_name.clone(), req_version.unwrap_or_else(|| "latest".to_string()));
                        let _ = manifest.save_to_file(&project_root.join("Fast.toml"));
                        println!("Package '{}' added to Fast.toml and installed in packages/.", pkg_name);
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            "install" => {
                if let Some(ref deps) = manifest.dependencies {
                    println!("Installing dependencies from Fast.toml ({} packages)...", deps.len());
                    for (name, ver) in deps {
                        if let Err(e) = crate::project::resolve_and_fetch_dependency(name, Some(ver.as_str()), &packages_dir) {
                            eprintln!("Warning: Failed to install package '{}': {}", name, e);
                        }
                    }
                    println!("Dependencies installation complete.");
                } else {
                    println!("No dependencies found in Fast.toml.");
                }
            }
            other => {
                eprintln!("Unknown pkg subcommand '{}'. Use 'add' or 'install'.", other);
                std::process::exit(1);
            }
        }
        return;
    }

    // 5. Subcommand: test
    if args.len() > 1 && args[1] == "test" {
        let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        let project_root = crate::project::FastManifest::find_in_dir_or_parents(&cwd)
            .map(|(_, root)| root)
            .unwrap_or(cwd);
        if run_project_tests(&project_root).is_err() {
            std::process::exit(1);
        }
        return;
    }

    let mut run_after_compile = false;
    let mut is_build_cmd = false;
    let mut i = 1;

    if args.len() > 1 && args[1] == "run" {
        // Check if args[2] is a custom script defined in Fast.toml
        let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        let manifest_info = crate::project::FastManifest::find_in_dir_or_parents(&cwd);
        if let Some((ref manifest, ref project_root)) = manifest_info {
            if args.len() > 2 && !args[2].starts_with('-') && args[2] != "--" {
                if let Some(ref scripts) = manifest.scripts {
                    if let Some(cmd) = scripts.get(&args[2]) {
                        let trailing_args = &args[3..];
                        #[cfg(windows)]
                        let mut process = std::process::Command::new("cmd");
                        #[cfg(windows)]
                        {
                            process.current_dir(project_root);
                            process.arg("/C").arg(cmd);
                            for a in trailing_args {
                                process.arg(a);
                            }
                        }
                        #[cfg(not(windows))]
                        let mut process = {
                            let mut full_cmd = cmd.clone();
                            for a in trailing_args {
                                full_cmd.push(' ');
                                full_cmd.push_str(a);
                            }
                            let mut cmd_proc = std::process::Command::new("sh");
                            cmd_proc.current_dir(project_root);
                            cmd_proc.arg("-c").arg(&full_cmd);
                            cmd_proc
                        };

                        let status = match process.status() {
                            Ok(s) => s.code().unwrap_or(0),
                            Err(e) => {
                                eprintln!("Failed to execute script '{}': {}", args[2], e);
                                std::process::exit(1);
                            }
                        };
                        std::process::exit(status);
                    }
                }
            }
        }

        run_after_compile = true;
        i = 2;
    } else if args.len() > 1 && args[1] == "build" {
        is_build_cmd = true;
        i = 2;
    }

    let mut user_target_file: Option<String> = None;
    let mut target = None;
    let mut custom_includes = Vec::new();
    let mut cli_backend: Option<String> = None;
    let mut _emit_ir = false;
    let mut _ir_output_file: Option<String> = None;
    let mut emit_cpp = false;
    let mut debug = false;
    let mut use_aot = false;
    let mut custom_output: Option<String> = None;
    let mut ast_output_file: Option<String> = None;
    let mut emit_ast = false;
    let mut clean_artifacts = false;
    let mut debug_errors = false;
    let mut run_args = Vec::new();

    while i < args.len() {
        if args[i] == "--" {
            run_args.extend(args[i + 1..].iter().cloned());
            break;
        } else if args[i] == "--help" || args[i] == "-h" || args[i] == "-help" {
            print_help();
            return;
        } else if args[i] == "--target" && i + 1 < args.len() {
            target = Some(args[i + 1].clone());
            i += 2;
        } else if (args[i] == "-I" || args[i] == "--include") && i + 1 < args.len() {
            let include_path = args[i + 1].clone();
            let include_path = if include_path.ends_with('/') || include_path.ends_with('\\') {
                include_path
            } else {
                format!("{}/", include_path)
            };
            custom_includes.push(include_path);
            i += 2;
        } else if (args[i] == "--backend" || args[i] == "-b") && i + 1 < args.len() {
            cli_backend = Some(args[i + 1].clone());
            i += 2;
        } else if (args[i] == "--ir-out" || args[i] == "--ir-file") && i + 1 < args.len() {
            _emit_ir = true;
            _ir_output_file = Some(args[i + 1].clone());
            i += 2;
        } else if args[i] == "--print-ir" || args[i] == "--emit-ir" {
            _emit_ir = true;
            if i + 1 < args.len()
                && !args[i + 1].starts_with('-')
                && (args[i + 1].ends_with(".ir") || args[i + 1].ends_with(".txt"))
            {
                _ir_output_file = Some(args[i + 1].clone());
                i += 2;
            } else {
                i += 1;
            }
        } else if args[i] == "--emit-cpp" || args[i] == "--cpp-only" {
            emit_cpp = true;
            i += 1;
        } else if (args[i] == "--ast-out" || args[i] == "--ast-file" || args[i] == "--json-ast")
            && i + 1 < args.len()
        {
            emit_ast = true;
            ast_output_file = Some(args[i + 1].clone());
            i += 2;
        } else if args[i] == "--emit-ast" || args[i] == "--ast" {
            emit_ast = true;
            if i + 1 < args.len()
                && !args[i + 1].starts_with('-')
                && (args[i + 1].ends_with(".json")
                    || args[i + 1].ends_with(".txt")
                    || args[i + 1].ends_with(".ast"))
            {
                ast_output_file = Some(args[i + 1].clone());
                i += 2;
            } else {
                i += 1;
            }
        } else if args[i] == "--debug" || args[i] == "-d" {
            debug = true;
            i += 1;
        } else if args[i] == "--debug-error" || args[i] == "--debug-errors" {
            debug_errors = true;
            i += 1;
        } else if (args[i] == "-o" || args[i] == "--output") && i + 1 < args.len() {
            custom_output = Some(args[i + 1].clone());
            i += 2;
        } else if args[i] == "--clean" {
            clean_artifacts = true;
            i += 1;
        } else if args[i] == "--aot" {
            use_aot = true;
            i += 1;
        } else if !args[i].starts_with('-') && user_target_file.is_none() {
            let is_file = args[i].ends_with(".fast")
                || args[i].ends_with(".fs")
                || std::path::Path::new(&args[i]).is_file();
            if is_file || !run_after_compile {
                user_target_file = Some(args[i].clone());
            } else {
                run_args.push(args[i].clone());
            }
            i += 1;
        } else if run_after_compile && !args[i].starts_with('-') {
            run_args.push(args[i].clone());
            i += 1;
        } else {
            i += 1;
        }
    }

    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let (path, manifest_info) = if let Some(ref target_f) = user_target_file {
        let pb = std::path::PathBuf::from(target_f);
        let m = crate::project::FastManifest::find_in_dir_or_parents(&pb)
            .or_else(|| crate::project::FastManifest::find_in_dir_or_parents(&cwd));
        (target_f.clone(), m)
    } else {
        let m = crate::project::FastManifest::find_in_dir_or_parents(&cwd);
        if let Some((ref manifest, ref project_root)) = m {
            let entry = manifest.get_entry_point(project_root);
            (entry.to_string_lossy().to_string(), m)
        } else if std::path::Path::new("src/main.fast").exists() {
            ("src/main.fast".to_string(), None)
        } else if std::path::Path::new("main.fast").exists() {
            ("main.fast".to_string(), None)
        } else if run_after_compile || is_build_cmd {
            eprintln!("Error: No Fast.toml project manifest found and no source file specified.");
            std::process::exit(1);
        } else {
            ("fast.fs".to_string(), None)
        }
    };

    let (manifest, project_root) = match manifest_info {
        Some((m, root)) => (Some(m), Some(root)),
        None => (None, None),
    };

    let backend = cli_backend
        .or_else(|| manifest.as_ref().and_then(|m| m.package.backend.clone()))
        .unwrap_or_else(|| "cpp".to_string());

    if custom_output.is_some() && backend == "cranelift" {
        use_aot = true;
    }

    let is_test_file = {
        let pb = std::path::Path::new(&path);
        pb.components().any(|c| c.as_os_str() == "tests" || c.as_os_str() == "test")
    };

    let is_module_project = !is_test_file
        && manifest
            .as_ref()
            .map(|m| m.package.project_type == crate::project::ProjectType::Module)
            .unwrap_or(false);

    if is_module_project && run_after_compile {
        eprintln!("Error: Cannot execute a module/library project with 'fast run'.");
        eprintln!("Use 'fast build' to compile the library, or 'fast test' to run tests.");
        std::process::exit(1);
    }

    let mut loader = match (&project_root, &manifest) {
        (Some(root), Some(m)) => loader::ProjectLoader::new_with_project(root.clone(), Some(m.clone())),
        _ => loader::ProjectLoader::new(),
    };
    for inc in custom_includes {
        loader.include_paths.push(inc);
    }

    let mut program = match loader.load(&path, target.as_deref()) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };

    if debug {
        println!("AST Length: {}", program.main_ast.len());
        std::fs::write("ast_debug.txt", format!("{:#?}", program.main_ast)).unwrap();
    }

    if emit_ast {
        let json_str = crate::frontend::parser::ast_json::ast_to_json_string(&program.main_ast);
        if let Some(out_f) = ast_output_file {
            std::fs::write(&out_f, &json_str).expect("Failed to write AST JSON file");
            println!("AST exported successfully to {}", out_f);
        } else {
            println!("{}", json_str);
        }
        return;
    }

    let mut envs: HashMap<String, Rc<RefCell<Environment>>> = HashMap::new();
    let mut all_fn_overloads: HashMap<
        String,
        HashMap<String, Vec<crate::middle_end::semantic::environment::FnSignature>>,
    > = HashMap::new();
    let mut analyzed_modules = HashSet::new();

    let mut all_module_dep_graphs: HashMap<String, HashSet<String>> = HashMap::new();
    let mut all_module_class_hierarchies: HashMap<String, String> = HashMap::new();

    // Analyze dependency modules in dependency order (e.g. std before std/list).
    while analyzed_modules.len() < program.modules.len() {
        let mut progressed = false;
        for module in &program.modules {
            if analyzed_modules.contains(&module.name) {
                continue;
            }

            let deps = module_import_deps(&module.ast);
            if !deps.iter().all(|(dep, _)| analyzed_modules.contains(dep)) {
                continue;
            }

            println!("Analyzing module: {}", module.name);
            let mut analyzer = SemanticAnalyzer::new(program.global_metadata.clone());
            analyzer.target_backend = backend.clone();
            analyzer.diagnostics.current_file = module.name.clone();
            if envs.contains_key("std/types") && module.name != "std/types" {
                inject_module_exports(
                    &mut analyzer,
                    "std/types",
                    &None,
                    &envs,
                    &all_fn_overloads,
                    debug,
                )
                .ok();
            }
            if envs.contains_key("std") && module.name != "std" {
                inject_module_exports(&mut analyzer, "std", &None, &envs, &all_fn_overloads, debug)
                    .ok();
            }
            for (dep_name, imports) in &deps {
                if let Err(e) = inject_module_exports(
                    &mut analyzer,
                    dep_name,
                    imports,
                    &envs,
                    &all_fn_overloads,
                    debug,
                ) {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
            }

            if let Err(e) = analyzer.analyze(&module.ast) {
                eprintln!("Semantic Error in module {}: {}", module.name, e);
                std::process::exit(1);
            }
            for (k, v) in analyzer.dependency_graph {
                all_module_dep_graphs.entry(k).or_default().extend(v);
            }
            for (k, v) in analyzer.class_hierarchy {
                all_module_class_hierarchies.insert(k, v);
            }
            envs.insert(module.name.clone(), analyzer.current_env);
            all_fn_overloads.insert(module.name.clone(), analyzer.fn_overloads.clone());
            analyzed_modules.insert(module.name.clone());
            progressed = true;
        }

        if !progressed {
            eprintln!("Error: cyclic or missing module dependencies detected.");
            std::process::exit(1);
        }
    }

    // Analyze main file
    if debug {
        println!("\n=== Semantic Analysis ===");
    }
    let mut main_analyzer = SemanticAnalyzer::new(program.global_metadata.clone());
    main_analyzer.target_backend = backend.clone();
    main_analyzer.diagnostics.current_file = path.clone();
    main_analyzer.diagnostics.source_code = std::fs::read_to_string(&path).ok();
    main_analyzer.scope.current_context = Some("main".to_string());
    main_analyzer.diagnostics.debug_errors = debug_errors;
    for (k, v) in all_module_dep_graphs {
        main_analyzer
            .dependency_graph
            .entry(k)
            .or_default()
            .extend(v);
    }
    for (k, v) in all_module_class_hierarchies {
        main_analyzer.class_hierarchy.insert(k, v);
    }

    if envs.contains_key("std/types") {
        inject_module_exports(
            &mut main_analyzer,
            "std/types",
            &None,
            &envs,
            &all_fn_overloads,
            debug,
        )
        .ok();
    }
    if envs.contains_key("std") {
        inject_module_exports(
            &mut main_analyzer,
            "std",
            &None,
            &envs,
            &all_fn_overloads,
            debug,
        )
        .ok();
    }

    // Inject exported symbols from main's imports
    for (mod_name, imports) in &program.main_deps {
        if let Err(e) = inject_module_exports(
            &mut main_analyzer,
            mod_name,
            imports,
            &envs,
            &all_fn_overloads,
            debug,
        ) {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }

    for stmt in &program.main_ast {
        if let Stmt::Declaration(Decl::Import {
            module_path,
            abi: Some(abi),
            ..
        }) = stmt
        {
            let mod_str = module_path.join("/");
            println!(
                "Including external library <{}> (ABI: @{})...",
                mod_str, abi
            );
        }
    }

    match main_analyzer.analyze(&program.main_ast) {
        Ok(_) => {
            if debug {
                println!("Semantic Analysis Passed successfully!");
            }
        }
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }

    if debug_errors {
        println!("Debug Errors check finished. No critical errors found.");
        return;
    }

    let mut combined_fn_overloads: HashMap<
        String,
        Vec<crate::middle_end::semantic::environment::FnSignature>,
    > = HashMap::new();
    for (_, mod_overloads) in &all_fn_overloads {
        for (fn_name, sigs) in mod_overloads {
            combined_fn_overloads
                .entry(fn_name.clone())
                .or_default()
                .extend(sigs.clone());
        }
    }
    for (fn_name, sigs) in &main_analyzer.fn_overloads {
        combined_fn_overloads
            .entry(fn_name.clone())
            .or_default()
            .extend(sigs.clone());
    }

    let reachable_symbols = main_analyzer.get_reachable_symbols();
    if debug {
        println!("\n=== Reachable Symbols (Tree-Shaking) ===");
        let mut sorted_reachable: Vec<_> = reachable_symbols.iter().collect();
        sorted_reachable.sort();
        for sym in sorted_reachable {
            println!("  reachable: {}", sym);
        }
    }

    // Compile-Time AST Evaluation & Constant Folding Pass via Interpreter
    let mut interp = crate::middle_end::interpreter::eval::Interpreter::new();
    interp.target_backend = backend.clone();
    interp.current_env = Some(main_analyzer.current_env.clone());
    interp.fn_overloads = main_analyzer.fn_overloads.clone();
    interp.load_from_program(&program.main_ast, &program.modules);
    interp.load_functions_from_env(&main_analyzer.current_env.borrow());
    for module in &mut program.modules {
        if let Err(e) = interp.evaluate_ast(&mut module.ast) {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }
    if let Err(e) = interp.evaluate_ast(&mut program.main_ast) {
        eprintln!("{}", e);
        std::process::exit(1);
    }

    // Code Generation
    let (out_path, exe_path, build_dir, out_path_base) = if let Some(ref out) = custom_output {
        if out.ends_with(".cpp") {
            let exe =
                out.trim_end_matches(".cpp").to_string() + if cfg!(windows) { ".exe" } else { "" };
            let p = std::path::Path::new(out);
            let b_dir = p.parent().unwrap_or(std::path::Path::new("")).to_path_buf();
            let stem = p
                .file_stem()
                .unwrap_or(std::ffi::OsStr::new("output"))
                .to_string_lossy();
            let base = b_dir
                .join(format!("{}", stem))
                .to_string_lossy()
                .into_owned();
            (out.clone(), exe, b_dir, base)
        } else if out.ends_with(".exe") || out.ends_with(".out") {
            let p = std::path::Path::new(out);
            let b_dir = p.parent().unwrap_or(std::path::Path::new("")).to_path_buf();
            let stem = p
                .file_stem()
                .unwrap_or(std::ffi::OsStr::new("output"))
                .to_string_lossy();
            let cpp = b_dir
                .join(format!("{}.cpp", stem))
                .to_string_lossy()
                .into_owned();
            let base = b_dir
                .join(format!("{}", stem))
                .to_string_lossy()
                .into_owned();
            (cpp, out.clone(), b_dir, base)
        } else {
            let out_p = std::path::Path::new(out);
            if out_p.is_dir() || !out.contains('.') {
                let _ = std::fs::create_dir_all(out_p);
                let stem = std::path::Path::new(&path)
                    .file_stem()
                    .unwrap_or(std::ffi::OsStr::new("output"))
                    .to_string_lossy();
                let cpp = out_p
                    .join(format!("{}.cpp", stem))
                    .to_string_lossy()
                    .into_owned();
                let exe = out_p
                    .join(if cfg!(windows) {
                        format!("{}.exe", stem)
                    } else {
                        format!("{}", stem)
                    })
                    .to_string_lossy()
                    .into_owned();
                let base = out_p
                    .join(format!("{}", stem))
                    .to_string_lossy()
                    .into_owned();
                (cpp, exe, out_p.to_path_buf(), base)
            } else {
                let b_dir = out_p
                    .parent()
                    .unwrap_or(std::path::Path::new(""))
                    .to_path_buf();
                let base = out.clone();
                (out.clone() + ".cpp", out.clone(), b_dir, base)
            }
        }
    } else {
        let source_path = std::path::Path::new(&path);
        let b_dir = if let Some(ref root) = project_root {
            root.join("build")
        } else {
            let parent_dir = source_path.parent().unwrap_or(std::path::Path::new(""));
            parent_dir.join("build")
        };
        let stem = if let Some(ref m) = manifest {
            std::borrow::Cow::Borrowed(m.package.name.as_str())
        } else {
            source_path
                .file_stem()
                .unwrap_or(std::ffi::OsStr::new("output"))
                .to_string_lossy()
        };
        if !b_dir.exists() {
            std::fs::create_dir_all(&b_dir).unwrap();
        }
        let cpp = b_dir
            .join(format!("{}.cpp", stem))
            .to_string_lossy()
            .into_owned();
        let exe = b_dir
            .join(if cfg!(windows) {
                format!("{}.exe", stem)
            } else {
                format!("{}", stem)
            })
            .to_string_lossy()
            .into_owned();
        let base = b_dir
            .join(format!("{}", stem))
            .to_string_lossy()
            .into_owned();
        (cpp, exe, b_dir, base)
    };

    let lib_dir = build_dir.join("lib");
    let clib_dir = build_dir.join("clib");
    let _ = std::fs::create_dir_all(&lib_dir);
    let _ = std::fs::create_dir_all(&clib_dir);

    // Copy C library headers/sources to build/clib
    let std_dirs = loader::get_std_dirs();
    let std_clib = std_dirs.iter().map(|d| d.join("clib")).find(|p| p.exists());
    if let Some(ref clib_src) = std_clib {
        for entry in std::fs::read_dir(clib_src).into_iter().flatten().flatten() {
            let target2 = clib_dir.join(entry.file_name());
            let should_copy = if target2.exists() {
                std::fs::read(entry.path()).ok() != std::fs::read(&target2).ok()
            } else {
                true
            };
            if should_copy {
                let _ = std::fs::copy(entry.path(), target2);
            }
        }
    }

    // Write build/clib/fast_runtime.h only for C++ backend
    if backend == "cpp" {
        let runtime_header = crate::backend::cpp::runtime::generate_runtime_header();
        let runtime_path = clib_dir.join("fast_runtime.h");
        let should_write = match std::fs::read_to_string(&runtime_path) {
            Ok(content) => content != runtime_header,
            Err(_) => true,
        };
        if should_write {
            let _ = std::fs::write(&runtime_path, &runtime_header);
        }
    }

    // Collect all C source files and precompiled object files in build/clib to pass directly to g++/gcc
    let mut clib_c_files = Vec::new();
    let mut clib_o_files = Vec::new();
    if clib_dir.exists() {
        for entry in std::fs::read_dir(&clib_dir).into_iter().flatten().flatten() {
            let p = entry.path();
            if p.extension().and_then(|e| e.to_str()) == Some("o") {
                clib_o_files.push(p);
            }
        }
        for entry in std::fs::read_dir(&clib_dir).into_iter().flatten().flatten() {
            let p = entry.path();
            if p.extension().and_then(|e| e.to_str()) == Some("c") {
                let o_path = p.with_extension("o");
                if !o_path.exists() {
                    clib_c_files.push(p);
                }
            }
        }
    }

    let check_pkg_dirs = vec![
        manifest.as_ref().and_then(|m| project_root.as_ref().map(|r| m.get_packages_dir(r))),
        Some(std::path::PathBuf::from("packages")),
    ];
    for p_dir_opt in check_pkg_dirs {
        if let Some(p_dir) = p_dir_opt {
            if p_dir.exists() {
                for pkg_entry in std::fs::read_dir(&p_dir).into_iter().flatten().flatten() {
                    if pkg_entry.path().is_dir() {
                        for f in std::fs::read_dir(pkg_entry.path()).into_iter().flatten().flatten() {
                            let p = f.path();
                            let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                            let already_linked = clib_o_files.iter().any(|existing| existing.file_stem().and_then(|s| s.to_str()) == Some(stem))
                                || clib_c_files.iter().any(|existing| existing.file_stem().and_then(|s| s.to_str()) == Some(stem));
                            if !already_linked {
                                if p.extension().and_then(|e| e.to_str()) == Some("o") {
                                    clib_o_files.push(p);
                                } else if p.extension().and_then(|e| e.to_str()) == Some("c") {
                                    let o_path = p.with_extension("o");
                                    if !o_path.exists() {
                                        clib_c_files.push(p);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if backend == "cranelift" {
        if debug {
            println!("\n=== Code Generation (Cranelift IR) ===");
        }

        // Modular Compilation:
        // Compile each loaded module into its own .o object file in lib_dir
        let mut module_obj_files = Vec::new();

        for m in &program.modules {
            let mod_safe_name = m
                .name
                .replace('/', "_")
                .replace('\\', "_")
                .replace('-', "_");
            let mod_out_o = lib_dir
                .join(format!("{}.o", mod_safe_name))
                .to_string_lossy()
                .into_owned();

            if use_aot {
                let mut mod_backend = crate::backend::cranelift::aot::CraneliftAotBackend::new();
                mod_backend.compile_ast(&m.ast);
                mod_backend.finalize(&mod_out_o);
                module_obj_files.push(mod_out_o);
            }
        }

        if use_aot {
            let out_o = format!("{}.o", out_path_base);

            let mut aot_backend = crate::backend::cranelift::aot::CraneliftAotBackend::new();
            aot_backend.compile_ast(&program.main_ast);
            aot_backend.finalize(&out_o);

            println!("Linking {} into {}...", out_o, exe_path);
            let mut gcc_cmd = std::process::Command::new("gcc");
            gcc_cmd.arg(&out_o);
            for mod_o in &module_obj_files {
                gcc_cmd.arg(mod_o);
            }
            for c_file in &clib_c_files {
                gcc_cmd.arg(c_file);
            }
            for o_file in &clib_o_files {
                gcc_cmd.arg(o_file);
            }
            gcc_cmd.arg("-Wl,--gc-sections").arg("-o").arg(&exe_path);
            let linker_status = gcc_cmd.status();

            match linker_status {
                Ok(s) if s.success() => {
                    println!(
                        "Native compilation successful! Executable is {}",
                        exe_path
                    );
                    if run_after_compile {
                        let mut run_cmd = std::process::Command::new(&exe_path);
                        run_cmd.args(&run_args);
                        match run_cmd.status() {
                            Ok(r_status) => {
                                if let Some(code) = r_status.code() {
                                    if code != 0 {
                                        std::process::exit(code);
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("Failed to execute '{}': {}", exe_path, e);
                                std::process::exit(1);
                            }
                        }
                    }
                }
                _ => {
                    eprintln!("Linker failed to produce executable from {}", out_o);
                }
            }
        } else {
            let mut cl_backend = cranelift::CraneliftBackend::new();
            cl_backend.compile_ast(&program.main_ast);
            cl_backend.finalize();
            println!("Cranelift JIT execution completed!");
        }
        return;
    }

    if debug {
        println!("\n=== Code Generation (C++) ===");
    }

    let mut initial_codegen = cpp::generator::CodeGenerator::new();
    for module in &program.modules {
        initial_codegen.scan_declarations(&module.ast);
    }
    initial_codegen.scan_declarations(&program.main_ast);

    let mut accumulated_machine_types = initial_codegen.machine_types;
    let mut accumulated_primitive_impl_methods = initial_codegen.primitive_impl_methods;
    let mut accumulated_property_access_types = initial_codegen.property_access_types;
    let mut accumulated_type_own_members = initial_codegen.type_own_members;
    let mut accumulated_struct_field_types = initial_codegen.struct_field_types;
    let mut accumulated_namespaces = initial_codegen.namespaces;
    let mut accumulated_type_aliases = initial_codegen.type_aliases;

    let base_std_names = [
        "std/types",
        "std/error",
        "std/mem",
        "std/fmt",
        "std/io",
        "std/string",
        "std/range",
        "std",
    ];
    let mut base_std_headers = Vec::new();
    let mut other_module_headers = Vec::new();

    for module in &program.modules {
        let raw_ns = module
            .name
            .split('/')
            .next()
            .unwrap_or(&module.name)
            .replace("-", "_");
        let cpp_namespace = if raw_ns == "std" {
            "fast_std".to_string()
        } else {
            raw_ns
        };

        // For module files, we do NOT prune at the AST level.
        // The C++ linker's --gc-sections handles dead code elimination for library code.
        // AST-level pruning of modules would corrupt shared .hpp files in concurrent
        // test scenarios where different tests need different symbol subsets.
        let filtered_ast: Vec<Stmt> = module
            .ast
            .iter()
            .filter_map(|stmt| match stmt {
                Stmt::Declaration(decl) => match decl {
                    Decl::FnDecl { name, .. } => {
                        if name.starts_with("@compile::") {
                            None
                        } else {
                            Some(stmt.clone())
                        }
                    }
                    _ => Some(stmt.clone()),
                },
                _ => Some(stmt.clone()),
            })
            .collect();

        let mut codegen = cpp::generator::CodeGenerator::new();
        codegen.machine_types = accumulated_machine_types.clone();
        codegen.primitive_impl_methods = accumulated_primitive_impl_methods.clone();
        codegen.property_access_types = accumulated_property_access_types.clone();
        codegen.type_own_members = accumulated_type_own_members.clone();
        codegen.struct_field_types = accumulated_struct_field_types.clone();
        codegen.namespaces = accumulated_namespaces.clone();
        codegen.type_aliases = accumulated_type_aliases.clone();
        let module_cpp = codegen.generate(&filtered_ast, false, false);
        accumulated_machine_types.extend(codegen.machine_types.clone());
        accumulated_struct_field_types.extend(codegen.struct_field_types.clone());
        accumulated_namespaces.extend(codegen.namespaces.clone());
        accumulated_type_aliases.extend(codegen.type_aliases.clone());
        for (k, v) in codegen.primitive_impl_methods {
            let list = accumulated_primitive_impl_methods
                .entry(k)
                .or_insert_with(Vec::new);
            for t in v {
                if !list.contains(&t) {
                    list.push(t);
                }
            }
        }
        accumulated_property_access_types.extend(codegen.property_access_types.clone());
        for (k, v) in codegen.type_own_members {
            accumulated_type_own_members
                .entry(k)
                .or_insert_with(std::collections::HashSet::new)
                .extend(v);
        }

        let safe_name = module.name.replace('/', "_") + ".hpp";
        let mut header_content = String::new();
        header_content.push_str("#pragma once\n");
        header_content.push_str("#include <clib/fast_runtime.h>\n");
        for inc in &codegen.c_includes {
            header_content.push_str(&format!("{}\n", inc));
        }
        if !base_std_names.contains(&module.name.as_str()) {
            header_content.push_str("#include <lib/fast_std.hpp>\n");
        }
        for (dep, _) in module_import_deps(&module.ast) {
            let dep_safe = dep.replace('/', "_") + ".hpp";
            header_content.push_str(&format!("#include <lib/{}>\n", dep_safe));
        }
        header_content.push_str(&format!("namespace {} {{\n", cpp_namespace));
        header_content.push_str("using ::fastlang_as_str;\n");
        header_content.push_str(&module_cpp);
        header_content.push_str(&format!("\n}} // namespace {}\n", cpp_namespace));

        let header_path = lib_dir.join(&safe_name);
        let should_write = match std::fs::read_to_string(&header_path) {
            Ok(content) => content != header_content,
            Err(_) => true,
        };
        if should_write {
            let _ = std::fs::write(&header_path, &header_content);
        }

        if base_std_names.contains(&module.name.as_str()) {
            base_std_headers.push(safe_name);
        } else {
            other_module_headers.push(safe_name);
        }
    }

    base_std_headers.sort_by_key(|h| {
        base_std_names
            .iter()
            .position(|name| {
                let safe = name.replace('/', "_") + ".hpp";
                &safe == h
            })
            .unwrap_or(usize::MAX)
    });

    // Generate build/lib/fast_std.hpp (core prelude: std/types, std/error, std)
    let mut fast_std_content = String::new();
    fast_std_content.push_str("#pragma once\n");
    fast_std_content.push_str("#include <clib/fast_runtime.h>\n");
    fast_std_content.push_str("#include <clib/fmt.h>\n");
    fast_std_content.push_str("#include <clib/io.h>\n");
    let core_prelude = ["std_types.hpp", "std_error.hpp", "std.hpp"];
    for h in &core_prelude {
        if lib_dir.join(h).exists() || base_std_headers.contains(&h.to_string()) {
            fast_std_content.push_str(&format!("#include <lib/{}>\n", h));
        }
    }
    let fast_std_path = lib_dir.join("fast_std.hpp");
    let should_write_std = match std::fs::read_to_string(&fast_std_path) {
        Ok(content) => content != fast_std_content,
        Err(_) => true,
    };
    if should_write_std {
        let _ = std::fs::write(&fast_std_path, &fast_std_content);
    }

    let mut main_codegen = cpp::generator::CodeGenerator::new();
    main_codegen.machine_types = accumulated_machine_types;
    main_codegen.primitive_impl_methods = accumulated_primitive_impl_methods;
    main_codegen.property_access_types = accumulated_property_access_types;
    main_codegen.type_own_members = accumulated_type_own_members;
    main_codegen.struct_field_types = accumulated_struct_field_types;
    main_codegen.namespaces = accumulated_namespaces;
    main_codegen.type_aliases = accumulated_type_aliases;
    let pruned_main_ast = if is_module_project {
        program.main_ast.clone()
    } else {
        prune_ast_for_reachability(
            &program.main_ast,
            &reachable_symbols,
            &combined_fn_overloads,
            true,
        )
    };
    let main_cpp = main_codegen.generate(&pruned_main_ast, false, !is_module_project);

    let mut final_cpp = String::new();
    final_cpp.push_str("#include <clib/fast_runtime.h>\n");

    if envs.contains_key("std") || envs.contains_key("std/types") || !base_std_headers.is_empty() {
        final_cpp.push_str("#include <lib/fast_std.hpp>\n");
    }

    // Include all other loaded modules directly in final_cpp
    for module in &program.modules {
        let safe_name = module.name.replace('/', "_") + ".hpp";
        if module.name != "std" && module.name != "std/types" && module.name != "std/error" {
            let inc_line = format!("#include <lib/{}>\n", safe_name);
            if !final_cpp.contains(&inc_line) {
                final_cpp.push_str(&inc_line);
            }
        }
    }

    for (dep_mod, _) in &program.main_deps {
        let safe_name = dep_mod.replace('/', "_") + ".hpp";
        let inc_line = format!("#include <lib/{}>\n", safe_name);
        if !final_cpp.contains(&inc_line) {
            final_cpp.push_str(&inc_line);
        }
    }

    for inc in &main_codegen.c_includes {
        final_cpp.push_str(&format!("{}\n", inc));
    }

    if envs.contains_key("std") || !base_std_headers.is_empty() {
        final_cpp.push_str("\nusing namespace fast_std;\nusing ::fastlang_as_str;\n\n");
    }

    final_cpp.push_str(&main_cpp);

    let mut write_success = false;
    for _ in 0..10 {
        if fs::write(&out_path, &final_cpp).is_ok() {
            write_success = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    if !write_success {
        fs::write(&out_path, &final_cpp).expect("Failed to write output.cpp");
    }
    if debug {
        println!("Successfully generated C++ code to {}", out_path);
    }

    if emit_cpp {
        println!("C++ emission complete (--emit-cpp specified, skipped g++ compilation).");
        return;
    }

    if is_module_project {
        let mod_stem = manifest
            .as_ref()
            .map(|m| m.package.name.as_str())
            .unwrap_or("lib");
        let obj_path = build_dir.join(format!("{}.o", mod_stem));
        let hpp_path = build_dir.join(format!("{}.hpp", mod_stem));
        let mut header_content = String::new();
        header_content.push_str("#pragma once\n");
        header_content.push_str(&final_cpp);
        let _ = fs::write(&hpp_path, &header_content);

        if debug {
            println!("Compiling library object to {}...", obj_path.display());
        }

        let mut gpp_cmd = std::process::Command::new("g++");
        gpp_cmd.arg("-c");
        gpp_cmd.arg(&out_path);
        gpp_cmd
            .arg("-o")
            .arg(&obj_path)
            .arg("-std=c++20")
            .arg("-ffunction-sections")
            .arg("-fdata-sections")
            .arg("-I.")
            .arg(format!("-I{}", build_dir.display()))
            .arg(format!("-I{}", lib_dir.display()))
            .arg("-Isrc/std")
            .arg("-Isrc");
        let check_pkg_includes = vec![
            manifest.as_ref().and_then(|m| project_root.as_ref().map(|r| m.get_packages_dir(r))),
            Some(std::path::PathBuf::from("packages")),
        ];
        for p_dir_opt in check_pkg_includes {
            if let Some(pkg_dir) = p_dir_opt {
                if pkg_dir.exists() {
                    gpp_cmd.arg(format!("-I{}", pkg_dir.display()));
                    if let Ok(entries) = std::fs::read_dir(&pkg_dir) {
                        for entry in entries.flatten() {
                            if entry.path().is_dir() {
                                gpp_cmd.arg(format!("-I{}", entry.path().display()));
                            }
                        }
                    }
                }
            }
        }
        if let Some(ref root) = project_root {
            gpp_cmd.arg(format!("-I{}", root.display()));
        }
        for s_dir in &std_dirs {
            gpp_cmd.arg(format!("-I{}", s_dir.display()));
            if let Some(parent) = s_dir.parent() {
                gpp_cmd.arg(format!("-I{}", parent.display()));
            }
        }

        let status = gpp_cmd.status();
        match status {
            Ok(s) if s.success() => {
                println!("Module library compiled successfully!");
                println!("  C++ Header: {}", hpp_path.display());
                println!("  Object:     {}", obj_path.display());
                if clean_artifacts {
                    let _ = fs::remove_file(&out_path);
                    let _ = fs::remove_dir_all(&lib_dir);
                    let _ = fs::remove_dir_all(&clib_dir);
                }
                return;
            }
            _ => {
                eprintln!("C++ module compilation failed! Check {} for errors.", out_path);
                std::process::exit(1);
            }
        }
    }

    if debug {
        println!("Compiling to {}...", exe_path);
    }

    let mut gpp_cmd = std::process::Command::new("g++");
    gpp_cmd.arg(&out_path);
    for c_file in &clib_c_files {
        gpp_cmd.arg(c_file);
    }
    for o_file in &clib_o_files {
        gpp_cmd.arg(o_file);
    }
    gpp_cmd
        .arg("-o")
        .arg(&exe_path)
        .arg("-std=c++20")
        .arg("-ffunction-sections")
        .arg("-fdata-sections")
        .arg("-Wl,--gc-sections")
        .arg("-I.")
        .arg(format!("-I{}", build_dir.display()))
        .arg(format!("-I{}", lib_dir.display()))
        .arg("-Isrc/std")
        .arg("-Isrc");
    if cfg!(windows) {
        gpp_cmd.arg("-lws2_32");
    }
    let check_pkg_includes = vec![
        manifest.as_ref().and_then(|m| project_root.as_ref().map(|r| m.get_packages_dir(r))),
        Some(std::path::PathBuf::from("packages")),
    ];
    for p_dir_opt in check_pkg_includes {
        if let Some(pkg_dir) = p_dir_opt {
            if pkg_dir.exists() {
                gpp_cmd.arg(format!("-I{}", pkg_dir.display()));
                if let Ok(entries) = std::fs::read_dir(&pkg_dir) {
                    for entry in entries.flatten() {
                        if entry.path().is_dir() {
                            gpp_cmd.arg(format!("-I{}", entry.path().display()));
                        }
                    }
                }
            }
        }
    }
    if let Some(ref root) = project_root {
        gpp_cmd.arg(format!("-I{}", root.display()));
    }
    for s_dir in &std_dirs {
        gpp_cmd.arg(format!("-I{}", s_dir.display()));
        if let Some(parent) = s_dir.parent() {
            gpp_cmd.arg(format!("-I{}", parent.display()));
        }
    }

    let status = gpp_cmd.status();

    match status {
        Ok(s) if s.success() => {
            println!("Compilation successful! Executable is {}", exe_path);
            if clean_artifacts {
                let _ = fs::remove_file(&out_path);
                let _ = fs::remove_dir_all(&lib_dir);
                let _ = fs::remove_dir_all(&clib_dir);
            }
            if run_after_compile {
                let mut run_cmd = std::process::Command::new(&exe_path);
                run_cmd.args(&run_args);
                match run_cmd.status() {
                    Ok(r_status) => {
                        if let Some(code) = r_status.code() {
                            if code != 0 {
                                std::process::exit(code);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to execute '{}': {}", exe_path, e);
                        std::process::exit(1);
                    }
                }
            }
        }
        _ => {
            eprintln!("C++ compilation failed! Check {} for errors.", out_path);
            std::process::exit(1);
        }
    }
}
