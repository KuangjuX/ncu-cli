use crate::analyzer::Analyzer;
use crate::metrics::KernelData;
use crate::severity::{Finding, Severity};

pub struct OccupancyAnalyzer;

impl Analyzer for OccupancyAnalyzer {
    fn name(&self) -> &str {
        "Occupancy & Register Spill Analysis"
    }

    fn description(&self) -> &str {
        "Flags register spills to local memory and low warp occupancy"
    }

    fn category(&self) -> &str {
        "occupancy"
    }

    fn analyze(&self, data: &KernelData) -> Vec<Finding> {
        let mut findings = Vec::new();

        if data.local_mem_store_sectors > 0.0 {
            findings.push(Finding {
                severity: Severity::Critical,
                title: "Register Spill Detected".into(),
                detail: format!(
                    "Local memory store sectors = {:.0}. \
                     Registers are spilling to local (slow) memory.",
                    data.local_mem_store_sectors
                ),
                action: "Reduce local variable usage, apply __launch_bounds__ to limit register \
                         pressure, or simplify per-thread logic."
                    .into(),
                source: String::new(),
            });
        }

        if data.warps_active_pct < 50.0 {
            let (limiter, limit_val) = data.occupancy_limiter();
            let limiter_detail = if limit_val > 0.0 {
                format!(
                    " Primary limiter: {} ({:.0} blocks/SM). \
                     Registers/thread: {:.0}, Shared mem/block: {:.1} KB.",
                    limiter, limit_val, data.registers_per_thread, data.shared_mem_per_block_kb
                )
            } else {
                String::new()
            };

            findings.push(Finding {
                severity: Severity::Warning,
                title: "Low Occupancy".into(),
                detail: format!(
                    "Active warps at {:.1}% of peak (threshold: 50%). \
                     SM resources may be underutilized.{}",
                    data.warps_active_pct, limiter_detail
                ),
                action: match limiter {
                    "Registers" => "Reduce register usage with __launch_bounds__ or simplify per-thread logic. \
                                    Consider trading registers for shared memory."
                        .into(),
                    "Shared Memory" => "Reduce shared memory per block, use dynamic sizing, \
                                        or split into smaller tile sizes."
                        .into(),
                    "Warps" => "Reduce block size to allow more concurrent blocks per SM.".into(),
                    _ => "Reduce per-thread register count or shared memory usage to allow more \
                          concurrent warps. Consider adjusting block size."
                        .into(),
                },
                source: String::new(),
            });
        }

        if data.theoretical_occupancy_pct > 0.0 && data.warps_active_pct > 0.0 {
            let ratio = data.warps_active_pct / data.theoretical_occupancy_pct;
            if ratio < 0.5 && data.theoretical_occupancy_pct > 10.0 {
                findings.push(Finding {
                    severity: Severity::Warning,
                    title: "Large Theoretical-Achieved Occupancy Gap".into(),
                    detail: format!(
                        "Theoretical occupancy: {:.1}%, achieved: {:.1}% ({:.0}% of theoretical). \
                         Significant runtime inefficiency is preventing warps from staying active.",
                        data.theoretical_occupancy_pct,
                        data.warps_active_pct,
                        ratio * 100.0
                    ),
                    action: "Investigate workload imbalance, excessive synchronization, \
                             or tail effects from grid size. Use CUDA occupancy calculator \
                             to verify theoretical limits."
                        .into(),
                    source: String::new(),
                });
            }
        }

        findings
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::KernelData;
    use crate::severity::Severity;

    fn make_data(local_sectors: f64, warps_pct: f64) -> KernelData {
        KernelData {
            kernel_name: "test".into(),
            device_name: "GPU".into(),
            grid_size: "1,1,1".into(),
            block_size: "256,1,1".into(),
            duration_us: 100.0,
            sm_throughput_pct: 50.0,
            mem_throughput_pct: 50.0,
            l1_sectors_global_ld: 0.0,
            l1_requests_global_ld: 0.0,
            l1_hit_rate_pct: 50.0,
            l2_hit_rate_pct: 70.0,
            l1_sectors_global_st: 0.0,
            l1_requests_global_st: 0.0,
            shared_mem_bank_conflicts: 0.0,
            local_mem_store_sectors: local_sectors,
            warps_active_pct: warps_pct,
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
    fn test_register_spill_critical() {
        let data = make_data(100.0, 60.0);
        let findings = OccupancyAnalyzer.analyze(&data);
        assert!(findings
            .iter()
            .any(|f| f.severity == Severity::Critical && f.title.contains("Register Spill")));
    }

    #[test]
    fn test_no_spill_no_critical() {
        let data = make_data(0.0, 60.0);
        let findings = OccupancyAnalyzer.analyze(&data);
        assert!(!findings.iter().any(|f| f.severity == Severity::Critical));
    }

    #[test]
    fn test_low_occupancy_warning() {
        let data = make_data(0.0, 30.0);
        let findings = OccupancyAnalyzer.analyze(&data);
        assert!(findings
            .iter()
            .any(|f| f.severity == Severity::Warning && f.title.contains("Occupancy")));
    }

    #[test]
    fn test_good_occupancy_no_warning() {
        let data = make_data(0.0, 70.0);
        let findings = OccupancyAnalyzer.analyze(&data);
        assert!(findings.is_empty());
    }
}
