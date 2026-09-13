use crate::analysis::{apply_duplicate_action, find_duplicates, show_detailed_analysis};
use crate::collect::{collect_files_extended, collect_files_recursive_extended};
use crate::display::{display_files, show_file_type_stats};
use crate::tree::print_tree;
use crate::types::{DuplicateAction, HashAlgorithm, SizeUnit, SortBy};
use colored::Colorize;
use sysinfo::Disks;
use std::path::Path;

/// List all available disks
pub fn list_disks(color: bool, size_unit: &SizeUnit, auto_size: bool) {
    let disks = Disks::new_with_refreshed_list();
    println!("");
    println!("{}", crate::i18n::tr("available_disks"));
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
            let name_str = format!("{}", name.blue().bold());
            let mount_str = format!("{}", mount_point);
            let total_str = format!("{}", total_space.cyan());
            let used_str = format!("{}", used_space.red());
            let avail_str = format!("{}", available_space.green());
            println!("{}", crate::i18n::tr_format("label_disk_format", &[&name_str, &mount_str, &total_str, &used_str, &avail_str]));
        } else {
            let name_str = name.to_string();
            let mount_str = format!("{}", mount_point);
            let total_str = total_space;
            let used_str = used_space;
            let avail_str = available_space;
            println!("{}", crate::i18n::tr_format("label_disk_format", &[&name_str, &mount_str, &total_str, &used_str, &avail_str]));
        }
    }
}

/// Show detailed information about a specific disk
pub fn show_disk_info(
    disk_name: &str,
    size_unit: &SizeUnit,
    color: bool,
    auto_size: bool,
    tree: bool,
    properties: bool,
    search_pattern: Option<&String>,
    excluding_pattern: Option<&String>,
    sort_by: Option<SortBy>,
    duplicates: bool,
    content_dups: bool,
    hash_algorithm: HashAlgorithm,
    show_size: bool,
    show_detailed_permissions: bool,
    exclude_dirs: bool,
    min_size: Option<u64>,
    max_size: Option<u64>,
    equal_size: Option<u64>,
    min_age_seconds: Option<i64>,
    max_age_seconds: Option<i64>,
    empty_only: bool,
    content_pattern: Option<&String>,
    duplicate_action: DuplicateAction,
    force: bool,
) {
    let disks = Disks::new_with_refreshed_list();
    let disk = disks.iter().find(|d| d.name().to_string_lossy() == disk_name);

    match disk {
        Some(disk) => {
            let mount_point = disk.mount_point();
            let total_space = disk.total_space();
            let available_space = disk.available_space();
            let used_space = total_space - available_space;
            let usage_percentage = used_space as f64 / total_space as f64 * 100.0;

            println!("");
            if color {
                println!("{} {}", crate::i18n::tr("disk_information"), disk_name.blue().bold());
                println!("{} {}", crate::i18n::tr("mount_point"), mount_point.display().to_string().cyan());
                println!("{} {}", crate::i18n::tr("total_space"), SizeUnit::auto_format_size(total_space).cyan());
                println!("{} {}", crate::i18n::tr("used_space"), SizeUnit::auto_format_size(used_space).red());
                println!(
                    "{} {}",
                    crate::i18n::tr("available_space"),
                    SizeUnit::auto_format_size(available_space).green()
                );
                println!("{}: {:.1}%", crate::i18n::tr("usage"), usage_percentage.to_string().yellow());
            } else {
                println!("{} {}", crate::i18n::tr("disk_information"), disk_name);
                println!("{} {}", crate::i18n::tr("mount_point"), mount_point.display());
                println!("{} {}", crate::i18n::tr("total_space"), SizeUnit::auto_format_size(total_space));
                println!("{} {}", crate::i18n::tr("used_space"), SizeUnit::auto_format_size(used_space));
                println!(
                    "{} {}",
                    crate::i18n::tr("available_space"),
                    SizeUnit::auto_format_size(available_space)
                );
                println!("{}: {:.1}%", crate::i18n::tr("usage"), usage_percentage);
            }

            let files = collect_files_extended(mount_point, None, None, None, None, exclude_dirs, false, min_size, max_size, equal_size, min_age_seconds, max_age_seconds, empty_only, content_pattern);
            if !files.is_empty() {
                let total_files = files.len();
                let total_dirs = files.iter().filter(|f| f.is_directory).count();
                let total_regular_files = total_files - total_dirs;
                let dir_size = get_file_size(mount_point);
                if color {
                    let dir_str = format!("{}", mount_point.display());
                    let items_label = crate::i18n::tr_format("label_files_dirs_format", &[&total_regular_files.to_string(), &total_dirs.to_string()]);
                    println!("{} {}", crate::i18n::tr("directory"), dir_str);
                    println!("{}", crate::i18n::tr_format("label_total_items_format", &[&total_files.to_string().cyan().to_string(), &items_label.yellow().to_string()]));
                    println!("{} {}", crate::i18n::tr("total_size"), SizeUnit::auto_format_size(dir_size).green().bold());
                } else {
                    let items_label = crate::i18n::tr_format("label_files_dirs_format", &[&total_regular_files.to_string(), &total_dirs.to_string()]);
                    println!("{} {}", crate::i18n::tr("directory"), mount_point.display());
                    println!("{}", crate::i18n::tr_format("label_total_items_format", &[&total_files.to_string(), &items_label]));
                    println!("{} {}", crate::i18n::tr("total_size"), SizeUnit::auto_format_size(dir_size));
                }
            }

            if duplicates {
                let groups = find_duplicates(mount_point, color, content_dups, hash_algorithm);
                if duplicate_action != DuplicateAction::None {
                    apply_duplicate_action(&groups, duplicate_action, force);
                }
            } else if tree {
                println!("\n{}", crate::i18n::tr("label_directory_tree"));
                print_tree(mount_point, "", color);
            } else if properties {
                let files = collect_files_recursive_extended(
                    mount_point,
                    search_pattern,
                    excluding_pattern,
                    None,
                    sort_by,
                    exclude_dirs,
                    false,
                    None,
                    min_size,
                    max_size,
                    equal_size,
                    min_age_seconds,
                    max_age_seconds,
                    empty_only,
                    content_pattern,
                );
                if files.is_empty() {
                    println!("{}", crate::i18n::tr("no_files_found"));
                } else {
                    let total_files = files.len();
                    let total_dirs = files.iter().filter(|f| f.is_directory).count();
                    let total_regular_files = total_files - total_dirs;
                    let _total_size: u64 = files.iter().map(|f| f.size).sum();
                    let dir_size = get_file_size(mount_point);
                    println!("");
                    if color {
                        let dir_str = format!("{}", mount_point.display());
                        let items_label = crate::i18n::tr_format("label_files_dirs_format", &[&total_regular_files.to_string(), &total_dirs.to_string()]);
                        println!("{} {}", crate::i18n::tr("directory"), dir_str);
                        println!("{}", crate::i18n::tr_format("label_total_items_format", &[&total_files.to_string().cyan().to_string(), &items_label.yellow().to_string()]));
                        println!("{} {}", crate::i18n::tr("total_size"), SizeUnit::auto_format_size(dir_size).green().bold());
                    } else {
                        let items_label = crate::i18n::tr_format("label_files_dirs_format", &[&total_regular_files.to_string(), &total_dirs.to_string()]);
                        println!("{} {}", crate::i18n::tr("directory"), mount_point.display());
                        println!("{}", crate::i18n::tr_format("label_total_items_format", &[&total_files.to_string(), &items_label]));
                        println!("{} {}", crate::i18n::tr("total_size"), SizeUnit::auto_format_size(dir_size));
                    }
                    println!("");
                    show_file_type_stats(&files, color);
                    show_detailed_analysis(&files, color);
                }
            } else if search_pattern.is_some() || excluding_pattern.is_some() || sort_by.is_some() {
                let files = collect_files_extended(mount_point, search_pattern, excluding_pattern, None, sort_by, exclude_dirs, false, min_size, max_size, equal_size, min_age_seconds, max_age_seconds, empty_only, content_pattern);
                if files.is_empty() {
                    if let Some(pattern) = search_pattern {
                        println!("{}", crate::i18n::tr_format("no_files_found_pattern", &[pattern]));
                    } else {
                        println!("{}", crate::i18n::tr("no_files_found"));
                    }
                } else {
                    display_files(
                        &files,
                        size_unit,
                        color,
                        false,
                        auto_size,
                        show_size,
                        None,
                        show_detailed_permissions,
                        false,
                        false,
                    );
                }
                show_file_type_stats(&files, color);
            }
        }
        None => {
            eprintln!("{}", crate::i18n::tr("error_disk_not_found").replace("{}", disk_name));
            eprintln!("{}", crate::i18n::tr("error_disk_use_list"));
            std::process::exit(1);
        }
    }
}

fn get_file_size(path: &Path) -> u64 {
    crate::utils::get_file_size(path)
}
