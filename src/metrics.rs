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

// Occupancy & register spills
pub const LOCAL_MEM_STORE_SECTORS: &str =
    "l1tex__t_sectors_pipe_lsu_mem_local_op_st.sum [sector]";
pub const WARPS_ACTIVE_PCT: &str =
    "sm__warps_active.avg.pct_of_peak_sustained_active [%]";

// Instruction / Tensor Core
pub const TENSOR_HMMA_PCT: &str =
    "sm__pipe_tensor_op_hmma_cycles_active.avg.pct_of_peak_sustained_active [%]";

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

    // Occupancy
    pub local_mem_store_sectors: f64,
    pub warps_active_pct: f64,

    // Instruction
    pub tensor_core_hmma_pct: f64,

    // Arch
    pub arch_sm: u32,

    // Arch-specific extras
    pub tma_cycles_active_pct: f64,
    pub lsu_pipe_utilization_pct: f64,
}
