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

    fn description(&self) -> &str {
        "Classifies kernel as Compute Bound, Memory Bound, Balanced, or Latency Bound based on SM and memory throughput"
    }

    fn category(&self) -> &str {
        "roofline"
    }

    fn analyze(&self, data: &KernelData) -> Vec<Finding> {
        let mut findings = Vec::new();
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
            Bottleneck::MemoryBound => {
                let sub_bottleneck = classify_memory_sublevel(data);
                Finding {
                    severity: Severity::Warning,
                    title: format!("Memory Bound ({})", sub_bottleneck),
                    detail: format!(
                        "Memory throughput ({memory:.1}%) significantly exceeds SM throughput ({compute:.1}%). \
                         Sub-classification: {sub_bottleneck}. \
                         L1 hit: {:.1}%, L2 hit: {:.1}%, DRAM throughput: {:.1}%.",
                        data.l1_hit_rate_pct, data.l2_hit_rate_pct, data.dram_throughput_pct
                    ),
                    action: match sub_bottleneck {
                        MemorySubLevel::DramBound => "DRAM bandwidth is the bottleneck. Reduce data movement via \
                                 mixed precision, compression, or algorithmic changes to improve arithmetic intensity. \
                                 Use L2 persistence hints (cudaAccessPolicyWindow) to cache hot data.".into(),
                        MemorySubLevel::L2Bound => "L2 cache misses are driving traffic to DRAM. Improve data reuse \
                                 with tiling, restructure access patterns for temporal locality, \
                                 or use L2 persistence policies on Ampere+.".into(),
                        MemorySubLevel::L1Bound => "L1 cache is the bottleneck. Use shared memory for frequently \
                                 accessed data, apply tiling strategies, or improve spatial locality \
                                 of global memory accesses.".into(),
                    },
                    source: String::new(),
                }
            }
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

        findings.push(finding);

        if data.dram_throughput_pct > 0.0 {
            findings.push(Finding {
                severity: Severity::Info,
                title: format!("DRAM Throughput: {:.1}%", data.dram_throughput_pct),
                detail: format!(
                    "DRAM bandwidth utilization: {:.1}% of peak. \
                     Read: {:.2} GB, Write: {:.2} GB.",
                    data.dram_throughput_pct, data.dram_read_gbytes, data.dram_write_gbytes
                ),
                action: String::new(),
                source: String::new(),
            });
        }

        findings
    }
}

#[derive(Debug, Clone, Copy)]
#[allow(clippy::enum_variant_names)]
enum MemorySubLevel {
    DramBound,
    L2Bound,
    L1Bound,
}

impl std::fmt::Display for MemorySubLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemorySubLevel::DramBound => write!(f, "DRAM-Bound"),
            MemorySubLevel::L2Bound => write!(f, "L2-Bound"),
            MemorySubLevel::L1Bound => write!(f, "L1-Bound"),
        }
    }
}

fn classify_memory_sublevel(data: &KernelData) -> MemorySubLevel {
    if data.dram_throughput_pct > 70.0 {
        return MemorySubLevel::DramBound;
    }
    if data.l2_hit_rate_pct < 50.0 && data.dram_throughput_pct > 40.0 {
        return MemorySubLevel::DramBound;
    }
    if data.l1_hit_rate_pct < 20.0 && data.l2_hit_rate_pct >= 50.0 {
        return MemorySubLevel::L2Bound;
    }
    if data.l1_hit_rate_pct < 20.0 {
        return MemorySubLevel::L1Bound;
    }
    // Default: if DRAM throughput is significant, it's DRAM-bound
    if data.dram_throughput_pct > 30.0 {
        MemorySubLevel::DramBound
    } else {
        MemorySubLevel::L2Bound
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
            l1_sectors_global_st: 0.0,
            l1_requests_global_st: 0.0,
            shared_mem_bank_conflicts: 0.0,
            local_mem_store_sectors: 0.0,
            warps_active_pct: 60.0,
            registers_per_thread: 32.0,
            shared_mem_per_block_kb: 0.0,
            occupancy_limit_registers: 8.0,
            occupancy_limit_shared_mem: 32.0,
            occupancy_limit_warps: 8.0,
            occupancy_limit_blocks: 32.0,
            theoretical_occupancy_pct: 100.0,
            dram_read_gbytes: 0.0,
            dram_write_gbytes: 0.0,
            dram_throughput_pct: 0.0,
            tensor_core_hmma_pct: 0.0,
            pipe_fma_pct: 0.0,
            pipe_alu_pct: 0.0,
            pipe_lsu_pct: 0.0,
            pipe_tensor_pct: 0.0,
            pipe_fma_fp16_pct: 0.0,
            avg_thread_executed: 0.0,
            avg_thread_executed_true: 0.0,
            warps_eligible_per_cycle: 2.0,
            stall_long_scoreboard: 0.0,
            stall_short_scoreboard: 0.0,
            stall_wait: 0.0,
            stall_sleeping: 0.0,
            stall_barrier: 0.0,
            stall_mio_throttle: 0.0,
            stall_lg_throttle: 0.0,
            stall_math_pipe_throttle: 0.0,
            stall_drain: 0.0,
            stall_not_selected: 0.0,
            stall_selected: 0.0,
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
    fn test_memory_bound_has_sublevel() {
        let mut data = make_data(27.0, 85.0);
        data.dram_throughput_pct = 85.0;
        let findings = RooflineAnalyzer.analyze(&data);
        assert!(findings
            .iter()
            .any(|f| f.title.contains("Memory Bound") && f.title.contains("DRAM")));
    }

    #[test]
    fn test_dram_throughput_info_reported() {
        let mut data = make_data(27.0, 85.0);
        data.dram_throughput_pct = 85.0;
        data.dram_read_gbytes = 1.07;
        data.dram_write_gbytes = 1.05;
        let findings = RooflineAnalyzer.analyze(&data);
        assert!(findings
            .iter()
            .any(|f| f.title.contains("DRAM Throughput")));
    }

    #[test]
    fn test_memory_sublevel_l2_bound() {
        let mut data = make_data(27.0, 85.0);
        data.l1_hit_rate_pct = 10.0;
        data.l2_hit_rate_pct = 60.0;
        data.dram_throughput_pct = 30.0;
        let findings = RooflineAnalyzer.analyze(&data);
        assert!(findings
            .iter()
            .any(|f| f.title.contains("L2-Bound")));
    }
}
