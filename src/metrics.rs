use serde::Serialize;

// NCU metric key constants — these match the CSV key column exactly (without unit suffixes).

pub const FUNCTION_NAME: &str = "Function Name";
pub const DEVICE_NAME: &str = "Device Name";
pub const GRID_SIZE: &str = "Grid Size";
pub const BLOCK_SIZE: &str = "Block Size [block]";
pub const GPU_TIME_DURATION: &str = "gpu__time_duration.sum [us]";

// Roofline
pub const SM_THROUGHPUT: &str =
    "sm__throughput.avg.pct_of_peak_sustained_elapsed [%]";
pub const MEM_THROUGHPUT: &str =
    "gpu__compute_memory_throughput.avg.pct_of_peak_sustained_elapsed [%]";

// Memory hierarchy
pub const L1_SECTORS_GLOBAL_LD: &str =
    "l1tex__t_sectors_pipe_lsu_mem_global_op_ld.sum [sector]";
pub const L1_REQUESTS_GLOBAL_LD: &str =
    "l1tex__t_requests_pipe_lsu_mem_global_op_ld.sum";
pub const L1_HIT_RATE: &str = "l1tex__t_sector_hit_rate.pct [%]";
pub const L2_HIT_RATE: &str = "lts__t_sector_hit_rate.pct [%]";
pub const L1_SECTORS_GLOBAL_ST: &str =
    "l1tex__t_sectors_pipe_lsu_mem_global_op_st.sum [sector]";
pub const L1_REQUESTS_GLOBAL_ST: &str =
    "l1tex__t_requests_pipe_lsu_mem_global_op_st.sum";

// Shared memory bank conflicts
pub const SHMEM_BANK_CONFLICTS: &str =
    "l1tex__data_bank_conflicts_pipe_lsu_mem_shared.sum";

// Occupancy & register spills
pub const LOCAL_MEM_STORE_SECTORS: &str =
    "l1tex__t_sectors_pipe_lsu_mem_local_op_st.sum [sector]";
pub const WARPS_ACTIVE_PCT: &str =
    "sm__warps_active.avg.pct_of_peak_sustained_active [%]";

// Launch configuration & occupancy limiters
pub const REGISTERS_PER_THREAD: &str = "launch__registers_per_thread [register/thread]";
pub const SHARED_MEM_PER_BLOCK: &str = "launch__shared_mem_per_block [Kbyte/block]";
pub const OCCUPANCY_LIMIT_REGISTERS: &str = "launch__occupancy_limit_registers [block]";
pub const OCCUPANCY_LIMIT_SHARED_MEM: &str = "launch__occupancy_limit_shared_mem [block]";
pub const OCCUPANCY_LIMIT_WARPS: &str = "launch__occupancy_limit_warps [block]";
pub const OCCUPANCY_LIMIT_BLOCKS: &str = "launch__occupancy_limit_blocks [block]";
pub const THEORETICAL_OCCUPANCY_PCT: &str = "launch__occupancy_cluster_pct [%]";

// DRAM bandwidth
pub const DRAM_BYTES_READ: &str = "dram__bytes_read.sum [Gbyte]";
pub const DRAM_BYTES_WRITE: &str = "dram__bytes_write.sum [Gbyte]";
pub const DRAM_THROUGHPUT_PCT: &str =
    "gpu__dram_throughput.avg.pct_of_peak_sustained_elapsed [%]";

// Instruction / Tensor Core
pub const TENSOR_HMMA_PCT: &str =
    "sm__pipe_tensor_op_hmma_cycles_active.avg.pct_of_peak_sustained_active [%]";

// Instruction mix (pipe utilization)
pub const PIPE_FMA_PCT: &str =
    "sm__inst_executed_pipe_fma.avg.pct_of_peak_sustained_active [%]";
pub const PIPE_ALU_PCT: &str =
    "sm__inst_executed_pipe_alu.avg.pct_of_peak_sustained_active [%]";
pub const PIPE_LSU_PCT: &str =
    "sm__inst_executed_pipe_lsu.avg.pct_of_peak_sustained_active [%]";
pub const PIPE_TENSOR_PCT: &str =
    "sm__pipe_tensor_cycles_active.avg.pct_of_peak_sustained_active [%]";
pub const PIPE_FMA_FP16_PCT: &str =
    "sm__inst_executed_pipe_fma_type_fp16.avg.pct_of_peak_sustained_active [%]";

// Thread divergence
pub const AVG_THREAD_EXECUTED: &str = "derived__avg_thread_executed [thread]";
pub const AVG_THREAD_EXECUTED_TRUE: &str = "derived__avg_thread_executed_true [thread]";

// Warp scheduling
pub const WARPS_ELIGIBLE_PER_CYCLE: &str =
    "smsp__warps_eligible.avg.per_cycle_active [warp]";

// Warp stall reasons (PC sampling)
pub const STALL_LONG_SCOREBOARD: &str =
    "smsp__pcsamp_warps_issue_stalled_long_scoreboard [warp]";
pub const STALL_SHORT_SCOREBOARD: &str =
    "smsp__pcsamp_warps_issue_stalled_short_scoreboard [warp]";
pub const STALL_WAIT: &str =
    "smsp__pcsamp_warps_issue_stalled_wait [warp]";
pub const STALL_SLEEPING: &str =
    "smsp__pcsamp_warps_issue_stalled_sleeping [warp]";
pub const STALL_BARRIER: &str =
    "smsp__pcsamp_warps_issue_stalled_barrier [warp]";
pub const STALL_MIO_THROTTLE: &str =
    "smsp__pcsamp_warps_issue_stalled_mio_throttle [warp]";
pub const STALL_LG_THROTTLE: &str =
    "smsp__pcsamp_warps_issue_stalled_lg_throttle [warp]";
pub const STALL_MATH_PIPE_THROTTLE: &str =
    "smsp__pcsamp_warps_issue_stalled_math_pipe_throttle [warp]";
pub const STALL_DRAIN: &str =
    "smsp__pcsamp_warps_issue_stalled_drain [warp]";
pub const STALL_NOT_SELECTED: &str =
    "smsp__pcsamp_warps_issue_stalled_not_selected [warp]";
pub const STALL_SELECTED: &str =
    "smsp__pcsamp_warps_issue_stalled_selected [warp]";

// Architecture detection
pub const DEVICE_ARCH: &str = "device__attribute_architecture";

// Hopper TMA
pub const TMA_CYCLES_ACTIVE: &str =
    "sm__pipe_tma_cycles_active.avg.pct_of_peak_sustained_elapsed";
// Ampere async copy (LSU pipe utilization as proxy)
pub const LSU_PIPE_UTILIZATION: &str =
    "smsp__inst_executed_pipe_lsu.avg.pct_of_peak_sustained_elapsed";

/// Parsed kernel profiling data extracted from an NCU CSV export.
#[derive(Debug, Clone, Serialize)]
pub struct KernelData {
    pub kernel_name: String,
    pub device_name: String,
    pub grid_size: String,
    pub block_size: String,
    pub duration_us: f64,

    // Roofline
    pub sm_throughput_pct: f64,
    pub mem_throughput_pct: f64,

    // Memory hierarchy
    pub l1_sectors_global_ld: f64,
    pub l1_requests_global_ld: f64,
    pub l1_hit_rate_pct: f64,
    pub l2_hit_rate_pct: f64,
    pub l1_sectors_global_st: f64,
    pub l1_requests_global_st: f64,

    // Shared memory
    pub shared_mem_bank_conflicts: f64,

    // Occupancy
    pub local_mem_store_sectors: f64,
    pub warps_active_pct: f64,

    // Launch configuration & occupancy limiters
    pub registers_per_thread: f64,
    pub shared_mem_per_block_kb: f64,
    pub occupancy_limit_registers: f64,
    pub occupancy_limit_shared_mem: f64,
    pub occupancy_limit_warps: f64,
    pub occupancy_limit_blocks: f64,
    pub theoretical_occupancy_pct: f64,

    // DRAM bandwidth
    pub dram_read_gbytes: f64,
    pub dram_write_gbytes: f64,
    pub dram_throughput_pct: f64,

    // Instruction
    pub tensor_core_hmma_pct: f64,

    // Instruction mix
    pub pipe_fma_pct: f64,
    pub pipe_alu_pct: f64,
    pub pipe_lsu_pct: f64,
    pub pipe_tensor_pct: f64,
    pub pipe_fma_fp16_pct: f64,

    // Thread divergence
    pub avg_thread_executed: f64,
    pub avg_thread_executed_true: f64,

    // Warp scheduling
    pub warps_eligible_per_cycle: f64,

    // Warp stall reasons (PC sampling counts)
    pub stall_long_scoreboard: f64,
    pub stall_short_scoreboard: f64,
    pub stall_wait: f64,
    pub stall_sleeping: f64,
    pub stall_barrier: f64,
    pub stall_mio_throttle: f64,
    pub stall_lg_throttle: f64,
    pub stall_math_pipe_throttle: f64,
    pub stall_drain: f64,
    pub stall_not_selected: f64,
    pub stall_selected: f64,

    // Arch
    pub arch_sm: u32,

    // Arch-specific extras
    pub tma_cycles_active_pct: f64,
    pub lsu_pipe_utilization_pct: f64,
}

impl KernelData {
    /// Total warp stall samples across all stall reasons.
    pub fn total_stall_samples(&self) -> f64 {
        self.stall_long_scoreboard
            + self.stall_short_scoreboard
            + self.stall_wait
            + self.stall_sleeping
            + self.stall_barrier
            + self.stall_mio_throttle
            + self.stall_lg_throttle
            + self.stall_math_pipe_throttle
            + self.stall_drain
            + self.stall_not_selected
            + self.stall_selected
    }

    /// Returns stall reasons sorted by count descending as (name, count, pct).
    pub fn stall_breakdown(&self) -> Vec<(&'static str, f64, f64)> {
        let total = self.total_stall_samples();
        if total == 0.0 {
            return Vec::new();
        }
        let mut reasons: Vec<(&str, f64)> = vec![
            ("Long Scoreboard", self.stall_long_scoreboard),
            ("Short Scoreboard", self.stall_short_scoreboard),
            ("Wait", self.stall_wait),
            ("Sleeping", self.stall_sleeping),
            ("Barrier", self.stall_barrier),
            ("MIO Throttle", self.stall_mio_throttle),
            ("LG Throttle", self.stall_lg_throttle),
            ("Math Pipe Throttle", self.stall_math_pipe_throttle),
            ("Drain", self.stall_drain),
            ("Not Selected", self.stall_not_selected),
            ("Selected", self.stall_selected),
        ];
        reasons.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        reasons
            .into_iter()
            .map(|(name, count)| (name, count, count / total * 100.0))
            .collect()
    }

    /// Thread divergence ratio: 0.0 = no divergence, higher = more divergence.
    pub fn divergence_pct(&self) -> f64 {
        if self.avg_thread_executed == 0.0 {
            return 0.0;
        }
        (1.0 - self.avg_thread_executed_true / self.avg_thread_executed) * 100.0
    }

    /// The minimum occupancy limiter (blocks per SM) and its name.
    pub fn occupancy_limiter(&self) -> (&'static str, f64) {
        let limiters = [
            ("Registers", self.occupancy_limit_registers),
            ("Shared Memory", self.occupancy_limit_shared_mem),
            ("Warps", self.occupancy_limit_warps),
            ("Blocks", self.occupancy_limit_blocks),
        ];
        limiters
            .into_iter()
            .filter(|(_, v)| *v > 0.0)
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .unwrap_or(("Unknown", 0.0))
    }
}
