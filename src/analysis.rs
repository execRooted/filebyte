use crate::types::FileInfo;
use colored::Colorize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Find duplicate files in a directory
pub fn find_duplicates(dir: &Path, color: bool) {
    let mut hash_map: HashMap<u64, Vec<String>> = HashMap::new();
    let mut duplicates = Vec::new();

    fn scan_for_duplicates(
        path: &Path,
        hash_map: &mut HashMap<u64, Vec<String>>,
        _duplicates: &mut Vec<(u64, Vec<String>)>,
    ) {
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                if entry_path.is_file() {
                    if let Ok(metadata) = entry.metadata() {
                        let size = metadata.len();
                        hash_map
                            .entry(size)
                            .or_insert_with(Vec::new)
                            .push(entry_path.to_string_lossy().to_string());
                    }
                } else if entry_path.is_dir() {
                    scan_for_duplicates(&entry_path, hash_map, _duplicates);
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
                println!(
                    "Size: {} ({})",
                    crate::types::SizeUnit::auto_format_size(size).cyan(),
                    paths.len().to_string().yellow()
                );
            } else {
                println!(
                    "Size: {} ({})",
                    crate::types::SizeUnit::auto_format_size(size),
                    paths.len()
                );
            }
            for path in &paths {
                println!("  {}", path);
            }
            println!();
        }
    }
}

/// Show detailed analysis of files
pub fn show_detailed_analysis(files: &[FileInfo], color: bool) {
    let total_files = files.len();
    let total_dirs = files.iter().filter(|f| f.is_directory).count();
    let total_regular_files = total_files - total_dirs;
    let _total_size: u64 = files.iter().map(|f| f.size).sum();
    println!("");
    println!("Detailed Analysis:");
    println!("{}", "-".repeat(50));

    if color {
        println!(
            "Total Items: {} ({})",
            total_files.to_string().cyan(),
            format!("{} files, {} dirs", total_regular_files, total_dirs).yellow()
        );
    } else {
        println!(
            "Total Items: {} ({} files, {} dirs)",
            total_files, total_regular_files, total_dirs
        );
    }

    // Size distribution
    let size_ranges = [
        ("Empty (0 B)", 0..1),
        ("Tiny (< 1 KB)", 1..1024),
        ("Small (1 KB - 1 MB)", 1024..1024 * 1024),
        ("Medium (1 MB - 100 MB)", 1024 * 1024..100 * 1024 * 1024),
        ("Large (100 MB - 1 GB)", 100 * 1024 * 1024..1024 * 1024 * 1024),
        ("Huge (> 1 GB)", 1024 * 1024 * 1024..u64::MAX),
    ];
    println!("\nSize Distribution:");
    for (label, range) in &size_ranges {
        let count = files.iter().filter(|f| range.contains(&f.size)).count();
        if count > 0 {
            let percentage = count as f64 / total_files as f64 * 100.0;
            if color {
                println!(
                    "  {}: {} files ({:.1}%)",
                    label.magenta(),
                    count.to_string().cyan(),
                    percentage
                );
            } else {
                println!("  {}: {} files ({:.1}%)", label, count, percentage);
            }
        }
    }

    // File age distribution
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
        let count = files
            .iter()
            .filter(|f| {
                if let Some(modified_str) = &f.modified {
                    if let Ok(modified_time) =
                        chrono::DateTime::parse_from_rfc3339(&format!("{}Z", modified_str.replace(" UTC", "")))
                    {
                        let duration = now
                            .duration_since(modified_time.with_timezone(&chrono::Utc).into())
                            .unwrap_or_default();
                        range.contains(&duration.as_secs())
                    } else {
                        false
                    }
                } else {
                    false
                }
            })
            .count();

        if count > 0 {
            let percentage = count as f64 / total_files as f64 * 100.0;
            if color {
                println!(
                    "  {}: {} files ({:.1}%)",
                    label.magenta(),
                    count.to_string().cyan(),
                    percentage
                );
            } else {
                println!("  {}: {} files ({:.1}%)", label, count, percentage);
            }
        }
    }

    // Largest and smallest files
    if let Some(largest) = files.iter().filter(|f| !f.is_directory).max_by_key(|f| f.size) {
        if color {
            println!(
                "\nLargest File: {} ({})",
                largest.name.cyan(),
                largest.size_human.green()
            );
        } else {
            println!("\nLargest File: {} ({})", largest.name, largest.size_human);
        }
    }
    if let Some(smallest) = files.iter().filter(|f| !f.is_directory && f.size > 0).min_by_key(|f| f.size) {
        if color {
            println!(
                "Smallest File: {} ({})",
                smallest.name.cyan(),
                smallest.size_human.green()
            );
        } else {
            println!("Smallest File: {} ({})", smallest.name, smallest.size_human);
        }
    }

    // Permissions summary
    let readable = files.iter().filter(|f| f.permissions.contains('r')).count();
    let writable = files.iter().filter(|f| f.permissions.contains('w')).count();
    let readable_only = files.iter().filter(|f| f.permissions == "r").count();
    let writable_only = files.iter().filter(|f| f.permissions == "rw").count();
    println!("\nPermissions Summary:");
    if color {
        println!(
            "  Readable: {} files ({:.1}%)",
            readable.to_string().cyan(),
            readable as f64 / total_files as f64 * 100.0
        );
        println!(
            "  Writable: {} files ({:.1}%)",
            writable.to_string().cyan(),
            writable as f64 / total_files as f64 * 100.0
        );
        println!(
            "  Read-only: {} files ({:.1}%)",
            readable_only.to_string().cyan(),
            readable_only as f64 / total_files as f64 * 100.0
        );
        println!(
            "  Read-write: {} files ({:.1}%)",
            writable_only.to_string().cyan(),
            writable_only as f64 / total_files as f64 * 100.0
        );
    } else {
        println!(
            "  Readable: {} files ({:.1}%)",
            readable,
            readable as f64 / total_files as f64 * 100.0
        );
        println!(
            "  Writable: {} files ({:.1}%)",
            writable,
            writable as f64 / total_files as f64 * 100.0
        );
        println!(
            "  Read-only: {} files ({:.1}%)",
            readable_only,
            readable_only as f64 / total_files as f64 * 100.0
        );
        println!(
            "  Read-write: {} files ({:.1}%)",
            writable_only,
            writable_only as f64 / total_files as f64 * 100.0
        );
    }
}
