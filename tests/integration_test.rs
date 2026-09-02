use std::fs;
use std::path::Path;
use std::process::Command;

#[test]
fn run_all_fs_tests() {
    let backend = std::env::var("FASTLANG_BACKEND").unwrap_or_else(|_| "cpp".to_string());
    run_tests_with_backend(&backend);
}

#[test]
fn run_cranelift_aot_tests() {
    run_tests_with_backend("cranelift");
}

fn run_tests_with_backend(backend: &str) {
    let test_dir = Path::new("tests");
    if !test_dir.exists() {
        println!("No tests directory found.");
        return;
    }

    let mut entries = fs::read_dir(test_dir)
        .expect("failed to read tests directory")
        .map(|res| res.map(|e| e.path()))
        .collect::<Result<Vec<_>, std::io::Error>>()
        .unwrap();

    entries.sort();

    use std::sync::{Arc, Mutex};
    use std::thread;

    let passed_tests = Arc::new(Mutex::new(0));
    let failed_tests = Arc::new(Mutex::new(0));
    let logs = Arc::new(Mutex::new(String::new()));

    // Limit concurrency to chunks of 8.
    for chunk in entries.chunks(8) {
        let mut chunk_handles = vec![];
        for path in chunk {
            let path = path.clone();
            let ext = path.extension().and_then(|s| s.to_str());
            if !path.is_file() || (ext != Some("fast") && ext != Some("fs")) {
                continue;
            }

            let backend = backend.to_string();
            let passed = Arc::clone(&passed_tests);
            let failed = Arc::clone(&failed_tests);
            let logs_arc = Arc::clone(&logs);

            chunk_handles.push(thread::spawn(move || {
                let filename = path.file_name().unwrap().to_str().unwrap().to_string();
                let should_fail = filename.starts_with("fail_")
                    || filename.contains("_fail_")
                    || filename.contains("fail");

                let parent = path.parent().unwrap_or(Path::new(""));
                let build_dir = parent.join("build");
                let base_name = filename.strip_suffix(".fast").or_else(|| filename.strip_suffix(".fs")).unwrap_or(&filename);
                let build_exe = build_dir.join(if cfg!(windows) {
                    format!("{}.exe", base_name)
                } else {
                    format!("{}", base_name)
                });
                let build_cpp = build_dir.join(format!("{}.cpp", base_name));
                let _ = fs::remove_file(&build_exe);
                let _ = fs::remove_file(&build_cpp);

                let mut cmd = Command::new(env!("CARGO_BIN_EXE_fast_lang"));
                cmd.arg(path.to_str().unwrap());
                cmd.arg("-o").arg(build_exe.to_str().unwrap());
                if backend == "cranelift" || backend == "native" {
                    cmd.arg("--backend").arg("cranelift").arg("--aot");
                }

                let output = cmd.output().expect("failed to execute process");

                let mut local_log = String::new();
                let mut is_success = true;

                if should_fail {
                    if output.status.success() {
                        local_log.push_str(&format!(
                            "
❌ FAILED: Test {} was expected to fail but succeeded!
Stdout: {}
",
                            filename,
                            String::from_utf8_lossy(&output.stdout)
                        ));
                        is_success = false;
                    } else {
                        local_log.push_str(&format!(
                            "✅ PASSED (Expected Failure): {}
",
                            filename
                        ));
                    }
                } else {
                    if !output.status.success() {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        local_log.push_str(&format!(
                            "
❌ FAILED (Unexpected Failure): {}
Stdout:
{}
Stderr:
{}
",
                            filename,
                            stdout.trim(),
                            stderr.trim()
                        ));
                        is_success = false;
                    } else {
                        let content = fs::read_to_string(&path).unwrap();
                        let mut expected_lines = Vec::new();
                        for line in content.lines() {
                            if let Some(idx) = line.find("// EXPECT:") {
                                expected_lines.push(line[idx + 10..].trim().to_string());
                            }
                        }

                        if !build_exe.exists() {
                            local_log.push_str(&format!(
                                "
❌ FAILED (Executable Not Found): {} (expected {:?})
",
                                filename, build_exe
                            ));
                            is_success = false;
                        } else {
                            let app_output = Command::new(&build_exe)
                                .output()
                                .expect("failed to execute compiled app");
                            if !app_output.status.success() {
                                let app_stderr = String::from_utf8_lossy(&app_output.stderr);
                                let app_stdout = String::from_utf8_lossy(&app_output.stdout);
                                local_log.push_str(&format!(
                                    "
❌ FAILED (Execution Error): {} exited with code {:?}
Stdout:
{}
Stderr:
{}
",
                                    filename,
                                    app_output.status.code(),
                                    app_stdout.trim(),
                                    app_stderr.trim()
                                ));
                                is_success = false;
                            } else if !expected_lines.is_empty() {
                                let app_stdout = String::from_utf8_lossy(&app_output.stdout);
                                let actual_lines: Vec<&str> = app_stdout
                                    .lines()
                                    .map(|s| s.trim())
                                    .filter(|s| !s.is_empty())
                                    .collect();
                                let mut expected_idx = 0;
                                for actual in actual_lines {
                                    if expected_idx < expected_lines.len()
                                        && actual == expected_lines[expected_idx]
                                    {
                                        expected_idx += 1;
                                    }
                                }
                                if expected_idx < expected_lines.len() {
                                    local_log.push_str(&format!(
                                        "
❌ FAILED (Output Mismatch): {}
Expected to find: '{}'
Actual Output:
{}
",
                                        filename, expected_lines[expected_idx], app_stdout
                                    ));
                                    is_success = false;
                                } else {
                                    local_log.push_str(&format!(
                                        "✅ PASSED: {}
",
                                        filename
                                    ));
                                }
                            } else {
                                local_log.push_str(&format!(
                                    "✅ PASSED: {}
",
                                    filename
                                ));
                            }
                        }
                    }
                }

                let _ = fs::remove_file(&build_exe); // Cleanup
                let _ = fs::remove_file(&build_cpp); // Cleanup

                let mut logs_lock = logs_arc.lock().unwrap();
                logs_lock.push_str(&local_log);
                print!("{}", local_log);

                if is_success {
                    *passed.lock().unwrap() += 1;
                } else {
                    *failed.lock().unwrap() += 1;
                }
            }));
        }
        for h in chunk_handles {
            h.join().unwrap();
        }
    }

    let passed = *passed_tests.lock().unwrap();
    let failed = *failed_tests.lock().unwrap();

    println!("==================================================");
    println!("Test Run Complete: {} Passed, {} Failed", passed, failed);

    if failed > 0 {
        panic!("{} tests failed! Check the logs above.", failed);
    }
}
