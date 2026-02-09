use chrono::{DateTime, Utc};
use clap::{Arg, Command};
use colored::Colorize;
use infer;
use std::fs;
use std::path::Path;
use std::process;

mod analysis;
mod collect;
mod display;
mod disk;
mod tree;
mod types;
mod utils;

use analysis::{find_duplicates, show_detailed_analysis};
use collect::{collect_files, collect_files_recursive};
use display::{display_files, show_file_type_stats};
use disk::{list_disks, show_disk_info};
use tree::print_tree;
use types::{SizeUnit, SortBy};
use utils::{can_delete, format_unix_permissions, get_file_size};

const VERSION: &str = "1.3.2";

fn main() {
    let matches = Command::new("filebyte")
        .version(VERSION)
        .author("execRooted <rooted@execrooted.com>")
        .about("List files and directories with sizes")
        .disable_version_flag(true)
        .disable_help_flag(true)
        .arg(Arg::new("path").help("Path to file or directory").index(1))
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
        println!("filebyte {}", VERSION);
        return;
    }

    if matches.get_flag("help") {
        println!();
        println!("filebyte {}", VERSION);
        println!("execRooted <rooted@execrooted.com>");
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
        return;
    }

    let show_size = matches.contains_id("size");
    let size_unit_str = matches
        .get_one::<String>("size")
        .unwrap_or(&"auto".to_string())
        .clone();
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
    let sort_by = matches
        .get_one::<String>("sort_by")
        .map(|s| match s.to_lowercase().as_str() {
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
            show_disk_info(
                disk_arg,
                &size_unit,
                color,
                auto_size,
                matches.get_flag("tree"),
                matches.get_flag("properties"),
                search_pattern,
                excluding_pattern,
                sort_by,
                matches.get_flag("duplicates"),
                show_size,
                show_detailed_permissions,
            );
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
                let modified_str = DateTime::<Utc>::from(modified)
                    .format("%Y-%m-%d %H:%M:%S UTC")
                    .to_string();
                let created_str = DateTime::<Utc>::from(created)
                    .format("%Y-%m-%d %H:%M:%S UTC")
                    .to_string();

                let file_type = infer::get_from_path(path)
                    .ok()
                    .flatten()
                    .map(|kind| kind.mime_type().to_string())
                    .unwrap_or_else(|| "unknown".to_string());

                let extension = if let Some(ext) = path.extension() {
                    ext.to_string_lossy().to_string()
                } else if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    if file_name.starts_with('.') {
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
                    println!(
                        "Path: {}",
                        path.canonicalize().unwrap_or(path.to_path_buf()).display()
                    );
                    println!("Size: {}", size_str.green().bold());
                    println!("Type: {}", file_type.magenta());
                    println!("Extension: {}", extension.cyan());
                    println!("Permissions: {}", permissions.yellow());
                    println!("Created: {}", created_str.yellow());
                    println!("Modified: {}", modified_str.yellow());
                } else {
                    println!("Name: {}", file_name);
                    println!(
                        "Path: {}",
                        path.canonicalize().unwrap_or(path.to_path_buf()).display()
                    );
                    println!("Size: {}", size_str);
                    println!("Type: {}", file_type);
                    println!("Extension: {}", extension);
                    println!("Permissions: {}", permissions);
                    println!("Created: {}", created_str);
                    println!("Modified: {}", modified_str);
                }
            } else if path.is_dir() {
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
                let modified_str = DateTime::<Utc>::from(modified)
                    .format("%Y-%m-%d %H:%M:%S UTC")
                    .to_string();
                let created_str = DateTime::<Utc>::from(created)
                    .format("%Y-%m-%d %H:%M:%S UTC")
                    .to_string();

                println!("");
                println!("Directory Analysis:");
                println!("{}", "─".repeat(50));
                if color {
                    println!(
                        "Name: {}",
                        path.file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .blue()
                            .bold()
                    );
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
                eprintln!(
                    "Error: Path '{}' is neither a file nor a directory",
                    path_str
                );
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
        let modified_str = DateTime::<Utc>::from(modified)
            .format("%Y-%m-%d %H:%M:%S UTC")
            .to_string();
        let created_str = DateTime::<Utc>::from(created)
            .format("%Y-%m-%d %H:%M:%S UTC")
            .to_string();

        let file_type = infer::get_from_path(path)
            .ok()
            .flatten()
            .map(|kind| kind.mime_type().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let extension = if let Some(ext) = path.extension() {
            ext.to_string_lossy().to_string()
        } else if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
            if file_name.starts_with('.') {
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
            println!(
                "Path: {}",
                path.canonicalize().unwrap_or(path.to_path_buf()).display()
            );
            println!("Size: {}", size_str.green().bold());
            println!("Type: {}", file_type.magenta());
            println!("Extension: {}", extension.cyan());
            println!("Permissions: {}", permissions.yellow());
            println!("Created: {}", created_str.yellow());
            println!("Modified: {}", modified_str.yellow());
        } else {
            println!("Name: {}", file_name);
            println!(
                "Path: {}",
                path.canonicalize().unwrap_or(path.to_path_buf()).display()
            );
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
        let modified_str = DateTime::<Utc>::from(modified)
            .format("%Y-%m-%d %H:%M:%S UTC")
            .to_string();
        let created_str = DateTime::<Utc>::from(created)
            .format("%Y-%m-%d %H:%M:%S UTC")
            .to_string();

        println!("");
        println!("Directory Analysis:");
        println!("{}", "─".repeat(50));
        if color {
            println!(
                "Name: {}",
                path.file_name().unwrap_or_default().to_string_lossy().blue().bold()
            );
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

    if path.is_file()
        && !matches.get_flag("tree")
        && !matches.get_flag("properties")
        && !matches.get_flag("duplicates")
        && !matches.get_flag("recursive")
        && search_pattern.is_none()
        && excluding_pattern.is_none()
        && sort_by.is_none()
        && matches.get_one::<String>("export").is_none()
    {
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
        let modified_str = DateTime::<Utc>::from(modified)
            .format("%Y-%m-%d %H:%M:%S UTC")
            .to_string();
        let created_str = DateTime::<Utc>::from(created)
            .format("%Y-%m-%d %H:%M:%S UTC")
            .to_string();

        let file_type = infer::get_from_path(path)
            .ok()
            .flatten()
            .map(|kind| kind.mime_type().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let extension = if let Some(ext) = path.extension() {
            ext.to_string_lossy().to_string()
        } else if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
            if file_name.starts_with('.') {
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
            println!(
                "Path: {}",
                path.canonicalize().unwrap_or(path.to_path_buf()).display()
            );
            println!("Size: {}", size_str.green().bold());
            println!("Type: {}", file_type.magenta());
            println!("Extension: {}", extension.cyan());
            println!("Permissions: {}", permissions.yellow());
            println!("Created: {}", created_str.yellow());
            println!("Modified: {}", modified_str.yellow());
        } else {
            println!("Name: {}", file_name);
            println!(
                "Path: {}",
                path.canonicalize().unwrap_or(path.to_path_buf()).display()
            );
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
            let modified_str = DateTime::<Utc>::from(modified)
                .format("%Y-%m-%d %H:%M:%S UTC")
                .to_string();
            let created_str = DateTime::<Utc>::from(created)
                .format("%Y-%m-%d %H:%M:%S UTC")
                .to_string();

            let file_type = infer::get_from_path(path)
                .ok()
                .flatten()
                .map(|kind| kind.mime_type().to_string())
                .unwrap_or_else(|| "unknown".to_string());

            let extension = path
                .extension()
                .map(|ext| ext.to_string_lossy().to_string())
                .unwrap_or_else(|| "none".to_string());

            println!("");
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
            let files =
                collect_files_recursive(path, search_pattern, excluding_pattern, sort_by);
            if files.is_empty() {
                println!("No files found in directory.");
            } else {
                let total_files = files.len();
                let total_dirs = files.iter().filter(|f| f.is_directory).count();
                let total_regular_files = total_files - total_dirs;
                let _total_size: u64 = files.iter().map(|f| f.size).sum();
                let dir_size = get_file_size(path);
                println!("");
                if color {
                    println!("Directory: {}", path.display());
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
                    println!("Directory: {}", path.display());
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
                    show_file_type_stats(&files, color);
                } else {
                    display_files(
                        &files,
                        &size_unit,
                        color,
                        matches.get_flag("properties"),
                        auto_size,
                        show_size,
                        matches.get_one::<String>("export"),
                        show_detailed_permissions,
                    );
                    if !matches.get_flag("properties") && matches.get_flag("recursive") {
                        show_file_type_stats(&files, color);
                    }
                }
            }
        }
    }
}
