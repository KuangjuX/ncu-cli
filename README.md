# ncu-cli

Automated CUDA kernel performance diagnostics from NVIDIA Nsight Compute (NCU) CSV exports.

Parses NCU profiling data, applies roofline analysis and architecture-aware heuristics, and outputs actionable optimization suggestions in the terminal.

## Features

- **Roofline Analysis** — classifies kernels as Compute Bound, Memory Bound, Balanced, or Latency Bound
- **Memory Hierarchy Diagnostics** — detects uncoalesced access, low L1/L2 cache hit rates
- **Occupancy & Register Spill Detection** — flags register spills (Critical) and low warp occupancy
- **Instruction Analysis** — identifies FP16 kernels with low Tensor Core utilization
- **Architecture-Specific Rules** — tailored advice for Ampere (cp.async), Hopper (TMA), and Blackwell (FP4/FP6)
- **Colored Terminal Output** — severity-coded findings with comfy-table formatted metrics

## Usage

Export profiling data from NCU:

```bash
ncu --csv --page raw -o profile_output ./your_cuda_app
```

Run the analyzer:

```bash
ncu-cli --input profile_output.csv
```

## Example Output

```
════════════════════════════════════════════════════════════════════════
  Kernel: kernel_cutlass_softmax_fp16...
  Arch:  SM_90 (Hopper)
  Device: NVIDIA H800
  Duration: 741.86 us
  Main Bottleneck: Memory Bound
════════════════════════════════════════════════════════════════════════

  [Metrics Overview]
  ╭──────────────────────────┬────────┬──────────╮
  │ Metric                   ┆ Value  ┆ Status   │
  ╞══════════════════════════╪════════╪══════════╡
  │ SM Throughput            ┆ 27.8%  ┆ Very Low │
  │ Memory Throughput        ┆ 85.6%  ┆ OK       │
  │ Occupancy (Active Warps) ┆ 23.9%  ┆ Very Low │
  │ ...                      ┆ ...    ┆ ...      │
  ╰──────────────────────────┴────────┴──────────╯

  [Analysis & Suggestions]

  1. [WARNING] Memory Bound
     Detail: Memory throughput (85.6%) significantly exceeds SM throughput (27.8%).
     Action: Check memory access patterns, improve L2 cache hit rate, ...

  2. [WARNING] Hopper: TMA Engine Underutilized
     Detail: Kernel is memory-bound but TMA pipe activity is only 0.0%.
     Action: Restructure memory access to use TMA-based async bulk copy ...
```

## Build

```bash
cargo build --release
```

## Test

```bash
cargo test
```

## License

See [LICENSE](LICENSE).
