/*
 * VexFS Control Tool Output Formatter
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

//! Output formatting utilities

use crate::Result;
use console::{style, Color, Style};
use serde::Serialize;
use serde_json;
use std::fmt;
use tabled::{Table, Tabled};

/// Output format options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// Human-readable format with colors and formatting
    Human,
    /// JSON format for machine consumption
    Json,
    /// Table format for structured data
    Table,
    /// Compact format for minimal output
    Compact,
}

impl fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OutputFormat::Human => write!(f, "human"),
            OutputFormat::Json => write!(f, "json"),
            OutputFormat::Table => write!(f, "table"),
            OutputFormat::Compact => write!(f, "compact"),
        }
    }
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "human" => Ok(OutputFormat::Human),
            "json" => Ok(OutputFormat::Json),
            "table" => Ok(OutputFormat::Table),
            "compact" => Ok(OutputFormat::Compact),
            _ => Err(format!("Invalid output format: {}", s)),
        }
    }
}

impl clap::ValueEnum for OutputFormat {
    fn value_variants<'a>() -> &'a [Self] {
        &[
            OutputFormat::Human,
            OutputFormat::Json,
            OutputFormat::Table,
            OutputFormat::Compact,
        ]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        Some(match self {
            OutputFormat::Human => clap::builder::PossibleValue::new("human"),
            OutputFormat::Json => clap::builder::PossibleValue::new("json"),
            OutputFormat::Table => clap::builder::PossibleValue::new("table"),
            OutputFormat::Compact => clap::builder::PossibleValue::new("compact"),
        })
    }
}

/// Output formatter for different display formats
pub struct Formatter {
    format: OutputFormat,
    colored: bool,
}

impl Formatter {
    /// Create a new formatter with the specified format
    pub fn new(format: OutputFormat) -> Self {
        Self {
            format,
            colored: console::colors_enabled(),
        }
    }

    /// Create a formatter with color support disabled
    pub fn without_color(format: OutputFormat) -> Self {
        Self {
            format,
            colored: false,
        }
    }

    /// Format a serializable value according to the configured format
    pub fn format<T: Serialize>(&self, value: &T) -> Result<String> {
        match self.format {
            OutputFormat::Json => Ok(serde_json::to_string_pretty(value)?),
            OutputFormat::Human => self.format_human(value),
            OutputFormat::Table => self.format_table(value),
            OutputFormat::Compact => self.format_compact(value),
        }
    }

    /// Format a table of data
    pub fn format_table_data<T: Tabled + Serialize>(&self, data: &[T]) -> Result<String> {
        match self.format {
            OutputFormat::Json => Ok(serde_json::to_string_pretty(data)?),
            OutputFormat::Table | OutputFormat::Human => {
                if data.is_empty() {
                    Ok("No data to display".to_string())
                } else {
                    Ok(Table::new(data).to_string())
                }
            }
            OutputFormat::Compact => {
                Ok(format!("{} items", data.len()))
            }
        }
    }

    /// Format a success message
    pub fn success(&self, message: &str) -> String {
        if self.colored {
            style(format!("✓ {}", message)).green().to_string()
        } else {
            format!("SUCCESS: {}", message)
        }
    }

    /// Format an error message
    pub fn error(&self, message: &str) -> String {
        if self.colored {
            style(format!("✗ {}", message)).red().to_string()
        } else {
            format!("ERROR: {}", message)
        }
    }

    /// Format a warning message
    pub fn warning(&self, message: &str) -> String {
        if self.colored {
            style(format!("⚠ {}", message)).yellow().to_string()
        } else {
            format!("WARNING: {}", message)
        }
    }

    /// Format an info message
    pub fn info(&self, message: &str) -> String {
        if self.colored {
            style(format!("ℹ {}", message)).blue().to_string()
        } else {
            format!("INFO: {}", message)
        }
    }

    /// Format a header
    pub fn header(&self, text: &str) -> String {
        if self.colored {
            style(text).bold().underlined().to_string()
        } else {
            format!("=== {} ===", text)
        }
    }

    /// Format a subheader
    pub fn subheader(&self, text: &str) -> String {
        if self.colored {
            style(text).bold().to_string()
        } else {
            format!("--- {} ---", text)
        }
    }

    /// Format a key-value pair
    pub fn key_value(&self, key: &str, value: &str) -> String {
        if self.colored {
            format!("{}: {}", style(key).bold(), value)
        } else {
            format!("{}: {}", key, value)
        }
    }

    /// Format a percentage value
    pub fn percentage(&self, value: f64) -> String {
        let formatted = format!("{:.1}%", value);
        if self.colored {
            let color = if value >= 80.0 {
                Color::Green
            } else if value >= 60.0 {
                Color::Yellow
            } else {
                Color::Red
            };
            style(formatted).fg(color).to_string()
        } else {
            formatted
        }
    }

    /// Format a file size in human-readable format
    pub fn file_size(&self, bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        let mut size = bytes as f64;
        let mut unit_index = 0;

        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }

        if unit_index == 0 {
            format!("{} {}", bytes, UNITS[unit_index])
        } else {
            format!("{:.1} {}", size, UNITS[unit_index])
        }
    }

    /// Format a duration in human-readable format
    pub fn duration(&self, microseconds: u64) -> String {
        if microseconds < 1000 {
            format!("{}μs", microseconds)
        } else if microseconds < 1_000_000 {
            format!("{:.1}ms", microseconds as f64 / 1000.0)
        } else if microseconds < 60_000_000 {
            format!("{:.1}s", microseconds as f64 / 1_000_000.0)
        } else {
            let seconds = microseconds / 1_000_000;
            let minutes = seconds / 60;
            let remaining_seconds = seconds % 60;
            format!("{}m{}s", minutes, remaining_seconds)
        }
    }

    /// Format a timestamp
    pub fn timestamp(&self, timestamp: u64) -> String {
        use chrono::{DateTime, Utc, TimeZone};
        
        let dt = Utc.timestamp_opt(timestamp as i64, 0).single();
        match dt {
            Some(dt) => dt.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
            None => format!("Invalid timestamp: {}", timestamp),
        }
    }

    /// Format a list of items
    pub fn list(&self, items: &[String]) -> String {
        if items.is_empty() {
            return "None".to_string();
        }

        match self.format {
            OutputFormat::Json => serde_json::to_string_pretty(items).unwrap_or_default(),
            OutputFormat::Compact => items.join(", "),
            _ => {
                items
                    .iter()
                    .map(|item| format!("  • {}", item))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
        }
    }

    fn format_human<T: Serialize>(&self, value: &T) -> Result<String> {
        // For human format, we'll use JSON as a fallback
        // Individual commands should implement their own human-readable formatting
        Ok(serde_json::to_string_pretty(value)?)
    }

    fn format_table<T: Serialize>(&self, value: &T) -> Result<String> {
        // For table format, we'll use JSON as a fallback
        // Individual commands should implement their own table formatting
        Ok(serde_json::to_string_pretty(value)?)
    }

    fn format_compact<T: Serialize>(&self, value: &T) -> Result<String> {
        // For compact format, use single-line JSON
        Ok(serde_json::to_string(value)?)
    }
}

impl Default for Formatter {
    fn default() -> Self {
        Self::new(OutputFormat::Human)
    }
}

/// Helper trait for formatting status indicators
pub trait StatusFormat {
    fn format_status(&self, formatter: &Formatter) -> String;
}

/// Status indicator for various operations
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Status {
    Ok,
    Warning,
    Error,
    Unknown,
    InProgress,
    Pending,
}

impl StatusFormat for Status {
    fn format_status(&self, formatter: &Formatter) -> String {
        if formatter.colored {
            match self {
                Status::Ok => style("OK").green().to_string(),
                Status::Warning => style("WARNING").yellow().to_string(),
                Status::Error => style("ERROR").red().to_string(),
                Status::Unknown => style("UNKNOWN").dim().to_string(),
                Status::InProgress => style("IN PROGRESS").blue().to_string(),
                Status::Pending => style("PENDING").cyan().to_string(),
            }
        } else {
            match self {
                Status::Ok => "OK".to_string(),
                Status::Warning => "WARNING".to_string(),
                Status::Error => "ERROR".to_string(),
                Status::Unknown => "UNKNOWN".to_string(),
                Status::InProgress => "IN PROGRESS".to_string(),
                Status::Pending => "PENDING".to_string(),
            }
        }
    }
}

/// Helper function to format a vector of key-value pairs
pub fn format_properties(formatter: &Formatter, properties: &[(String, String)]) -> String {
    properties
        .iter()
        .map(|(key, value)| formatter.key_value(key, value))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_format_parsing() {
        assert_eq!("human".parse::<OutputFormat>().unwrap(), OutputFormat::Human);
        assert_eq!("json".parse::<OutputFormat>().unwrap(), OutputFormat::Json);
        assert_eq!("table".parse::<OutputFormat>().unwrap(), OutputFormat::Table);
        assert_eq!("compact".parse::<OutputFormat>().unwrap(), OutputFormat::Compact);
        assert!("invalid".parse::<OutputFormat>().is_err());
    }

    #[test]
    fn test_file_size_formatting() {
        let formatter = Formatter::without_color(OutputFormat::Human);
        assert_eq!(formatter.file_size(512), "512 B");
        assert_eq!(formatter.file_size(1024), "1.0 KB");
        assert_eq!(formatter.file_size(1536), "1.5 KB");
        assert_eq!(formatter.file_size(1048576), "1.0 MB");
    }

    #[test]
    fn test_duration_formatting() {
        let formatter = Formatter::without_color(OutputFormat::Human);
        assert_eq!(formatter.duration(500), "500μs");
        assert_eq!(formatter.duration(1500), "1.5ms");
        assert_eq!(formatter.duration(1500000), "1.5s");
        assert_eq!(formatter.duration(90000000), "1m30s");
    }

    #[test]
    fn test_percentage_formatting() {
        let formatter = Formatter::without_color(OutputFormat::Human);
        assert_eq!(formatter.percentage(85.7), "85.7%");
        assert_eq!(formatter.percentage(0.0), "0.0%");
        assert_eq!(formatter.percentage(100.0), "100.0%");
    }
}