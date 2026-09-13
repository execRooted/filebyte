use crate::types::{FileInfo, HashAlgorithm};
use crate::utils::{compute_file_hash, delete_duplicate_file, merge_duplicate_file};
use colored::Colorize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub fn find_duplicates(
    dir: &Path,
    color: bool,
    content_dups: bool,
    hash_algorithm: HashAlgorithm,
) -> Vec<DuplicateGroup> {
    let mut size_map: HashMap<u64, Vec<String>> = HashMap::new();
    let mut duplicates: Vec<DuplicateGroup> = Vec::new();

    fn scan_for_duplicates(path: &Path, size_map: &mut HashMap<u64, Vec<String>>) {
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                let file_name = entry_path.file_name().unwrap_or_default().to_string_lossy();
                if file_name == ".kilo" && entry_path.is_dir() {
                    continue;
                }
                if entry_path.is_file() {
                    if let Ok(metadata) = entry.metadata() {
                        let size = metadata.len();
                        size_map
                            .entry(size)
                            .or_insert_with(Vec::new)
                            .push(entry_path.to_string_lossy().to_string());
                    }
                } else if entry_path.is_dir() {
                    scan_for_duplicates(&entry_path, size_map);
                }
            }
        }
    }

    scan_for_duplicates(dir, &mut size_map);

    for (size, paths) in size_map.iter() {
        if paths.len() > 1 {
            if content_dups {
                let mut hash_map: HashMap<String, Vec<String>> = HashMap::new();
                for path_str in paths {
                    if let Some(hash) = compute_file_hash(Path::new(path_str), hash_algorithm) {
                        hash_map
                            .entry(hash)
                            .or_insert_with(Vec::new)
                            .push(path_str.clone());
                    }
                }
                for (hash, dup_paths) in hash_map.iter() {
                    if dup_paths.len() > 1 {
                        duplicates.push(DuplicateGroup {
                            size: *size,
                            hash: Some(hash.clone()),
                            paths: dup_paths.clone(),
                        });
                    }
                }
            } else {
                duplicates.push(DuplicateGroup {
                    size: *size,
                    hash: None,
                    paths: paths.clone(),
                });
            }
        }
    }

    print_duplicate_groups(&duplicates, color, hash_algorithm);
    duplicates
}

fn print_duplicate_groups(
    groups: &[DuplicateGroup],
    color: bool,
    hash_algorithm: HashAlgorithm,
) {
    if groups.is_empty() {
        println!("{}", crate::i18n::tr("no_duplicate_files"));
    } else {
        println!("{}", crate::i18n::tr("duplicate_files_found"));
        println!("{}", "─".repeat(50));

        for group in groups {
            if color {
                let size_str = crate::types::SizeUnit::auto_format_size(group.size).cyan();
                if let Some(hash) = &group.hash {
                    let hash_display = hash.chars().take(16).collect::<String>();
                    let algo_name = hash_algorithm.display_name().yellow();
                    let files_count = group.paths.len().to_string().yellow();
                    let template = crate::i18n::tr("duplicate_group_hash");
                    let result = template
                        .replacen("{}", &format!("{}", size_str), 1)
                        .replacen("{}", &format!("{}", algo_name), 1)
                        .replacen("{}", &hash_display, 1)
                        .replacen("{}", &files_count, 1);
                    println!("{}", result);
                } else {
                    let files_count = group.paths.len().to_string().yellow();
                    let template = crate::i18n::tr("duplicate_group_size");
                    let result = template
                        .replacen("{}", &format!("{}", size_str), 1)
                        .replacen("{}", &files_count, 1);
                    println!("{}", result);
                }
            } else {
                let size_str = crate::types::SizeUnit::auto_format_size(group.size);
                if let Some(hash) = &group.hash {
                    let hash_display = hash.chars().take(16).collect::<String>();
                    let template = crate::i18n::tr("duplicate_group_hash");
                    let result = template
                        .replacen("{}", &size_str, 1)
                        .replacen("{}", &hash_algorithm.display_name(), 1)
                        .replacen("{}", &hash_display, 1)
                        .replacen("{}", &group.paths.len().to_string(), 1);
                    println!("{}", result);
                } else {
                    let template = crate::i18n::tr("duplicate_group_size");
                    let result = template
                        .replacen("{}", &size_str, 1)
                        .replacen("{}", &group.paths.len().to_string(), 1);
                    println!("{}", result);
                }
            }
            for path in &group.paths {
                println!("  {}", path);
            }
            println!();
        }
    }
}

pub fn apply_duplicate_action(
    groups: &[DuplicateGroup],
    action: crate::types::DuplicateAction,
    force: bool,
) {
    if groups.is_empty() {
        return;
    }

    match action {
        crate::types::DuplicateAction::Delete => {
            for group in groups {
                if group.paths.len() <= 1 {
                    continue;
                }
                for path in &group.paths[1..] {
                    let p = Path::new(path);
                    if p.exists() {
                        if delete_duplicate_file(p, force) {
                            if force {
                                println!("{}", crate::i18n::tr_format("action_deleted_format", &[path]));
                            }
                        } else if !force {
                            println!("{}", crate::i18n::tr_format("action_skipped_format", &[path]));
                        }
                    }
                }
            }
        }
        crate::types::DuplicateAction::Merge => {
            for group in groups {
                if group.paths.len() <= 1 {
                    continue;
                }
                let target = Path::new(&group.paths[0]);
                for path in &group.paths[1..] {
                    let p = Path::new(path);
                    if p.exists() {
                        if merge_duplicate_file(p, target) {
                            println!("{}", crate::i18n::tr_format("action_merged_format", &[path, &group.paths[0]]));
                        } else {
                            eprintln!("{}", crate::i18n::tr_format("action_merge_failed_format", &[path]));
                        }
                    }
                }
            }
        }
        crate::types::DuplicateAction::None => {}
    }
}

pub fn show_detailed_analysis(files: &[FileInfo], color: bool) {
    let total_files = files.len();
    let total_dirs = files.iter().filter(|f| f.is_directory).count();
    let total_regular_files = total_files - total_dirs;
    let _total_size: u64 = files.iter().map(|f| f.size).sum();
    println!("");
    println!("{}", crate::i18n::tr("detailed_analysis"));
    println!("{}", "-".repeat(50));

    if color {
        let items_label = format!("{} {}", total_files.to_string().cyan(), crate::i18n::tr_format("label_files_dirs_format", &[&total_regular_files.to_string(), &total_dirs.to_string()]).yellow());
        println!("{}", crate::i18n::tr_format("label_total_items_format", &[&total_files.to_string().cyan().to_string(), &items_label]));
    } else {
        let items_label = crate::i18n::tr_format("label_files_dirs_format", &[&total_regular_files.to_string(), &total_dirs.to_string()]);
        println!("{}", crate::i18n::tr_format("label_total_items_format", &[&total_files.to_string(), &items_label]));
    }

    let size_ranges: [(String, std::ops::Range<u64>); 6] = [
        (crate::i18n::tr("size_range_empty"), 0..1),
        (crate::i18n::tr("tiny"), 1..1024),
        (crate::i18n::tr("small"), 1024..1024 * 1024),
        (crate::i18n::tr("medium"), 1024 * 1024..100 * 1024 * 1024),
        (crate::i18n::tr("large"), 100 * 1024 * 1024..1024 * 1024 * 1024),
        (crate::i18n::tr("huge"), 1024 * 1024 * 1024..u64::MAX),
    ];
    println!("\n{}", crate::i18n::tr("size_distribution"));
    for (label, range) in &size_ranges {
        let count = files.iter().filter(|f| range.contains(&f.size)).count();
        if count > 0 {
            let percentage = count as f64 / total_files as f64 * 100.0;
            let pct_str = format!("{:.1}", percentage);
            let label_str = if color { format!("{}", label.magenta()) } else { label.clone() };
            let count_str = if color { count.to_string().cyan().to_string() } else { count.to_string() };
            println!("{}", crate::i18n::tr_format("size_distribution_item", &[&label_str, &count_str, &pct_str]));
        }
    }

    let now = std::time::SystemTime::now();
    let age_ranges = [
        crate::i18n::tr("today"),
        crate::i18n::tr("this_week"),
        crate::i18n::tr("this_month"),
        crate::i18n::tr("this_year"),
        crate::i18n::tr("older"),
    ];
    let age_seconds: [std::ops::Range<u64>; 5] = [
        0..86400,
        86400..604800,
        604800..2592000,
        2592000..31536000,
        31536000..u64::MAX,
    ];
    println!("\n{}", crate::i18n::tr("file_age_distribution"));
    for (i, label) in age_ranges.iter().enumerate() {
        let range = &age_seconds[i];
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
            let pct_str = format!("{:.1}", percentage);
            let label_str = if color { format!("{}", label.magenta()) } else { label.clone() };
            let count_str = if color { count.to_string().cyan().to_string() } else { count.to_string() };
            println!("{}", crate::i18n::tr_format("size_distribution_item", &[&label_str, &count_str, &pct_str]));
        }
    }

    if let Some(largest) = files.iter().filter(|f| !f.is_directory).max_by_key(|f| f.size) {
        if color {
            println!(
                "\n{}",
                crate::i18n::tr_format("largest_file_format", &[&largest.name.cyan().to_string(), &largest.size_human.green().to_string()])
            );
        } else {
            println!("\n{}", crate::i18n::tr_format("largest_file_format", &[&largest.name, &largest.size_human]));
        }
    }
    if let Some(smallest) = files.iter().filter(|f| !f.is_directory && f.size > 0).min_by_key(|f| f.size) {
        if color {
            println!(
                "{}",
                crate::i18n::tr_format("smallest_file_format", &[&smallest.name.cyan().to_string(), &smallest.size_human.green().to_string()])
            );
        } else {
            println!("{}", crate::i18n::tr_format("smallest_file_format", &[&smallest.name, &smallest.size_human]));
        }
    }

    let readable = files.iter().filter(|f| f.permissions.contains('r')).count();
    let writable = files.iter().filter(|f| f.permissions.contains('w')).count();
    let readable_only = files.iter().filter(|f| f.permissions == "r").count();
    let writable_only = files.iter().filter(|f| f.permissions == "rw").count();
    println!("\n{}", crate::i18n::tr("permissions_summary"));
    if color {
        let pct_r = format!("{:.1}", readable as f64 / total_files as f64 * 100.0);
        let pct_w = format!("{:.1}", writable as f64 / total_files as f64 * 100.0);
        let pct_ro = format!("{:.1}", readable_only as f64 / total_files as f64 * 100.0);
        let pct_rw = format!("{:.1}", writable_only as f64 / total_files as f64 * 100.0);
        println!("{}", crate::i18n::tr_format("permissions_readable", &[&readable.to_string().cyan().to_string(), &pct_r]));
        println!("{}", crate::i18n::tr_format("permissions_writable", &[&writable.to_string().cyan().to_string(), &pct_w]));
        println!("{}", crate::i18n::tr_format("permissions_read_only", &[&readable_only.to_string().cyan().to_string(), &pct_ro]));
        println!("{}", crate::i18n::tr_format("permissions_read_write", &[&writable_only.to_string().cyan().to_string(), &pct_rw]));
    } else {
        let pct_r = format!("{:.1}", readable as f64 / total_files as f64 * 100.0);
        let pct_w = format!("{:.1}", writable as f64 / total_files as f64 * 100.0);
        let pct_ro = format!("{:.1}", readable_only as f64 / total_files as f64 * 100.0);
        let pct_rw = format!("{:.1}", writable_only as f64 / total_files as f64 * 100.0);
        println!("{}", crate::i18n::tr_format("permissions_readable", &[&readable.to_string(), &pct_r]));
        println!("{}", crate::i18n::tr_format("permissions_writable", &[&writable.to_string(), &pct_w]));
        println!("{}", crate::i18n::tr_format("permissions_read_only", &[&readable_only.to_string(), &pct_ro]));
        println!("{}", crate::i18n::tr_format("permissions_read_write", &[&writable_only.to_string(), &pct_rw]));
    }
}

pub(crate) struct DuplicateGroup {
    size: u64,
    hash: Option<String>,
    paths: Vec<String>,
}
