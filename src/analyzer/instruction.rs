use crate::analyzer::Analyzer;
use crate::metrics::KernelData;
use crate::severity::{Finding, Severity};

pub struct InstructionAnalyzer;

impl Analyzer for InstructionAnalyzer {
    fn name(&self) -> &str {
        "Instruction Execution Analysis"
    }

    fn description(&self) -> &str {
        "Analyzes instruction mix, Tensor Core utilization, thread divergence, and warp scheduling efficiency"
    }

    fn category(&self) -> &str {
        "instruction"
    }

    fn analyze(&self, data: &KernelData) -> Vec<Finding> {
        let mut findings = Vec::new();

        check_tensor_core_utilization(data, &mut findings);
        check_instruction_mix(data, &mut findings);
        check_thread_divergence(data, &mut findings);

        findings
    }
}

fn check_tensor_core_utilization(data: &KernelData, findings: &mut Vec<Finding>) {
    let name_lower = data.kernel_name.to_lowercase();
    let likely_fp16_by_name = name_lower.contains("f16")
        || name_lower.contains("fp16")
        || name_lower.contains("half")
        || name_lower.contains("hmma")
        || name_lower.contains("h16");

    let likely_fp16_by_pipe = data.pipe_fma_fp16_pct > 5.0;

    if (likely_fp16_by_name || likely_fp16_by_pipe) && data.tensor_core_hmma_pct < 10.0 {
        let detection = if likely_fp16_by_pipe && !likely_fp16_by_name {
            format!(
                "FP16 FMA pipe at {:.1}% indicates FP16 computation, ",
                data.pipe_fma_fp16_pct
            )
        } else {
            String::new()
        };
        findings.push(Finding {
            severity: Severity::Warning,
            title: "Low Tensor Core Utilization".into(),
            detail: format!(
                "{}Tensor Core (HMMA) utilization is only {:.1}% (threshold: 10%).",
                detection, data.tensor_core_hmma_pct
            ),
            action: "Use wmma or mma PTX instructions, or CUTLASS/cuBLAS with Tensor Core \
                     policies. Ensure matrix dimensions are multiples of 16."
                .into(),
            source: String::new(),
        });
    }
}

fn check_instruction_mix(data: &KernelData, findings: &mut Vec<Finding>) {
    let has_pipe_data =
        data.pipe_fma_pct > 0.0 || data.pipe_alu_pct > 0.0 || data.pipe_lsu_pct > 0.0;
    if !has_pipe_data {
        return;
    }

    // Flag if LSU pipe dominates (memory-instruction heavy kernel)
    let total_compute = data.pipe_fma_pct + data.pipe_alu_pct;
    if data.pipe_lsu_pct > total_compute && data.pipe_lsu_pct > 15.0 {
        findings.push(Finding {
            severity: Severity::Info,
            title: "Memory-Instruction Dominated".into(),
            detail: format!(
                "Instruction mix: FMA {:.1}%, ALU {:.1}%, LSU {:.1}%, Tensor {:.1}%. \
                 Load/store instructions dominate over compute.",
                data.pipe_fma_pct, data.pipe_alu_pct, data.pipe_lsu_pct, data.pipe_tensor_pct
            ),
            action: "Consider fusing memory operations, using vectorized loads (float4/int4), \
                     or restructuring to increase compute-to-memory instruction ratio."
                .into(),
            source: String::new(),
        });
    }
}

fn check_thread_divergence(data: &KernelData, findings: &mut Vec<Finding>) {
    let divergence = data.divergence_pct();
    if divergence < 5.0 {
        return;
    }

    let severity = if divergence >= 20.0 {
        Severity::Warning
    } else {
        Severity::Info
    };

    findings.push(Finding {
        severity,
        title: format!("Thread Divergence ({:.1}%)", divergence),
        detail: format!(
            "Average threads executed: {:.0}, active (predicated-on): {:.0}. \
             {:.1}% of threads are predicated off, indicating warp divergence.",
            data.avg_thread_executed, data.avg_thread_executed_true, divergence
        ),
        action: "Restructure conditional logic to reduce divergent branches within a warp. \
                 Consider sorting work items by execution path or using warp-level voting."
            .into(),
        source: String::new(),
    });
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
            tensor_core_hmma_pct: hmma_pct,
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
        assert!(findings.iter().any(|f| f.title.contains("Tensor Core")));
    }

    #[test]
    fn test_fp16_pipe_detection_without_name() {
        let mut data = make_data("generic_kernel", 1.0);
        data.pipe_fma_fp16_pct = 10.0;
        let findings = InstructionAnalyzer.analyze(&data);
        assert!(findings.iter().any(|f| f.title.contains("Tensor Core")));
    }

    #[test]
    fn test_thread_divergence_detected() {
        let mut data = make_data("my_kernel", 50.0);
        data.avg_thread_executed = 32000.0;
        data.avg_thread_executed_true = 24000.0;
        let findings = InstructionAnalyzer.analyze(&data);
        assert!(findings.iter().any(|f| f.title.contains("Divergence")));
    }

    #[test]
    fn test_no_divergence_when_close() {
        let mut data = make_data("my_kernel", 50.0);
        data.avg_thread_executed = 32000.0;
        data.avg_thread_executed_true = 31500.0;
        let findings = InstructionAnalyzer.analyze(&data);
        assert!(!findings.iter().any(|f| f.title.contains("Divergence")));
    }

    #[test]
    fn test_lsu_dominated_instruction_mix() {
        let mut data = make_data("my_kernel", 50.0);
        data.pipe_lsu_pct = 20.0;
        data.pipe_fma_pct = 5.0;
        data.pipe_alu_pct = 3.0;
        let findings = InstructionAnalyzer.analyze(&data);
        assert!(findings
            .iter()
            .any(|f| f.title.contains("Memory-Instruction")));
    }
}
