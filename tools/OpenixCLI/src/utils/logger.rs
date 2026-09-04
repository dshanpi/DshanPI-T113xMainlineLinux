//! Logger implementation
//!
//! Provides logging and progress reporting functionality for flash operations

use super::terminal::{log_debug, log_error, log_info, log_stage_complete, log_success, log_warn};
use crate::process::global_progress::set_tui_mode;
use crate::process::{ProgressReporter, StageType};
use serde_json::json;
use std::sync::Arc;

/// Logger
///
/// Provides a unified interface for logging and progress reporting
#[derive(Clone)]
pub struct Logger {
    verbose: bool,
    reporter: Arc<ProgressReporter>,
    jsonl_route: Option<&'static str>,
}

impl Logger {
    /// Create a new logger with default settings
    pub fn new() -> Self {
        Self {
            verbose: false,
            reporter: Arc::new(ProgressReporter::new()),
            jsonl_route: None,
        }
    }

    /// Create a new logger with verbose mode
    pub fn with_verbose(verbose: bool) -> Self {
        Self {
            verbose,
            reporter: Arc::new(ProgressReporter::new()),
            jsonl_route: None,
        }
    }

    /// Create a logger that emits one JSON object per line and no ANSI output.
    pub fn with_jsonl(verbose: bool, route: &'static str) -> Self {
        set_tui_mode(true);
        Self {
            verbose,
            reporter: Arc::new(ProgressReporter::new()),
            jsonl_route: Some(route),
        }
    }

    fn emit_log(&self, level: &str, message: &str) -> bool {
        if let Some(route) = self.jsonl_route {
            println!(
                "{}",
                json!({"event":"log","route":route,"level":level,"message":message})
            );
            true
        } else {
            false
        }
    }

    fn emit_progress(&self, event: &str) {
        let Some(route) = self.jsonl_route else {
            return;
        };
        let snapshot = self.reporter.snapshot();
        let stage = snapshot
            .stages
            .get(snapshot.current_stage_index)
            .map(|item| item.stage_type.key());
        println!(
            "{}",
            json!({
                "event":event,
                "route":route,
                "phase":stage,
                "progress":snapshot.precise_progress,
                "writtenBytes":snapshot.stage_progress,
                "totalBytes":snapshot.total_bytes,
                "speedBytesPerSecond":snapshot.speed,
                "partition":if snapshot.current_partition.is_empty() { None } else { Some(snapshot.current_partition) }
            })
        );
    }

    /// Log an info message
    pub fn info(&self, message: &str) {
        if !self.emit_log("info", message) {
            log_info(message);
        }
    }

    /// Log a success message
    #[allow(dead_code)]
    pub fn success(&self, message: &str) {
        if !self.emit_log("success", message) {
            log_success(message);
        }
    }

    /// Log a warning message
    pub fn warn(&self, message: &str) {
        if !self.emit_log("warning", message) {
            log_warn(message);
        }
    }

    /// Log an error message
    pub fn error(&self, message: &str) {
        if !self.emit_log("error", message) {
            log_error(message);
        }
    }

    /// Log a debug message (only if verbose mode is enabled)
    pub fn debug(&self, message: &str) {
        if self.verbose && !self.emit_log("debug", message) {
            log_debug(message);
        }
    }

    /// Log a stage completion message
    pub fn stage_complete(&self, message: &str) {
        if !self.emit_log("stage_complete", message) {
            log_stage_complete(message);
        }
    }

    /// Start global progress tracking
    pub fn start_global_progress(&self) {
        self.reporter.start();
        self.emit_progress("progress_started");
    }

    /// Define stages for progress tracking
    pub fn define_stages(&self, stages: &[StageType]) {
        self.reporter.define_stages(stages);
    }

    /// Begin a specific stage
    pub fn begin_stage(&self, stage_type: StageType) {
        self.reporter.begin_stage(stage_type);
        self.emit_progress("phase");
    }

    /// Set partition stage weight for progress calculation
    pub fn set_partition_stage_weight(&self, total_bytes: u64) {
        self.reporter.set_partition_stage_weight(total_bytes);
    }

    /// Set current partition name for display
    pub fn set_current_partition(&self, partition_name: &str) {
        self.reporter.set_current_partition(partition_name);
        self.emit_progress("partition");
    }

    /// Update progress (bytes written)
    #[allow(dead_code)]
    pub fn update_progress(&self, current: u64) {
        self.reporter.update_progress(current);
        self.emit_progress("progress");
    }

    /// Update progress with speed calculation
    pub fn update_progress_with_speed(&self, current: u64) {
        self.reporter.update_progress_with_speed(current);
        self.emit_progress("progress");
    }

    /// Mark current stage as completed
    pub fn complete_stage(&self) {
        self.reporter.complete_stage();
        self.emit_progress("phase_complete");
    }

    /// Finish progress tracking
    pub fn finish_progress(&self) {
        self.emit_progress("progress_complete");
        self.reporter.finish();
    }

    /// Update progress by percentage
    #[allow(dead_code)]
    pub fn update_progress_percent(&self, percent: u8) {
        self.reporter.update_progress_percent(percent);
        self.emit_progress("progress");
    }

    /// Get current progress percentage (0-100)
    #[allow(dead_code)]
    pub fn get_progress(&self) -> u8 {
        self.reporter.get_progress()
    }
}

impl Default for Logger {
    fn default() -> Self {
        Self::new()
    }
}
