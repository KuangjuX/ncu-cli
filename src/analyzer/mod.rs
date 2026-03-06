pub mod arch;
pub mod instruction;
pub mod memory;
pub mod occupancy;
pub mod roofline;

use crate::metrics::KernelData;
use crate::severity::Finding;

pub trait Analyzer {
    fn name(&self) -> &str;
    fn analyze(&self, data: &KernelData) -> Vec<Finding>;
}

/// Run all analyzers in sequence and collect findings.
pub fn run_all(data: &KernelData) -> Vec<Finding> {
    let analyzers: Vec<Box<dyn Analyzer>> = vec![
        Box::new(roofline::RooflineAnalyzer),
        Box::new(memory::MemoryAnalyzer),
        Box::new(occupancy::OccupancyAnalyzer),
        Box::new(instruction::InstructionAnalyzer),
        Box::new(arch::ArchAnalyzer),
    ];

    let mut findings = Vec::new();
    for a in &analyzers {
        let name = a.name();
        for mut f in a.analyze(data) {
            f.source = name.to_string();
            findings.push(f);
        }
    }

    // Sort by severity descending (Critical first)
    findings.sort_by(|a, b| b.severity.cmp(&a.severity));
    findings
}
