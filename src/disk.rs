use crate::analysis::{find_duplicates, show_detailed_analysis};
use crate::collect::{collect_files, collect_files_recursive};
use crate::display::{display_files, show_file_type_stats};
use crate::tree::print_tree;
use crate::types::{SizeUnit, SortBy};
use colored::Colorize;
use sysinfo::Disks;
use std::path::Path;

/// List all available disks
pub fn list_disks(color: bool, size_unit: &SizeUnit, auto_size: bool) {
    let disks = Disks::new_with_refreshed_list();
    println!("");
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
            println!(
                "{} ({}) - Total: {} | Used: {} | Available: {}",
                name.blue().bold(),
                mount_point,
                total_space.cyan(),
                used_space.red(),
                available_space.green()
            );
        } else {
            println!(
                "{} ({}) - Total: {} | Used: {} | Available: {}",
                name, mount_point, total_space, used_space, available_space
            );
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
    show_size: bool,
    show_detailed_permissions: bool,
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
                println!("Disk Information: {}", disk_name.blue().bold());
                println!("Mount Point: {}", mount_point.display().to_string().cyan());
                println!("Total Space: {}", SizeUnit::auto_format_size(total_space).cyan());
                println!("Used Space: {}", SizeUnit::auto_format_size(used_space).red());
                println!(
                    "Available Space: {}",
                    SizeUnit::auto_format_size(available_space).green()
                );
                println!("Usage: {:.1}%", usage_percentage.to_string().yellow());
            } else {
                println!("Disk Information: {}", disk_name);
                println!("Mount Point: {}", mount_point.display());
                println!("Total Space: {}", SizeUnit::auto_format_size(total_space));
                println!("Used Space: {}", SizeUnit::auto_format_size(used_space));
                println!(
                    "Available Space: {}",
                    SizeUnit::auto_format_size(available_space)
                );
                println!("Usage: {:.1}%", usage_percentage);
            }

            let files = collect_files(mount_point, None, None, None);
            if !files.is_empty() {
                let total_files = files.len();
                let total_dirs = files.iter().filter(|f| f.is_directory).count();
                let total_regular_files = total_files - total_dirs;
                let dir_size = get_file_size(mount_point);
                if color {
                    println!("Directory: {}", mount_point.display());
                    println!(
                        "Total Items: {} ({})",
                        total_files.to_string().cyan(),
                        format!("{} files, {} dirs", total_regular_files, total_dirs).yellow()
                    );
                    println!(
                        "Total Size: {}",
                        SizeUnit::auto_format_size(dir_size).green().bold()
                    );
                } else {
                    println!("Directory: {}", mount_point.display());
                    println!(
                        "Total Items: {} ({} files, {} dirs)",
                        total_files, total_regular_files, total_dirs
                    );
                    println!("Total Size: {}", SizeUnit::auto_format_size(dir_size));
                }
            }

            if duplicates {
                find_duplicates(mount_point, color);
            } else if tree {
                println!("\nDirectory Tree:");
                print_tree(mount_point, "", color);
            } else if properties {
                let files = collect_files_recursive(
                    mount_point,
                    search_pattern,
                    excluding_pattern,
                    sort_by,
                );
                if files.is_empty() {
                    println!("No files found.");
                } else {
                    let total_files = files.len();
                    let total_dirs = files.iter().filter(|f| f.is_directory).count();
                    let total_regular_files = total_files - total_dirs;
                    let _total_size: u64 = files.iter().map(|f| f.size).sum();
                    let dir_size = get_file_size(mount_point);
                    println!("");
                    if color {
                        println!("Directory: {}", mount_point.display());
                        println!(
                            "Total Items: {} ({})",
                            total_files.to_string().cyan(),
                            format!("{} files, {} dirs", total_regular_files, total_dirs).yellow()
                        );
                        println!(
                            "Total Size: {}",
                            SizeUnit::auto_format_size(dir_size).green().bold()
                        );
                    } else {
                        println!("Directory: {}", mount_point.display());
                        println!(
                            "Total Items: {} ({} files, {} dirs)",
                            total_files, total_regular_files, total_dirs
                        );
                        println!("Total Size: {}", SizeUnit::auto_format_size(dir_size));
                    }
                    println!("");
                    show_file_type_stats(&files, color);
                    show_detailed_analysis(&files, color);
                }
            } else if search_pattern.is_some() || excluding_pattern.is_some() || sort_by.is_some() {
                let files = collect_files(mount_point, search_pattern, excluding_pattern, sort_by);
                if files.is_empty() {
                    if let Some(pattern) = search_pattern {
                        println!("No files found matching pattern: {}", pattern);
                    } else {
                        println!("No files found.");
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
                    );
                }
                show_file_type_stats(&files, color);
            }
        }
        None => {
            eprintln!("Error: Disk '{}' not found", disk_name);
            eprintln!("Use 'filebyte --disk list' to see available disks");
            std::process::exit(1);
        }
    }
}

fn get_file_size(path: &Path) -> u64 {
    crate::utils::get_file_size(path)
}
