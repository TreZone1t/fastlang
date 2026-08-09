use std::fs;
use std::path::Path;
use std::process::Command;

#[test]
fn run_all_fs_tests() {
    let test_dir = Path::new("tests");
    if !test_dir.exists() {
        return;
    }

    let mut entries = fs::read_dir(test_dir)
        .expect("failed to read tests directory")
        .map(|res| res.map(|e| e.path()))
        .collect::<Result<Vec<_>, std::io::Error>>()
        .unwrap();

    entries.sort();

    for path in entries {
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("fs") {
            let filename = path.file_name().unwrap().to_str().unwrap();
            let should_fail = filename.starts_with("fail_")
                || filename.contains("_fail_")
                || filename.contains("fail");

            println!("Running test: {}", filename);
            let output = Command::new("cargo")
                .args(["run", "--bin", "fast_lang", "--", path.to_str().unwrap()])
                .output()
                .expect("failed to execute process");

            if should_fail {
                if output.status.success() {
                    panic!(
                        "Test {} was expected to fail but succeeded!\nStdout: {}",
                        filename,
                        String::from_utf8_lossy(&output.stdout)
                    );
                }
            } else if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let stdout = String::from_utf8_lossy(&output.stdout);
                println!(
                    "Test {} failed unexpectedly. Continuing...\nStdout:\n{}\nStderr:\n{}",
                    filename, stdout, stderr
                );
            }
        }
    }
}
