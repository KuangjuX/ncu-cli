use crate::analyzer::Analyzer;
use crate::metrics::KernelData;
use crate::severity::{Finding, Severity};

pub struct InstructionAnalyzer;

impl Analyzer for InstructionAnalyzer {
    fn name(&self) -> &str {
        "Instruction Execution Analysis"
    }

    fn analyze(&self, data: &KernelData) -> Vec<Finding> {
        let mut findings = Vec::new();

        // Detect FP16 kernels by name heuristics (common patterns in CUTLASS/cuDNN)
        let name_lower = data.kernel_name.to_lowercase();
        let likely_fp16 = name_lower.contains("f16")
            || name_lower.contains("fp16")
            || name_lower.contains("half")
            || name_lower.contains("hmma")
            || name_lower.contains("h16");

        if likely_fp16 && data.tensor_core_hmma_pct < 10.0 {
            findings.push(Finding {
                severity: Severity::Warning,
                title: "Low Tensor Core Utilization".into(),
                detail: format!(
                    "Kernel appears to use FP16 data but Tensor Core (HMMA) utilization \
                     is only {:.1}% (threshold: 10%).",
                    data.tensor_core_hmma_pct
                ),
                action: "Use wmma or mma PTX instructions, or CUTLASS/cuBLAS with Tensor Core \
                         policies. Ensure matrix dimensions are multiples of 16."
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

    fn make_data(name: &str, hmma_pct: f64) -> KernelData {
        KernelData {
            kernel_name: name.into(),
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
            local_mem_store_sectors: 0.0,
            warps_active_pct: 60.0,
            tensor_core_hmma_pct: hmma_pct,
            arch_sm: 90,
            tma_cycles_active_pct: 0.0,
            lsu_pipe_utilization_pct: 0.0,
        }
    }

    #[test]
    fn test_fp16_low_tc_warns() {
        let data = make_data("my_kernel_fp16_gemm", 2.0);
        let findings = InstructionAnalyzer.analyze(&data);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].title.contains("Tensor Core"));
    }

    #[test]
    fn test_fp16_high_tc_no_warning() {
        let data = make_data("my_kernel_fp16_gemm", 50.0);
        let findings = InstructionAnalyzer.analyze(&data);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_non_fp16_kernel_no_warning() {
        let data = make_data("my_kernel_fp32_reduce", 1.0);
        let findings = InstructionAnalyzer.analyze(&data);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_f16_in_name_detected() {
        let data = make_data("tensorptrf16gmemalign", 0.5);
        let findings = InstructionAnalyzer.analyze(&data);
        assert_eq!(findings.len(), 1);
    }
}
