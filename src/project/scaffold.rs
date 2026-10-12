use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use super::manifest::{FastManifest, ProjectType};

pub fn create_new_project(
    raw_name: &str,
    target_parent: &Path,
    cli_type: Option<ProjectType>,
    non_interactive: bool,
) -> Result<PathBuf, String> {
    let (project_name, project_dir) = if raw_name == "." {
        let current = std::env::current_dir().map_err(|e| e.to_string())?;
        let name = current
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("my_project")
            .to_string();
        (name, current)
    } else {
        let name = raw_name.to_string();
        let dir = target_parent.join(&name);
        (name, dir)
    };

    let p_type = if let Some(t) = cli_type {
        t
    } else if non_interactive {
        ProjectType::Bin
    } else {
        prompt_project_type()
    };

    if project_dir.exists() && raw_name != "." {
        return Err(format!(
            "Directory '{}' already exists. Use 'fast init' inside it instead.",
            project_dir.display()
        ));
    }

    fs::create_dir_all(&project_dir)
        .map_err(|e| format!("Failed to create project directory: {}", e))?;

    init_project_in_dir(&project_dir, project_name, p_type)?;

    Ok(project_dir)
}

pub fn init_project_in_dir(
    dir: &Path,
    project_name: String,
    project_type: ProjectType,
) -> Result<(), String> {
    let manifest_path = dir.join(FastManifest::MANIFEST_FILENAME);
    if manifest_path.exists() {
        return Err(format!(
            "Project manifest '{}' already exists.",
            manifest_path.display()
        ));
    }

    let manifest = FastManifest::new(project_name.clone(), project_type);
    manifest.save_to_file(&manifest_path)?;

    // Create src/ folder
    let src_dir = dir.join("src");
    fs::create_dir_all(&src_dir).map_err(|e| e.to_string())?;

    // Create entry point file
    match project_type {
        ProjectType::Bin => {
            let main_path = src_dir.join("main.fast");
            if !main_path.exists() {
                let main_code = format!(
                    r#"// {} - FastLang Application
import std::fmt;

fn main() -> int32 {{
    println("Hello from {}!");
    return 0;
}}
"#,
                    project_name, project_name
                );
                fs::write(&main_path, main_code).map_err(|e| e.to_string())?;
            }
        }
        ProjectType::Module => {
            let lib_path = src_dir.join("lib.fast");
            if !lib_path.exists() {
                let lib_code = format!(
                    r#"// {} - FastLang Module/Library
import std::fmt;

public fn info() -> str {{
    return "{} library loaded successfully";
}}

public fn add(a: int32, b: int32) -> int32 {{
    return a + b;
}}
"#,
                    project_name, project_name
                );
                fs::write(&lib_path, lib_code).map_err(|e| e.to_string())?;
            }
        }
    }

    // Create tests/ directory only for module/library projects
    if project_type == ProjectType::Module {
        let tests_dir = dir.join("tests");
        if !tests_dir.exists() {
            let _ = fs::create_dir_all(&tests_dir);
            let sample_test = tests_dir.join("test_basic.fast");
            if !sample_test.exists() {
                let test_code = format!(
                    r#"// Test suite for {} library
import std::fmt;

fn main() -> int32 {{
    println("Testing {} library... PASSED");
    return 0;
}}
"#,
                    project_name, project_name
                );
                let _ = fs::write(&sample_test, test_code);
            }
        }
    }

    // Create packages/ directory
    let packages_dir = dir.join("packages");
    if !packages_dir.exists() {
        let _ = fs::create_dir_all(&packages_dir);
    }

    // Create .gitignore
    let gitignore_path = dir.join(".gitignore");
    if !gitignore_path.exists() {
        let gitignore_content = r#"# FastLang Build Artifacts
build/
*.exe
*.o
*.obj
*.pdb
*.cpp
# Packages cache (optional)
# packages/
"#;
        let _ = fs::write(&gitignore_path, gitignore_content);
    }

    Ok(())
}

fn prompt_project_type() -> ProjectType {
    print!("Select project type [1: Binary (default), 2: Module/Library]: ");
    let _ = io::stdout().flush();
    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_ok() {
        let trimmed = input.trim();
        if trimmed == "2" || trimmed.eq_ignore_ascii_case("lib") || trimmed.eq_ignore_ascii_case("module") {
            return ProjectType::Module;
        }
    }
    ProjectType::Bin
}
