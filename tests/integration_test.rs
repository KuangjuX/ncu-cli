use std::process::Command;

fn cli_binary() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ncu-cli"))
}

#[test]
fn test_sample_csv_runs_successfully() {
    let output = cli_binary()
        .args(["--input", "exp_cute_csnippets.csv"])
        .output()
        .expect("failed to execute ncu-cli");

    assert!(output.status.success(), "ncu-cli exited with error");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Kernel:"), "missing kernel header");
    assert!(stdout.contains("SM_90"), "missing arch detection");
    assert!(stdout.contains("NVIDIA H800"), "missing device name");
    assert!(stdout.contains("Memory Bound"), "missing bottleneck classification");
    assert!(stdout.contains("Metrics Overview"), "missing metrics table");
    assert!(stdout.contains("Analysis & Suggestions"), "missing findings section");
}

#[test]
fn test_missing_file_returns_error() {
    let output = cli_binary()
        .args(["--input", "nonexistent.csv"])
        .output()
        .expect("failed to execute ncu-cli");

    assert!(!output.status.success(), "should fail on missing file");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Failed") || stderr.contains("Error") || stderr.contains("error"),
        "should report an error message, got: {stderr}"
    );
}

#[test]
fn test_no_args_shows_help() {
    let output = cli_binary()
        .output()
        .expect("failed to execute ncu-cli");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--input") || stderr.contains("Usage"),
        "should show usage info"
    );
}
