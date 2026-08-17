use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::rc::Rc;

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

    println!("\n\x1b[31;1mError:\x1b[0m {}", err_msg);
    println!("  \x1b[34m-->\x1b[0m line {}:{}", line, column);
    println!("   \x1b[34m|\x1b[0m");

    for i in start_line..=end_line {
        let i_minus_1 = i - 1;
        if i_minus_1 < lines.len() {
            let l_text = lines[i_minus_1];
            if i == line {
                println!("{:3}\x1b[34m |\x1b[0m {}", i, l_text);
                let padding = " ".repeat(column);
                println!("    \x1b[34m|\x1b[0m{}\x1b[31;1m^-- Here\x1b[0m", padding);
            } else {
                println!("{:3}\x1b[34m |\x1b[0m {}", i, l_text);
            }
        }
    }
    println!("   \x1b[34m|\x1b[0m\n");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut path = "fast.fs".to_string();
    let mut target = None;
    let mut custom_includes = Vec::new();
    let mut backend = "cpp".to_string();
    let mut emit_ir = false;
    let mut use_aot = false;

    let mut i = 1;
    while i < args.len() {
        if args[i] == "--target" && i + 1 < args.len() {
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
        } else if args[i] == "--print-ir" || args[i] == "--emit-ir" {
            emit_ir = true;
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

    let program = match loader.load(&path, target.as_deref()) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };

    println!("AST Length: {}", program.main_ast.len());
    std::fs::write("ast_debug.txt", format!("{:#?}", program.main_ast)).unwrap();

    let mut envs: HashMap<String, Rc<RefCell<Environment>>> = HashMap::new();

    // Parse and analyze dependencies
    for module in &program.modules {
        let mut analyzer = SemanticAnalyzer::new(program.global_metadata.clone());
        if let Err(e) = analyzer.analyze(&module.ast) {
            eprintln!("Semantic Error in module {}: {}", module.name, e);
            std::process::exit(1);
        }
        envs.insert(module.name.clone(), analyzer.current_env);
    }

    // Analyze main file
    println!("\n=== Semantic Analysis ===");
    let mut main_analyzer = SemanticAnalyzer::new(program.global_metadata.clone());

    // Inject exported symbols
    for (mod_name, imports) in &program.main_deps {
        if let Some(env) = envs.get(mod_name) {
            let symbols = env.borrow().symbols.clone();
            for (sym_name, info) in symbols {
                if info.visibility == Visibility::Public {
                    let should_inject = match imports {
                        Some(selected) => selected.contains(&sym_name),
                        None => true,
                    };

                    if should_inject {
                        println!("Module {} injected symbol: {}", mod_name, sym_name);
                        main_analyzer
                            .current_env
                            .borrow_mut()
                            .define(sym_name, info)
                            .ok();
                    }
                }
            }
        } else {
            eprintln!("Error: Module {} not found.", mod_name);
            std::process::exit(1);
        }
    }

    match main_analyzer.analyze(&program.main_ast) {
        Ok(_) => println!("Semantic Analysis Passed successfully! ?"),
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }

    // Code Generation
    if backend == "cranelift" {
        println!("\n=== Code Generation (Cranelift IR) ===");

        let builder = crate::middle_end::ir::builder::IRBuilder::new(
            "main".to_string(),
            &program.global_metadata,
        );
        let ir_module = builder.build(&program.main_ast);
        println!(
            "Generated Custom IR Module with {} functions.",
            ir_module.functions.len()
        );

        if emit_ir {
            println!("\n=== Custom IR Output ===");
            println!("{}", ir_module);
            println!("========================\n");
            return;
        }

        if use_aot {
            println!("=== Compiling Ahead-of-Time (AOT) ===");
            let source_path = std::path::Path::new(&path);
            let parent_dir = source_path.parent().unwrap_or(std::path::Path::new(""));
            let build_dir = parent_dir.join("build");
            if !build_dir.exists() {
                std::fs::create_dir_all(&build_dir).unwrap();
            }
        /*
            let out_path = build_dir.join("output.o").to_string_lossy().into_owned();

        let mut aot_backend = crate::backend::codegen::cranelift::aot::CraneliftAotBackend::new();
        aot_backend.compile_module(&ir_module);
        aot_backend.finalize(&out_path);
        */
        } else {
            let mut cl_backend = crate::backend::codegen::cranelift::CraneliftBackend::new();
            cl_backend.compile_module(&ir_module);
            cl_backend.finalize();
            println!("Cranelift JIT execution completed!");
        }
        return;
    }

    println!("\n=== Code Generation (C++) ===");
    let mut final_cpp = String::new();

    let mut header_gen = crate::backend::codegen::generator::CodeGenerator::new();
    final_cpp.push_str(&header_gen.generate(&vec![], true, false));

    for module in &program.modules {
        let cpp_namespace = module.name.replace("/", "_");
        final_cpp.push_str(&format!("namespace {} {{\n", cpp_namespace));
        let mut codegen = crate::backend::codegen::generator::CodeGenerator::new();
        let module_cpp = codegen.generate(&module.ast, false, false);
        final_cpp.push_str(&module_cpp);
        final_cpp.push_str(&format!("\n}} // namespace {}\n\n", cpp_namespace));
    }

    let mut main_codegen = crate::backend::codegen::generator::CodeGenerator::new();
    let main_cpp = main_codegen.generate(&program.main_ast, false, true);
    final_cpp.push_str(&main_cpp);

    let source_path = std::path::Path::new(&path);
    let parent_dir = source_path.parent().unwrap_or(std::path::Path::new(""));
    let build_dir = parent_dir.join("build");
    if !build_dir.exists() {
        std::fs::create_dir_all(&build_dir).unwrap();
    }

    let out_path = build_dir.join("output.cpp").to_string_lossy().into_owned();
    let exe_path = build_dir.join("app.exe").to_string_lossy().into_owned();
    fs::write(&out_path, &final_cpp).expect("Failed to write output.cpp");

    println!("Successfully generated C++ code to {}", out_path);
    println!("Compiling to {}...", exe_path);

    let status = std::process::Command::new("g++")
        .arg(&out_path)
        .arg("-o")
        .arg(&exe_path)
        .arg("-std=c++17")
        .status();

    match status {
        Ok(s) if s.success() => {
            println!("Compilation successful! Executable is {} 🚀", exe_path);
        }
        _ => {
            eprintln!("C++ compilation failed! Check {} for errors.", out_path);
        }
    }
}
