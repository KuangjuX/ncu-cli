# ncu-cli

Automated CUDA kernel performance diagnostics from NVIDIA Nsight Compute (NCU) CSV exports.

Parses NCU profiling data, applies roofline analysis and architecture-aware heuristics, and outputs actionable optimization suggestions — in the terminal, JSON, CSV, or Markdown.

Inspired by [nsys-ai](https://github.com/GindaChen/nsys-ai), which provides AI-powered analysis for Nsight Systems profiles. ncu-cli brings a similar philosophy — structured analysis skills, profile diffing, and actionable diagnostics — to the Nsight Compute side of the GPU profiling workflow.

## Features

- **Roofline Analysis** — classifies kernels as Compute Bound, Memory Bound (DRAM/L2/L1 sub-levels), Balanced, or Latency Bound
- **Memory Hierarchy Diagnostics** — detects uncoalesced loads/stores, low L1/L2 cache hit rates, shared memory bank conflicts, and high DRAM bandwidth utilization
- **Occupancy & Launch Config Analysis** — flags register spills, low warp occupancy, occupancy limiters (registers / shared memory / warps), and theoretical-vs-achieved occupancy gaps
- **Warp Stall Analysis** — identifies dominant stall reasons from PC sampling data (Long Scoreboard, Barrier, MIO Throttle, etc.) with targeted optimization advice
- **Instruction Mix Analysis** — detects FP16 kernels with low Tensor Core utilization, LSU-dominated instruction mix, and thread divergence
- **Architecture-Specific Rules** — tailored advice for Ampere (cp.async), Hopper (TMA), and Blackwell (FP4/FP6)
- **Profile Diff** — compare two NCU CSV exports side-by-side to spot regressions and improvements
- **Modular Skill System** — run individual analysis skills independently or all at once
- **Multi-Format Output** — Terminal (colored, severity-coded), JSON, CSV, and Markdown

## Quick Start

### 1. Export profiling data from NCU

```bash
ncu --csv --page raw -o profile_output ./your_cuda_app
```

### 2. Run the analyzer

```bash
# Full analysis (default)
ncu-cli analyze profile_output.csv

# Backward-compatible shorthand
ncu-cli --input profile_output.csv

# Filter to a specific kernel
ncu-cli analyze profile_output.csv --kernel softmax
```

### 3. Explore further

```bash
# Profile metadata (device, arch, kernel count)
ncu-cli info profile_output.csv

# Summary table of all kernels
ncu-cli summary profile_output.csv

# Compare two profiles
ncu-cli diff before.csv after.csv

# Export structured data
ncu-cli export profile_output.csv --format json -o kernels.json
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

  1. [WARNING] Memory Bound (DRAM-Bound)
     Detail: Memory throughput (85.6%) significantly exceeds SM throughput (27.8%).
             L1 hit: 45.2%, L2 hit: 32.1%, DRAM throughput: 85.0%.
     Action: DRAM bandwidth is the bottleneck. Reduce data movement via mixed
             precision, compression, or algorithmic changes to improve arithmetic
             intensity.

  2. [WARNING] Hopper: TMA Engine Underutilized
     Detail: Kernel is memory-bound but TMA pipe activity is only 0.0%.
     Action: Restructure memory access to use TMA-based async bulk copy
             (e.g., via CUTLASS 3.x or CuTe TMA descriptors).

  3. [CRITICAL] Stall: Long Scoreboard (42.3%)
     Detail: Long Scoreboard accounts for 42.3% of all warp stall samples.
     Action: Use async copy (cp.async / TMA), increase data prefetching,
             improve L2 cache locality, or restructure access patterns.
```

## Commands

| Command   | Description                                     |
| --------- | ----------------------------------------------- |
| `analyze` | Full kernel diagnostics (default)               |
| `info`    | Profile metadata — device, arch, kernel count   |
| `summary` | Summary table of all kernels                    |
| `diff`    | Compare two profiles side-by-side               |
| `export`  | Export kernel data as JSON, CSV, or Markdown     |
| `skill`   | List or run individual analysis skills          |

All commands support `--format` (terminal / json / csv / markdown) and `--output` to write to a file.

## Skills (Analysis Building Blocks)

ncu-cli ships with 7 built-in analysis skills — self-contained diagnostic modules that can be run independently:

```bash
# List all available skills
ncu-cli skill list

# Run a specific skill
ncu-cli skill run roofline profile.csv
ncu-cli skill run memory profile.csv --kernel gemm
ncu-cli skill run warp_stall profile.csv --format json
```

| Skill           | What it analyzes                                                           |
| --------------- | -------------------------------------------------------------------------- |
| `roofline`      | Compute/memory/latency bound classification with DRAM/L2/L1 sub-levels    |
| `memory`        | Coalescing, L1/L2 hit rates, bank conflicts, DRAM bandwidth               |
| `occupancy`     | Register spills, warp occupancy, theoretical-vs-achieved gap               |
| `instruction`   | Tensor Core utilization, instruction mix, thread divergence                |
| `warp_stall`    | Dominant stall reasons from PC sampling with targeted actions              |
| `launch_config` | Occupancy limiters, register pressure, launch parameter analysis           |
| `arch`          | Architecture-specific advice (Ampere cp.async, Hopper TMA, Blackwell FP4) |

## Profile Diff

Compare two profiles to spot regressions and improvements after a code change:

```bash
ncu-cli diff before.csv after.csv
ncu-cli diff before.csv after.csv --format markdown -o diff.md
ncu-cli diff before.csv after.csv --format json
```

The report shows:
- **Top regressions** — kernels that got slower (by delta time and percentage)
- **Top improvements** — kernels that got faster
- **New / removed kernels** — workload changes across runs

## Build

```bash
cargo build --release
```

## Test

```bash
cargo test
```

## Acknowledgements

This project is inspired by [nsys-ai](https://github.com/GindaChen/nsys-ai), which pioneered the idea of structured, skill-based GPU profile analysis with actionable diagnostics. ncu-cli adapts this approach for NVIDIA Nsight Compute workflows.

## License

See [LICENSE](LICENSE).
