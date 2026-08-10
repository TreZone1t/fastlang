use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::rc::Rc;

use crate::lexer::scanner::Scanner;
use crate::lexer::token::TokenKind;
use crate::parser::ast::Stmt;
use crate::parser::parser::Parser;
use crate::semantic::analyzer::SemanticAnalyzer;
use crate::semantic::environment::Environment;

pub mod codegen;
pub mod lexer;
pub mod parser;
pub mod semantic;

fn parse_file(path: &str) -> Result<Vec<Stmt>, String> {
    let contents = fs::read_to_string(path)
        .unwrap_or_else(|_| panic!("Could not read '{}'. Make sure the file exists.", path));

    let mut scanner = Scanner::new(contents);
    let mut tokens = Vec::new();

    loop {
        let tok = scanner.next_token();
        let is_eof = tok.kind == TokenKind::EOF;

        if let TokenKind::Error(ref msg) = tok.kind {
            eprintln!("[Lexer Error in {}] {}", path, msg);
        }

        match tok.kind {
            TokenKind::InlineComment | TokenKind::MultiLineComment => {}
            _ => tokens.push(tok),
        }

        if is_eof {
            break;
        }
    }

    let mut parser = Parser::new(tokens);
    parser.parse_program().map(|program| program.statements)
}

fn main() {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "fast.fs".to_string());

    // Multi-file state
    let mut asts: HashMap<String, Vec<Stmt>> = HashMap::new();
    let mut envs: HashMap<String, Rc<RefCell<Environment>>> = HashMap::new();

    println!("Compiling {}...", path);
    let main_ast = match parse_file(&path) {
        Ok(ast) => {
            println!("AST Length: {}", ast.len());
            std::fs::write("ast_debug.txt", format!("{:#?}", ast)).unwrap();
            ast
        },
        Err(err) => {
            eprintln!("{}", err);
            std::process::exit(1);
        }
    };

    // Collect dependencies
    let mut deps = Vec::new();
    for stmt in &main_ast {
        if let Stmt::Use {
            module_path,
            imports,
        } = stmt
        {
            let mut path = module_path.clone();
            let mut mod_name = path.join("/");
            let mut actual_imports = imports.clone();

            let get_fs_path = |name: &str| -> String {
                if name.starts_with("std/") {
                    format!("src/{}.fs", name)
                } else {
                    format!("src/examples/{}.fs", name) // temporary fallback
                }
            };

            if !Path::new(&get_fs_path(&mod_name)).exists() && path.len() > 1 {
                let last = path.pop().unwrap();
                let parent_mod = path.join("/");
                if Path::new(&get_fs_path(&parent_mod)).exists() {
                    mod_name = parent_mod;
                    actual_imports = Some(vec![last]);
                }
            }

            deps.push((mod_name, actual_imports));
        }
    }

    // Parse and analyze dependencies
    for (mod_name, _) in &deps {
        if !asts.contains_key(mod_name) {
            let actual_path = if mod_name.starts_with("std/") {
                format!("src/{}.fs", mod_name)
            } else {
                let test = format!("src/examples/{}.fs", mod_name);
                if Path::new(&test).exists() {
                    test
                } else {
                    format!("{}.fs", mod_name)
                }
            };

            println!("Loading module {}...", mod_name);
            let mod_ast = match parse_file(&actual_path) {
                Ok(ast) => ast,
                Err(err) => {
                    eprintln!("{}", err);
                    std::process::exit(1);
                }
            };

            let mut analyzer = SemanticAnalyzer::new();
            if let Err(e) = analyzer.analyze(&mod_ast) {
                eprintln!("Semantic Error in module {}: {}", mod_name, e);
                std::process::exit(1);
            }

            envs.insert(mod_name.clone(), analyzer.current_env);
            asts.insert(mod_name.clone(), mod_ast);
        }
    }

    // Analyze main file
    println!("\n=== Semantic Analysis ===");
    let mut main_analyzer = SemanticAnalyzer::new();

    // Inject exported symbols
    for (mod_name, imports) in &deps {
        if let Some(env) = envs.get(mod_name) {
            let symbols = env.borrow().symbols.clone();
            for (sym_name, info) in symbols {
                if info.visibility == crate::parser::ast::Visibility::Public {
                    // Check if selective imports are used
                    let should_inject = match imports {
                        Some(selected) => selected.contains(&sym_name),
                        None => true, // inject all if no curly braces used
                    };

                    if should_inject {
                        println!("Module {} injected symbol: {}", mod_name, sym_name);
                        main_analyzer
                            .current_env
                            .borrow_mut()
                            .define(sym_name, info)
                            .ok(); // ignore if already defined (e.g. stdlib)
                    }
                }
            }
        } else {
            eprintln!("Error: Module {} not found.", mod_name);
            std::process::exit(1);
        }
    }

    match main_analyzer.analyze(&main_ast) {
        Ok(_) => println!("Semantic Analysis Passed successfully! ✅"),
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }

    // Code Generation
    println!("\n=== Code Generation (C++) ===");
    let mut final_cpp = String::new();

    // Generate headers once
    let mut header_gen = crate::codegen::generator::CodeGenerator::new();
    final_cpp.push_str(&header_gen.generate(&vec![], true, false));

    // Generate modules
    for (name, ast) in &asts {
        let cpp_namespace = name.replace("/", "_");
        final_cpp.push_str(&format!("namespace {} {{\n", cpp_namespace));
        let mut codegen = crate::codegen::generator::CodeGenerator::new();
        let module_cpp = codegen.generate(ast, false, false);
        final_cpp.push_str(&module_cpp);
        final_cpp.push_str(&format!("\n}} // namespace {}\n\n", cpp_namespace));
    }

    // Generate main
    let mut main_codegen = crate::codegen::generator::CodeGenerator::new();
    let main_cpp = main_codegen.generate(&main_ast, false, true);
    final_cpp.push_str(&main_cpp);

    let out_path = "output.cpp";
    fs::write(out_path, &final_cpp).expect("Failed to write output.cpp");

    println!("Successfully generated C++ code to {}", out_path);
    println!("Compiling to app.exe...");

    let status = std::process::Command::new("g++")
        .arg(out_path)
        .arg("-o")
        .arg("app.exe")
        .arg("-std=c++20")
        .status();

    match status {
        Ok(s) if s.success() => {
            println!("Compilation successful! Executable is app.exe 🚀");
        }
        _ => {
            eprintln!("C++ compilation failed! Check output.cpp for errors.");
        }
    }
}
