mod analyzer;
mod metrics;
mod parser;
mod report;
mod severity;

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;

/// ncu-cli: Automated CUDA kernel performance diagnostics from NCU CSV exports.
#[derive(Parser, Debug)]
#[command(version, about)]
struct Cli {
    /// Path to the NCU CSV export file
    #[arg(short, long)]
    input: PathBuf,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let data = parser::parse_ncu_csv(&cli.input)
        .with_context(|| format!("Failed to parse NCU CSV: {}", cli.input.display()))?;

    let findings = analyzer::run_all(&data);

    report::print_report(&data, &findings);

    Ok(())
}
