use crate::analyzer::Analyzer;
use crate::metrics::KernelData;
use crate::severity::{Finding, Severity};

pub struct OccupancyAnalyzer;

impl Analyzer for OccupancyAnalyzer {
    fn name(&self) -> &str {
        "Occupancy & Register Spill Analysis"
    }

    fn analyze(&self, data: &KernelData) -> Vec<Finding> {
        let mut findings = Vec::new();

        // --- Register spill detection (local memory store) ---
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

        // --- Low occupancy ---
        if data.warps_active_pct < 50.0 {
            findings.push(Finding {
                severity: Severity::Warning,
                title: "Low Occupancy".into(),
                detail: format!(
                    "Active warps at {:.1}% of peak (threshold: 50%). \
                     SM resources may be underutilized.",
                    data.warps_active_pct
                ),
                action: "Reduce per-thread register count or shared memory usage to allow more \
                         concurrent warps. Consider adjusting block size."
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
            local_mem_store_sectors: local_sectors,
            warps_active_pct: warps_pct,
            tensor_core_hmma_pct: 0.0,
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
