use crate::analyzer::Analyzer;
use crate::metrics::KernelData;
use crate::severity::{Finding, Severity};

pub struct LaunchConfigAnalyzer;

impl Analyzer for LaunchConfigAnalyzer {
    fn name(&self) -> &str {
        "Launch Configuration Analysis"
    }

    fn description(&self) -> &str {
        "Analyzes launch parameters (registers, shared memory, block size) and identifies the occupancy limiter"
    }

    fn category(&self) -> &str {
        "launch_config"
    }

    fn analyze(&self, data: &KernelData) -> Vec<Finding> {
        let mut findings = Vec::new();

        analyze_occupancy_limiter(data, &mut findings);
        analyze_occupancy_gap(data, &mut findings);
        analyze_register_pressure(data, &mut findings);

        findings
    }
}

fn analyze_occupancy_limiter(data: &KernelData, findings: &mut Vec<Finding>) {
    let (limiter_name, limiter_blocks) = data.occupancy_limiter();
    if limiter_blocks == 0.0 {
        return;
    }

    let all_equal = data.occupancy_limit_registers == data.occupancy_limit_shared_mem
        && data.occupancy_limit_shared_mem == data.occupancy_limit_warps;
    if all_equal {
        return;
    }

    let severity = if limiter_blocks <= 2.0 {
        Severity::Critical
    } else if limiter_blocks <= 4.0 {
        Severity::Warning
    } else {
        Severity::Info
    };

    let action = match limiter_name {
        "Registers" => format!(
            "Register pressure limits to {:.0} blocks/SM. Current: {:.0} registers/thread. \
             Apply __launch_bounds__(threads, minBlocks) to cap register usage, \
             simplify per-thread logic, or move data to shared memory.",
            limiter_blocks, data.registers_per_thread
        ),
        "Shared Memory" => format!(
            "Shared memory limits to {:.0} blocks/SM. Current: {:.1} KB/block. \
             Reduce shared memory allocation, use dynamic shared memory sizing, \
             or trade shared memory for register usage.",
            limiter_blocks, data.shared_mem_per_block_kb
        ),
        "Warps" => format!(
            "Block size limits to {:.0} blocks/SM (too many warps per block). \
             Consider reducing block size to allow more concurrent blocks.",
            limiter_blocks
        ),
        _ => format!(
            "Occupancy is limited to {:.0} blocks/SM by {}.",
            limiter_blocks, limiter_name
        ),
    };

    findings.push(Finding {
        severity,
        title: format!("Occupancy Limited by {}", limiter_name),
        detail: format!(
            "Occupancy limiter breakdown: Registers={:.0}, Shared Mem={:.0}, Warps={:.0}, Blocks={:.0} blocks/SM. \
             {} is the binding constraint at {:.0} blocks/SM.",
            data.occupancy_limit_registers,
            data.occupancy_limit_shared_mem,
            data.occupancy_limit_warps,
            data.occupancy_limit_blocks,
            limiter_name,
            limiter_blocks
        ),
        action,
        source: String::new(),
    });
}

fn analyze_occupancy_gap(data: &KernelData, findings: &mut Vec<Finding>) {
    if data.theoretical_occupancy_pct == 0.0 || data.warps_active_pct == 0.0 {
        return;
    }

    let gap = data.warps_active_pct - data.theoretical_occupancy_pct;
    if gap <= 0.0 {
        return;
    }

    // Achieved > theoretical can happen with dynamic parallelism or measurement artifacts.
    // Only report when achieved is significantly below theoretical.
    let achieved_ratio = data.warps_active_pct / data.theoretical_occupancy_pct;
    if achieved_ratio >= 0.8 {
        return;
    }

    findings.push(Finding {
        severity: Severity::Warning,
        title: "Achieved Occupancy Below Theoretical".into(),
        detail: format!(
            "Theoretical occupancy: {:.1}%, achieved: {:.1}% ({:.0}% of theoretical). \
             Runtime factors (imbalanced workload, synchronization, or tail effects) \
             are preventing full utilization.",
            data.theoretical_occupancy_pct,
            data.warps_active_pct,
            achieved_ratio * 100.0
        ),
        action: "Check for workload imbalance across thread blocks, reduce synchronization \
                 points, or improve load balancing. Tail effects from grid size not being \
                 a multiple of SM count can also cause this gap."
            .into(),
        source: String::new(),
    });
}

fn analyze_register_pressure(data: &KernelData, findings: &mut Vec<Finding>) {
    if data.registers_per_thread < 64.0 {
        return;
    }

    let severity = if data.registers_per_thread >= 128.0 {
        Severity::Critical
    } else if data.registers_per_thread >= 96.0 {
        Severity::Warning
    } else {
        Severity::Info
    };

    findings.push(Finding {
        severity,
        title: format!("High Register Usage ({:.0} regs/thread)", data.registers_per_thread),
        detail: format!(
            "Using {:.0} registers per thread (allocated {:.0} after rounding). \
             On SM_{}, the register file is 64K registers per SM.",
            data.registers_per_thread,
            (data.registers_per_thread / 8.0).ceil() * 8.0,
            data.arch_sm
        ),
        action: "Use __launch_bounds__(maxThreadsPerBlock, minBlocksPerMultiprocessor) \
                 to hint the compiler to reduce register usage. Alternatively, refactor \
                 the kernel to reduce live variables or use shared memory for intermediates."
            .into(),
        source: String::new(),
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::KernelData;

    fn make_launch_data(
        regs: f64,
        shmem_kb: f64,
        limit_regs: f64,
        limit_shmem: f64,
        limit_warps: f64,
        theoretical: f64,
        achieved: f64,
    ) -> KernelData {
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
            warps_active_pct: achieved,
            registers_per_thread: regs,
            shared_mem_per_block_kb: shmem_kb,
            occupancy_limit_registers: limit_regs,
            occupancy_limit_shared_mem: limit_shmem,
            occupancy_limit_warps: limit_warps,
            occupancy_limit_blocks: 32.0,
            theoretical_occupancy_pct: theoretical,
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
    fn test_register_limited() {
        let data = make_launch_data(86.0, 34.0, 2.0, 3.0, 8.0, 6.25, 23.87);
        let findings = LaunchConfigAnalyzer.analyze(&data);
        assert!(findings
            .iter()
            .any(|f| f.title.contains("Limited by Registers")));
    }

    #[test]
    fn test_high_register_usage() {
        let data = make_launch_data(128.0, 0.0, 1.0, 32.0, 8.0, 3.0, 3.0);
        let findings = LaunchConfigAnalyzer.analyze(&data);
        assert!(findings
            .iter()
            .any(|f| f.title.contains("High Register Usage")));
    }

    #[test]
    fn test_no_issues_balanced() {
        let data = make_launch_data(32.0, 0.0, 8.0, 8.0, 8.0, 50.0, 48.0);
        let findings = LaunchConfigAnalyzer.analyze(&data);
        assert!(findings.is_empty());
    }
}
