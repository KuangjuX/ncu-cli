use std::io::Write;

use colored::Colorize;
use comfy_table::{modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, Cell, Table};

use crate::analyzer::arch::arch_display_name;
use crate::analyzer::roofline;
use crate::metrics::KernelData;
use crate::severity::{Finding, Severity};

#[allow(dead_code)]
pub fn print_report(data: &KernelData, findings: &[Finding]) {
    let mut stdout = std::io::stdout().lock();
    let _ = write_report(&mut stdout, data, findings);
}

pub fn write_report(w: &mut dyn Write, data: &KernelData, findings: &[Finding]) -> anyhow::Result<()> {
    writeln!(w)?;
    write_header(w, data)?;
    writeln!(w)?;
    write_metrics_table(w, data)?;
    writeln!(w)?;
    write_stall_summary(w, data)?;
    write_optimization_priorities(w, findings)?;
    write_findings(w, findings)?;
    writeln!(w)?;
    Ok(())
}

fn write_header(w: &mut dyn Write, data: &KernelData) -> anyhow::Result<()> {
    let bottleneck = roofline::classify(data);
    let arch_name = arch_display_name(data.arch_sm);
    let sm_label = if data.arch_sm > 0 {
        format!("SM_{} ({})", data.arch_sm, arch_name)
    } else {
        "Unknown".into()
    };

    writeln!(w, "{}", "═".repeat(72).dimmed())?;
    writeln!(w, "  {} {}", "Kernel:".bold(), truncate_name(&data.kernel_name, 58))?;
    writeln!(w, "  {}  {}", "Arch:".bold(), sm_label)?;
    writeln!(w, "  {} {}", "Device:".bold(), data.device_name)?;
    writeln!(w, "  {} {:.2} us", "Duration:".bold(), data.duration_us)?;
    writeln!(
        w,
        "  {} {}",
        "Main Bottleneck:".bold(),
        format!("{bottleneck}").yellow().bold()
    )?;
    writeln!(w, "{}", "═".repeat(72).dimmed())?;
    Ok(())
}

fn write_metrics_table(w: &mut dyn Write, data: &KernelData) -> anyhow::Result<()> {
    writeln!(w, "  {}", "[Metrics Overview]".bold())?;

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
        Cell::new("DRAM Throughput"),
        Cell::new(format!("{:.1}%", data.dram_throughput_pct)),
        Cell::new(level_indicator(data.dram_throughput_pct, 60.0, 40.0)),
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
        Cell::new("Registers / Thread"),
        Cell::new(if data.registers_per_thread > 0.0 {
            format!("{:.0}", data.registers_per_thread)
        } else {
            "--".into()
        }),
        Cell::new(if data.registers_per_thread >= 128.0 {
            "Very High"
        } else if data.registers_per_thread >= 64.0 {
            "High"
        } else {
            "OK"
        }),
    ]);
    table.add_row(vec![
        Cell::new("Shared Mem / Block"),
        Cell::new(if data.shared_mem_per_block_kb > 0.0 {
            format!("{:.1} KB", data.shared_mem_per_block_kb)
        } else {
            "--".into()
        }),
        Cell::new("--"),
    ]);
    table.add_row(vec![
        Cell::new("Bank Conflicts"),
        Cell::new(if data.shared_mem_bank_conflicts > 0.0 {
            format!("{:.0}", data.shared_mem_bank_conflicts)
        } else {
            "0".into()
        }),
        Cell::new(if data.shared_mem_bank_conflicts > 1_000_000.0 {
            "Critical"
        } else if data.shared_mem_bank_conflicts > 100_000.0 {
            "High"
        } else {
            "OK"
        }),
    ]);
    table.add_row(vec![
        Cell::new("Eligible Warps / Cycle"),
        Cell::new(if data.warps_eligible_per_cycle > 0.0 {
            format!("{:.2}", data.warps_eligible_per_cycle)
        } else {
            "--".into()
        }),
        Cell::new(if data.warps_eligible_per_cycle >= 2.0 {
            "OK"
        } else if data.warps_eligible_per_cycle >= 1.0 {
            "Low"
        } else if data.warps_eligible_per_cycle > 0.0 {
            "Very Low"
        } else {
            "--"
        }),
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

    for line in table.to_string().lines() {
        writeln!(w, "  {line}")?;
    }
    Ok(())
}

fn write_stall_summary(w: &mut dyn Write, data: &KernelData) -> anyhow::Result<()> {
    let breakdown = data.stall_breakdown();
    if breakdown.is_empty() {
        return Ok(());
    }

    writeln!(w, "  {}", "[Top Warp Stall Reasons]".bold())?;

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS);
    table.set_header(vec![
        Cell::new("Stall Reason"),
        Cell::new("Samples"),
        Cell::new("Percentage"),
    ]);

    for &(reason, count, pct) in breakdown.iter().take(5) {
        if pct < 1.0 {
            continue;
        }
        table.add_row(vec![
            Cell::new(reason),
            Cell::new(format!("{:.0}", count)),
            Cell::new(format!("{:.1}%", pct)),
        ]);
    }

    for line in table.to_string().lines() {
        writeln!(w, "  {line}")?;
    }
    writeln!(w)?;
    Ok(())
}

fn write_optimization_priorities(w: &mut dyn Write, findings: &[Finding]) -> anyhow::Result<()> {
    let critical: Vec<_> = findings
        .iter()
        .filter(|f| f.severity == Severity::Critical)
        .collect();
    let warnings: Vec<_> = findings
        .iter()
        .filter(|f| f.severity == Severity::Warning)
        .collect();

    if critical.is_empty() && warnings.is_empty() {
        return Ok(());
    }

    writeln!(w, "  {}", "[Optimization Priorities]".bold())?;
    writeln!(w)?;

    let mut rank = 1;
    for f in critical.iter().take(3) {
        writeln!(
            w,
            "  {}. {} {}",
            rank,
            ">>>".red().bold(),
            f.title.red().bold()
        )?;
        rank += 1;
    }
    for f in warnings.iter().take(3usize.saturating_sub(critical.len())) {
        writeln!(
            w,
            "  {}. {} {}",
            rank,
            " >>".yellow(),
            f.title.yellow()
        )?;
        rank += 1;
    }
    writeln!(w)?;
    Ok(())
}

fn write_findings(w: &mut dyn Write, findings: &[Finding]) -> anyhow::Result<()> {
    writeln!(w, "  {}", "[Analysis & Suggestions]".bold())?;
    writeln!(w)?;

    if findings.is_empty() {
        writeln!(w, "  No issues detected.")?;
        return Ok(());
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
        writeln!(w, "  {num}. {title}")?;
        writeln!(w, "     Detail: {}", f.detail)?;
        writeln!(w, "     Action: {}", f.action)?;
        writeln!(w)?;
    }
    Ok(())
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
