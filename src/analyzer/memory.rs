use crate::analyzer::Analyzer;
use crate::metrics::KernelData;
use crate::severity::{Finding, Severity};

pub struct MemoryAnalyzer;

impl Analyzer for MemoryAnalyzer {
    fn name(&self) -> &str {
        "Memory Hierarchy Analysis"
    }

    fn analyze(&self, data: &KernelData) -> Vec<Finding> {
        let mut findings = Vec::new();

        // --- Coalescing check ---
        // Ratio of sectors to requests; for 32-bit data, ideal is 4 sectors/request.
        // A ratio > 8 indicates uncoalesced access.
        if data.l1_requests_global_ld > 0.0 {
            let ratio = data.l1_sectors_global_ld / data.l1_requests_global_ld;
            if ratio > 8.0 {
                findings.push(Finding {
                    severity: Severity::Warning,
                    title: "Uncoalesced Global Memory Access".into(),
                    detail: format!(
                        "L1 sectors/requests ratio is {ratio:.1} (ideal <= 4 for 32-bit). \
                         Threads are accessing non-contiguous memory addresses."
                    ),
                    action: "Switch to Structure-of-Arrays (SoA) layout, ensure threads in a warp \
                             access consecutive addresses, or pad data for alignment."
                        .into(),
                    source: String::new(),
                });
            }
        }

        // --- L1 cache hit rate ---
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

        // --- L2 cache hit rate ---
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

        findings
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
            local_mem_store_sectors: 0.0,
            warps_active_pct: 60.0,
            tensor_core_hmma_pct: 0.0,
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
}
