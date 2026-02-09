use serde::{Deserialize, Serialize};

/// Enum representing different size units for formatting file sizes
#[derive(Debug, Clone)]
pub enum SizeUnit {
    Bytes,
    Kilobytes,
    Megabytes,
    Gigabytes,
    Terabytes,
}

/// Enum representing sorting criteria for files
#[derive(Debug, Clone)]
pub enum SortBy {
    Name,
    Size,
    Date,
}

/// Struct containing detailed information about a file or directory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub size_human: String,
    pub file_type: String,
    pub created: Option<String>,
    pub modified: Option<String>,
    pub permissions: String,
    pub is_directory: bool,
}

impl SizeUnit {
    /// Parse a size unit from a string
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "b" | "bytes" => Ok(SizeUnit::Bytes),
            "kb" | "kilobytes" => Ok(SizeUnit::Kilobytes),
            "mb" | "megabytes" => Ok(SizeUnit::Megabytes),
            "gb" | "gigabytes" => Ok(SizeUnit::Gigabytes),
            "tb" | "terabytes" => Ok(SizeUnit::Terabytes),
            "auto" => Ok(SizeUnit::Bytes),
            _ => Err(format!("Invalid size unit: {}", s)),
        }
    }

    /// Format a byte count using this unit
    pub fn format_size(&self, bytes: u64) -> String {
        match self {
            SizeUnit::Bytes => format!("{} B", bytes),
            SizeUnit::Kilobytes => format!("{:.2} KB", bytes as f64 / 1024.0),
            SizeUnit::Megabytes => format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0)),
            SizeUnit::Gigabytes => format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0)),
            SizeUnit::Terabytes => format!("{:.2} TB", bytes as f64 / (1024.0 * 1024.0 * 1024.0 * 1024.0)),
        }
    }

    /// Automatically choose the best unit for a byte count
    pub fn auto_format_size(bytes: u64) -> String {
        let units = [
            (SizeUnit::Terabytes, 1024u64.pow(4)),
            (SizeUnit::Gigabytes, 1024u64.pow(3)),
            (SizeUnit::Megabytes, 1024u64.pow(2)),
            (SizeUnit::Kilobytes, 1024u64),
            (SizeUnit::Bytes, 1),
        ];

        for (unit, threshold) in units.iter() {
            if bytes >= *threshold {
                return unit.format_size(bytes);
            }
        }
        format!("{} B", bytes)
    }
}
