pub mod arch;
pub mod instruction;
pub mod launch_config;
pub mod memory;
pub mod occupancy;
pub mod roofline;
pub mod warp_stall;

use crate::metrics::KernelData;
use crate::severity::Finding;

pub trait Analyzer {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn category(&self) -> &str;
    fn analyze(&self, data: &KernelData) -> Vec<Finding>;
}

/// Return all registered analyzers (skills).
pub fn all_analyzers() -> Vec<Box<dyn Analyzer>> {
    vec![
        Box::new(roofline::RooflineAnalyzer),
        Box::new(memory::MemoryAnalyzer),
        Box::new(occupancy::OccupancyAnalyzer),
        Box::new(instruction::InstructionAnalyzer),
        Box::new(warp_stall::WarpStallAnalyzer),
        Box::new(launch_config::LaunchConfigAnalyzer),
        Box::new(arch::ArchAnalyzer),
    ]
}

/// Look up an analyzer by name (case-insensitive prefix match).
pub fn get_analyzer(name: &str) -> Option<Box<dyn Analyzer>> {
    let lower = name.to_lowercase();
    all_analyzers()
        .into_iter()
        .find(|a| a.category().to_lowercase() == lower)
}

/// Run all analyzers in sequence and collect findings.
pub fn run_all(data: &KernelData) -> Vec<Finding> {
    run_analyzers(&all_analyzers(), data)
}

/// Run a specific set of analyzers and collect findings.
pub fn run_analyzers(analyzers: &[Box<dyn Analyzer>], data: &KernelData) -> Vec<Finding> {
    let mut findings = Vec::new();
    for a in analyzers {
        let name = a.name();
        for mut f in a.analyze(data) {
            f.source = name.to_string();
            findings.push(f);
        }
    }
    findings.sort_by(|a, b| b.severity.cmp(&a.severity));
    findings
}
