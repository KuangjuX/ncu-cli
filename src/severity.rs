use std::fmt;

use colored::Colorize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Info,
    Warning,
    Critical,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Severity::Info => "INFO".cyan(),
            Severity::Warning => "WARNING".yellow(),
            Severity::Critical => "CRITICAL".red().bold(),
        };
        write!(f, "{label}")
    }
}

#[derive(Debug, Clone)]
pub struct Finding {
    pub severity: Severity,
    pub title: String,
    pub detail: String,
    pub action: String,
    /// Which analyzer produced this finding (populated by the pipeline).
    pub source: String,
}

