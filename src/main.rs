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
        let mut total = 0;
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                total += get_file_size(&entry.path());
            }
        }
        total
    } else {
        0
    }
}

fn can_delete(path: &Path) -> bool {
    if let Some(parent) = path.parent() {
        if let Ok(parent_meta) = fs::metadata(parent) {
            !parent_meta.permissions().readonly()
        } else {
            false
        }
    } else {
        false
    }
}

fn format_unix_permissions(metadata: &fs::Metadata, detailed: bool) -> String {
    use std::os::unix::fs::PermissionsExt;

    if detailed {
        let mode = metadata.permissions().mode();
        let file_type = if metadata.is_dir() { 'd' } else { '-' };

        let user_read = if mode & 0o400 != 0 { 'r' } else { '-' };
        let user_write = if mode & 0o200 != 0 { 'w' } else { '-' };
        let user_exec = if mode & 0o100 != 0 { 'x' } else { '-' };

        let group_read = if mode & 0o040 != 0 { 'r' } else { '-' };
        let group_write = if mode & 0o020 != 0 { 'w' } else { '-' };
        let group_exec = if mode & 0o010 != 0 { 'x' } else { '-' };

        let other_read = if mode & 0o004 != 0 { 'r' } else { '-' };
        let other_write = if mode & 0o002 != 0 { 'w' } else { '-' };
        let other_exec = if mode & 0o001 != 0 { 'x' } else { '-' };

        format!("{}{}{}{}{}{}{}{}{}{}",
                file_type, user_read, user_write, user_exec,
                group_read, group_write, group_exec,
                other_read, other_write, other_exec)
    } else {
        // Original simplified format
        if metadata.permissions().readonly() {
            if can_delete(&std::path::Path::new("")) { "r-x" } else { "r--" }
        } else {
            if can_delete(&std::path::Path::new("")) { "rwx" } else { "rw-" }
        }.to_string()
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

    fn collect_recursive(path: &Path, files: &mut Vec<FileInfo>, search_pattern: Option<&String>, excluding_regex: Option<&Regex>) {
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let entry_path = entry.path();

                let file_name = entry_path.file_name().unwrap_or_default().to_string_lossy();

                if let Some(regex) = excluding_regex {
                    if regex.is_match(&file_name) {
                        continue;
                    }
                }

                // Check if file matches search pattern (supports both regex and substring matching)
                if let Some(pattern) = search_pattern {
                    let matches = if pattern.starts_with('^') || pattern.ends_with('$') || pattern.contains(".*") || pattern.contains("[") || pattern.contains("]") {
                        // Use regex matching for patterns that look like regex
                        if let Ok(regex) = Regex::new(pattern) {
                            regex.is_match(&file_name)
                        } else {
                            false
                        }
                    } else {
                        // Use substring matching for simple patterns
                        file_name.contains(pattern)
                    };

                    if !matches {
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

                    let permissions = if metadata.permissions().readonly() {
                        if can_delete(&entry_path) { "r-x" } else { "r--" }
                    } else {
                        if can_delete(&entry_path) { "rwx" } else { "rw-" }
                    };

                    files.push(FileInfo {
                        name: file_name.to_string(),
                        path: entry_path.to_string_lossy().to_string(),
                        size: get_file_size(&entry_path),
                        size_human: SizeUnit::auto_format_size(get_file_size(&entry_path)),
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

    let excluding_regex = excluding_pattern.and_then(|p| Regex::new(p).ok());
    collect_recursive(dir, &mut files, search_pattern, excluding_regex.as_ref());

    
    if let Some(sort_criteria) = sort_by {
        match sort_criteria {
            SortBy::Name => files.sort_by(|a, b| {
                
                match (a.is_directory, b.is_directory) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.name.cmp(&b.name),
                }
            }),
            SortBy::Size => files.sort_by(|a, b| {
                
                match (a.is_directory, b.is_directory) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => b.size.cmp(&a.size),
                }
            }),
            SortBy::Date => files.sort_by(|a, b| {
                
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

    fn collect_all_recursive(path: &Path, files: &mut Vec<FileInfo>, search_pattern: Option<&String>, excluding_regex: Option<&Regex>) {
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let entry_path = entry.path();

                let file_name = entry_path.file_name().unwrap_or_default().to_string_lossy();

                if let Some(regex) = excluding_regex {
                    if regex.is_match(&file_name) {
                        continue;
                    }
                }

                // Check if file matches search pattern (supports both regex and substring matching)
                if let Some(pattern) = search_pattern {
                    let matches = if pattern.starts_with('^') || pattern.ends_with('$') || pattern.contains(".*") || pattern.contains("[") || pattern.contains("]") {
                        // Use regex matching for patterns that look like regex
                        if let Ok(regex) = Regex::new(pattern) {
                            regex.is_match(&file_name)
                        } else {
                            false
                        }
                    } else {
                        // Use substring matching for simple patterns
                        file_name.contains(pattern)
                    };

                    if !matches {
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

                    let permissions = if metadata.permissions().readonly() {
                        if can_delete(&entry_path) { "r-x" } else { "r--" }
                    } else {
                        if can_delete(&entry_path) { "rwx" } else { "rw-" }
                    };

                    files.push(FileInfo {
                        name: file_name.to_string(),
                        path: entry_path.to_string_lossy().to_string(),
                        size: metadata.len(),
                        size_human: SizeUnit::auto_format_size(metadata.len()),
                        file_type,
                        created,
                        modified,
                        permissions: permissions.to_string(),
                        is_directory: entry_path.is_dir(),
                    });


                    if entry_path.is_dir() {
                        collect_all_recursive(&entry_path, files, search_pattern, excluding_regex);
                    }
                }
            }
        }
    }

    let excluding_regex = excluding_pattern.and_then(|p| Regex::new(p).ok());
    collect_all_recursive(dir, &mut files, search_pattern, excluding_regex.as_ref());

    
    if let Some(sort_criteria) = sort_by {
        match sort_criteria {
            SortBy::Name => files.sort_by(|a, b| {
                
                match (a.is_directory, b.is_directory) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.name.cmp(&b.name),
                }
            }),
            SortBy::Size => files.sort_by(|a, b| {
                
                match (a.is_directory, b.is_directory) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => b.size.cmp(&a.size),
                }
            }),
            SortBy::Date => files.sort_by(|a, b| {
                
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

fn display_files(files: &[FileInfo], size_unit: &SizeUnit, color: bool, properties: bool, auto_size: bool, show_size: bool, export_path: Option<&String>, show_detailed_permissions: bool) {
    println!("");
    for file in files {
        let size_str = if auto_size {
            file.size_human.clone()
        } else {
            size_unit.format_size(file.size)
        };

        let mut output = if color {
            if file.is_directory {
                if show_size {
                    format!("{} {} {}", file.name.blue().bold(), size_str.cyan().bold(), "[DIR]".blue())
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
                            format_unix_permissions(&metadata, true)
                        } else {
                            file.permissions.clone()
                        }
                    } else {
                        file.permissions.clone()
                    };
                    format!("{} {} {}", file.name, permissions_display.magenta(), modified_short.yellow())
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
                output.push_str(&format!(" [{} {} {}]", file.permissions.yellow(), created_info.yellow(), modified_info.yellow()));
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

fn show_file_type_stats(files: &[FileInfo], color: bool) {
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
        println!("\nFile Type Statistics:");
        println!("{}", "─".repeat(40));

        let mut sorted_types: Vec<_> = type_counts.iter()
            .filter(|(file_type, _)| file_type.as_str() != "unknown")
            .collect();
        sorted_types.sort_by(|a, b| b.1.cmp(a.1));

        for (file_type, count) in sorted_types {
            let percentage = (*count as f64 / total_files as f64) * 100.0;
            if color {
                println!("{}: {} files ({:.1}%)", file_type.magenta(), count.to_string().cyan(), percentage);
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

fn show_search_results(files: &[FileInfo], search_pattern: &str, color: bool) {
    println!("\nSearch Results for '{}':", search_pattern);
    println!("{}", "─".repeat(40));

    for file in files {
        if color {
            println!("{} ({})", file.name.cyan(), file.path.magenta());
        } else {
            println!("{} ({})", file.name, file.path);
        }
    }

    if color {
        println!("\nFound {} matching files", files.len().to_string().cyan());
    } else {
        println!("\nFound {} matching files", files.len());
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
    let _total_size: u64 = files.iter().map(|f| f.size).sum();

    println!("\nDetailed Analysis:");
    println!("{}", "-".repeat(50));

    if color {
        println!("Total Items: {} ({})", total_files.to_string().cyan(), format!("{} files, {} dirs", total_regular_files, total_dirs).yellow());
    } else {
        println!("Total Items: {} ({} files, {} dirs)", total_files, total_regular_files, total_dirs);
    }

    
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

    
    let now = std::time::SystemTime::now();
    let age_ranges = [
        ("Today", 0..86400), 
        ("This Week", 86400..604800), 
        ("This Month", 604800..2592000), 
        ("This Year", 2592000..31536000), 
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

    
    let readable = files.iter().filter(|f| f.permissions.contains('r')).count();
    let writable = files.iter().filter(|f| f.permissions.contains('w')).count();
    let readable_only = files.iter().filter(|f| f.permissions == "r").count();
    let writable_only = files.iter().filter(|f| f.permissions == "rw").count();

    println!("\nPermissions Summary:");
    if color {
        println!("  Readable: {} files ({:.1}%)", readable.to_string().cyan(), (readable as f64 / total_files as f64) * 100.0);
        println!("  Writable: {} files ({:.1}%)", writable.to_string().cyan(), (writable as f64 / total_files as f64) * 100.0);
        println!("  Read-only: {} files ({:.1}%)", readable_only.to_string().cyan(), (readable_only as f64 / total_files as f64) * 100.0);
        println!("  Read-write: {} files ({:.1}%)", writable_only.to_string().cyan(), (writable_only as f64 / total_files as f64) * 100.0);
    } else {
        println!("  Readable: {} files ({:.1}%)", readable, (readable as f64 / total_files as f64) * 100.0);
        println!("  Writable: {} files ({:.1}%)", writable, (writable as f64 / total_files as f64) * 100.0);
        println!("  Read-only: {} files ({:.1}%)", readable_only, (readable_only as f64 / total_files as f64) * 100.0);
        println!("  Read-write: {} files ({:.1}%)", writable_only, (writable_only as f64 / total_files as f64) * 100.0);
    }
}

fn show_disk_info(disk_name: &str, size_unit: &SizeUnit, color: bool, auto_size: bool, tree: bool, properties: bool, search_pattern: Option<&String>, excluding_pattern: Option<&String>, sort_by: Option<SortBy>, duplicates: bool, show_size: bool, show_detailed_permissions: bool) {
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
                    display_files(&files, &size_unit, color, properties, auto_size, show_size, None, show_detailed_permissions);
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
        .version("0.3.7")
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
            Arg::new("size")
                .short('s')
                .long("size")
                .help("Show file sizes with specified unit (auto, b/bytes, kb/kilobytes, mb/megabytes, gb/gigabytes, tb/terabytes)")
                .value_name("UNIT")
                .num_args(0..=1),
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
                .help("Show detailed file properties and analysis")
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
        .arg(
            Arg::new("file")
                .short('f')
                .long("file")
                .help("Analyze a specific file")
                .value_name("FILE"),
        )
        .arg(
            Arg::new("directory")
                .short('d')
                .long("directory")
                .help("Analyze a directory as a whole (not its contents)")
                .value_name("DIR"),
        )
        .arg(
            Arg::new("recursive")
                .short('r')
                .long("recursive")
                .help("Enable recursive searching and analysis")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("whole")
                .short('w')
                .long("whole")
                .help("Analyze the path as a whole (auto-detects if file or directory)")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();

    if matches.get_flag("version") {
        println!("filebyte {}", env!("CARGO_PKG_VERSION"));
        return;
    }

    if matches.get_flag("help") {
        println!("filebyte 0.3.7");
        println!("execRooted <execrooted@gmail.com>");
        println!("List files and directories with sizes");
        println!();
        println!("USAGE:");
        println!("    filebyte [OPTIONS] [PATH]");
        println!("    filebyte --disk <DISK> [OPTIONS]");
        println!("    filebyte -f <FILE> | --file <FILE>");
        println!("    filebyte -d <DIR> | --directory <DIR>");
        println!();
        println!("ARGS:");
        println!("    <PATH>    Path to file or directory");
        println!();
        println!("OPTIONS:");
        println!("    -v, --version                    Show version information");
        println!("    -h, --help                       Show help information");
        
        println!("    -s, --size <UNIT>                Size unit (auto, b/bytes, kb/kilobytes, mb/megabytes, gb/gigabytes, tb/terabytes) [default: auto]");
        println!("    -t, --tree                       Show directory tree");
        println!("    -p, --properties                 Show file properties");
        
        println!("        --no-color                   Disable colored output");
        println!("    -m, --disk <DISK>                Disk operations: 'list' to show all disks, or specify disk name for info");
        println!("    -e, --search <PATTERN>           Search for files using regex pattern");
        println!("    -x, --excluding <PATTERN>        Exclude files matching regex pattern");
        println!("        --sort-by <CRITERIA>         Sort files by: name, size, date");
        println!("        --duplicates                 Find duplicate files");
        println!("        --export <FILE>              Export results to file (json/csv)");
        println!("    -f, --file <FILE>                Analyze a specific file");
        println!("    -d, --directory <DIR>            Analyze a directory as a whole");
        println!("    -r, --recursive                  Enable recursive searching and analysis");
        println!("    -w, --whole                      Analyze the path as a whole (auto-detects if file or directory)");
        println!();
        println!("EXAMPLES:");
        println!("    filebyte                         List files in current directory");
        println!("    filebyte /home/user              List files in /home/user");
        println!("    filebyte --size mb               Show sizes in megabytes");
        println!("    filebyte --search \"\\.rs$\"        Search for Rust files");
        println!("    filebyte --excluding \"^\\.\"       Exclude hidden files");
        println!("    filebyte --disk list             List all available disks");
        println!("    filebyte --disk sda1 --tree      Show tree for disk sda1");
        println!("    filebyte -s                      Show file sizes in auto units");
        println!("    filebyte -s mb                   Show file sizes in megabytes");
        println!("    filebyte -f /path/to/file        Analyze specific file");
        println!("    filebyte -d /path/to/dir         Analyze directory as a whole");
        println!("    filebyte -r                     Enable recursive searching");
        return;
    }

    let show_size = matches.contains_id("size");
    let size_unit_str = matches.get_one::<String>("size").unwrap_or(&"auto".to_string()).clone();
    let auto_size = size_unit_str == "auto";
    let size_unit = match SizeUnit::from_str(&size_unit_str) {
        Ok(unit) => unit,
        Err(e) => {
            eprintln!("Error: {}", e);
            eprintln!("Available options are: auto, b/bytes, kb/kilobytes, mb/megabytes, gb/gigabytes, tb/terabytes");
            process::exit(1);
        }
    };

    let color = !matches.get_flag("no-color");
    let show_detailed_permissions = true;

    let search_pattern = matches.get_one::<String>("search");
    let excluding_pattern = matches.get_one::<String>("excluding");
    let sort_by = matches.get_one::<String>("sort_by").map(|s| match s.to_lowercase().as_str() {
        "name" => SortBy::Name,
        "size" => SortBy::Size,
        "date" => SortBy::Date,
        _ => SortBy::Name,
    });

    
    if let Some(disk_arg) = matches.get_one::<String>("disk") {
        if disk_arg == "list" {
            list_disks(color, &size_unit, auto_size);
            return;
        } else {
            show_disk_info(disk_arg, &size_unit, color, auto_size, matches.get_flag("tree"), matches.get_flag("properties"), search_pattern, excluding_pattern, sort_by, matches.get_flag("duplicates"), show_size, show_detailed_permissions);
            return;
        }
    }

    let file_path = matches.get_one::<String>("file");
    let dir_path = matches.get_one::<String>("directory");
    let whole_path = matches.get_one::<String>("path");

    if matches.get_flag("whole") {
        if let Some(path_str) = whole_path {
            let path = Path::new(path_str);
            if !path.exists() {
                eprintln!("Error: Path '{}' does not exist", path_str);
                process::exit(1);
            }

            if path.is_file() {
                // Analyze the file
                let size = get_file_size(path);
                let size_str = if auto_size {
                    SizeUnit::auto_format_size(size)
                } else {
                    size_unit.format_size(size)
                };
                let file_name = path.file_name().unwrap_or_default().to_string_lossy();

                let metadata = match fs::metadata(path) {
                    Ok(m) => m,
                    Err(e) => {
                        eprintln!("Error reading metadata: {}", e);
                        process::exit(1);
                    }
                };

                let permissions = if metadata.permissions().readonly() {
                    if can_delete(path) { "r-x" } else { "r--" }
                } else {
                    if can_delete(path) { "rwx" } else { "rw-" }
                };
                let modified = metadata.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                let created = metadata.created().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                let modified_str = DateTime::<Utc>::from(modified).format("%Y-%m-%d %H:%M:%S UTC").to_string();
                let created_str = DateTime::<Utc>::from(created).format("%Y-%m-%d %H:%M:%S UTC").to_string();

                let file_type = infer::get_from_path(path)
                    .ok()
                    .flatten()
                    .map(|kind| kind.mime_type().to_string())
                    .unwrap_or_else(|| "unknown".to_string());

                let extension = if let Some(ext) = path.extension() {
                    ext.to_string_lossy().to_string()
                } else if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    if file_name.starts_with('.') {
                        // For dotfiles like .gitignore, extract the extension part
                        let parts: Vec<&str> = file_name.split('.').collect();
                        if parts.len() >= 2 {
                            parts[1..].join(".")
                        } else {
                            "none".to_string()
                        }
                    } else {
                        "none".to_string()
                    }
                } else {
                    "none".to_string()
                };

                println!("");
                println!("File Analysis:");
                println!("{}", "─".repeat(50));
                if color {
                    println!("Name: {}", file_name.blue().bold());
                    println!("Path: {}", path.canonicalize().unwrap_or(path.to_path_buf()).display());
                    println!("Size: {}", size_str.green().bold());
                    println!("Type: {}", file_type.magenta());
                    println!("Extension: {}", extension.cyan());
                    println!("Permissions: {}", permissions.yellow());
                    println!("Created: {}", created_str.yellow());
                    println!("Modified: {}", modified_str.yellow());
                } else {
                    println!("Name: {}", file_name);
                    println!("Path: {}", path.canonicalize().unwrap_or(path.to_path_buf()).display());
                    println!("Size: {}", size_str);
                    println!("Type: {}", file_type);
                    println!("Extension: {}", extension);
                    println!("Permissions: {}", permissions);
                    println!("Created: {}", created_str);
                    println!("Modified: {}", modified_str);
                }
            } else if path.is_dir() {
                // Analyze the directory as a whole
                let dir_size = get_file_size(path);
                let size_str = if auto_size {
                    SizeUnit::auto_format_size(dir_size)
                } else {
                    size_unit.format_size(dir_size)
                };

                let metadata = match fs::metadata(path) {
                    Ok(m) => m,
                    Err(e) => {
                        eprintln!("Error reading metadata: {}", e);
                        process::exit(1);
                    }
                };

                let permissions = format_unix_permissions(&metadata, show_detailed_permissions);
                let modified = metadata.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                let created = metadata.created().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                let modified_str = DateTime::<Utc>::from(modified).format("%Y-%m-%d %H:%M:%S UTC").to_string();
                let created_str = DateTime::<Utc>::from(created).format("%Y-%m-%d %H:%M:%S UTC").to_string();

                println!("                                        ");
                println!("Directory Analysis:");
                println!("{}", "─".repeat(50));
                if color {
                    println!("Name: {}", path.file_name().unwrap_or_default().to_string_lossy().blue().bold());
                    println!("Path: {}", path.display());
                    println!("Size: {}", size_str.green().bold());
                    println!("Permissions: {}", permissions.yellow());
                    println!("Created: {}", created_str.yellow());
                    println!("Modified: {}", modified_str.yellow());
                } else {
                    println!("Name: {}", path.file_name().unwrap_or_default().to_string_lossy());
                    println!("Path: {}", path.display());
                    println!("Size: {}", size_str);
                    println!("Permissions: {}", permissions);
                    println!("Created: {}", created_str);
                    println!("Modified: {}", modified_str);
                }
            } else {
                eprintln!("Error: Path '{}' is neither a file nor a directory", path_str);
                process::exit(1);
            }
        } else {
            eprintln!("Error: --whole requires a path argument");
            process::exit(1);
        }
        return;
    }

    if let Some(file) = file_path {
        let path = Path::new(file);
        if !path.exists() {
            eprintln!("Error: File '{}' not found", file);
            process::exit(1);
        }
        if !path.is_file() {
            eprintln!("Error: '{}' is not a file", file);
            process::exit(1);
        }
        // Analyze the file
        let size = get_file_size(path);
        let size_str = if auto_size {
            SizeUnit::auto_format_size(size)
        } else {
            size_unit.format_size(size)
        };
        let file_name = path.file_name().unwrap_or_default().to_string_lossy();

        let metadata = match fs::metadata(path) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("Error reading metadata: {}", e);
                process::exit(1);
            }
        };

        let permissions = format_unix_permissions(&metadata, show_detailed_permissions);
        let modified = metadata.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        let created = metadata.created().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        let modified_str = DateTime::<Utc>::from(modified).format("%Y-%m-%d %H:%M:%S UTC").to_string();
        let created_str = DateTime::<Utc>::from(created).format("%Y-%m-%d %H:%M:%S UTC").to_string();

        let file_type = infer::get_from_path(path)
            .ok()
            .flatten()
            .map(|kind| kind.mime_type().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let extension = if let Some(ext) = path.extension() {
            ext.to_string_lossy().to_string()
        } else if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
            if file_name.starts_with('.') {
                // For dotfiles like .gitignore, extract the extension part
                let parts: Vec<&str> = file_name.split('.').collect();
                if parts.len() >= 2 {
                    parts[1..].join(".")
                } else {
                    "none".to_string()
                }
            } else {
                "none".to_string()
            }
        } else {
            "none".to_string()
        };

        println!("");
        println!("File Analysis:");
        println!("{}", "─".repeat(50));
        if color {
            println!("Name: {}", file_name.blue().bold());
            println!("Path: {}", path.canonicalize().unwrap_or(path.to_path_buf()).display());
            println!("Size: {}", size_str.green().bold());
            println!("Type: {}", file_type.magenta());
            println!("Extension: {}", extension.cyan());
            println!("Permissions: {}", permissions.yellow());
            println!("Created: {}", created_str.yellow());
            println!("Modified: {}", modified_str.yellow());
        } else {
            println!("Name: {}", file_name);
            println!("Path: {}", path.canonicalize().unwrap_or(path.to_path_buf()).display());
            println!("Size: {}", size_str);
            println!("Type: {}", file_type);
            println!("Extension: {}", extension);
            println!("Permissions: {}", permissions);
            println!("Created: {}", created_str);
            println!("Modified: {}", modified_str);
        }
        return;
    }

    if let Some(dir) = dir_path {
        let path = Path::new(dir);
        if !path.exists() {
            eprintln!("Error: Directory '{}' not found", dir);
            process::exit(1);
        }
        if !path.is_dir() {
            eprintln!("Error: '{}' is not a directory", dir);
            process::exit(1);
        }
        // Analyze the directory as a whole
        let dir_size = get_file_size(path);
        let size_str = if auto_size {
            SizeUnit::auto_format_size(dir_size)
        } else {
            size_unit.format_size(dir_size)
        };

        let metadata = match fs::metadata(path) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("Error reading metadata: {}", e);
                process::exit(1);
            }
        };

        let permissions = format_unix_permissions(&metadata, show_detailed_permissions);
        let modified = metadata.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        let created = metadata.created().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        let modified_str = DateTime::<Utc>::from(modified).format("%Y-%m-%d %H:%M:%S UTC").to_string();
        let created_str = DateTime::<Utc>::from(created).format("%Y-%m-%d %H:%M:%S UTC").to_string();

        println!("                                        ");
        println!("Directory Analysis:");
        println!("{}", "─".repeat(50));
        if color {
            println!("Name: {}", path.file_name().unwrap_or_default().to_string_lossy().blue().bold());
            println!("Path: {}", path.display());
            println!("Size: {}", size_str.green().bold());
            println!("Permissions: {}", permissions.yellow());
            println!("Created: {}", created_str.yellow());
            println!("Modified: {}", modified_str.yellow());
        } else {
            println!("Name: {}", path.file_name().unwrap_or_default().to_string_lossy());
            println!("Path: {}", path.display());
            println!("Size: {}", size_str);
            println!("Permissions: {}", permissions);
            println!("Created: {}", created_str);
            println!("Modified: {}", modified_str);
        }
        return;
    }

    let path = if let Some(path_arg) = matches.get_one::<String>("path") {
        Path::new(path_arg)
    } else {
        Path::new(".")
    };

    if !path.exists() {
        eprintln!("Error: Path '{}' does not exist", path.display());
        process::exit(1);
    }

    // If path is a file and no specific flags are set, analyze it directly
    if path.is_file() && !matches.get_flag("tree") && !matches.get_flag("properties") && !matches.get_flag("duplicates") && !matches.get_flag("recursive") && search_pattern.is_none() && excluding_pattern.is_none() && sort_by.is_none() && matches.get_one::<String>("export").is_none() {
        // Analyze the file directly
        let size = get_file_size(path);
        let size_str = if auto_size {
            SizeUnit::auto_format_size(size)
        } else {
            size_unit.format_size(size)
        };
        let file_name = path.file_name().unwrap_or_default().to_string_lossy();

        let metadata = match fs::metadata(path) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("Error reading metadata: {}", e);
                process::exit(1);
            }
        };

        let permissions = format_unix_permissions(&metadata, show_detailed_permissions);
        let modified = metadata.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        let created = metadata.created().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        let modified_str = DateTime::<Utc>::from(modified).format("%Y-%m-%d %H:%M:%S UTC").to_string();
        let created_str = DateTime::<Utc>::from(created).format("%Y-%m-%d %H:%M:%S UTC").to_string();

        let file_type = infer::get_from_path(path)
            .ok()
            .flatten()
            .map(|kind| kind.mime_type().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let extension = if let Some(ext) = path.extension() {
            ext.to_string_lossy().to_string()
        } else if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
            if file_name.starts_with('.') {
                // For dotfiles like .gitignore, extract the extension part
                let parts: Vec<&str> = file_name.split('.').collect();
                if parts.len() >= 2 {
                    parts[1..].join(".")
                } else {
                    "none".to_string()
                }
            } else {
                "none".to_string()
            }
        } else {
            "none".to_string()
        };

        println!("");
        println!("File Analysis:");
        println!("{}", "─".repeat(50));
        if color {
            println!("Name: {}", file_name.blue().bold());
            println!("Path: {}", path.canonicalize().unwrap_or(path.to_path_buf()).display());
            println!("Size: {}", size_str.green().bold());
            println!("Type: {}", file_type.magenta());
            println!("Extension: {}", extension.cyan());
            println!("Permissions: {}", permissions.yellow());
            println!("Created: {}", created_str.yellow());
            println!("Modified: {}", modified_str.yellow());
        } else {
            println!("Name: {}", file_name);
            println!("Path: {}", path.canonicalize().unwrap_or(path.to_path_buf()).display());
            println!("Size: {}", size_str);
            println!("Type: {}", file_type);
            println!("Extension: {}", extension);
            println!("Permissions: {}", permissions);
            println!("Created: {}", created_str);
            println!("Modified: {}", modified_str);
        }
        return;
    }

    if matches.get_flag("tree") {
        if path.is_dir() {
            println!("{}", path.display());
            print_tree(path, "", color);
        } else {
            eprintln!("Error: --tree can only be used with directories");
            process::exit(1);
        }
    } else if matches.get_flag("properties") {
        if path.is_file() {
            let size = get_file_size(path);
            let size_str = if auto_size {
                SizeUnit::auto_format_size(size)
            } else {
                size_unit.format_size(size)
            };
            let file_name = path.file_name().unwrap_or_default().to_string_lossy();

            let metadata = match fs::metadata(path) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("Error reading metadata: {}", e);
                    process::exit(1);
                }
            };

            let permissions = format_unix_permissions(&metadata, show_detailed_permissions);
            let modified = metadata.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            let created = metadata.created().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            let modified_str = DateTime::<Utc>::from(modified).format("%Y-%m-%d %H:%M:%S UTC").to_string();
            let created_str = DateTime::<Utc>::from(created).format("%Y-%m-%d %H:%M:%S UTC").to_string();

            let file_type = infer::get_from_path(path)
                .ok()
                .flatten()
                .map(|kind| kind.mime_type().to_string())
                .unwrap_or_else(|| "unknown".to_string());

            let extension = path.extension()
                .map(|ext| ext.to_string_lossy().to_string())
                .unwrap_or_else(|| "none".to_string());

            println!("                                        ");
            println!("File Analysis:");
            println!("{}", "─".repeat(50));
            if color {
                println!("Name: {}", file_name.blue().bold());
                println!("Path: {}", path.display());
                println!("Size: {}", size_str.green().bold());
                println!("Type: {}", file_type.magenta());
                println!("Extension: {}", extension.cyan());
                println!("Permissions: {}", permissions.yellow());
                println!("Created: {}", created_str.yellow());
                println!("Modified: {}", modified_str.yellow());
            } else {
                println!("Name: {}", file_name);
                println!("Path: {}", path.display());
                println!("Size: {}", size_str);
                println!("Type: {}", file_type);
                println!("Extension: {}", extension);
                println!("Permissions: {}", permissions);
                println!("Created: {}", created_str);
                println!("Modified: {}", modified_str);
            }
        } else if path.is_dir() {
            
            let files = collect_files_recursive(path, search_pattern, excluding_pattern, sort_by);
            if files.is_empty() {
                println!("No files found in directory.");
            } else {
                let total_files = files.len();
                let total_dirs = files.iter().filter(|f| f.is_directory).count();
                let total_regular_files = total_files - total_dirs;
                let _total_size: u64 = files.iter().map(|f| f.size).sum();

                
                let dir_size = get_file_size(path);
                if color {
                    println!("Directory: {}", path.display());
                    println!("Total Items: {} ({})", total_files.to_string().cyan(), format!("{} files, {} dirs", total_regular_files, total_dirs).yellow());
                    println!("Total Size: {}", SizeUnit::auto_format_size(dir_size).green().bold());
                } else {
                    println!("Directory: {}", path.display());
                    println!("Total Items: {} ({} files, {} dirs)", total_files, total_regular_files, total_dirs);
                    println!("Total Size: {}", SizeUnit::auto_format_size(dir_size));
                }

                show_file_type_stats(&files, color);
                show_detailed_analysis(&files, color);
            }
        } else {
            eprintln!("Error: Path '{}' does not exist", path.display());
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
            let files = if matches.get_flag("recursive") {
                collect_files_recursive(path, search_pattern, excluding_pattern, sort_by)
            } else {
                collect_files(path, search_pattern, excluding_pattern, sort_by)
            };
            if files.is_empty() {
                if let Some(pattern) = search_pattern {
                    println!("No files found matching pattern: {}", pattern);
                } else {
                    println!("No files found.");
                }
            } else {
                if search_pattern.is_some() {
                    show_search_results(&files, search_pattern.unwrap(), color);
                } else {
                    display_files(&files, &size_unit, color, matches.get_flag("properties"), auto_size, show_size, matches.get_one::<String>("export"), show_detailed_permissions);
                    if !matches.get_flag("properties") && matches.get_flag("recursive") {
                        show_file_type_stats(&files, color);
                    }
                }
            }


        }
    }
}