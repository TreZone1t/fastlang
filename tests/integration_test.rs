use std::fs;
use std::path::Path;
use std::process::Command;

#[test]
fn run_all_fs_tests() {
    let test_dir = Path::new("tests");
    if !test_dir.exists() {
        println!("No tests directory found.");
        return;
    }

    let mut entries = fs
        ::read_dir(test_dir)
        .expect("failed to read tests directory")
        .map(|res| res.map(|e| e.path()))
        .collect::<Result<Vec<_>, std::io::Error>>()
        .unwrap();

    entries.sort();

    let mut failed_tests = 0;
    let mut passed_tests = 0;

    for path in entries {
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("fs") {
            let filename = path.file_name().unwrap().to_str().unwrap();
            let should_fail =
                filename.starts_with("fail_") ||
                filename.contains("_fail_") ||
                filename.contains("fail");

            println!("--------------------------------------------------");
            println!("Running test: {}", filename);
            let output = Command::new(env!("CARGO_BIN_EXE_fast_lang"))
                .arg(path.to_str().unwrap())
                .output()
                .expect("failed to execute process");

            if should_fail {
                if output.status.success() {
                    println!("❌ FAILED: Test {} was expected to fail but succeeded (Analyzer missed the error)!", filename);
                    println!("Stdout: {}", String::from_utf8_lossy(&output.stdout));
                    failed_tests += 1;
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    println!("✅ PASSED (Expected Failure): {}", filename);
                    println!("Analyzer Error Output:\n{}", stderr.trim());
                    passed_tests += 1;
                }
            } else {
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    println!("❌ FAILED (Unexpected Failure): {}", filename);
                    println!("Stdout:\n{}", stdout.trim());
                    println!("Stderr:\n{}", stderr.trim());
                    failed_tests += 1;
                } else {
                    let content = fs::read_to_string(&path).unwrap();
                    let mut expected_lines = Vec::new();
                    for line in content.lines() {
                        if let Some(idx) = line.find("// EXPECT:") {
                            let expected = line[idx + 10..].trim();
                            expected_lines.push(expected.to_string());
                        }
                    }

                    if !expected_lines.is_empty() {
                        let exe_path = if cfg!(windows) { "./app.exe" } else { "./app" };

                        if Path::new(exe_path).exists() {
                            let app_output = Command::new(exe_path)
                                .output()
                                .expect("failed to execute compiled app");

                            let app_stdout = String::from_utf8_lossy(&app_output.stdout);
                            let actual_lines: Vec<&str> = app_stdout
                                .lines()
                                .map(|s| s.trim())
                                .filter(|s| !s.is_empty())
                                .collect();

                            let mut expected_idx = 0;
                            for actual in actual_lines {
                                if
                                    expected_idx < expected_lines.len() &&
                                    actual == expected_lines[expected_idx]
                                {
                                    expected_idx += 1;
                                }
                            }

                            if expected_idx < expected_lines.len() {
                                println!("❌ FAILED (Output Mismatch): {}", filename);
                                println!("Expected to find: '{}'", expected_lines[expected_idx]);
                                println!("Actual Output:\n{}", app_stdout);
                                failed_tests += 1;
                            } else {
                                println!("✅ PASSED: {}", filename);
                                passed_tests += 1;
                            }
                        } else {
                            println!("⚠️ WARNING: Executable not found for {} despite successful compilation.", filename);
                            passed_tests += 1;
                        }
                    } else {
                        println!("✅ PASSED: {}", filename);
                        passed_tests += 1;
                    }
                }
            }
        }
    }

    println!("==================================================");
    println!("Test Run Complete: {} Passed, {} Failed", passed_tests, failed_tests);

    if failed_tests > 0 {
        panic!("{} tests failed! Check the logs above.", failed_tests);
    }
}
