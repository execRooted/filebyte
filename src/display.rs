use crate::types::FileInfo;
use colored::Colorize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Display files with various formatting options
pub fn display_files(
    files: &[FileInfo],
    size_unit: &crate::types::SizeUnit,
    color: bool,
    properties: bool,
    auto_size: bool,
    show_size: bool,
    export_path: Option<&String>,
    show_detailed_permissions: bool,
) {
    for file in files {
        let size_str = if auto_size {
            file.size_human.clone()
        } else {
            size_unit.format_size(file.size)
        };

        let mut output = if color {
            if file.is_directory {
                if show_size {
                    format!(
                        "{} {} {}",
                        file.name.blue().bold(),
                        size_str.cyan().bold(),
                        "[DIR]".blue()
                    )
                } else {
                    format!("{} {}", file.name.blue().bold(), "[DIR]".blue())
                }
            } else {
                if show_size {
                    format!("{} {}", file.name, size_str.green())
                } else {
                    let modified_short = file.modified.as_ref().map(|m| {
                        if let Some(date_part) = m.split(' ').next() {
                            date_part.to_string()
                        } else {
                            m.clone()
                        }
                    }).unwrap_or_else(|| "unknown".to_string());
                    let permissions_display = if show_detailed_permissions {
                        if let Ok(metadata) = fs::metadata(&Path::new(&file.path)) {
                            crate::utils::format_unix_permissions(&metadata, true)
                        } else {
                            file.permissions.clone()
                        }
                    } else {
                        file.permissions.clone()
                    };
                    format!(
                        "{} {} {}",
                        file.name,
                        permissions_display.magenta(),
                        modified_short.yellow()
                    )
                }
            }
        } else {
            if file.is_directory {
                if show_size {
                    format!("{} {} [DIR]", file.name, size_str)
                } else {
                    format!("{} [DIR]", file.name)
                }
            } else {
                if show_size {
                    format!("{} {}", file.name, size_str)
                } else {
                    let modified_short = file.modified.as_ref().map(|m| {
                        if let Some(date_part) = m.split(' ').next() {
                            date_part.to_string()
                        } else {
                            m.clone()
                        }
                    }).unwrap_or_else(|| "unknown".to_string());
                    format!("{} {} {}", file.name, file.permissions, modified_short)
                }
            }
        };

        if properties {
            let created_info = file.created.as_ref().map(|c| format!("Created: {}", c)).unwrap_or_default();
            let modified_info = file.modified.as_ref().map(|m| format!("Modified: {}", m)).unwrap_or_default();
            if color {
                output.push_str(&format!(
                    " [{} {} {}]",
                    file.permissions.yellow(),
                    created_info.yellow(),
                    modified_info.yellow()
                ));
            } else {
                output.push_str(&format!(" [{} {} {}]", file.permissions, created_info, modified_info));
            }
        }

        println!("{}", output);
    }

    if let Some(export_file) = export_path {
        if export_file.ends_with(".json") {
            export_to_json(files, export_file);
        } else if export_file.ends_with(".csv") {
            export_to_csv(files, export_file);
        }
    }
}

/// Show file type statistics
pub fn show_file_type_stats(files: &[FileInfo], color: bool) {
    let mut type_counts = HashMap::new();
    let mut _total_size = 0u64;
    let mut total_files = 0u64;

    for file in files {
        if !file.is_directory {
            *type_counts.entry(&file.file_type).or_insert(0) += 1;
            _total_size += file.size;
            total_files += 1;
        }
    }

    if !type_counts.is_empty() {
        println!("");
        println!("File Type Statistics:");
        println!("{}", "─".repeat(40));

        let mut sorted_types: Vec<_> = type_counts
            .iter()
            .filter(|(file_type, _)| file_type.as_str() != "unknown")
            .collect();
        sorted_types.sort_by(|a, b| b.1.cmp(a.1));

        for (file_type, count) in sorted_types {
            let percentage = (*count as f64 / total_files as f64) * 100.0;
            if color {
                println!(
                    "{}: {} files ({:.1}%)",
                    file_type.magenta(),
                    count.to_string().cyan(),
                    percentage
                );
            } else {
                println!("{}: {} files ({:.1}%)", file_type, count, percentage);
            }
        }

        if color {
            println!("\nTotal Files: {}", total_files.to_string().cyan());
        } else {
            println!("\nTotal Files: {}", total_files);
        }
    }
}

/// Export files to JSON format
pub fn export_to_json(files: &[FileInfo], filename: &str) {
    if let Ok(json) = serde_json::to_string_pretty(files) {
        if fs::write(filename, json).is_ok() {
            println!("Results exported to {}", filename);
        } else {
            eprintln!("Failed to write to {}", filename);
        }
    } else {
        eprintln!("Failed to serialize data to JSON");
    }
}

/// Export files to CSV format
pub fn export_to_csv(files: &[FileInfo], filename: &str) {
    let mut wtr = csv::Writer::from_path(filename).unwrap();
    for file in files {
        wtr.serialize(file).unwrap();
    }
    wtr.flush().unwrap();
    println!("Results exported to {}", filename);
}
