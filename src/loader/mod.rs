use std::collections::{HashMap, HashSet};
use std::path::Path;
use crate::frontend::parser::ast::{Decl, Stmt};
use crate::frontend::parser::ast::TypeMetadata;

pub struct LoadedModule {
    pub name: String,
    pub path: String,
    pub ast: Vec<Stmt>,
}

pub struct Program {
    pub main_ast: Vec<Stmt>,
    pub modules: Vec<LoadedModule>,
    pub global_metadata: HashMap<String, TypeMetadata>,
    pub main_deps: Vec<(String, Option<Vec<String>>)>,
}

pub struct ProjectLoader {
    pub include_paths: Vec<String>,
}

impl ProjectLoader {
    pub fn new() -> Self {
        ProjectLoader {
            include_paths: vec![
                "src/".to_string(),
                "src/examples/".to_string(),
                "src/std/".to_string(),
            ],
        }
    }

    pub fn resolve_path(&self, mod_name: &str) -> Option<String> {
        if mod_name.starts_with("std/") {
            return Some(format!("src/{}.fs", mod_name));
        } else if mod_name == "std" {
            return Some("src/std/std.fs".to_string());
        }
        
        for path in &self.include_paths {
            let test = format!("{}{}.fs", path, mod_name);
            if Path::new(&test).exists() {
                return Some(test);
            }
        }
        
        let local_test = format!("{}.fs", mod_name);
        if Path::new(&local_test).exists() {
            return Some(local_test);
        }
        None
    }

    fn collect_deps_from_ast(ast: &[Stmt]) -> Vec<(String, Option<Vec<String>>)> {
        let mut deps = Vec::new();
        for stmt in ast {
            if let Stmt::Declaration(Decl::Import {
                module_path,
                imports,
            }) = stmt
            {
                let mut path_clone = module_path.clone();
                let mut mod_name = path_clone.join("/");
                let mut actual_imports = imports.clone();

                if let Some(selected) = imports {
                    for sym in selected {
                        let sub_mod = format!("{}/{}", mod_name, sym);
                        if ProjectLoader::resolve_path_static(&sub_mod).is_some() {
                            deps.push((sub_mod, Some(vec![sym.clone()])));
                        }
                    }
                }

                if ProjectLoader::resolve_path_static(&mod_name).is_none() && path_clone.len() > 1 {
                    let last = path_clone.pop().unwrap();
                    let parent_mod = path_clone.join("/");
                    if ProjectLoader::resolve_path_static(&parent_mod).is_some() {
                        mod_name = parent_mod;
                        actual_imports = Some(vec![last]);
                    }
                }

                if ProjectLoader::resolve_path_static(&mod_name).is_some() {
                    deps.push((mod_name, actual_imports));
                }
            }
        }
        deps
    }

    fn resolve_path_static(mod_name: &str) -> Option<String> {
        if mod_name.starts_with("std/") {
            return Some(format!("src/{}.fs", mod_name));
        } else if mod_name == "std" {
            return Some("src/std/std.fs".to_string());
        }

        for path in &["src/", "src/examples/", "src/std/"] {
            let test = format!("{}{}.fs", path, mod_name);
            if Path::new(&test).exists() {
                return Some(test);
            }
        }

        let local_test = format!("{}.fs", mod_name);
        if Path::new(&local_test).exists() {
            return Some(local_test);
        }
        None
    }

    fn resolve_import_deps(
        &self,
        deps: &[(String, Option<Vec<String>>)],
        loaded_names: &mut HashSet<String>,
        loaded_modules: &mut Vec<LoadedModule>,
        global_metadata: &mut HashMap<String, TypeMetadata>,
    ) -> Result<(), String> {
        let mut pending: Vec<(String, Option<Vec<String>>)> = deps.to_vec();

        while let Some((mod_name, _imports)) = pending.pop() {
            if loaded_names.contains(&mod_name) {
                continue;
            }

            let actual_path = self.resolve_path(&mod_name)
                .ok_or_else(|| format!("Module '{}' not found.", mod_name))?;

            println!("Loading module {}...", mod_name);
            let mod_ast = self.parse_file(&actual_path)?;
            global_metadata.extend(mod_ast.1.clone());

            let nested_deps = Self::collect_deps_from_ast(&mod_ast.0);
            pending.extend(nested_deps);

            loaded_modules.push(LoadedModule {
                name: mod_name.clone(),
                path: actual_path,
                ast: mod_ast.0,
            });
            loaded_names.insert(mod_name);
        }

        Ok(())
    }

    fn parse_file(&self, path: &str) -> Result<(Vec<Stmt>, HashMap<String, TypeMetadata>), String> {
        let contents = std::fs::read_to_string(path)
            .unwrap_or_else(|_| panic!("Could not read '{}'. Make sure the file exists.", path));
    
        let mut scanner = crate::frontend::lexer::scanner::Scanner::new(contents.clone());
        let mut tokens = Vec::new();
    
        loop {
            let tok = scanner.next_token();
            let is_eof = tok.kind == crate::frontend::lexer::token::TokenKind::EOF;
    
            if let crate::frontend::lexer::token::TokenKind::Error(ref msg) = tok.kind {
                eprintln!("[Lexer Error in {}] {}", path, msg);
            }
    
            match tok.kind {
                crate::frontend::lexer::token::TokenKind::InlineComment | crate::frontend::lexer::token::TokenKind::MultiLineComment => {}
                _ => tokens.push(tok),
            }
    
            if is_eof {
                break;
            }
        }
    
        let mut parser = crate::frontend::parser::parser::Parser::new(tokens);
    
        match parser.parse_program() {
            Ok(program) => Ok((program.statements, parser.metadata.clone())),
            Err(err_msg) => {
                let mut line = 1;
                let mut column = 1;
                let mut clean_msg = err_msg.clone();
    
                if let Some(line_idx) = err_msg.find("at line ") {
                    let after_line = &err_msg[line_idx + 8..];
                    if let Some(comma_idx) = after_line.find(',') {
                        if let Ok(l) = after_line[..comma_idx].parse::<usize>() {
                            line = l;
                        }
                        if let Some(col_idx) = after_line.find("column ") {
                            let after_col = &after_line[col_idx + 7..];
                            let num_str = after_col.trim_matches(|c: char| !c.is_ascii_digit());
                            if let Ok(c) = num_str.parse::<usize>() {
                                column = c;
                            }
                        }
                    }
                    clean_msg = err_msg[..line_idx].trim().to_string();
                }
    
                crate::report_visual_error(&contents, line, column, &clean_msg);
    
                Err(err_msg)
            }
        }
    }

    pub fn load(&self, path: &str, target_module: Option<&str>) -> Result<Program, String> {
        let entry_path = if let Some(target) = target_module {
            self.resolve_path(target).ok_or_else(|| format!("Target module '{}' not found in include paths.", target))?
        } else {
            path.to_string()
        };

        if !Path::new(&entry_path).exists() {
            return Err(format!("Entry file '{}' not found.", entry_path));
        }

        println!("Compiling {}...", entry_path);
        let mut global_metadata = HashMap::new();
        let main_ast = match self.parse_file(&entry_path) {
            Ok(res) => {
                global_metadata.extend(res.1);
                res.0
            }
            Err(e) => return Err(e),
        };

        let mut deps = Self::collect_deps_from_ast(&main_ast);

        let mut loaded_modules = Vec::new();
        let mut loaded_names = HashSet::new();
        self.resolve_import_deps(&deps, &mut loaded_names, &mut loaded_modules, &mut global_metadata)?;

        Ok(Program {
            main_ast,
            modules: loaded_modules,
            global_metadata,
            main_deps: deps,
        })
    }
}
