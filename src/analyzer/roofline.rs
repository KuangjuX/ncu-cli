use crate::analyzer::Analyzer;
use crate::metrics::KernelData;
use crate::severity::{Finding, Severity};

pub struct RooflineAnalyzer;

#[derive(Debug, Clone, Copy)]
pub enum Bottleneck {
    ComputeBound,
    MemoryBound,
    Balanced,
    LatencyBound,
}

impl std::fmt::Display for Bottleneck {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Bottleneck::ComputeBound => write!(f, "Compute Bound"),
            Bottleneck::MemoryBound => write!(f, "Memory Bound"),
            Bottleneck::Balanced => write!(f, "Balanced"),
            Bottleneck::LatencyBound => write!(f, "Latency Bound"),
        }
    }
}

pub fn classify(data: &KernelData) -> Bottleneck {
    let compute = data.sm_throughput_pct;
    let memory = data.mem_throughput_pct;

    if compute < 40.0 && memory < 40.0 {
        Bottleneck::LatencyBound
    } else if compute > memory + 20.0 {
        Bottleneck::ComputeBound
    } else if memory > compute + 20.0 {
        Bottleneck::MemoryBound
    } else if (compute - memory).abs() < 20.0 && compute > 60.0 && memory > 60.0 {
        Bottleneck::Balanced
    } else {
        // Moderate utilization, slight imbalance
        if compute > memory {
            Bottleneck::ComputeBound
        } else {
            Bottleneck::MemoryBound
        }
    }
}

impl Analyzer for RooflineAnalyzer {
    fn name(&self) -> &str {
        "Roofline Analysis"
    }

    fn analyze(&self, data: &KernelData) -> Vec<Finding> {
        let bottleneck = classify(data);
        let compute = data.sm_throughput_pct;
        let memory = data.mem_throughput_pct;

        let finding = match bottleneck {
            Bottleneck::ComputeBound => Finding {
                severity: Severity::Warning,
                title: "Compute Bound".into(),
                detail: format!(
                    "SM throughput ({compute:.1}%) significantly exceeds memory throughput ({memory:.1}%)."
                ),
                action: "Check for operator fusion opportunities, reduce redundant computation, \
                         and verify Tensor Core utilization."
                    .into(),
                source: String::new(),
            },
            Bottleneck::MemoryBound => Finding {
                severity: Severity::Warning,
                title: "Memory Bound".into(),
                detail: format!(
                    "Memory throughput ({memory:.1}%) significantly exceeds SM throughput ({compute:.1}%)."
                ),
                action: "Check memory access patterns, improve L2 cache hit rate, \
                         and consider data layout optimizations."
                    .into(),
                source: String::new(),
            },
            Bottleneck::Balanced => Finding {
                severity: Severity::Info,
                title: "Balanced Utilization".into(),
                detail: format!(
                    "SM throughput ({compute:.1}%) and memory throughput ({memory:.1}%) are both high and balanced."
                ),
                action: "Kernel is near peak utilization. Focus on micro-optimizations or algorithmic changes."
                    .into(),
                source: String::new(),
            },
            Bottleneck::LatencyBound => Finding {
                severity: Severity::Critical,
                title: "Latency Bound".into(),
                detail: format!(
                    "Both SM throughput ({compute:.1}%) and memory throughput ({memory:.1}%) are low (<40%)."
                ),
                action: "Analyze warp stall reasons. Consider increasing occupancy, \
                         reducing synchronization barriers, or restructuring the kernel launch configuration."
                    .into(),
                source: String::new(),
            },
        };

        vec![finding]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::KernelData;

    fn make_data(sm: f64, mem: f64) -> KernelData {
        KernelData {
            kernel_name: "test_kernel".into(),
            device_name: "Test GPU".into(),
            grid_size: "1,1,1".into(),
            block_size: "256,1,1".into(),
            duration_us: 100.0,
            sm_throughput_pct: sm,
            mem_throughput_pct: mem,
            l1_sectors_global_ld: 0.0,
            l1_requests_global_ld: 0.0,
            l1_hit_rate_pct: 50.0,
            l2_hit_rate_pct: 70.0,
            local_mem_store_sectors: 0.0,
            warps_active_pct: 60.0,
            tensor_core_hmma_pct: 0.0,
            arch_sm: 90,
            tma_cycles_active_pct: 0.0,
            lsu_pipe_utilization_pct: 0.0,
        }
    }

    #[test]
    fn test_compute_bound() {
        let data = make_data(80.0, 30.0);
        assert!(matches!(classify(&data), Bottleneck::ComputeBound));
    }

    #[test]
    fn test_memory_bound() {
        let data = make_data(27.0, 85.0);
        assert!(matches!(classify(&data), Bottleneck::MemoryBound));
    }

    #[test]
    fn test_balanced() {
        let data = make_data(70.0, 75.0);
        assert!(matches!(classify(&data), Bottleneck::Balanced));
    }

    #[test]
    fn test_latency_bound() {
        let data = make_data(10.0, 15.0);
        assert!(matches!(classify(&data), Bottleneck::LatencyBound));
    }

    #[test]
    fn test_analyzer_returns_one_finding() {
        let data = make_data(27.0, 85.0);
        let findings = RooflineAnalyzer.analyze(&data);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Warning);
    }
}
