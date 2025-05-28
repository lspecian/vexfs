/*
 * VexFS Control Tool Progress Indicators
 * Copyright 2025 VexFS Contributors
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

//! Progress indicators for long-running operations

use indicatif::{ProgressBar as IndicatifProgressBar, ProgressStyle as IndicatifProgressStyle};
use std::time::Duration;

/// Progress bar wrapper for vexctl operations
pub struct ProgressBar {
    inner: IndicatifProgressBar,
}

impl ProgressBar {
    /// Create a new progress bar with the specified length
    pub fn new(length: u64) -> Self {
        let pb = IndicatifProgressBar::new(length);
        pb.set_style(
            IndicatifProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
                .unwrap()
                .progress_chars("#>-"),
        );
        Self { inner: pb }
    }

    /// Create a new spinner for indeterminate progress
    pub fn new_spinner() -> Self {
        let pb = IndicatifProgressBar::new_spinner();
        pb.set_style(
            IndicatifProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap(),
        );
        pb.enable_steady_tick(Duration::from_millis(100));
        Self { inner: pb }
    }

    /// Set the progress bar message
    pub fn set_message(&self, message: &str) {
        self.inner.set_message(message.to_string());
    }

    /// Set the current position
    pub fn set_position(&self, position: u64) {
        self.inner.set_position(position);
    }

    /// Increment the position by 1
    pub fn inc(&self, delta: u64) {
        self.inner.inc(delta);
    }

    /// Finish the progress bar with a message
    pub fn finish_with_message(&self, message: &str) {
        self.inner.finish_with_message(message.to_string());
    }

    /// Finish the progress bar and clear it
    pub fn finish_and_clear(&self) {
        self.inner.finish_and_clear();
    }

    /// Set the progress bar style
    pub fn set_style(&self, style: ProgressStyle) {
        match style {
            ProgressStyle::Bar => {
                self.inner.set_style(
                    IndicatifProgressStyle::default_bar()
                        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
                        .unwrap()
                        .progress_chars("#>-"),
                );
            }
            ProgressStyle::Spinner => {
                self.inner.set_style(
                    IndicatifProgressStyle::default_spinner()
                        .template("{spinner:.green} {msg}")
                        .unwrap(),
                );
            }
            ProgressStyle::Bytes => {
                self.inner.set_style(
                    IndicatifProgressStyle::default_bar()
                        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} {msg}")
                        .unwrap()
                        .progress_chars("#>-"),
                );
            }
            ProgressStyle::Percentage => {
                self.inner.set_style(
                    IndicatifProgressStyle::default_bar()
                        .template("{spinner:.green} [{elapsed_precise}] {percent}% {msg}")
                        .unwrap(),
                );
            }
        }
    }

    /// Set the length of the progress bar
    pub fn set_length(&self, length: u64) {
        self.inner.set_length(length);
    }

    /// Check if the progress bar is hidden
    pub fn is_hidden(&self) -> bool {
        self.inner.is_hidden()
    }
}

/// Progress bar styles
#[derive(Debug, Clone, Copy)]
pub enum ProgressStyle {
    /// Standard progress bar with position/length
    Bar,
    /// Spinner for indeterminate progress
    Spinner,
    /// Progress bar showing bytes transferred
    Bytes,
    /// Progress bar showing percentage
    Percentage,
}

/// Progress tracker for operations with multiple steps
pub struct MultiProgress {
    steps: Vec<ProgressStep>,
    current_step: usize,
}

impl MultiProgress {
    /// Create a new multi-step progress tracker
    pub fn new(steps: Vec<&str>) -> Self {
        let progress_steps = steps
            .into_iter()
            .map(|name| ProgressStep {
                name: name.to_string(),
                progress: None,
                completed: false,
            })
            .collect();

        Self {
            steps: progress_steps,
            current_step: 0,
        }
    }

    /// Start the next step
    pub fn next_step(&mut self, length: Option<u64>) -> Option<&ProgressBar> {
        if self.current_step < self.steps.len() {
            let step = &mut self.steps[self.current_step];
            
            let pb = if let Some(len) = length {
                ProgressBar::new(len)
            } else {
                ProgressBar::new_spinner()
            };
            
            pb.set_message(&format!("Step {}: {}", self.current_step + 1, step.name));
            step.progress = Some(pb);
            
            self.current_step += 1;
            step.progress.as_ref()
        } else {
            None
        }
    }

    /// Complete the current step
    pub fn complete_current_step(&mut self, message: Option<&str>) {
        if self.current_step > 0 {
            let step = &mut self.steps[self.current_step - 1];
            step.completed = true;
            
            if let Some(ref pb) = step.progress {
                let msg = message.unwrap_or("Completed");
                pb.finish_with_message(msg);
            }
        }
    }

    /// Get the current step progress bar
    pub fn current_progress(&self) -> Option<&ProgressBar> {
        if self.current_step > 0 && self.current_step <= self.steps.len() {
            self.steps[self.current_step - 1].progress.as_ref()
        } else {
            None
        }
    }

    /// Check if all steps are completed
    pub fn is_complete(&self) -> bool {
        self.steps.iter().all(|step| step.completed)
    }

    /// Get the completion percentage
    pub fn completion_percentage(&self) -> f64 {
        if self.steps.is_empty() {
            return 100.0;
        }
        
        let completed = self.steps.iter().filter(|step| step.completed).count();
        (completed as f64 / self.steps.len() as f64) * 100.0
    }
}

struct ProgressStep {
    name: String,
    progress: Option<ProgressBar>,
    completed: bool,
}

/// Helper function to create a progress bar for file operations
pub fn file_progress(total_bytes: u64, filename: &str) -> ProgressBar {
    let pb = ProgressBar::new(total_bytes);
    pb.set_style(ProgressStyle::Bytes);
    pb.set_message(&format!("Processing {}", filename));
    pb
}

/// Helper function to create a progress bar for search operations
pub fn search_progress(total_vectors: u64) -> ProgressBar {
    let pb = ProgressBar::new(total_vectors);
    pb.set_style(ProgressStyle::Bar);
    pb.set_message("Searching vectors");
    pb
}

/// Helper function to create a spinner for indeterminate operations
pub fn operation_spinner(operation: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_message(operation);
    pb
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_bar_creation() {
        let pb = ProgressBar::new(100);
        assert!(!pb.is_hidden());
    }

    #[test]
    fn test_spinner_creation() {
        let spinner = ProgressBar::new_spinner();
        assert!(!spinner.is_hidden());
    }

    #[test]
    fn test_multi_progress() {
        let mut multi = MultiProgress::new(vec!["Step 1", "Step 2", "Step 3"]);
        
        assert_eq!(multi.completion_percentage(), 0.0);
        assert!(!multi.is_complete());
        
        multi.next_step(Some(100));
        multi.complete_current_step(Some("Done"));
        
        assert_eq!(multi.completion_percentage(), 33.333333333333336);
        assert!(!multi.is_complete());
        
        multi.next_step(Some(50));
        multi.complete_current_step(Some("Done"));
        
        multi.next_step(None);
        multi.complete_current_step(Some("Done"));
        
        assert_eq!(multi.completion_percentage(), 100.0);
        assert!(multi.is_complete());
    }

    #[test]
    fn test_helper_functions() {
        let file_pb = file_progress(1024, "test.txt");
        assert!(!file_pb.is_hidden());
        
        let search_pb = search_progress(1000);
        assert!(!search_pb.is_hidden());
        
        let spinner = operation_spinner("Testing");
        assert!(!spinner.is_hidden());
    }
}