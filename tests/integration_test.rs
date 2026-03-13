use std::process::Command;

fn cli_binary() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ncu-cli"))
}

// ---------------------------------------------------------------------------
// Backward compatibility: `ncu-cli --input <path>` still works
// ---------------------------------------------------------------------------

#[test]
fn test_legacy_input_flag_runs_successfully() {
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

// ---------------------------------------------------------------------------
// Analyze subcommand
// ---------------------------------------------------------------------------

#[test]
fn test_analyze_subcommand() {
    let output = cli_binary()
        .args(["analyze", "exp_cute_csnippets.csv"])
        .output()
        .expect("failed to execute ncu-cli");

    assert!(output.status.success(), "analyze subcommand failed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Kernel:"));
    assert!(stdout.contains("Memory Bound"));
}

#[test]
fn test_analyze_json_format() {
    let output = cli_binary()
        .args(["analyze", "exp_cute_csnippets.csv", "--format", "json"])
        .output()
        .expect("failed to execute ncu-cli");

    assert!(output.status.success(), "json format failed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .expect("output should be valid JSON");
    assert!(parsed["bottleneck"].as_str().unwrap().contains("Memory Bound"));
    assert!(parsed["kernel"]["device_name"].as_str().unwrap().contains("H800"));
}

#[test]
fn test_analyze_markdown_format() {
    let output = cli_binary()
        .args(["analyze", "exp_cute_csnippets.csv", "--format", "markdown"])
        .output()
        .expect("failed to execute ncu-cli");

    assert!(output.status.success(), "markdown format failed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("# Kernel Analysis"));
    assert!(stdout.contains("Memory Bound"));
}

#[test]
fn test_analyze_csv_format() {
    let output = cli_binary()
        .args(["analyze", "exp_cute_csnippets.csv", "--format", "csv"])
        .output()
        .expect("failed to execute ncu-cli");

    assert!(output.status.success(), "csv format failed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("kernel_name,"));
    assert!(stdout.contains("Memory Bound"));
}

// ---------------------------------------------------------------------------
// Info subcommand
// ---------------------------------------------------------------------------

#[test]
fn test_info_subcommand() {
    let output = cli_binary()
        .args(["info", "exp_cute_csnippets.csv"])
        .output()
        .expect("failed to execute ncu-cli");

    assert!(output.status.success(), "info subcommand failed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Profile Info"));
    assert!(stdout.contains("NVIDIA H800"));
    assert!(stdout.contains("Kernels:"));
}

#[test]
fn test_info_json() {
    let output = cli_binary()
        .args(["info", "exp_cute_csnippets.csv", "--format", "json"])
        .output()
        .expect("failed to execute ncu-cli");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .expect("output should be valid JSON");
    assert_eq!(parsed["kernel_count"].as_u64().unwrap(), 1);
    assert!(parsed["device"].as_str().unwrap().contains("H800"));
}

// ---------------------------------------------------------------------------
// Summary subcommand
// ---------------------------------------------------------------------------

#[test]
fn test_summary_subcommand() {
    let output = cli_binary()
        .args(["summary", "exp_cute_csnippets.csv"])
        .output()
        .expect("failed to execute ncu-cli");

    assert!(output.status.success(), "summary subcommand failed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Kernel Summary"));
}

#[test]
fn test_summary_json() {
    let output = cli_binary()
        .args(["summary", "exp_cute_csnippets.csv", "--format", "json"])
        .output()
        .expect("failed to execute ncu-cli");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .expect("output should be valid JSON");
    assert!(parsed.as_array().unwrap().len() >= 1);
}

// ---------------------------------------------------------------------------
// Skill subcommand
// ---------------------------------------------------------------------------

#[test]
fn test_skill_list() {
    let output = cli_binary()
        .args(["skill", "list"])
        .output()
        .expect("failed to execute ncu-cli");

    assert!(output.status.success(), "skill list failed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("roofline"));
    assert!(stdout.contains("memory"));
    assert!(stdout.contains("occupancy"));
    assert!(stdout.contains("instruction"));
    assert!(stdout.contains("arch"));
}

#[test]
fn test_skill_run_roofline() {
    let output = cli_binary()
        .args(["skill", "run", "roofline", "exp_cute_csnippets.csv"])
        .output()
        .expect("failed to execute ncu-cli");

    assert!(output.status.success(), "skill run roofline failed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Memory Bound") || stdout.contains("Compute Bound"));
}

#[test]
fn test_skill_run_unknown() {
    let output = cli_binary()
        .args(["skill", "run", "nonexistent", "exp_cute_csnippets.csv"])
        .output()
        .expect("failed to execute ncu-cli");

    assert!(!output.status.success(), "should fail on unknown skill");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Unknown skill"));
}

// ---------------------------------------------------------------------------
// Diff subcommand
// ---------------------------------------------------------------------------

#[test]
fn test_diff_same_file() {
    let output = cli_binary()
        .args(["diff", "exp_cute_csnippets.csv", "exp_cute_csnippets.csv"])
        .output()
        .expect("failed to execute ncu-cli");

    assert!(output.status.success(), "diff subcommand failed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Profile Diff"));
    assert!(stdout.contains("No differences found."));
}

#[test]
fn test_diff_json() {
    let output = cli_binary()
        .args(["diff", "exp_cute_csnippets.csv", "exp_cute_csnippets.csv", "--format", "json"])
        .output()
        .expect("failed to execute ncu-cli");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .expect("output should be valid JSON");
    assert!(parsed["regressions"].as_array().unwrap().is_empty());
    assert!(parsed["improvements"].as_array().unwrap().is_empty());
}

// ---------------------------------------------------------------------------
// Export subcommand
// ---------------------------------------------------------------------------

#[test]
fn test_export_json() {
    let output = cli_binary()
        .args(["export", "exp_cute_csnippets.csv", "--format", "json"])
        .output()
        .expect("failed to execute ncu-cli");

    assert!(output.status.success(), "export json failed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .expect("output should be valid JSON");
    assert!(parsed.as_array().unwrap().len() >= 1);
}

#[test]
fn test_export_csv() {
    let output = cli_binary()
        .args(["export", "exp_cute_csnippets.csv", "--format", "csv"])
        .output()
        .expect("failed to execute ncu-cli");

    assert!(output.status.success(), "export csv failed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("kernel_name,"));
}

// ---------------------------------------------------------------------------
// Error handling
// ---------------------------------------------------------------------------

#[test]
fn test_missing_file_returns_error() {
    let output = cli_binary()
        .args(["analyze", "nonexistent.csv"])
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

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");
    assert!(
        combined.contains("Usage") || combined.contains("analyze") || combined.contains("ncu-cli"),
        "should show usage info, got: {combined}"
    );
}
