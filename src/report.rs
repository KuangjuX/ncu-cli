use colored::Colorize;
use comfy_table::{modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, Cell, Table};

use crate::analyzer::arch::arch_display_name;
use crate::analyzer::roofline;
use crate::metrics::KernelData;
use crate::severity::{Finding, Severity};

pub fn print_report(data: &KernelData, findings: &[Finding]) {
    println!();
    print_header(data);
    println!();
    print_metrics_table(data);
    println!();
    print_findings(findings);
    println!();
}

fn print_header(data: &KernelData) {
    let bottleneck = roofline::classify(data);
    let arch_name = arch_display_name(data.arch_sm);
    let sm_label = if data.arch_sm > 0 {
        format!("SM_{} ({})", data.arch_sm, arch_name)
    } else {
        "Unknown".into()
    };

    println!("{}", "═".repeat(72).dimmed());
    println!(
        "  {} {}",
        "Kernel:".bold(),
        truncate_name(&data.kernel_name, 58)
    );
    println!("  {}  {}", "Arch:".bold(), sm_label);
    println!("  {} {}", "Device:".bold(), data.device_name);
    println!(
        "  {} {:.2} us",
        "Duration:".bold(),
        data.duration_us
    );
    println!(
        "  {} {}",
        "Main Bottleneck:".bold(),
        format!("{bottleneck}").yellow().bold()
    );
    println!("{}", "═".repeat(72).dimmed());
}

fn print_metrics_table(data: &KernelData) {
    println!("  {}", "[Metrics Overview]".bold());

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS);

    table.set_header(vec![
        Cell::new("Metric"),
        Cell::new("Value"),
        Cell::new("Status"),
    ]);

    let sm = data.sm_throughput_pct;
    let mem = data.mem_throughput_pct;
    let occ = data.warps_active_pct;

    table.add_row(vec![
        Cell::new("SM Throughput"),
        Cell::new(format!("{sm:.1}%")),
        Cell::new(level_indicator(sm, 60.0, 40.0)),
    ]);
    table.add_row(vec![
        Cell::new("Memory Throughput"),
        Cell::new(format!("{mem:.1}%")),
        Cell::new(level_indicator(mem, 60.0, 40.0)),
    ]);
    table.add_row(vec![
        Cell::new("Occupancy (Active Warps)"),
        Cell::new(format!("{occ:.1}%")),
        Cell::new(level_indicator(occ, 50.0, 25.0)),
    ]);
    table.add_row(vec![
        Cell::new("L1 Cache Hit Rate"),
        Cell::new(format!("{:.1}%", data.l1_hit_rate_pct)),
        Cell::new(level_indicator(data.l1_hit_rate_pct, 50.0, 20.0)),
    ]);
    table.add_row(vec![
        Cell::new("L2 Cache Hit Rate"),
        Cell::new(format!("{:.1}%", data.l2_hit_rate_pct)),
        Cell::new(level_indicator(data.l2_hit_rate_pct, 70.0, 50.0)),
    ]);
    table.add_row(vec![
        Cell::new("Tensor Core (HMMA)"),
        Cell::new(format!("{:.1}%", data.tensor_core_hmma_pct)),
        Cell::new("--"),
    ]);
    table.add_row(vec![
        Cell::new("Grid Size"),
        Cell::new(&data.grid_size),
        Cell::new("--"),
    ]);
    table.add_row(vec![
        Cell::new("Block Size"),
        Cell::new(&data.block_size),
        Cell::new("--"),
    ]);

    // Indent each line of the table
    for line in table.to_string().lines() {
        println!("  {line}");
    }
}

fn print_findings(findings: &[Finding]) {
    println!("  {}", "[Analysis & Suggestions]".bold());
    println!();

    if findings.is_empty() {
        println!("  No issues detected.");
        return;
    }

    for (i, f) in findings.iter().enumerate() {
        let num = i + 1;
        let title = match f.severity {
            Severity::Info => format!("[{}] {}", f.severity, f.title).cyan().to_string(),
            Severity::Warning => format!("[{}] {}", f.severity, f.title)
                .yellow()
                .to_string(),
            Severity::Critical => format!("[{}] {}", f.severity, f.title)
                .red()
                .bold()
                .to_string(),
        };
        println!("  {num}. {title}");
        println!("     Detail: {}", f.detail);
        println!("     Action: {}", f.action);
        println!();
    }
}

fn level_indicator(value: f64, good: f64, bad: f64) -> &'static str {
    if value >= good {
        "OK"
    } else if value >= bad {
        "Low"
    } else {
        "Very Low"
    }
}

fn truncate_name(name: &str, max_len: usize) -> String {
    if name.len() <= max_len {
        name.to_string()
    } else {
        format!("{}...", &name[..max_len - 3])
    }
}
