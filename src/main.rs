mod analyzer;
mod diff;
mod formatter;
mod metrics;
mod parser;
mod report;
mod severity;

use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};

use formatter::OutputFormat;

/// ncu-cli: Automated CUDA kernel performance diagnostics from NCU CSV exports.
#[derive(Parser, Debug)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Path to the NCU CSV export file (shorthand for `analyze <path>`)
    #[arg(short, long, global = true)]
    input: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Analyze kernel(s) and show diagnostics (default command)
    Analyze {
        /// Path to the NCU CSV export file
        path: PathBuf,
        /// Select a specific kernel by name substring
        #[arg(short, long)]
        kernel: Option<String>,
        /// Output format
        #[arg(short, long, value_enum, default_value_t = OutputFormat::Terminal)]
        format: OutputFormat,
        /// Write output to file instead of stdout
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Show profile metadata (device, arch, kernel count)
    Info {
        /// Path to the NCU CSV export file
        path: PathBuf,
        #[arg(short, long, value_enum, default_value_t = OutputFormat::Terminal)]
        format: OutputFormat,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Summary table of all kernels in the profile
    Summary {
        /// Path to the NCU CSV export file
        path: PathBuf,
        #[arg(short, long, value_enum, default_value_t = OutputFormat::Terminal)]
        format: OutputFormat,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// List or run individual analysis skills
    Skill {
        #[command(subcommand)]
        action: SkillAction,
    },
    /// Compare two profiles side-by-side
    Diff {
        /// Path to the "before" NCU CSV
        before: PathBuf,
        /// Path to the "after" NCU CSV
        after: PathBuf,
        #[arg(short, long, value_enum, default_value_t = OutputFormat::Terminal)]
        format: OutputFormat,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Export kernel data in a structured format
    Export {
        /// Path to the NCU CSV export file
        path: PathBuf,
        #[arg(short, long, value_enum, default_value_t = OutputFormat::Json)]
        format: OutputFormat,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

#[derive(Subcommand, Debug)]
enum SkillAction {
    /// List all available analysis skills
    List {
        #[arg(short, long, value_enum, default_value_t = OutputFormat::Terminal)]
        format: OutputFormat,
    },
    /// Run a specific skill on a profile
    Run {
        /// Skill category name (e.g. roofline, memory, occupancy, instruction, arch)
        name: String,
        /// Path to the NCU CSV export file
        path: PathBuf,
        /// Select a specific kernel by name substring
        #[arg(short, long)]
        kernel: Option<String>,
        #[arg(short, long, value_enum, default_value_t = OutputFormat::Terminal)]
        format: OutputFormat,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(cmd) => run_command(cmd),
        None => {
            // Backward-compatible: `ncu-cli --input <path>` runs analyze
            if let Some(path) = cli.input {
                run_command(Commands::Analyze {
                    path,
                    kernel: None,
                    format: OutputFormat::Terminal,
                    output: None,
                })
            } else {
                // No subcommand and no --input: show help
                use clap::CommandFactory;
                Cli::command().print_help()?;
                println!();
                Ok(())
            }
        }
    }
}

fn run_command(cmd: Commands) -> Result<()> {
    match cmd {
        Commands::Analyze { path, kernel, format, output } => {
            cmd_analyze(&path, kernel.as_deref(), format, output.as_deref())
        }
        Commands::Info { path, format, output } => {
            cmd_info(&path, format, output.as_deref())
        }
        Commands::Summary { path, format, output } => {
            cmd_summary(&path, format, output.as_deref())
        }
        Commands::Skill { action } => cmd_skill(action),
        Commands::Diff { before, after, format, output } => {
            cmd_diff(&before, &after, format, output.as_deref())
        }
        Commands::Export { path, format, output } => {
            cmd_export(&path, format, output.as_deref())
        }
    }
}

fn open_output(path: Option<&std::path::Path>) -> Result<Box<dyn Write>> {
    match path {
        Some(p) => {
            let file = File::create(p)
                .with_context(|| format!("Failed to create output file: {}", p.display()))?;
            Ok(Box::new(BufWriter::new(file)))
        }
        None => Ok(Box::new(BufWriter::new(io::stdout().lock()))),
    }
}

fn load_kernels(path: &std::path::Path) -> Result<Vec<metrics::KernelData>> {
    parser::parse_ncu_csv(path)
        .with_context(|| format!("Failed to parse NCU CSV: {}", path.display()))
}

fn select_kernel<'a>(
    kernels: &'a [metrics::KernelData],
    filter: Option<&str>,
) -> Result<&'a metrics::KernelData> {
    match filter {
        Some(name) => {
            let lower = name.to_lowercase();
            kernels
                .iter()
                .find(|k| k.kernel_name.to_lowercase().contains(&lower))
                .with_context(|| format!("No kernel matching '{name}' found"))
        }
        None => kernels.first().context("No kernels found in profile"),
    }
}

// ---------------------------------------------------------------------------
// Subcommand implementations
// ---------------------------------------------------------------------------

fn cmd_analyze(
    path: &std::path::Path,
    kernel: Option<&str>,
    format: OutputFormat,
    output: Option<&std::path::Path>,
) -> Result<()> {
    let kernels = load_kernels(path)?;
    let data = select_kernel(&kernels, kernel)?;
    let findings = analyzer::run_all(data);
    let mut w = open_output(output)?;
    formatter::format_analyze(&mut w, data, &findings, format)?;
    w.flush()?;
    Ok(())
}

fn cmd_info(
    path: &std::path::Path,
    format: OutputFormat,
    output: Option<&std::path::Path>,
) -> Result<()> {
    let kernels = load_kernels(path)?;
    let mut w = open_output(output)?;
    formatter::format_info(&mut w, &kernels, format)?;
    w.flush()?;
    Ok(())
}

fn cmd_summary(
    path: &std::path::Path,
    format: OutputFormat,
    output: Option<&std::path::Path>,
) -> Result<()> {
    let kernels = load_kernels(path)?;
    let mut w = open_output(output)?;
    formatter::format_summary(&mut w, &kernels, format)?;
    w.flush()?;
    Ok(())
}

fn cmd_skill(action: SkillAction) -> Result<()> {
    match action {
        SkillAction::List { format } => {
            let mut w = open_output(None)?;
            formatter::format_skill_list(&mut w, format)?;
            w.flush()?;
            Ok(())
        }
        SkillAction::Run { name, path, kernel, format, output } => {
            let skill = analyzer::get_analyzer(&name)
                .with_context(|| {
                    let available: Vec<String> = analyzer::all_analyzers()
                        .iter()
                        .map(|a| a.category().to_string())
                        .collect();
                    format!(
                        "Unknown skill '{name}'. Available: {}",
                        available.join(", ")
                    )
                })?;

            let kernels = load_kernels(&path)?;
            let data = select_kernel(&kernels, kernel.as_deref())?;

            let mut findings = skill.analyze(data);
            for f in &mut findings {
                f.source = skill.name().to_string();
            }
            findings.sort_by(|a, b| b.severity.cmp(&a.severity));

            let mut w = open_output(output.as_deref())?;
            formatter::format_analyze(&mut w, data, &findings, format)?;
            w.flush()?;
            Ok(())
        }
    }
}

fn cmd_diff(
    before_path: &std::path::Path,
    after_path: &std::path::Path,
    format: OutputFormat,
    output: Option<&std::path::Path>,
) -> Result<()> {
    let before = load_kernels(before_path)?;
    let after = load_kernels(after_path)?;
    let report = diff::diff_profiles(&before, &after);
    let mut w = open_output(output)?;
    formatter::format_diff(&mut w, &report, format)?;
    w.flush()?;
    Ok(())
}

fn cmd_export(
    path: &std::path::Path,
    format: OutputFormat,
    output: Option<&std::path::Path>,
) -> Result<()> {
    let kernels = load_kernels(path)?;
    let mut w = open_output(output)?;

    match format {
        OutputFormat::Json => {
            serde_json::to_writer_pretty(&mut w, &kernels)?;
        }
        OutputFormat::Csv => {
            writeln!(w, "kernel_name,device,arch_sm,duration_us,sm_throughput_pct,mem_throughput_pct,occupancy_pct,l1_hit_pct,l2_hit_pct,tensor_core_pct")?;
            for k in &kernels {
                writeln!(
                    w,
                    "\"{}\",\"{}\",{},{:.2},{:.1},{:.1},{:.1},{:.1},{:.1},{:.1}",
                    k.kernel_name, k.device_name, k.arch_sm, k.duration_us,
                    k.sm_throughput_pct, k.mem_throughput_pct, k.warps_active_pct,
                    k.l1_hit_rate_pct, k.l2_hit_rate_pct, k.tensor_core_hmma_pct,
                )?;
            }
        }
        OutputFormat::Markdown => {
            writeln!(w, "# Kernel Data Export")?;
            writeln!(w)?;
            for (i, k) in kernels.iter().enumerate() {
                writeln!(w, "## {}. `{}`", i + 1, k.kernel_name)?;
                writeln!(w)?;
                writeln!(w, "| Metric | Value |")?;
                writeln!(w, "|--------|-------|")?;
                writeln!(w, "| Device | {} |", k.device_name)?;
                writeln!(w, "| Arch SM | {} |", k.arch_sm)?;
                writeln!(w, "| Duration | {:.2} us |", k.duration_us)?;
                writeln!(w, "| SM Throughput | {:.1}% |", k.sm_throughput_pct)?;
                writeln!(w, "| Memory Throughput | {:.1}% |", k.mem_throughput_pct)?;
                writeln!(w, "| Occupancy | {:.1}% |", k.warps_active_pct)?;
                writeln!(w, "| L1 Hit Rate | {:.1}% |", k.l1_hit_rate_pct)?;
                writeln!(w, "| L2 Hit Rate | {:.1}% |", k.l2_hit_rate_pct)?;
                writeln!(w, "| Tensor Core | {:.1}% |", k.tensor_core_hmma_pct)?;
                writeln!(w)?;
            }
        }
        OutputFormat::Terminal => {
            bail!("Use --format json, csv, or markdown for export");
        }
    }

    w.flush()?;
    Ok(())
}
