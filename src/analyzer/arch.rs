use crate::analyzer::Analyzer;
use crate::metrics::KernelData;
use crate::severity::{Finding, Severity};

pub struct ArchAnalyzer;

/// Architecture family derived from SM version.
#[derive(Debug, Clone, Copy)]
enum ArchFamily {
    Ampere,
    Hopper,
    Blackwell,
    Unknown,
}

fn arch_family(sm: u32) -> ArchFamily {
    match sm {
        80..=89 => ArchFamily::Ampere,
        90..=99 => ArchFamily::Hopper,
        100..=109 => ArchFamily::Blackwell,
        _ => ArchFamily::Unknown,
    }
}

pub fn arch_display_name(sm: u32) -> &'static str {
    match arch_family(sm) {
        ArchFamily::Ampere => "Ampere",
        ArchFamily::Hopper => "Hopper",
        ArchFamily::Blackwell => "Blackwell",
        ArchFamily::Unknown => "Unknown",
    }
}

impl Analyzer for ArchAnalyzer {
    fn name(&self) -> &str {
        "Architecture-Specific Analysis"
    }

    fn analyze(&self, data: &KernelData) -> Vec<Finding> {
        let mut findings = Vec::new();
        let family = arch_family(data.arch_sm);

        match family {
            ArchFamily::Ampere => {
                analyze_ampere(data, &mut findings);
            }
            ArchFamily::Hopper => {
                analyze_hopper(data, &mut findings);
            }
            ArchFamily::Blackwell => {
                analyze_blackwell(data, &mut findings);
            }
            ArchFamily::Unknown => {
                if data.arch_sm != 0 {
                    findings.push(Finding {
                        severity: Severity::Info,
                        title: "Unknown Architecture".into(),
                        detail: format!(
                            "SM version {} is not in the known architecture database.",
                            data.arch_sm
                        ),
                        action: "Architecture-specific optimizations are not available.".into(),
                        source: String::new(),
                    });
                }
            }
        }

        findings
    }
}

fn analyze_ampere(data: &KernelData, findings: &mut Vec<Finding>) {
    // Ampere introduced cp.async for asynchronous global-to-shared memory copies.
    // High LSU pipe utilization with memory-bound behavior suggests synchronous loads.
    if data.mem_throughput_pct > data.sm_throughput_pct + 20.0
        && data.lsu_pipe_utilization_pct > 30.0
    {
        findings.push(Finding {
            severity: Severity::Info,
            title: "Ampere: Consider cp.async".into(),
            detail: format!(
                "Kernel is memory-bound with LSU pipe at {:.1}%. \
                 Synchronous global loads may be stalling warps.",
                data.lsu_pipe_utilization_pct
            ),
            action: "Use cp.async (or cuda::memcpy_async) to overlap global-to-shared \
                     memory transfers with computation, hiding global memory latency."
                .into(),
            source: String::new(),
        });
    }
}

fn analyze_hopper(data: &KernelData, findings: &mut Vec<Finding>) {
    // Hopper's TMA engine enables hardware-accelerated async bulk transfers.
    if data.mem_throughput_pct > data.sm_throughput_pct + 20.0
        && data.tma_cycles_active_pct < 5.0
    {
        findings.push(Finding {
            severity: Severity::Warning,
            title: "Hopper: TMA Engine Underutilized".into(),
            detail: format!(
                "Kernel is memory-bound but TMA pipe activity is only {:.1}%. \
                 The Tensor Memory Accelerator is not being leveraged.",
                data.tma_cycles_active_pct
            ),
            action: "Restructure memory access to use TMA-based async bulk copy \
                     (e.g., via CUTLASS 3.x or CuTe TMA descriptors). \
                     This can significantly reduce memory access latency on Hopper."
                .into(),
            source: String::new(),
        });
    }
}

fn analyze_blackwell(data: &KernelData, findings: &mut Vec<Finding>) {
    // Blackwell supports FP4/FP6 micro-scaling formats for inference acceleration.
    let name_lower = data.kernel_name.to_lowercase();
    let is_fp16_heavy = name_lower.contains("f16")
        || name_lower.contains("fp16")
        || name_lower.contains("half");

    if is_fp16_heavy && data.sm_throughput_pct > 60.0 {
        findings.push(Finding {
            severity: Severity::Info,
            title: "Blackwell: Consider FP4/FP6 Micro-Scaling".into(),
            detail: "Kernel uses FP16 and is compute-heavy on Blackwell architecture.".into(),
            action: "Evaluate new micro-scaling data formats (FP4/FP6) for inference workloads \
                     to potentially double or quadruple throughput."
                .into(),
            source: String::new(),
        });
    }
}
