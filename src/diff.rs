use std::collections::HashMap;

use serde::Serialize;

use crate::metrics::KernelData;

#[derive(Debug, Serialize)]
pub struct KernelDiff {
    pub kernel_name: String,
    pub before_us: f64,
    pub after_us: f64,
    /// Positive = slower (regression), negative = faster (improvement).
    pub delta_us: f64,
    pub delta_pct: f64,
}

#[derive(Debug, Serialize)]
pub struct DiffReport {
    pub regressions: Vec<KernelDiff>,
    pub improvements: Vec<KernelDiff>,
    pub new_kernels: Vec<String>,
    pub removed_kernels: Vec<String>,
}

/// Compare two sets of kernel profiles and produce a diff report.
///
/// Kernels are matched by name. When a kernel appears in both profiles,
/// duration deltas are computed. Unmatched kernels are classified as
/// new or removed.
pub fn diff_profiles(before: &[KernelData], after: &[KernelData]) -> DiffReport {
    let before_map: HashMap<&str, &KernelData> = before
        .iter()
        .map(|k| (k.kernel_name.as_str(), k))
        .collect();
    let after_map: HashMap<&str, &KernelData> = after
        .iter()
        .map(|k| (k.kernel_name.as_str(), k))
        .collect();

    let mut regressions = Vec::new();
    let mut improvements = Vec::new();
    let mut removed_kernels = Vec::new();

    for (name, bk) in &before_map {
        if let Some(ak) = after_map.get(name) {
            let delta_us = ak.duration_us - bk.duration_us;
            let delta_pct = if bk.duration_us > 0.0 {
                (delta_us / bk.duration_us) * 100.0
            } else {
                0.0
            };
            let diff = KernelDiff {
                kernel_name: name.to_string(),
                before_us: bk.duration_us,
                after_us: ak.duration_us,
                delta_us,
                delta_pct,
            };
            if delta_us > 0.0 {
                regressions.push(diff);
            } else if delta_us < 0.0 {
                improvements.push(diff);
            }
        } else {
            removed_kernels.push(name.to_string());
        }
    }

    let new_kernels: Vec<String> = after_map
        .keys()
        .filter(|name| !before_map.contains_key(*name))
        .map(|name| name.to_string())
        .collect();

    regressions.sort_by(|a, b| b.delta_us.partial_cmp(&a.delta_us).unwrap());
    improvements.sort_by(|a, b| a.delta_us.partial_cmp(&b.delta_us).unwrap());

    DiffReport {
        regressions,
        improvements,
        new_kernels,
        removed_kernels,
    }
}
