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
                if abi.is_some() || module_path.first().map_or(false, |p| p.ends_with(".h") || p.ends_with(".hpp")) {
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
    all_fn_overloads: &HashMap<String, HashMap<String, Vec<crate::middle_end::semantic::environment::FnSignature>>>,
    debug: bool,
) -> Result<(), String> {
    let env = envs
        .get(dep_name)
        .ok_or_else(|| format!("Error: Module {} not found.", dep_name))?;
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
                analyzer.current_env.borrow_mut().define(sym_name, info).ok();
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
                analyzer.fn_overloads.entry(fn_name.clone()).or_default().extend(sigs.clone());
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
            analyzer.current_env.borrow_mut().define_blueprint(bp_name, bp_data);
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
                if let Some(pruned_decl) = prune_decl_for_reachability(decl, reachable, fn_overloads, is_main_file) {
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
        Decl::FnDecl {
            name,
            params,
            ..
        } => {
            if name == "main" {
                return Some(decl.clone());
            }
            let sig_key = crate::middle_end::semantic::analyzer::decl::make_sig_key(name, params);
            let has_multiple_overloads = fn_overloads
                .get(name)
                .map_or(0, |sigs| {
                    let mut distinct = HashSet::new();
                    for s in sigs {
                        distinct.insert(crate::middle_end::semantic::analyzer::decl::make_sig_key(&s.name, &s.params));
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
            // For module files, keep entire ImplDecl if the target type is reachable.
            if !is_main_file && reachable.contains(target) {
                return Some(decl.clone());
            }

            let filter_methods: Vec<Decl> = methods
                .iter()
                .filter(|d| match d {
                    Decl::FnDecl { name: m_name, .. } => {
                        let full_m = format!("{}::{}", target, m_name);
                        reachable.contains(&full_m)
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


        Decl::BlueprintDecl {
            name,
            ..
        } => {
            if !reachable.contains(name) {
                return None;
            }
            Some(decl.clone())
        }

        Decl::MachineDecl {
            name,
            ..
        } => {
            if !reachable.contains(name) {
                return None;
            }
            Some(decl.clone())
        }

        Decl::ObjectDecl {
            name,
            ..
        } => {
            if !reachable.contains(name) {
                return None;
            }
            Some(decl.clone())
        }

        Decl::ExternFnDecl { name, alias, .. } => {
            let is_kept = reachable.contains(name)
                || alias.as_ref().map_or(false, |a| reachable.contains(a));
            if is_kept {
                Some(decl.clone())
            } else {
                None
            }
        }

        Decl::ExternBlockDecl { abi, decls } => {
            let filtered_decls: Vec<Decl> = decls
                .iter()
                .filter_map(|d| prune_decl_for_reachability(d, reachable, fn_overloads, is_main_file))
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
    println!(r#"FastLang Compiler (fast_lang) v0.1.0
High-performance compiled language with zero-cost custom scopes and Coroutine State Machines.

USAGE:
    fast_lang [OPTIONS] <SOURCE_FILE>

ARGUMENTS:
    <SOURCE_FILE>                  Path to the main entry source file (.fs)

OPTIONS:
    -h, --help, -help              Print this help information and exit
    -d, --debug                    Enable verbose debug and compilation trace output
    -o, --output <PATH>            Specify output binary or build directory path
    -I, --include <DIR>            Add module search directory
    -b, --backend <BACKEND>        Set code generator backend: 'cpp' (default) or 'cranelift'
    --ast, --emit-ast [FILE]       Dump parsed AST as formatted JSON (to stdout or FILE)
    --ast-file, --json-ast <FILE>  Export parsed AST as formatted JSON directly to FILE
    --emit-cpp, --cpp-only         Generate and emit C++ source file without compiling binary
    --emit-ir, --print-ir          Print intermediate representation (IR) output
    --aot                          Enable Ahead-Of-Time (AOT) compilation
    --target <TARGET>              Specify target architecture/platform
    --clean                        Remove temporary C++ and library files after successful build

SUBCOMMANDS:
    clean [PATH]                   Delete build/ directory and cached artifacts in PATH (or cwd)

EXAMPLES:
    fast_lang app.fs
    fast_lang app.fs -o build/app.exe
    fast_lang app.fs --ast-file app_ast.json
    fast_lang app.fs --emit-cpp
    fast_lang app.fs -I ./std/ -d
    fast_lang clean
"#);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 1 || args.contains(&"--help".to_string()) || args.contains(&"-h".to_string()) || args.contains(&"-help".to_string()) {
        print_help();
        return;
    }

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
            println!("Nothing to clean. ({}) does not exist.", build_dir.display());
        }
        return;
    }

    let mut path = "fast.fs".to_string();
    let mut target = None;
    let mut custom_includes = Vec::new();
    let mut backend = "cpp".to_string();
    let mut emit_ir = false;
    let mut ir_output_file: Option<String> = None;
    let mut emit_cpp = false;
    let mut debug = false;
    let mut use_aot = false;
    let mut custom_output: Option<String> = None;
    let mut ast_output_file: Option<String> = None;
    let mut emit_ast = false;
    let mut clean_artifacts = false;

    let mut i = 1;
    while i < args.len() {
        if args[i] == "--help" || args[i] == "-h" || args[i] == "-help" {
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
            backend = args[i + 1].clone();
            i += 2;
        } else if (args[i] == "--ir-out" || args[i] == "--ir-file") && i + 1 < args.len() {
            emit_ir = true;
            ir_output_file = Some(args[i + 1].clone());
            i += 2;
        } else if args[i] == "--print-ir" || args[i] == "--emit-ir" {
            emit_ir = true;
            if i + 1 < args.len() && !args[i + 1].starts_with('-') && (args[i + 1].ends_with(".ir") || args[i + 1].ends_with(".txt")) {
                ir_output_file = Some(args[i + 1].clone());
                i += 2;
            } else {
                i += 1;
            }
        } else if args[i] == "--emit-cpp" || args[i] == "--cpp-only" {
            emit_cpp = true;
            i += 1;
        } else if (args[i] == "--ast-out" || args[i] == "--ast-file" || args[i] == "--json-ast") && i + 1 < args.len() {
            emit_ast = true;
            ast_output_file = Some(args[i + 1].clone());
            i += 2;
        } else if args[i] == "--emit-ast" || args[i] == "--ast" {
            emit_ast = true;
            if i + 1 < args.len() && !args[i + 1].starts_with('-') && (args[i + 1].ends_with(".json") || args[i + 1].ends_with(".txt") || args[i + 1].ends_with(".ast")) {
                ast_output_file = Some(args[i + 1].clone());
                i += 2;
            } else {
                i += 1;
            }
        } else if args[i] == "--debug" || args[i] == "-d" {
            debug = true;
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
        } else {
            path = args[i].clone();
            i += 1;
        }
    }

    let mut loader = loader::ProjectLoader::new();
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
    let mut all_fn_overloads: HashMap<String, HashMap<String, Vec<crate::middle_end::semantic::environment::FnSignature>>> = HashMap::new();
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

            let mut analyzer = SemanticAnalyzer::new(program.global_metadata.clone());
            analyzer.current_file = module.name.clone();
            if envs.contains_key("std/types") && module.name != "std/types" {
                inject_module_exports(&mut analyzer, "std/types", &None, &envs, &all_fn_overloads, debug).ok();
            }
            if envs.contains_key("std") && module.name != "std" {
                inject_module_exports(&mut analyzer, "std", &None, &envs, &all_fn_overloads, debug).ok();
            }
            for (dep_name, imports) in &deps {
                if let Err(e) = inject_module_exports(&mut analyzer, dep_name, imports, &envs, &all_fn_overloads, debug) {
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
    main_analyzer.current_file = path.clone();
    main_analyzer.source_code = std::fs::read_to_string(&path).ok();
    main_analyzer.current_context = Some("main".to_string());
    for (k, v) in all_module_dep_graphs {
        main_analyzer.dependency_graph.entry(k).or_default().extend(v);
    }
    for (k, v) in all_module_class_hierarchies {
        main_analyzer.class_hierarchy.insert(k, v);
    }

    if envs.contains_key("std/types") {
        inject_module_exports(&mut main_analyzer, "std/types", &None, &envs, &all_fn_overloads, debug).ok();
    }
    if envs.contains_key("std") {
        inject_module_exports(&mut main_analyzer, "std", &None, &envs, &all_fn_overloads, debug).ok();
    }

    // Inject exported symbols from main's imports
    for (mod_name, imports) in &program.main_deps {
        if let Err(e) = inject_module_exports(&mut main_analyzer, mod_name, imports, &envs, &all_fn_overloads, debug) {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }

    for stmt in &program.main_ast {
        if let Stmt::Declaration(Decl::Import { module_path, abi: Some(abi), .. }) = stmt {
            let mod_str = module_path.join("/");
            println!("Including external library <{}> (ABI: @{})...", mod_str, abi);
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

    let mut combined_fn_overloads: HashMap<String, Vec<crate::middle_end::semantic::environment::FnSignature>> = HashMap::new();
    for (_, mod_overloads) in &all_fn_overloads {
        for (fn_name, sigs) in mod_overloads {
            combined_fn_overloads.entry(fn_name.clone()).or_default().extend(sigs.clone());
        }
    }
    for (fn_name, sigs) in &main_analyzer.fn_overloads {
        combined_fn_overloads.entry(fn_name.clone()).or_default().extend(sigs.clone());
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
            let exe = out.trim_end_matches(".cpp").to_string() + if cfg!(windows) { ".exe" } else { "" };
            let p = std::path::Path::new(out);
            let b_dir = p.parent().unwrap_or(std::path::Path::new("")).to_path_buf();
            let stem = p.file_stem().unwrap_or(std::ffi::OsStr::new("output")).to_string_lossy();
            let base = b_dir.join(format!("{}", stem)).to_string_lossy().into_owned();
            (out.clone(), exe, b_dir, base)
        } else if out.ends_with(".exe") || out.ends_with(".out") {
            let p = std::path::Path::new(out);
            let b_dir = p.parent().unwrap_or(std::path::Path::new("")).to_path_buf();
            let stem = p.file_stem().unwrap_or(std::ffi::OsStr::new("output")).to_string_lossy();
            let cpp = b_dir.join(format!("{}.cpp", stem)).to_string_lossy().into_owned();
            let base = b_dir.join(format!("{}", stem)).to_string_lossy().into_owned();
            (cpp, out.clone(), b_dir, base)
        } else {
            let out_p = std::path::Path::new(out);
            if out_p.is_dir() || !out.contains('.') {
                let _ = std::fs::create_dir_all(out_p);
                let stem = std::path::Path::new(&path).file_stem().unwrap_or(std::ffi::OsStr::new("output")).to_string_lossy();
                let cpp = out_p.join(format!("{}.cpp", stem)).to_string_lossy().into_owned();
                let exe = out_p.join(if cfg!(windows) { format!("{}.exe", stem) } else { format!("{}", stem) }).to_string_lossy().into_owned();
                let base = out_p.join(format!("{}", stem)).to_string_lossy().into_owned();
                (cpp, exe, out_p.to_path_buf(), base)
            } else {
                let b_dir = out_p.parent().unwrap_or(std::path::Path::new("")).to_path_buf();
                let base = out.clone();
                (out.clone() + ".cpp", out.clone(), b_dir, base)
            }
        }
    } else {
        let source_path = std::path::Path::new(&path);
        let parent_dir = source_path.parent().unwrap_or(std::path::Path::new(""));
        let build_dir = parent_dir.join("build");
        let stem = source_path.file_stem().unwrap_or(std::ffi::OsStr::new("output")).to_string_lossy();
        if !build_dir.exists() {
            std::fs::create_dir_all(&build_dir).unwrap();
        }
        let cpp = build_dir.join(format!("{}.cpp", stem)).to_string_lossy().into_owned();
        let exe = build_dir.join(if cfg!(windows) { format!("{}.exe", stem) } else { format!("{}", stem) }).to_string_lossy().into_owned();
        let base = build_dir.join(format!("{}", stem)).to_string_lossy().into_owned();
        (cpp, exe, build_dir, base)
    };

    let lib_dir = build_dir.join("lib");
    let clib_dir = build_dir.join("clib");
    let _ = std::fs::create_dir_all(&lib_dir);
    let _ = std::fs::create_dir_all(&clib_dir);

    // Copy C library headers/sources to build/clib
    let std_clib = std::path::Path::new("src/std/clib");
    if std_clib.exists() {
        for entry in std::fs::read_dir(std_clib).into_iter().flatten().flatten() {
            let target2 = clib_dir.join(entry.file_name());
            let _ = std::fs::copy(entry.path(), target2);
        }
    }

    // Write build/clib/fast_runtime.h
    let runtime_header = crate::backend::cpp::runtime::generate_runtime_header();
    let runtime_path = clib_dir.join("fast_runtime.h");
    let _ = std::fs::write(&runtime_path, &runtime_header);

    // Collect all C source files in build/clib to pass directly to g++/gcc
    let mut clib_c_files = Vec::new();
    if clib_dir.exists() {
        for entry in std::fs::read_dir(&clib_dir).into_iter().flatten().flatten() {
            let p = entry.path();
            if p.extension().and_then(|e| e.to_str()) == Some("c") {
                clib_c_files.push(p);
            }
        }
    }

    if backend == "cranelift" {
        if debug {
            println!("\n=== Code Generation (Cranelift IR) ===");
        }

        let builder = crate::middle_end::ir::builder::IRBuilder::new(
            "main".to_string(),
            &program.global_metadata,
        );
        let mut all_stmts = Vec::new();
        for m in &program.modules {
            all_stmts.extend(m.ast.clone());
        }
        all_stmts.extend(program.main_ast.clone());
        let ir_module = builder.build(&all_stmts);
        if debug {
            println!(
                "Generated Custom IR Module with {} functions.",
                ir_module.functions.len()
            );
        }

        if emit_ir {
            let ir_str = format!("{}", ir_module);
            if let Some(out_f) = &ir_output_file {
                std::fs::write(out_f, &ir_str).expect("Failed to write IR file");
                println!("IR exported successfully to {}", out_f);
            } else {
                println!("\n=== Custom IR Output ===");
                println!("{}", ir_str);
                println!("========================\n");
            }
            if !use_aot {
                return;
            }
        }

        if use_aot {
            let ir_file = format!("{}.ir", out_path_base);
            std::fs::write(&ir_file, format!("{}", ir_module)).ok();

            let out_o = format!("{}.o", out_path_base);

            let mut aot_backend = crate::backend::cranelift::aot::CraneliftAotBackend::new();
            aot_backend.compile_module(&ir_module);
            aot_backend.finalize(&out_o);

            println!("Linking {} into {}...", out_o, exe_path);
            let mut gcc_cmd = std::process::Command::new("gcc");
            gcc_cmd.arg(&out_o);
            for c_file in &clib_c_files {
                gcc_cmd.arg(c_file);
            }
            gcc_cmd.arg("-Wl,--gc-sections").arg("-o").arg(&exe_path);
            let linker_status = gcc_cmd.status();

            match linker_status {
                Ok(s) if s.success() => {
                    println!("Native compilation successful! Executable is {} 🚀", exe_path);
                }
                _ => {
                    eprintln!("Linker failed to produce executable from {}", out_o);
                }
            }
        } else {
            let mut cl_backend = cranelift::CraneliftBackend::new();
            cl_backend.compile_module(&ir_module);
            cl_backend.finalize();
            println!("Cranelift JIT execution completed!");
        }
        return;
    }

    if debug {
        println!("\n=== Code Generation (C++) ===");
    }

    let mut accumulated_custom_scopes = std::collections::HashSet::new();
    let mut accumulated_primitive_impl_methods = std::collections::HashMap::new();
    let mut accumulated_property_access_types = std::collections::HashSet::new();
    let mut accumulated_type_own_members = std::collections::HashMap::new();

    let base_std_names = ["std/types", "std/error", "std/io", "std/string", "std/range", "std"];
    let mut base_std_headers = Vec::new();
    let mut other_module_headers = Vec::new();

    for module in &program.modules {
        let raw_ns = module.name.split('/').next().unwrap_or(&module.name).replace("-", "_");
        let cpp_namespace = if raw_ns == "std" { "fast_std".to_string() } else { raw_ns };

        // For module files, we do NOT prune at the AST level.
        // The C++ linker's --gc-sections handles dead code elimination for library code.
        // AST-level pruning of modules would corrupt shared .hpp files in concurrent
        // test scenarios where different tests need different symbol subsets.
        let filtered_ast: Vec<Stmt> = module.ast.iter().filter_map(|stmt| {
            match stmt {
                Stmt::Declaration(decl) => {
                    match decl {
                        Decl::FnDecl { name, body, .. } => {
                            let is_comptime = main_analyzer
                                .current_env
                                .borrow()
                                .lookup(name)
                                .map_or(false, |info| info.is_compilable)
                                || name.starts_with("@compile::")
                                || crate::middle_end::semantic::analyzer::detect_execution_mode(body)
                                    == crate::frontend::parser::ast::ExecutionMode::FullyCompilable;
                            if is_comptime {
                                None
                            } else {
                                Some(stmt.clone())
                            }
                        }
                        _ => Some(stmt.clone()),
                    }
                }
                _ => Some(stmt.clone()),
            }
        }).collect();


        let mut codegen = cpp::generator::CodeGenerator::new();
        codegen.custom_scope_types = accumulated_custom_scopes.clone();
        codegen.primitive_impl_methods = accumulated_primitive_impl_methods.clone();
        codegen.property_access_types = accumulated_property_access_types.clone();
        codegen.type_own_members = accumulated_type_own_members.clone();
        let module_cpp = codegen.generate(&filtered_ast, false, false);
        accumulated_custom_scopes.extend(codegen.custom_scope_types.clone());
        for (k, v) in codegen.primitive_impl_methods {
            let list = accumulated_primitive_impl_methods.entry(k).or_insert_with(Vec::new);
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
        header_content.push_str("\n");
        header_content.push_str(&format!("namespace {} {{\n", cpp_namespace));
        header_content.push_str("using ::fastlang_as_str;\n");
        header_content.push_str(&module_cpp);
        header_content.push_str(&format!("\n}} // namespace {}\n", cpp_namespace));

        let header_path = lib_dir.join(&safe_name);
        let _ = std::fs::write(&header_path, &header_content);

        if base_std_names.contains(&module.name.as_str()) {
            base_std_headers.push(safe_name);
        } else {
            other_module_headers.push(safe_name);
        }
    }

    base_std_headers.sort_by_key(|h| {
        base_std_names.iter().position(|name| {
            let safe = name.replace('/', "_") + ".hpp";
            &safe == h
        }).unwrap_or(usize::MAX)
    });

    // Generate build/lib/fast_std.hpp
    let mut fast_std_content = String::new();
    fast_std_content.push_str("#pragma once\n");
    fast_std_content.push_str("#include <clib/fast_runtime.h>\n");
    for h in &base_std_headers {
        fast_std_content.push_str(&format!("#include <lib/{}>\n", h));
    }
    let fast_std_path = lib_dir.join("fast_std.hpp");
    let _ = std::fs::write(&fast_std_path, &fast_std_content);

    let mut main_codegen = cpp::generator::CodeGenerator::new();
    main_codegen.custom_scope_types = accumulated_custom_scopes;
    main_codegen.primitive_impl_methods = accumulated_primitive_impl_methods;
    main_codegen.property_access_types = accumulated_property_access_types;
    main_codegen.type_own_members = accumulated_type_own_members;
    let pruned_main_ast = prune_ast_for_reachability(&program.main_ast, &reachable_symbols, &combined_fn_overloads, true);
    let main_cpp = main_codegen.generate(&pruned_main_ast, false, true);

    let mut final_cpp = String::new();
    final_cpp.push_str("#include <clib/fast_runtime.h>\n");

    if envs.contains_key("std") || !base_std_headers.is_empty() {
        final_cpp.push_str("#include <lib/fast_std.hpp>\n");
    }

    for (dep_mod, _) in &program.main_deps {
        let safe_name = dep_mod.replace('/', "_") + ".hpp";
        if !base_std_headers.contains(&safe_name) {
            final_cpp.push_str(&format!("#include <lib/{}>\n", safe_name));
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

    if debug {
        println!("Compiling to {}...", exe_path);
    }

    let mut gpp_cmd = std::process::Command::new("g++");
    gpp_cmd.arg(&out_path);
    for c_file in &clib_c_files {
        gpp_cmd.arg(c_file);
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

    let status = gpp_cmd.status();

    match status {
        Ok(s) if s.success() => {
            println!("Compilation successful! Executable is {} 🚀", exe_path);
            if clean_artifacts {
                let _ = fs::remove_file(&out_path);
                let _ = fs::remove_dir_all(&lib_dir);
                let _ = fs::remove_dir_all(&clib_dir);
            }
        }
        _ => {
            eprintln!("C++ compilation failed! Check {} for errors.", out_path);
            std::process::exit(1);
        }
    }
}
