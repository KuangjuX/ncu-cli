use crate::analyzer::Analyzer;
use crate::metrics::KernelData;
use crate::severity::{Finding, Severity};

pub struct MemoryAnalyzer;

impl Analyzer for MemoryAnalyzer {
    fn name(&self) -> &str {
        "Memory Hierarchy Analysis"
    }

    fn description(&self) -> &str {
        "Detects uncoalesced global memory access and low L1/L2 cache hit rates"
    }

    fn category(&self) -> &str {
        "memory"
    }

    fn analyze(&self, data: &KernelData) -> Vec<Finding> {
        let mut findings = Vec::new();

        check_load_coalescing(data, &mut findings);
        check_store_coalescing(data, &mut findings);
        check_l1_hit_rate(data, &mut findings);
        check_l2_hit_rate(data, &mut findings);
        check_bank_conflicts(data, &mut findings);
        check_dram_bandwidth(data, &mut findings);

        findings
    }
}

fn check_load_coalescing(data: &KernelData, findings: &mut Vec<Finding>) {
    if data.l1_requests_global_ld > 0.0 {
        let ratio = data.l1_sectors_global_ld / data.l1_requests_global_ld;
        if ratio > 8.0 {
            findings.push(Finding {
                severity: Severity::Warning,
                title: "Uncoalesced Global Load Access".into(),
                detail: format!(
                    "L1 load sectors/requests ratio is {ratio:.1} (ideal <= 4 for 32-bit). \
                     Threads are loading from non-contiguous memory addresses."
                ),
                action: "Switch to Structure-of-Arrays (SoA) layout, ensure threads in a warp \
                         access consecutive addresses, or pad data for alignment."
                    .into(),
                source: String::new(),
            });
        }
    }
}

fn check_store_coalescing(data: &KernelData, findings: &mut Vec<Finding>) {
    if data.l1_requests_global_st > 0.0 {
        let ratio = data.l1_sectors_global_st / data.l1_requests_global_st;
        if ratio > 8.0 {
            findings.push(Finding {
                severity: Severity::Warning,
                title: "Uncoalesced Global Store Access".into(),
                detail: format!(
                    "L1 store sectors/requests ratio is {ratio:.1} (ideal <= 4 for 32-bit). \
                     Threads are writing to non-contiguous memory addresses."
                ),
                action: "Ensure store addresses are contiguous within a warp. \
                         Consider using shared memory as a staging buffer for scatter writes."
                    .into(),
                source: String::new(),
            });
        }
    }
}

fn check_l1_hit_rate(data: &KernelData, findings: &mut Vec<Finding>) {
    if data.l1_hit_rate_pct < 20.0 {
        findings.push(Finding {
            severity: Severity::Warning,
            title: "Low L1 Cache Hit Rate".into(),
            detail: format!(
                "L1 texture cache hit rate is {:.1}% (threshold: 20%).",
                data.l1_hit_rate_pct
            ),
            action: "Use __shared__ memory for frequently accessed data, \
                     apply tiling strategies to improve spatial locality."
                .into(),
            source: String::new(),
        });
    }
}

fn check_l2_hit_rate(data: &KernelData, findings: &mut Vec<Finding>) {
    if data.l2_hit_rate_pct < 50.0 {
        findings.push(Finding {
            severity: Severity::Warning,
            title: "Low L2 Cache Hit Rate".into(),
            detail: format!(
                "L2 cache hit rate is {:.1}% (threshold: 50%).",
                data.l2_hit_rate_pct
            ),
            action: "Consider using cudaAccessPolicyWindow (Ampere+) to pin hot data in L2, \
                     or restructure access patterns to improve temporal locality."
                .into(),
            source: String::new(),
        });
    }
}

fn check_bank_conflicts(data: &KernelData, findings: &mut Vec<Finding>) {
    if data.shared_mem_bank_conflicts > 100_000.0 {
        let severity = if data.shared_mem_bank_conflicts > 1_000_000.0 {
            Severity::Critical
        } else {
            Severity::Warning
        };
        findings.push(Finding {
            severity,
            title: "Shared Memory Bank Conflicts".into(),
            detail: format!(
                "Detected {:.0} shared memory bank conflicts. \
                 Bank conflicts serialize shared memory accesses within a warp, \
                 directly increasing latency.",
                data.shared_mem_bank_conflicts
            ),
            action: "Pad shared memory arrays (e.g., add +1 column) to avoid stride-based conflicts. \
                     Rearrange access patterns so threads in a warp access different banks. \
                     For ldgsts (async copy), ensure source alignment matches bank layout."
                .into(),
            source: String::new(),
        });
    }
}

fn check_dram_bandwidth(data: &KernelData, findings: &mut Vec<Finding>) {
    if data.dram_throughput_pct > 80.0 {
        let total_gb = data.dram_read_gbytes + data.dram_write_gbytes;
        let rw_ratio = if data.dram_write_gbytes > 0.0 {
            data.dram_read_gbytes / data.dram_write_gbytes
        } else {
            f64::INFINITY
        };

        let mut detail = format!(
            "DRAM throughput at {:.1}% of peak. Total transfer: {:.2} GB (read: {:.2} GB, write: {:.2} GB).",
            data.dram_throughput_pct, total_gb, data.dram_read_gbytes, data.dram_write_gbytes
        );

        if rw_ratio < 1.5 && rw_ratio > 0.0 {
            detail.push_str(&format!(
                " Read/write ratio is {:.2}:1, indicating heavy write-back traffic.",
                rw_ratio
            ));
        }

        findings.push(Finding {
            severity: Severity::Info,
            title: format!("High DRAM Bandwidth Utilization ({:.1}%)", data.dram_throughput_pct),
            detail,
            action: "Kernel is near DRAM bandwidth ceiling. Reduce data movement via \
                     compression, mixed precision, or algorithmic changes that improve \
                     arithmetic intensity. Consider L2 persistence hints to reduce DRAM traffic."
                .into(),
            source: String::new(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::KernelData;
    fn make_data(sectors: f64, requests: f64, l1_hit: f64, l2_hit: f64) -> KernelData {
        KernelData {
            kernel_name: "test".into(),
            device_name: "GPU".into(),
            grid_size: "1,1,1".into(),
            block_size: "256,1,1".into(),
            duration_us: 100.0,
            sm_throughput_pct: 50.0,
            mem_throughput_pct: 50.0,
            l1_sectors_global_ld: sectors,
            l1_requests_global_ld: requests,
            l1_hit_rate_pct: l1_hit,
            l2_hit_rate_pct: l2_hit,
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
    fn test_uncoalesced_access_detected() {
        let data = make_data(33554432.0, 2097152.0, 50.0, 70.0);
        let findings = MemoryAnalyzer.analyze(&data);
        assert!(findings.iter().any(|f| f.title.contains("Uncoalesced")));
    }

    #[test]
    fn test_coalesced_access_no_warning() {
        let data = make_data(4000.0, 1000.0, 50.0, 70.0);
        let findings = MemoryAnalyzer.analyze(&data);
        assert!(!findings.iter().any(|f| f.title.contains("Uncoalesced")));
    }

    #[test]
    fn test_low_l1_hit_rate() {
        let data = make_data(0.0, 0.0, 5.0, 70.0);
        let findings = MemoryAnalyzer.analyze(&data);
        assert!(findings.iter().any(|f| f.title.contains("L1 Cache")));
    }

    #[test]
    fn test_low_l2_hit_rate() {
        let data = make_data(0.0, 0.0, 50.0, 30.0);
        let findings = MemoryAnalyzer.analyze(&data);
        assert!(findings.iter().any(|f| f.title.contains("L2 Cache")));
    }

    #[test]
    fn test_healthy_cache_no_warnings() {
        let data = make_data(4000.0, 1000.0, 60.0, 80.0);
        let findings = MemoryAnalyzer.analyze(&data);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_store_uncoalesced() {
        let mut data = make_data(0.0, 0.0, 60.0, 80.0);
        data.l1_sectors_global_st = 33554432.0;
        data.l1_requests_global_st = 2097152.0;
        let findings = MemoryAnalyzer.analyze(&data);
        assert!(findings
            .iter()
            .any(|f| f.title.contains("Uncoalesced Global Store")));
    }

    #[test]
    fn test_bank_conflicts_critical() {
        let mut data = make_data(0.0, 0.0, 60.0, 80.0);
        data.shared_mem_bank_conflicts = 2_000_000.0;
        let findings = MemoryAnalyzer.analyze(&data);
        assert!(findings
            .iter()
            .any(|f| f.severity == Severity::Critical && f.title.contains("Bank Conflicts")));
    }

    #[test]
    fn test_bank_conflicts_warning() {
        let mut data = make_data(0.0, 0.0, 60.0, 80.0);
        data.shared_mem_bank_conflicts = 500_000.0;
        let findings = MemoryAnalyzer.analyze(&data);
        assert!(findings
            .iter()
            .any(|f| f.severity == Severity::Warning && f.title.contains("Bank Conflicts")));
    }

    #[test]
    fn test_high_dram_bandwidth() {
        let mut data = make_data(0.0, 0.0, 60.0, 80.0);
        data.dram_throughput_pct = 85.0;
        data.dram_read_gbytes = 1.07;
        data.dram_write_gbytes = 1.05;
        let findings = MemoryAnalyzer.analyze(&data);
        assert!(findings
            .iter()
            .any(|f| f.title.contains("DRAM Bandwidth")));
    }
}
