use std::io::Write;

use serde::Serialize;

use crate::analyzer::roofline;
use crate::analyzer::arch::arch_display_name;
use crate::metrics::KernelData;
use crate::severity::Finding;

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum OutputFormat {
    Terminal,
    Json,
    Csv,
    Markdown,
}

impl Default for OutputFormat {
    fn default() -> Self {
        Self::Terminal
    }
}

// ---------------------------------------------------------------------------
// Analyze report
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct AnalyzeReport<'a> {
    kernel: &'a KernelData,
    bottleneck: String,
    findings: &'a [Finding],
}

pub fn format_analyze(
    w: &mut dyn Write,
    data: &KernelData,
    findings: &[Finding],
    format: OutputFormat,
) -> anyhow::Result<()> {
    match format {
        OutputFormat::Terminal => crate::report::write_report(w, data, findings),
        OutputFormat::Json => {
            let report = AnalyzeReport {
                kernel: data,
                bottleneck: roofline::classify(data).to_string(),
                findings,
            };
            serde_json::to_writer_pretty(w, &report)?;
            Ok(())
        }
        OutputFormat::Markdown => write_analyze_markdown(w, data, findings),
        OutputFormat::Csv => write_analyze_csv(w, data, findings),
    }
}

fn write_analyze_markdown(
    w: &mut dyn Write,
    data: &KernelData,
    findings: &[Finding],
) -> anyhow::Result<()> {
    let bottleneck = roofline::classify(data);
    let arch_name = arch_display_name(data.arch_sm);
    let sm_label = if data.arch_sm > 0 {
        format!("SM_{} ({})", data.arch_sm, arch_name)
    } else {
        "Unknown".into()
    };

    writeln!(w, "# Kernel Analysis")?;
    writeln!(w)?;
    writeln!(w, "- **Kernel:** `{}`", data.kernel_name)?;
    writeln!(w, "- **Arch:** {sm_label}")?;
    writeln!(w, "- **Device:** {}", data.device_name)?;
    writeln!(w, "- **Duration:** {:.2} us", data.duration_us)?;
    writeln!(w, "- **Main Bottleneck:** {bottleneck}")?;
    writeln!(w)?;

    writeln!(w, "## Metrics")?;
    writeln!(w)?;
    writeln!(w, "| Metric | Value | Status |")?;
    writeln!(w, "|--------|-------|--------|")?;
    writeln!(w, "| SM Throughput | {:.1}% | {} |", data.sm_throughput_pct, level_indicator(data.sm_throughput_pct, 60.0, 40.0))?;
    writeln!(w, "| Memory Throughput | {:.1}% | {} |", data.mem_throughput_pct, level_indicator(data.mem_throughput_pct, 60.0, 40.0))?;
    writeln!(w, "| Occupancy | {:.1}% | {} |", data.warps_active_pct, level_indicator(data.warps_active_pct, 50.0, 25.0))?;
    writeln!(w, "| L1 Hit Rate | {:.1}% | {} |", data.l1_hit_rate_pct, level_indicator(data.l1_hit_rate_pct, 50.0, 20.0))?;
    writeln!(w, "| L2 Hit Rate | {:.1}% | {} |", data.l2_hit_rate_pct, level_indicator(data.l2_hit_rate_pct, 70.0, 50.0))?;
    writeln!(w, "| Tensor Core (HMMA) | {:.1}% | -- |", data.tensor_core_hmma_pct)?;
    writeln!(w)?;

    if !findings.is_empty() {
        writeln!(w, "## Findings")?;
        writeln!(w)?;
        for (i, f) in findings.iter().enumerate() {
            writeln!(w, "{}. **[{}] {}**", i + 1, f.severity.as_str(), f.title)?;
            writeln!(w, "   - Detail: {}", f.detail)?;
            writeln!(w, "   - Action: {}", f.action)?;
        }
    }
    Ok(())
}

fn write_analyze_csv(
    w: &mut dyn Write,
    data: &KernelData,
    findings: &[Finding],
) -> anyhow::Result<()> {
    writeln!(w, "kernel_name,device,arch_sm,duration_us,sm_throughput_pct,mem_throughput_pct,occupancy_pct,l1_hit_pct,l2_hit_pct,tensor_core_pct,bottleneck,finding_count")?;
    writeln!(
        w,
        "\"{}\",\"{}\",{},{:.2},{:.1},{:.1},{:.1},{:.1},{:.1},{:.1},{},{}",
        data.kernel_name,
        data.device_name,
        data.arch_sm,
        data.duration_us,
        data.sm_throughput_pct,
        data.mem_throughput_pct,
        data.warps_active_pct,
        data.l1_hit_rate_pct,
        data.l2_hit_rate_pct,
        data.tensor_core_hmma_pct,
        roofline::classify(data),
        findings.len(),
    )?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Info report
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct InfoReport {
    kernel_count: usize,
    device: String,
    arch: String,
    arch_sm: u32,
    total_duration_us: f64,
    kernels: Vec<String>,
}

pub fn format_info(
    w: &mut dyn Write,
    kernels: &[KernelData],
    format: OutputFormat,
) -> anyhow::Result<()> {
    match format {
        OutputFormat::Terminal => write_info_terminal(w, kernels),
        OutputFormat::Json => {
            let device = kernels.first().map(|k| k.device_name.clone()).unwrap_or_default();
            let arch_sm = kernels.first().map(|k| k.arch_sm).unwrap_or(0);
            let report = InfoReport {
                kernel_count: kernels.len(),
                device,
                arch: arch_display_name(arch_sm).to_string(),
                arch_sm,
                total_duration_us: kernels.iter().map(|k| k.duration_us).sum(),
                kernels: kernels.iter().map(|k| k.kernel_name.clone()).collect(),
            };
            serde_json::to_writer_pretty(w, &report)?;
            Ok(())
        }
        OutputFormat::Markdown => write_info_markdown(w, kernels),
        OutputFormat::Csv => write_info_csv(w, kernels),
    }
}

fn write_info_terminal(w: &mut dyn Write, kernels: &[KernelData]) -> anyhow::Result<()> {
    use colored::Colorize;

    let device = kernels.first().map(|k| k.device_name.as_str()).unwrap_or("Unknown");
    let arch_sm = kernels.first().map(|k| k.arch_sm).unwrap_or(0);
    let arch_name = arch_display_name(arch_sm);
    let total_us: f64 = kernels.iter().map(|k| k.duration_us).sum();

    writeln!(w)?;
    writeln!(w, "{}", "═".repeat(60).dimmed())?;
    writeln!(w, "  {} {}", "Profile Info".bold(), "")?;
    writeln!(w, "{}", "═".repeat(60).dimmed())?;
    writeln!(w, "  {} {}", "Device:".bold(), device)?;
    writeln!(w, "  {} SM_{} ({})", "Arch:".bold(), arch_sm, arch_name)?;
    writeln!(w, "  {} {}", "Kernels:".bold(), kernels.len())?;
    writeln!(w, "  {} {:.2} us", "Total Duration:".bold(), total_us)?;
    writeln!(w)?;

    for (i, k) in kernels.iter().enumerate() {
        writeln!(w, "  {}. {} ({:.2} us)", i + 1, truncate_name(&k.kernel_name, 50), k.duration_us)?;
    }
    writeln!(w)?;
    Ok(())
}

fn write_info_markdown(w: &mut dyn Write, kernels: &[KernelData]) -> anyhow::Result<()> {
    let device = kernels.first().map(|k| k.device_name.as_str()).unwrap_or("Unknown");
    let arch_sm = kernels.first().map(|k| k.arch_sm).unwrap_or(0);
    let total_us: f64 = kernels.iter().map(|k| k.duration_us).sum();

    writeln!(w, "# Profile Info")?;
    writeln!(w)?;
    writeln!(w, "- **Device:** {device}")?;
    writeln!(w, "- **Arch:** SM_{} ({})", arch_sm, arch_display_name(arch_sm))?;
    writeln!(w, "- **Kernels:** {}", kernels.len())?;
    writeln!(w, "- **Total Duration:** {total_us:.2} us")?;
    writeln!(w)?;
    for (i, k) in kernels.iter().enumerate() {
        writeln!(w, "{}. `{}` ({:.2} us)", i + 1, k.kernel_name, k.duration_us)?;
    }
    Ok(())
}

fn write_info_csv(w: &mut dyn Write, kernels: &[KernelData]) -> anyhow::Result<()> {
    writeln!(w, "index,kernel_name,device,arch_sm,duration_us")?;
    for (i, k) in kernels.iter().enumerate() {
        writeln!(w, "{},\"{}\",\"{}\",{},{:.2}", i + 1, k.kernel_name, k.device_name, k.arch_sm, k.duration_us)?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Summary report
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct SummaryEntry {
    index: usize,
    kernel_name: String,
    duration_us: f64,
    bottleneck: String,
    sm_throughput_pct: f64,
    mem_throughput_pct: f64,
    occupancy_pct: f64,
}

pub fn format_summary(
    w: &mut dyn Write,
    kernels: &[KernelData],
    format: OutputFormat,
) -> anyhow::Result<()> {
    let entries: Vec<SummaryEntry> = kernels
        .iter()
        .enumerate()
        .map(|(i, k)| SummaryEntry {
            index: i + 1,
            kernel_name: k.kernel_name.clone(),
            duration_us: k.duration_us,
            bottleneck: roofline::classify(k).to_string(),
            sm_throughput_pct: k.sm_throughput_pct,
            mem_throughput_pct: k.mem_throughput_pct,
            occupancy_pct: k.warps_active_pct,
        })
        .collect();

    match format {
        OutputFormat::Terminal => write_summary_terminal(w, &entries),
        OutputFormat::Json => {
            serde_json::to_writer_pretty(w, &entries)?;
            Ok(())
        }
        OutputFormat::Markdown => write_summary_markdown(w, &entries),
        OutputFormat::Csv => write_summary_csv(w, &entries),
    }
}

fn write_summary_terminal(w: &mut dyn Write, entries: &[SummaryEntry]) -> anyhow::Result<()> {
    use colored::Colorize;
    use comfy_table::{modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, Cell, Table};

    writeln!(w)?;
    writeln!(w, "  {}", "[Kernel Summary]".bold())?;
    writeln!(w)?;

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS);

    table.set_header(vec![
        Cell::new("#"),
        Cell::new("Kernel"),
        Cell::new("Duration (us)"),
        Cell::new("Bottleneck"),
        Cell::new("SM %"),
        Cell::new("Mem %"),
        Cell::new("Occ %"),
    ]);

    for e in entries {
        table.add_row(vec![
            Cell::new(e.index),
            Cell::new(truncate_name(&e.kernel_name, 40)),
            Cell::new(format!("{:.2}", e.duration_us)),
            Cell::new(&e.bottleneck),
            Cell::new(format!("{:.1}", e.sm_throughput_pct)),
            Cell::new(format!("{:.1}", e.mem_throughput_pct)),
            Cell::new(format!("{:.1}", e.occupancy_pct)),
        ]);
    }

    for line in table.to_string().lines() {
        writeln!(w, "  {line}")?;
    }
    writeln!(w)?;
    Ok(())
}

fn write_summary_markdown(w: &mut dyn Write, entries: &[SummaryEntry]) -> anyhow::Result<()> {
    writeln!(w, "# Kernel Summary")?;
    writeln!(w)?;
    writeln!(w, "| # | Kernel | Duration (us) | Bottleneck | SM % | Mem % | Occ % |")?;
    writeln!(w, "|---|--------|---------------|------------|------|-------|-------|")?;
    for e in entries {
        writeln!(
            w,
            "| {} | `{}` | {:.2} | {} | {:.1} | {:.1} | {:.1} |",
            e.index, e.kernel_name, e.duration_us, e.bottleneck,
            e.sm_throughput_pct, e.mem_throughput_pct, e.occupancy_pct,
        )?;
    }
    Ok(())
}

fn write_summary_csv(w: &mut dyn Write, entries: &[SummaryEntry]) -> anyhow::Result<()> {
    writeln!(w, "index,kernel_name,duration_us,bottleneck,sm_throughput_pct,mem_throughput_pct,occupancy_pct")?;
    for e in entries {
        writeln!(
            w,
            "{},\"{}\",{:.2},{},{:.1},{:.1},{:.1}",
            e.index, e.kernel_name, e.duration_us, e.bottleneck,
            e.sm_throughput_pct, e.mem_throughput_pct, e.occupancy_pct,
        )?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Diff report
// ---------------------------------------------------------------------------

use crate::diff::DiffReport;

pub fn format_diff(
    w: &mut dyn Write,
    report: &DiffReport,
    format: OutputFormat,
) -> anyhow::Result<()> {
    match format {
        OutputFormat::Terminal => write_diff_terminal(w, report),
        OutputFormat::Json => {
            serde_json::to_writer_pretty(w, report)?;
            Ok(())
        }
        OutputFormat::Markdown => write_diff_markdown(w, report),
        OutputFormat::Csv => write_diff_csv(w, report),
    }
}

fn write_diff_terminal(w: &mut dyn Write, report: &DiffReport) -> anyhow::Result<()> {
    use colored::Colorize;
    use comfy_table::{modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, Cell, Table};

    writeln!(w)?;
    writeln!(w, "  {}", "[Profile Diff]".bold())?;
    writeln!(w)?;

    if !report.regressions.is_empty() {
        writeln!(w, "  {} (slower)", "Regressions".red().bold())?;
        let mut table = Table::new();
        table.load_preset(UTF8_FULL).apply_modifier(UTF8_ROUND_CORNERS);
        table.set_header(vec![
            Cell::new("Kernel"),
            Cell::new("Before (us)"),
            Cell::new("After (us)"),
            Cell::new("Delta (us)"),
            Cell::new("Delta %"),
        ]);
        for d in &report.regressions {
            table.add_row(vec![
                Cell::new(truncate_name(&d.kernel_name, 35)),
                Cell::new(format!("{:.2}", d.before_us)),
                Cell::new(format!("{:.2}", d.after_us)),
                Cell::new(format!("+{:.2}", d.delta_us)),
                Cell::new(format!("+{:.1}%", d.delta_pct)),
            ]);
        }
        for line in table.to_string().lines() {
            writeln!(w, "  {line}")?;
        }
        writeln!(w)?;
    }

    if !report.improvements.is_empty() {
        writeln!(w, "  {} (faster)", "Improvements".green().bold())?;
        let mut table = Table::new();
        table.load_preset(UTF8_FULL).apply_modifier(UTF8_ROUND_CORNERS);
        table.set_header(vec![
            Cell::new("Kernel"),
            Cell::new("Before (us)"),
            Cell::new("After (us)"),
            Cell::new("Delta (us)"),
            Cell::new("Delta %"),
        ]);
        for d in &report.improvements {
            table.add_row(vec![
                Cell::new(truncate_name(&d.kernel_name, 35)),
                Cell::new(format!("{:.2}", d.before_us)),
                Cell::new(format!("{:.2}", d.after_us)),
                Cell::new(format!("{:.2}", d.delta_us)),
                Cell::new(format!("{:.1}%", d.delta_pct)),
            ]);
        }
        for line in table.to_string().lines() {
            writeln!(w, "  {line}")?;
        }
        writeln!(w)?;
    }

    if !report.new_kernels.is_empty() {
        writeln!(w, "  {}", "New Kernels".cyan().bold())?;
        for k in &report.new_kernels {
            writeln!(w, "    + {} ({:.2} us)", truncate_name(k, 50), 0.0)?;
        }
        writeln!(w)?;
    }

    if !report.removed_kernels.is_empty() {
        writeln!(w, "  {}", "Removed Kernels".yellow().bold())?;
        for k in &report.removed_kernels {
            writeln!(w, "    - {} ", truncate_name(k, 50))?;
        }
        writeln!(w)?;
    }

    if report.regressions.is_empty()
        && report.improvements.is_empty()
        && report.new_kernels.is_empty()
        && report.removed_kernels.is_empty()
    {
        writeln!(w, "  No differences found.")?;
        writeln!(w)?;
    }

    Ok(())
}

fn write_diff_markdown(w: &mut dyn Write, report: &DiffReport) -> anyhow::Result<()> {
    writeln!(w, "# Profile Diff")?;
    writeln!(w)?;

    if !report.regressions.is_empty() {
        writeln!(w, "## Regressions (slower)")?;
        writeln!(w)?;
        writeln!(w, "| Kernel | Before (us) | After (us) | Delta (us) | Delta % |")?;
        writeln!(w, "|--------|-------------|------------|------------|---------|")?;
        for d in &report.regressions {
            writeln!(
                w,
                "| `{}` | {:.2} | {:.2} | +{:.2} | +{:.1}% |",
                d.kernel_name, d.before_us, d.after_us, d.delta_us, d.delta_pct,
            )?;
        }
        writeln!(w)?;
    }

    if !report.improvements.is_empty() {
        writeln!(w, "## Improvements (faster)")?;
        writeln!(w)?;
        writeln!(w, "| Kernel | Before (us) | After (us) | Delta (us) | Delta % |")?;
        writeln!(w, "|--------|-------------|------------|------------|---------|")?;
        for d in &report.improvements {
            writeln!(
                w,
                "| `{}` | {:.2} | {:.2} | {:.2} | {:.1}% |",
                d.kernel_name, d.before_us, d.after_us, d.delta_us, d.delta_pct,
            )?;
        }
        writeln!(w)?;
    }

    if !report.new_kernels.is_empty() {
        writeln!(w, "## New Kernels")?;
        writeln!(w)?;
        for k in &report.new_kernels {
            writeln!(w, "- `{k}`")?;
        }
        writeln!(w)?;
    }

    if !report.removed_kernels.is_empty() {
        writeln!(w, "## Removed Kernels")?;
        writeln!(w)?;
        for k in &report.removed_kernels {
            writeln!(w, "- `{k}`")?;
        }
        writeln!(w)?;
    }

    Ok(())
}

fn write_diff_csv(w: &mut dyn Write, report: &DiffReport) -> anyhow::Result<()> {
    writeln!(w, "kernel_name,change,before_us,after_us,delta_us,delta_pct")?;
    for d in &report.regressions {
        writeln!(
            w,
            "\"{}\",regression,{:.2},{:.2},{:.2},{:.1}",
            d.kernel_name, d.before_us, d.after_us, d.delta_us, d.delta_pct,
        )?;
    }
    for d in &report.improvements {
        writeln!(
            w,
            "\"{}\",improvement,{:.2},{:.2},{:.2},{:.1}",
            d.kernel_name, d.before_us, d.after_us, d.delta_us, d.delta_pct,
        )?;
    }
    for k in &report.new_kernels {
        writeln!(w, "\"{k}\",new,,,,")?;
    }
    for k in &report.removed_kernels {
        writeln!(w, "\"{k}\",removed,,,,")?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Skill list
// ---------------------------------------------------------------------------

pub fn format_skill_list(
    w: &mut dyn Write,
    format: OutputFormat,
) -> anyhow::Result<()> {
    let analyzers = crate::analyzer::all_analyzers();

    match format {
        OutputFormat::Terminal => {
            use colored::Colorize;
            writeln!(w)?;
            writeln!(w, "  {}", "[Available Skills]".bold())?;
            writeln!(w)?;
            for a in &analyzers {
                writeln!(w, "  {} {}", format!("{}:", a.category()).cyan().bold(), a.name())?;
                writeln!(w, "    {}", a.description())?;
                writeln!(w)?;
            }
            Ok(())
        }
        OutputFormat::Json => {
            #[derive(Serialize)]
            struct SkillInfo { category: String, name: String, description: String }
            let skills: Vec<SkillInfo> = analyzers
                .iter()
                .map(|a| SkillInfo {
                    category: a.category().to_string(),
                    name: a.name().to_string(),
                    description: a.description().to_string(),
                })
                .collect();
            serde_json::to_writer_pretty(w, &skills)?;
            Ok(())
        }
        OutputFormat::Markdown => {
            writeln!(w, "# Available Skills")?;
            writeln!(w)?;
            writeln!(w, "| Category | Name | Description |")?;
            writeln!(w, "|----------|------|-------------|")?;
            for a in &analyzers {
                writeln!(w, "| {} | {} | {} |", a.category(), a.name(), a.description())?;
            }
            Ok(())
        }
        OutputFormat::Csv => {
            writeln!(w, "category,name,description")?;
            for a in &analyzers {
                writeln!(w, "{},{},\"{}\"", a.category(), a.name(), a.description())?;
            }
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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
