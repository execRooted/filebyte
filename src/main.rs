use clap::{Arg, Command};
use colored::*;
use std::fs;
use std::path::Path;
use std::process;
use sysinfo::Disks;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use infer;

#[derive(Debug)]
enum SizeUnit {
    Bytes,
    Kilobytes,
    Megabytes,
    Gigabytes,
    Terabytes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FileInfo {
    name: String,
    path: String,
    size: u64,
    size_human: String,
    file_type: String,
    created: Option<String>,
    modified: Option<String>,
    permissions: String,
    is_directory: bool,
}

#[derive(Debug)]
enum SortBy {
    Name,
    Size,
    Date,
}

impl SizeUnit {
    fn from_str(s: &str) -> Result<Self, String> {
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

    fn format_size(&self, bytes: u64) -> String {
        match self {
            SizeUnit::Bytes => format!("{} B", bytes),
            SizeUnit::Kilobytes => format!("{:.2} KB", bytes as f64 / 1024.0),
            SizeUnit::Megabytes => format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0)),
            SizeUnit::Gigabytes => format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0)),
            SizeUnit::Terabytes => format!("{:.2} TB", bytes as f64 / (1024.0 * 1024.0 * 1024.0 * 1024.0)),
        }
    }

    fn auto_format_size(bytes: u64) -> String {
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

fn get_file_size(path: &Path) -> u64 {
    if path.is_file() {
        fs::metadata(path).map(|m| m.len()).unwrap_or(0)
    } else if path.is_dir() {
        fs::read_dir(path)
            .map(|entries| {
                entries
                    .filter_map(|entry| entry.ok())
                    .map(|entry| get_file_size(&entry.path()))
                    .sum()
            })
            .unwrap_or(0)
    } else {
        0
    }
}



fn print_tree(path: &Path, prefix: &str, color: bool) {
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries.collect::<Vec<_>>(),
        Err(e) => {
            eprintln!("Error reading directory {}: {}", path.display(), e);
            return;
        }
    };

    for (i, entry) in entries.iter().enumerate() {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                eprintln!("Error reading entry: {}", e);
                continue;
            }
        };

        let path = entry.path();
        let file_name = path.file_name().unwrap_or_default().to_string_lossy();
        let is_last = i == entries.len() - 1;
        let connector = if is_last { "└── " } else { "├── " };
        let new_prefix = format!("{}{}", prefix, if is_last { "    " } else { "│   " });

        let display_name = if path.is_dir() {
            if color {
                format!("{}{}", connector, file_name.blue().bold())
            } else {
                format!("{}{}", connector, file_name)
            }
        } else {
            if color {
                format!("{}{}", connector, file_name)
            } else {
                format!("{}{}", connector, file_name)
            }
        };

        println!("{}{}", prefix, display_name);

        if path.is_dir() {
            print_tree(&path, &new_prefix, color);
        }
    }
}

fn list_disks(color: bool, size_unit: &SizeUnit, auto_size: bool) {
    let disks = Disks::new_with_refreshed_list();
    println!("Available disks:");
    println!("{}", "─".repeat(60));

    for disk in &disks {
        let name = disk.name().to_string_lossy();
        let mount_point = disk.mount_point().display();
        let total_space = if auto_size {
            SizeUnit::auto_format_size(disk.total_space())
        } else {
            size_unit.format_size(disk.total_space())
        };
        let available_space = if auto_size {
            SizeUnit::auto_format_size(disk.available_space())
        } else {
            size_unit.format_size(disk.available_space())
        };
        let used_space = if auto_size {
            SizeUnit::auto_format_size(disk.total_space() - disk.available_space())
        } else {
            size_unit.format_size(disk.total_space() - disk.available_space())
        };

        if color {
            println!("{} ({}) - Total: {} | Used: {} | Available: {}",
                    name.blue().bold(),
                    mount_point,
                    total_space.cyan(),
                    used_space.red(),
                    available_space.green());
        } else {
            println!("{} ({}) - Total: {} | Used: {} | Available: {}",
                    name,
                    mount_point,
                    total_space,
                    used_space,
                    available_space);
        }
    }
}

fn collect_files(dir: &Path, search_pattern: Option<&String>, excluding_pattern: Option<&String>, sort_by: Option<SortBy>) -> Vec<FileInfo> {
    let mut files = Vec::new();

    fn collect_recursive(path: &Path, files: &mut Vec<FileInfo>, search_regex: Option<&Regex>, excluding_regex: Option<&Regex>) {
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let entry_path = entry.path();

                // Apply excluding filter first
                if let Some(regex) = excluding_regex {
                    if regex.is_match(&entry_path.file_name().unwrap_or_default().to_string_lossy()) {
                        continue;
                    }
                }

                // Apply search filter
                if let Some(regex) = search_regex {
                    if !regex.is_match(&entry_path.file_name().unwrap_or_default().to_string_lossy()) {
                        continue;
                    }
                }

                if let Ok(metadata) = entry.metadata() {
                    let file_type = if entry_path.is_dir() {
                        "directory".to_string()
                    } else {
                        infer::get_from_path(&entry_path)
                            .ok()
                            .flatten()
                            .map(|kind| kind.mime_type().to_string())
                            .unwrap_or_else(|| "unknown".to_string())
                    };

                    let created = metadata.created()
                        .ok()
                        .map(|t| DateTime::<Utc>::from(t).format("%Y-%m-%d %H:%M:%S UTC").to_string());

                    let modified = metadata.modified()
                        .ok()
                        .map(|t| DateTime::<Utc>::from(t).format("%Y-%m-%d %H:%M:%S UTC").to_string());

                    let permissions = if metadata.permissions().readonly() { "r" } else { "rw" };

                    files.push(FileInfo {
                        name: entry_path.file_name().unwrap_or_default().to_string_lossy().to_string(),
                        path: entry_path.to_string_lossy().to_string(),
                        size: metadata.len(),
                        size_human: SizeUnit::auto_format_size(metadata.len()),
                        file_type,
                        created,
                        modified,
                        permissions: permissions.to_string(),
                        is_directory: entry_path.is_dir(),
                    });
                }
            }
        }
    }

    let search_regex = search_pattern.and_then(|p| Regex::new(p).ok());
    let excluding_regex = excluding_pattern.and_then(|p| Regex::new(p).ok());
    collect_recursive(dir, &mut files, search_regex.as_ref(), excluding_regex.as_ref());

    // Apply sorting
    if let Some(sort_criteria) = sort_by {
        match sort_criteria {
            SortBy::Name => files.sort_by(|a, b| {
                // Directories first, then files, both alphabetically
                match (a.is_directory, b.is_directory) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.name.cmp(&b.name),
                }
            }),
            SortBy::Size => files.sort_by(|a, b| {
                // Directories first, then by size descending
                match (a.is_directory, b.is_directory) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => b.size.cmp(&a.size),
                }
            }),
            SortBy::Date => files.sort_by(|a, b| {
                // Directories first, then by date descending
                match (a.is_directory, b.is_directory) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => {
                        let a_date = a.modified.as_ref().map(|s| s.as_str()).unwrap_or("");
                        let b_date = b.modified.as_ref().map(|s| s.as_str()).unwrap_or("");
                        b_date.cmp(a_date)
                    }
                }
            }),
        }
    } else {
        // Default sorting: directories first, then files, both alphabetically
        files.sort_by(|a, b| {
            match (a.is_directory, b.is_directory) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            }
        });
    }

    files
}

fn collect_files_recursive(dir: &Path, search_pattern: Option<&String>, excluding_pattern: Option<&String>, sort_by: Option<SortBy>) -> Vec<FileInfo> {
    let mut files = Vec::new();

    fn collect_all_recursive(path: &Path, files: &mut Vec<FileInfo>, search_regex: Option<&Regex>, excluding_regex: Option<&Regex>) {
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let entry_path = entry.path();

                // Apply excluding filter first
                if let Some(regex) = excluding_regex {
                    if regex.is_match(&entry_path.file_name().unwrap_or_default().to_string_lossy()) {
                        continue;
                    }
                }

                // Apply search filter
                if let Some(regex) = search_regex {
                    if !regex.is_match(&entry_path.file_name().unwrap_or_default().to_string_lossy()) {
                        continue;
                    }
                }

                if let Ok(metadata) = entry.metadata() {
                    let file_type = if entry_path.is_dir() {
                        "directory".to_string()
                    } else {
                        infer::get_from_path(&entry_path)
                            .ok()
                            .flatten()
                            .map(|kind| kind.mime_type().to_string())
                            .unwrap_or_else(|| "unknown".to_string())
                    };

                    let created = metadata.created()
                        .ok()
                        .map(|t| DateTime::<Utc>::from(t).format("%Y-%m-%d %H:%M:%S UTC").to_string());

                    let modified = metadata.modified()
                        .ok()
                        .map(|t| DateTime::<Utc>::from(t).format("%Y-%m-%d %H:%M:%S UTC").to_string());

                    let permissions = if metadata.permissions().readonly() { "r" } else { "rw" };

                    files.push(FileInfo {
                        name: entry_path.file_name().unwrap_or_default().to_string_lossy().to_string(),
                        path: entry_path.to_string_lossy().to_string(),
                        size: metadata.len(),
                        size_human: SizeUnit::auto_format_size(metadata.len()),
                        file_type,
                        created,
                        modified,
                        permissions: permissions.to_string(),
                        is_directory: entry_path.is_dir(),
                    });

                    // Recursively collect from subdirectories
                    if entry_path.is_dir() {
                        collect_all_recursive(&entry_path, files, search_regex, excluding_regex);
                    }
                }
            }
        }
    }

    let search_regex = search_pattern.and_then(|p| Regex::new(p).ok());
    let excluding_regex = excluding_pattern.and_then(|p| Regex::new(p).ok());
    collect_all_recursive(dir, &mut files, search_regex.as_ref(), excluding_regex.as_ref());

    // Apply sorting
    if let Some(sort_criteria) = sort_by {
        match sort_criteria {
            SortBy::Name => files.sort_by(|a, b| {
                // Directories first, then files, both alphabetically
                match (a.is_directory, b.is_directory) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.name.cmp(&b.name),
                }
            }),
            SortBy::Size => files.sort_by(|a, b| {
                // Directories first, then by size descending
                match (a.is_directory, b.is_directory) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => b.size.cmp(&a.size),
                }
            }),
            SortBy::Date => files.sort_by(|a, b| {
                // Directories first, then by date descending
                match (a.is_directory, b.is_directory) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => {
                        let a_date = a.modified.as_ref().map(|s| s.as_str()).unwrap_or("");
                        let b_date = b.modified.as_ref().map(|s| s.as_str()).unwrap_or("");
                        b_date.cmp(a_date)
                    }
                }
            }),
        }
    } else {
        // Default sorting: directories first, then files, both alphabetically
        files.sort_by(|a, b| {
            match (a.is_directory, b.is_directory) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            }
        });
    }

    files
}

fn display_files(files: &[FileInfo], size_unit: &SizeUnit, color: bool, properties: bool, auto_size: bool, export_path: Option<&String>) {
    for file in files {
        let size_str = if auto_size {
            file.size_human.clone()
        } else {
            size_unit.format_size(file.size)
        };

        let mut output = if color {
            if file.is_directory {
                format!("{} {} {}", file.name.blue().bold(), size_str.cyan().bold(), "[DIR]".blue())
            } else {
                format!("{} {}", file.name, size_str.green())
            }
        } else {
            if file.is_directory {
                format!("{} {} [DIR]", file.name, size_str)
            } else {
                format!("{} {}", file.name, size_str)
            }
        };

        if properties {
            let created_info = file.created.as_ref().map(|c| format!("Created: {}", c)).unwrap_or_default();
            let modified_info = file.modified.as_ref().map(|m| format!("Modified: {}", m)).unwrap_or_default();
            if color {
                output.push_str(&format!(" [{} {} {}]", file.permissions.yellow(), created_info.yellow(), modified_info.yellow()));
            } else {
                output.push_str(&format!(" [{} {} {}]", file.permissions, created_info, modified_info));
            }
        }

        println!("{}", output);
    }

    // Export functionality
    if let Some(export_file) = export_path {
        if export_file.ends_with(".json") {
            export_to_json(files, export_file);
        } else if export_file.ends_with(".csv") {
            export_to_csv(files, export_file);
        }
    }
}

fn show_file_type_stats(files: &[FileInfo], color: bool) {
    let mut type_counts = HashMap::new();
    let mut total_size = 0u64;

    for file in files {
        if !file.is_directory {
            *type_counts.entry(&file.file_type).or_insert(0) += 1;
            total_size += file.size;
        }
    }

    if !type_counts.is_empty() {
        println!("\nFile Type Statistics:");
        println!("{}", "─".repeat(40));

        let mut sorted_types: Vec<_> = type_counts.iter().collect();
        sorted_types.sort_by(|a, b| b.1.cmp(a.1));

        for (file_type, count) in sorted_types {
            let percentage = (*count as f64 / type_counts.values().sum::<u64>() as f64) * 100.0;
            if color {
                println!("{}: {} files ({:.1}%)", file_type.magenta(), count.to_string().cyan(), percentage);
            } else {
                println!("{}: {} files ({:.1}%)", file_type, count, percentage);
            }
        }

        if color {
            println!("Total files analyzed: {}", files.len().to_string().green());
            println!("Total size: {}", SizeUnit::auto_format_size(total_size).green());
        } else {
            println!("Total files analyzed: {}", files.len());
            println!("Total size: {}", SizeUnit::auto_format_size(total_size));
        }
    }
}

fn find_duplicates(dir: &Path, color: bool) {
    let mut hash_map = HashMap::new();
    let mut duplicates = Vec::new();

    fn scan_for_duplicates(path: &Path, hash_map: &mut HashMap<u64, Vec<String>>, duplicates: &mut Vec<(u64, Vec<String>)>) {
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                if entry_path.is_file() {
                    if let Ok(metadata) = entry.metadata() {
                        let size = metadata.len();
                        hash_map.entry(size).or_insert_with(Vec::new).push(entry_path.to_string_lossy().to_string());
                    }
                } else if entry_path.is_dir() {
                    scan_for_duplicates(&entry_path, hash_map, duplicates);
                }
            }
        }
    }

    scan_for_duplicates(dir, &mut hash_map, &mut duplicates);

    for (size, paths) in hash_map.iter() {
        if paths.len() > 1 {
            duplicates.push((*size, paths.clone()));
        }
    }

    if duplicates.is_empty() {
        println!("No duplicate files found.");
    } else {
        println!("Duplicate files found:");
        println!("{}", "─".repeat(50));

        for (size, paths) in duplicates {
            if color {
                println!("Size: {} ({})", SizeUnit::auto_format_size(size).cyan(), paths.len().to_string().yellow());
            } else {
                println!("Size: {} ({})", SizeUnit::auto_format_size(size), paths.len());
            }
            for path in &paths {
                println!("  {}", path);
            }
            println!();
        }
    }
}

fn export_to_json(files: &[FileInfo], filename: &str) {
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

fn export_to_csv(files: &[FileInfo], filename: &str) {
    let mut wtr = csv::Writer::from_path(filename).unwrap();
    for file in files {
        wtr.serialize(file).unwrap();
    }
    wtr.flush().unwrap();
    println!("Results exported to {}", filename);
}

fn show_detailed_analysis(files: &[FileInfo], color: bool) {
    let total_files = files.len();
    let total_dirs = files.iter().filter(|f| f.is_directory).count();
    let total_regular_files = total_files - total_dirs;
    let total_size: u64 = files.iter().map(|f| f.size).sum();

    println!("\nDetailed Analysis:");
    println!("{}", "-".repeat(50));

    if color {
        println!("Total Items: {} ({})", total_files.to_string().cyan(), format!("{} files, {} dirs", total_regular_files, total_dirs).yellow());
        println!("Total Size: {}", SizeUnit::auto_format_size(total_size).green());
    } else {
        println!("Total Items: {} ({} files, {} dirs)", total_files, total_regular_files, total_dirs);
        println!("Total Size: {}", SizeUnit::auto_format_size(total_size));
    }

    // Size distribution
    let size_ranges = [
        ("Empty (0 B)", 0..1),
        ("Tiny (< 1 KB)", 1..1024),
        ("Small (1 KB - 1 MB)", 1024..1024*1024),
        ("Medium (1 MB - 100 MB)", 1024*1024..100*1024*1024),
        ("Large (100 MB - 1 GB)", 100*1024*1024..1024*1024*1024),
        ("Huge (> 1 GB)", 1024*1024*1024..u64::MAX),
    ];

    println!("\nSize Distribution:");
    for (label, range) in &size_ranges {
        let count = files.iter().filter(|f| range.contains(&f.size)).count();
        if count > 0 {
            let percentage = (count as f64 / total_files as f64) * 100.0;
            if color {
                println!("  {}: {} files ({:.1}%)", label.magenta(), count.to_string().cyan(), percentage);
            } else {
                println!("  {}: {} files ({:.1}%)", label, count, percentage);
            }
        }
    }

    // Age analysis
    let now = std::time::SystemTime::now();
    let age_ranges = [
        ("Today", 0..86400), // 24 hours
        ("This Week", 86400..604800), // 7 days
        ("This Month", 604800..2592000), // 30 days
        ("This Year", 2592000..31536000), // 365 days
        ("Older", 31536000..u64::MAX),
    ];

    println!("\nFile Age Distribution:");
    for (label, range) in &age_ranges {
        let count = files.iter().filter(|f| {
            if let Some(modified_str) = &f.modified {
                if let Ok(modified_time) = chrono::DateTime::parse_from_rfc3339(&format!("{}Z", modified_str.replace(" UTC", ""))) {
                    let duration = now.duration_since(modified_time.with_timezone(&chrono::Utc).into()).unwrap_or_default();
                    range.contains(&duration.as_secs())
                } else {
                    false
                }
            } else {
                false
            }
        }).count();

        if count > 0 {
            let percentage = (count as f64 / total_files as f64) * 100.0;
            if color {
                println!("  {}: {} files ({:.1}%)", label.magenta(), count.to_string().cyan(), percentage);
            } else {
                println!("  {}: {} files ({:.1}%)", label, count, percentage);
            }
        }
    }

    // Largest and smallest files
    if let Some(largest) = files.iter().filter(|f| !f.is_directory).max_by_key(|f| f.size) {
        if color {
            println!("\nLargest File: {} ({})", largest.name.cyan(), largest.size_human.green());
        } else {
            println!("\nLargest File: {} ({})", largest.name, largest.size_human);
        }
    }

    if let Some(smallest) = files.iter().filter(|f| !f.is_directory && f.size > 0).min_by_key(|f| f.size) {
        if color {
            println!("Smallest File: {} ({})", smallest.name.cyan(), smallest.size_human.green());
        } else {
            println!("Smallest File: {} ({})", smallest.name, smallest.size_human);
        }
    }

    // Permissions summary
    let readable = files.iter().filter(|f| f.permissions.contains('r')).count();
    let writable = files.iter().filter(|f| f.permissions.contains('w')).count();

    println!("\nPermissions Summary:");
    if color {
        println!("  Readable: {} files ({:.1}%)", readable.to_string().cyan(), (readable as f64 / total_files as f64) * 100.0);
        println!("  Writable: {} files ({:.1}%)", writable.to_string().cyan(), (writable as f64 / total_files as f64) * 100.0);
    } else {
        println!("  Readable: {} files ({:.1}%)", readable, (readable as f64 / total_files as f64) * 100.0);
        println!("  Writable: {} files ({:.1}%)", writable, (writable as f64 / total_files as f64) * 100.0);
    }
}

fn show_disk_info(disk_name: &str, size_unit: &SizeUnit, color: bool, auto_size: bool, tree: bool, properties: bool, search_pattern: Option<&String>, excluding_pattern: Option<&String>, sort_by: Option<SortBy>, duplicates: bool) {
    let disks = Disks::new_with_refreshed_list();
    let disk = disks.iter().find(|d| d.name().to_string_lossy() == disk_name);

    match disk {
        Some(disk) => {
            let mount_point = disk.mount_point();
            let total_space = disk.total_space();
            let available_space = disk.available_space();
            let used_space = total_space - available_space;
            let usage_percentage = (used_space as f64 / total_space as f64) * 100.0;

            if color {
                println!("Disk Information: {}", disk_name.blue().bold());
                println!("Mount Point: {}", mount_point.display());
                println!("Total Space: {}", SizeUnit::auto_format_size(total_space).cyan());
                println!("Used Space: {}", SizeUnit::auto_format_size(used_space).red());
                println!("Available Space: {}", SizeUnit::auto_format_size(available_space).green());
                println!("Usage: {:.1}%", usage_percentage);
            } else {
                println!("Disk Information: {}", disk_name);
                println!("Mount Point: {}", mount_point.display());
                println!("Total Space: {}", SizeUnit::auto_format_size(total_space));
                println!("Used Space: {}", SizeUnit::auto_format_size(used_space));
                println!("Available Space: {}", SizeUnit::auto_format_size(available_space));
                println!("Usage: {:.1}%", usage_percentage);
            }

            if duplicates {
                find_duplicates(mount_point, color);
            } else if tree {
                println!("\nDirectory Tree:");
                print_tree(mount_point, "", color);
            } else {
                let files = collect_files(mount_point, search_pattern, excluding_pattern, sort_by);
                if files.is_empty() {
                    if let Some(pattern) = search_pattern {
                        println!("No files found matching pattern: {}", pattern);
                    } else {
                        println!("No files found.");
                    }
                } else {
                    display_files(&files, size_unit, color, properties, auto_size, None);
                }
                show_file_type_stats(&files, color);
            }
        }
        None => {
            eprintln!("Error: Disk '{}' not found", disk_name);
            eprintln!("Use 'filebyte --disk list' to see available disks");
            process::exit(1);
        }
    }
}

fn main() {
    let matches = Command::new("filebyte")
        .version("0.1.2")
        .author("execRooted <execrooted@gmail.com>")
        .about("List files and directories with sizes")
        .disable_version_flag(true)
        .disable_help_flag(true)
        .arg(
            Arg::new("path")
                .help("Path to file or directory")
                .index(1),
        )
        .arg(
            Arg::new("version")
                .short('v')
                .long("version")
                .help("Show version information")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("help")
                .short('h')
                .long("help")
                .help("Show help information")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("directory")
                .short('d')
                .long("directory")
                .help("Specify directory to list")
                .value_name("DIR"),
        )
        .arg(
            Arg::new("file")
                .short('f')
                .long("file")
                .help("Specify file to show properties")
                .value_name("FILE"),
        )
        .arg(
            Arg::new("size")
                .short('s')
                .long("size")
                .help("Size unit (auto, b/bytes, kb/kilobytes, mb/megabytes, gb/gigabytes, tb/terabytes)")
                .value_name("UNIT")
                .default_value("auto"),
        )
        .arg(
            Arg::new("tree")
                .short('t')
                .long("tree")
                .help("Show directory tree")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("properties")
                .short('p')
                .long("properties")
                .help("Show file properties")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("properties_all")
                .long("properties-all")
                .help("Show file properties for all files recursively")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("no-color")
                .long("no-color")
                .help("Disable colored output")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("disk")
                .short('m')
                .long("disk")
                .help("Disk operations: 'list' to show all disks, or specify disk name for info")
                .value_name("DISK"),
        )
        .arg(
            Arg::new("search")
                .short('e')
                .long("search")
                .help("Search for files using regex pattern")
                .value_name("PATTERN"),
        )
        .arg(
            Arg::new("excluding")
                .short('x')
                .long("excluding")
                .help("Exclude files matching regex pattern")
                .value_name("PATTERN"),
        )
        .arg(
            Arg::new("sort_by")
                .long("sort-by")
                .help("Sort files by: name, size, date")
                .value_name("CRITERIA"),
        )
        .arg(
            Arg::new("duplicates")
                .long("duplicates")
                .help("Find duplicate files")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("export")
                .long("export")
                .help("Export results to file (json/csv)")
                .value_name("FILE"),
        )
        .get_matches();

    if matches.get_flag("version") {
        println!("filebyte {}", env!("CARGO_PKG_VERSION"));
        return;
    }

    if matches.get_flag("help") {
        println!("filebyte 0.1.0");
        println!("execRooted <execrooted@gmail.com>");
        println!("List files and directories with sizes");
        println!();
        println!("USAGE:");
        println!("    filebyte [OPTIONS] [PATH]");
        println!("    filebyte --disk <DISK> [OPTIONS]");
        println!();
        println!("ARGS:");
        println!("    <PATH>    Path to file or directory");
        println!();
        println!("OPTIONS:");
        println!("    -v, --version                    Show version information");
        println!("    -h, --help                       Show help information");
        println!("    -d, --directory <DIR>            Specify directory to list");
        println!("    -f, --file <FILE>                Specify file to show properties");
        println!("    -s, --size <UNIT>                Size unit (auto, b/bytes, kb/kilobytes, mb/megabytes, gb/gigabytes, tb/terabytes) [default: auto]");
        println!("    -t, --tree                       Show directory tree");
        println!("    -p, --properties                 Show file properties");
        println!("        --properties-all             Show file properties for all files recursively");
        println!("        --no-color                   Disable colored output");
        println!("    -m, --disk <DISK>                Disk operations: 'list' to show all disks, or specify disk name for info");
        println!("    -e, --search <PATTERN>           Search for files using regex pattern");
        println!("    -x, --excluding <PATTERN>        Exclude files matching regex pattern");
        println!("        --sort-by <CRITERIA>         Sort files by: name, size, date");
        println!("        --duplicates                 Find duplicate files");
        println!("        --export <FILE>              Export results to file (json/csv)");
        println!();
        println!("EXAMPLES:");
        println!("    filebyte                         List files in current directory");
        println!("    filebyte /home/user              List files in /home/user");
        println!("    filebyte --size mb               Show sizes in megabytes");
        println!("    filebyte --search \"\\.rs$\"        Search for Rust files");
        println!("    filebyte --excluding \"^\\.\"       Exclude hidden files");
        println!("    filebyte --disk list             List all available disks");
        println!("    filebyte --disk sda1 --tree      Show tree for disk sda1");
        return;
    }

    let size_unit_str = matches.get_one::<String>("size").unwrap();
    let auto_size = size_unit_str == "auto";
    let size_unit = match SizeUnit::from_str(size_unit_str) {
        Ok(unit) => unit,
        Err(e) => {
            eprintln!("Error: {}", e);
            eprintln!("Available options are: auto, b/bytes, kb/kilobytes, mb/megabytes, gb/gigabytes, tb/terabytes");
            process::exit(1);
        }
    };

    let color = !matches.get_flag("no-color");

    let search_pattern = matches.get_one::<String>("search");
    let excluding_pattern = matches.get_one::<String>("excluding");
    let sort_by = matches.get_one::<String>("sort_by").map(|s| match s.to_lowercase().as_str() {
        "name" => SortBy::Name,
        "size" => SortBy::Size,
        "date" => SortBy::Date,
        _ => SortBy::Name,
    });

    // Handle disk operations
    if let Some(disk_arg) = matches.get_one::<String>("disk") {
        if disk_arg == "list" {
            list_disks(color, &size_unit, auto_size);
            return;
        } else {
            show_disk_info(disk_arg, &size_unit, color, auto_size, matches.get_flag("tree"), matches.get_flag("properties"), search_pattern, excluding_pattern, sort_by, matches.get_flag("duplicates"));
            return;
        }
    }

    let path = if let Some(dir) = matches.get_one::<String>("directory") {
        Path::new(dir)
    } else if let Some(file) = matches.get_one::<String>("file") {
        Path::new(file)
    } else if let Some(path_arg) = matches.get_one::<String>("path") {
        Path::new(path_arg)
    } else {
        Path::new(".")
    };

    if !path.exists() {
        eprintln!("Error: Path '{}' does not exist", path.display());
        process::exit(1);
    }

    if matches.get_flag("tree") {
        if path.is_dir() {
            println!("{}", path.display());
            print_tree(path, "", color);
        } else {
            eprintln!("Error: --tree can only be used with directories");
            process::exit(1);
        }
    } else if matches.get_one::<String>("file").is_some() {
        if path.is_file() {
            let size = get_file_size(path);
            let size_str = if auto_size {
                SizeUnit::auto_format_size(size)
            } else {
                size_unit.format_size(size)
            };
            let file_name = path.file_name().unwrap_or_default().to_string_lossy();

            let mut output = if color {
                format!("{} {}", file_name, size_str.green().bold())
            } else {
                format!("{} {}", file_name, size_str)
            };

            if matches.get_flag("properties") {
                let metadata = match fs::metadata(path) {
                    Ok(m) => m,
                    Err(e) => {
                        eprintln!("Error reading metadata: {}", e);
                        process::exit(1);
                    }
                };
                let permissions = if metadata.permissions().readonly() { "r" } else { "rw" };
                let modified = metadata.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                let modified_str = format!("{:?}", modified);
                output.push_str(&format!(" [{} {}]", permissions, &modified_str[..19]));
            }

            println!("{}", output);
        } else {
            eprintln!("Error: --file can only be used with files");
            process::exit(1);
        }
    } else if matches.get_flag("properties_all") {
        if path.is_dir() {
            let files = collect_files_recursive(path, search_pattern, excluding_pattern, sort_by);
            display_files(&files, &size_unit, color, true, auto_size, matches.get_one::<String>("export"));

            if matches.get_flag("properties_all") {
                show_file_type_stats(&files, color);
                show_detailed_analysis(&files, color);
            }
        } else {
            eprintln!("Error: --properties-all can only be used with directories");
            process::exit(1);
        }
    } else {
        if matches.get_flag("duplicates") {
            find_duplicates(path, color);
        } else if matches.get_flag("tree") {
            if path.is_dir() {
                println!("{}", path.display());
                print_tree(path, "", color);
            } else {
                eprintln!("Error: --tree can only be used with directories");
                process::exit(1);
            }
        } else {
            let files = collect_files(path, search_pattern, excluding_pattern, sort_by);
            if files.is_empty() {
                if let Some(pattern) = search_pattern {
                    println!("No files found matching pattern: {}", pattern);
                } else {
                    println!("No files found.");
                }
            } else {
                display_files(&files, &size_unit, color, matches.get_flag("properties"), auto_size, matches.get_one::<String>("export"));
            }

            if matches.get_flag("properties") || matches.get_flag("properties_all") {
                show_file_type_stats(&files, color);
                show_detailed_analysis(&files, color);
            }
        }
    }
}