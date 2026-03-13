use std::collections::HashMap;
use std::path::Path;

use anyhow::{bail, Context, Result};

use crate::metrics::{self, KernelData};

/// Parse an NCU CSV export into one or more `KernelData` entries.
///
/// NCU CSV is a vertical key-value format (`metric_name,value`).
/// When multiple kernels are profiled, the same keys repeat — each time
/// `"Function Name"` appears, it starts a new kernel page.
pub fn parse_ncu_csv(path: &Path) -> Result<Vec<KernelData>> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_path(path)
        .with_context(|| format!("Failed to open CSV file: {}", path.display()))?;

    let mut pages: Vec<HashMap<String, String>> = Vec::new();
    let mut current: HashMap<String, String> = HashMap::new();

    for result in reader.records() {
        let record = result.with_context(|| "Failed to read CSV record")?;
        if record.len() < 2 {
            continue;
        }
        let key = record[0].trim().to_string();
        if key.starts_with("breakdown:") {
            continue;
        }
        let value = record[1].trim().to_string();

        if key == metrics::FUNCTION_NAME && current.contains_key(metrics::FUNCTION_NAME) {
            pages.push(std::mem::take(&mut current));
        }
        current.insert(key, value);
    }

    if !current.is_empty() {
        pages.push(current);
    }

    if pages.is_empty() {
        bail!("No kernel data found in CSV file: {}", path.display());
    }

    pages
        .iter()
        .enumerate()
        .map(|(i, map)| {
            build_kernel_data(map)
                .with_context(|| format!("Failed to parse kernel #{} from CSV", i + 1))
        })
        .collect()
}

fn build_kernel_data(map: &HashMap<String, String>) -> Result<KernelData> {
    Ok(KernelData {
        kernel_name: get_str(map, metrics::FUNCTION_NAME),
        device_name: get_str(map, metrics::DEVICE_NAME),
        grid_size: get_str(map, metrics::GRID_SIZE),
        block_size: get_str(map, metrics::BLOCK_SIZE),
        duration_us: get_f64(map, metrics::GPU_TIME_DURATION),

        sm_throughput_pct: get_f64(map, metrics::SM_THROUGHPUT),
        mem_throughput_pct: get_f64(map, metrics::MEM_THROUGHPUT),

        l1_sectors_global_ld: get_f64(map, metrics::L1_SECTORS_GLOBAL_LD),
        l1_requests_global_ld: get_f64(map, metrics::L1_REQUESTS_GLOBAL_LD),
        l1_hit_rate_pct: get_f64(map, metrics::L1_HIT_RATE),
        l2_hit_rate_pct: get_f64(map, metrics::L2_HIT_RATE),
        l1_sectors_global_st: get_f64(map, metrics::L1_SECTORS_GLOBAL_ST),
        l1_requests_global_st: get_f64(map, metrics::L1_REQUESTS_GLOBAL_ST),

        shared_mem_bank_conflicts: get_f64(map, metrics::SHMEM_BANK_CONFLICTS),

        local_mem_store_sectors: get_f64(map, metrics::LOCAL_MEM_STORE_SECTORS),
        warps_active_pct: get_f64(map, metrics::WARPS_ACTIVE_PCT),

        registers_per_thread: get_f64(map, metrics::REGISTERS_PER_THREAD),
        shared_mem_per_block_kb: get_f64(map, metrics::SHARED_MEM_PER_BLOCK),
        occupancy_limit_registers: get_f64(map, metrics::OCCUPANCY_LIMIT_REGISTERS),
        occupancy_limit_shared_mem: get_f64(map, metrics::OCCUPANCY_LIMIT_SHARED_MEM),
        occupancy_limit_warps: get_f64(map, metrics::OCCUPANCY_LIMIT_WARPS),
        occupancy_limit_blocks: get_f64(map, metrics::OCCUPANCY_LIMIT_BLOCKS),
        theoretical_occupancy_pct: get_f64(map, metrics::THEORETICAL_OCCUPANCY_PCT),

        dram_read_gbytes: get_f64(map, metrics::DRAM_BYTES_READ),
        dram_write_gbytes: get_f64(map, metrics::DRAM_BYTES_WRITE),
        dram_throughput_pct: get_f64(map, metrics::DRAM_THROUGHPUT_PCT),

        tensor_core_hmma_pct: get_f64(map, metrics::TENSOR_HMMA_PCT),

        pipe_fma_pct: get_f64(map, metrics::PIPE_FMA_PCT),
        pipe_alu_pct: get_f64(map, metrics::PIPE_ALU_PCT),
        pipe_lsu_pct: get_f64(map, metrics::PIPE_LSU_PCT),
        pipe_tensor_pct: get_f64(map, metrics::PIPE_TENSOR_PCT),
        pipe_fma_fp16_pct: get_f64(map, metrics::PIPE_FMA_FP16_PCT),

        avg_thread_executed: get_f64(map, metrics::AVG_THREAD_EXECUTED),
        avg_thread_executed_true: get_f64(map, metrics::AVG_THREAD_EXECUTED_TRUE),

        warps_eligible_per_cycle: get_f64(map, metrics::WARPS_ELIGIBLE_PER_CYCLE),

        stall_long_scoreboard: get_f64(map, metrics::STALL_LONG_SCOREBOARD),
        stall_short_scoreboard: get_f64(map, metrics::STALL_SHORT_SCOREBOARD),
        stall_wait: get_f64(map, metrics::STALL_WAIT),
        stall_sleeping: get_f64(map, metrics::STALL_SLEEPING),
        stall_barrier: get_f64(map, metrics::STALL_BARRIER),
        stall_mio_throttle: get_f64(map, metrics::STALL_MIO_THROTTLE),
        stall_lg_throttle: get_f64(map, metrics::STALL_LG_THROTTLE),
        stall_math_pipe_throttle: get_f64(map, metrics::STALL_MATH_PIPE_THROTTLE),
        stall_drain: get_f64(map, metrics::STALL_DRAIN),
        stall_not_selected: get_f64(map, metrics::STALL_NOT_SELECTED),
        stall_selected: get_f64(map, metrics::STALL_SELECTED),

        arch_sm: parse_arch_sm(get_f64(map, metrics::DEVICE_ARCH) as u32),

        tma_cycles_active_pct: get_f64(map, metrics::TMA_CYCLES_ACTIVE),
        lsu_pipe_utilization_pct: get_f64(map, metrics::LSU_PIPE_UTILIZATION),
    })
}

fn get_str(map: &HashMap<String, String>, key: &str) -> String {
    map.get(key).cloned().unwrap_or_default()
}

/// Extract a numeric value, stripping sampling-count suffixes like ` {929}`.
fn get_f64(map: &HashMap<String, String>, key: &str) -> f64 {
    map.get(key)
        .map(|v| strip_sample_suffix(v))
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(0.0)
}

/// `"5280946840 {929}"` -> `"5280946840"`
fn strip_sample_suffix(s: &str) -> String {
    match s.find('{') {
        Some(pos) => s[..pos].trim().to_string(),
        None => s.trim().to_string(),
    }
}

/// NCU `device__attribute_architecture` encodes SM version as `version * 10 * 4`.
/// e.g. 384 -> 384/4 = 96 -> SM 9.0 (Hopper), 800 -> SM 8.0 (Ampere).
/// We return the "major * 10 + minor" form, e.g. 90, 80, 86, 100.
fn parse_arch_sm(raw: u32) -> u32 {
    if raw == 0 {
        return 0;
    }
    match raw {
        800 | 80 => 80,
        860 | 86 => 86,
        890 | 89 => 89,
        900 | 90 => 90,
        1000 | 100 => 100,
        // NCU internal architecture IDs (observed in practice)
        384 => 90, // H800/H100 Hopper
        _ => {
            if raw > 200 {
                raw / 10
            } else {
                raw
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_sample_suffix() {
        assert_eq!(strip_sample_suffix("5280946840 {929}"), "5280946840");
        assert_eq!(strip_sample_suffix("741.86"), "741.86");
        assert_eq!(strip_sample_suffix("0 {8}"), "0");
    }

    #[test]
    fn test_parse_arch_sm() {
        assert_eq!(parse_arch_sm(384), 90);
        assert_eq!(parse_arch_sm(800), 80);
        assert_eq!(parse_arch_sm(860), 86);
        assert_eq!(parse_arch_sm(900), 90);
        assert_eq!(parse_arch_sm(1000), 100);
        assert_eq!(parse_arch_sm(0), 0);
    }
}
