use crate::analyzer::Analyzer;
use crate::metrics::KernelData;
use crate::severity::{Finding, Severity};

pub struct WarpStallAnalyzer;

const TOP_N: usize = 3;
const DOMINANT_THRESHOLD_PCT: f64 = 25.0;
const NOTABLE_THRESHOLD_PCT: f64 = 10.0;

impl Analyzer for WarpStallAnalyzer {
    fn name(&self) -> &str {
        "Warp Stall Analysis"
    }

    fn description(&self) -> &str {
        "Identifies dominant warp stall reasons from PC sampling data and suggests targeted optimizations"
    }

    fn category(&self) -> &str {
        "warp_stall"
    }

    fn analyze(&self, data: &KernelData) -> Vec<Finding> {
        let mut findings = Vec::new();
        let breakdown = data.stall_breakdown();

        if breakdown.is_empty() {
            return findings;
        }

        if data.warps_eligible_per_cycle > 0.0 && data.warps_eligible_per_cycle < 1.0 {
            findings.push(Finding {
                severity: Severity::Warning,
                title: "Low Warp Scheduling Efficiency".into(),
                detail: format!(
                    "Only {:.2} eligible warps per cycle (ideal >= 2). \
                     The warp scheduler frequently has insufficient work to issue.",
                    data.warps_eligible_per_cycle
                ),
                action: "Increase occupancy or reduce per-warp latency to keep the scheduler busy."
                    .into(),
                source: String::new(),
            });
        }

        for &(reason, _count, pct) in breakdown.iter().take(TOP_N) {
            if pct < NOTABLE_THRESHOLD_PCT {
                continue;
            }
            let (severity, action) = stall_action(reason, pct);
            findings.push(Finding {
                severity,
                title: format!("Stall: {} ({:.1}%)", reason, pct),
                detail: format!(
                    "{} accounts for {:.1}% of all warp stall samples.",
                    reason, pct
                ),
                action: action.into(),
                source: String::new(),
            });
        }

        findings
    }
}

fn stall_action(reason: &str, pct: f64) -> (Severity, &'static str) {
    let severity = if pct >= DOMINANT_THRESHOLD_PCT {
        Severity::Critical
    } else {
        Severity::Warning
    };

    let action = match reason {
        "Long Scoreboard" => {
            "Warps are waiting for global/L2 memory. Use async copy (cp.async / TMA), \
             increase data prefetching, improve L2 cache locality, or restructure access patterns."
        }
        "Short Scoreboard" => {
            "Warps are waiting for shared memory or L1 results. Reduce shared memory bank conflicts \
             by padding arrays, reorder shared memory accesses, or reduce dependency chains."
        }
        "Wait" => {
            "Warps are stalled on explicit wait instructions (e.g., cp.async.wait_all, named barriers). \
             Overlap more computation with async operations, or reduce wait granularity."
        }
        "Sleeping" => {
            "Warps are explicitly sleeping (nanosleep or yield). Check for unnecessary sleep calls \
             or overly conservative synchronization in the kernel."
        }
        "Barrier" => {
            "Warps are stalled at __syncthreads() or barrier instructions. Reduce barrier frequency, \
             use warp-level primitives (__shfl_sync, cooperative_groups), or split work to avoid barriers."
        }
        "MIO Throttle" => {
            "The MIO (Memory Input/Output) pipeline is saturated. Reduce the rate of shared memory \
             or special function unit (SFU) operations, or interleave them with other instructions."
        }
        "LG Throttle" => {
            "The local/global memory pipeline is throttled. Reduce outstanding memory requests \
             or improve memory access patterns to reduce L1 tag pressure."
        }
        "Math Pipe Throttle" => {
            "The math (FMA/ALU) pipeline is fully utilized. This is generally positive for compute-bound \
             kernels. Consider using Tensor Cores or reducing instruction count via algorithmic changes."
        }
        "Drain" => {
            "Warps are draining at the end of the kernel or thread block. This is usually unavoidable \
             but can be reduced by balancing work across warps and avoiding tail effects."
        }
        "Not Selected" => {
            "Warps are eligible but not selected by the scheduler (contention). This indicates \
             good occupancy but scheduler pressure. Usually not actionable directly."
        }
        "Selected" => {
            "Warps were selected and issued. This represents productive work and is not a concern."
        }
        _ => "Unknown stall reason. Consult NCU documentation for details.",
    };

    (severity, action)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::KernelData;

    fn make_stall_data(long_sb: f64, short_sb: f64, wait: f64, eligible: f64) -> KernelData {
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
            warps_eligible_per_cycle: eligible,
            stall_long_scoreboard: long_sb,
            stall_short_scoreboard: short_sb,
            stall_wait: wait,
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
    fn test_dominant_long_scoreboard() {
        let data = make_stall_data(30000.0, 5000.0, 2000.0, 0.5);
        let findings = WarpStallAnalyzer.analyze(&data);
        assert!(findings.iter().any(|f| f.title.contains("Long Scoreboard")));
        assert!(findings
            .iter()
            .any(|f| f.severity == Severity::Critical && f.title.contains("Long Scoreboard")));
    }

    #[test]
    fn test_low_eligible_warps() {
        let data = make_stall_data(100.0, 100.0, 100.0, 0.3);
        let findings = WarpStallAnalyzer.analyze(&data);
        assert!(findings
            .iter()
            .any(|f| f.title.contains("Scheduling Efficiency")));
    }

    #[test]
    fn test_no_stall_data() {
        let data = make_stall_data(0.0, 0.0, 0.0, 2.0);
        let findings = WarpStallAnalyzer.analyze(&data);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_minor_stalls_ignored() {
        // All stall reasons below 10% threshold when spread across many categories
        let mut data = make_stall_data(5.0, 3.0, 2.0, 2.0);
        data.stall_sleeping = 5.0;
        data.stall_barrier = 5.0;
        data.stall_drain = 5.0;
        data.stall_not_selected = 5.0;
        data.stall_selected = 70.0;
        // "Selected" at 70% is productive work, the rest are all < 10%
        let findings = WarpStallAnalyzer.analyze(&data);
        // Only "Selected" exceeds threshold, but it gets "not a concern" action
        assert!(findings.iter().all(|f| f.title.contains("Selected")));
    }
}
