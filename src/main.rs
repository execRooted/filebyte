use chrono::{DateTime, Utc};
use clap::{Arg, Command};
use colored::Colorize;
use infer;
use std::fs;
use std::io::{self, Write};
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
use utils::{can_delete, filter_files, format_unix_permissions, get_file_extension, get_file_size, preview_file};

const VERSION: &str = "2.2.2";

fn clear_screen() {
    #[cfg(unix)]
    {
        print!("\x1B[2J\x1B[H");
        io::stdout().flush().unwrap();
    }
    #[cfg(not(unix))]
    {
        println!("\n\n");
    }
}

fn count_lines(path: &Path) -> u64 {
    match fs::read_to_string(path) {
        Ok(content) => content.lines().count() as u64,
        Err(_) => 0,
    }
}

fn return_to_menu(_color: bool) {
    println!();
    print!("Press Enter to return to menu... ");
    io::stdout().flush().unwrap();
    let mut _input = String::new();
    io::stdin().read_line(&mut _input).unwrap();
    clear_screen();
}

fn main() {
    let matches = Command::new("filebyte")
        .version(VERSION)
        .author("execRooted <rooted@execrooted.com>")
        .about("A CLI tool for file analysis")
        .disable_version_flag(true)
        .disable_help_flag(true)
        .arg(Arg::new("path").help("Path to file or directory").index(1).num_args(1..))
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
                .help("Analyze specific file(s)")
                .value_name("FILE")
                .num_args(1..),
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
        .arg(
            Arg::new("interactive")
                .short('i')
                .long("interactive")
                .help("Enable interactive menu mode")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("lines")
                .short('l')
                .long("lines")
                .help("Count lines in files (uses path argument or current directory)")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("preview")
                .short('P')
                .long("preview")
                .help("Preview file contents (N, f/N, or l/N for first/last N lines)")
                .value_name("MODE")
                .num_args(1),
        )
        .arg(
            Arg::new("exclude_dirs")
                .short('X')
                .long("exclude-dirs")
                .help("Exclude directories from results (files only)")
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
        println!("A CLI tool for file analysis");
        println!();
        println!("USAGE:");
        println!("    filebyte [OPTIONS] [PATH]...");
        println!("    filebyte --disk <DISK> [OPTIONS]");
        println!("    filebyte -f <FILE>... | --file <FILE>...");
        println!("    filebyte -d <DIR> | --directory <DIR>");
        println!();
        println!("ARGS:");
        println!("    <PATH>...    Path to file or directory");
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
        println!("    -f, --file <FILE>...             Analyze specific file(s)");
        println!("    -d, --directory <DIR>            Analyze a directory as a whole");
        println!("    -r, --recursive                  Enable recursive searching and analysis");
        println!("    -w, --whole                      Analyze the path as a whole (auto-detects if file or directory)");
        println!("    -i, --interactive                Enable interactive menu mode");
    println!("    -l, --lines                      Count lines in files");
    println!("    -P, --preview [MODE]             Preview file contents (N, f, l, fN, lN, f:N, l:N)");
    println!("    -X, --exclude-dirs               Exclude directories from results (files only)");
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

    // Interactive menu mode
    if matches.get_flag("interactive") {
        run_interactive_mode(color, &size_unit, auto_size, matches.get_flag("exclude_dirs"));
        return;
    }

    // Warn if no arguments provided
    let no_args = !matches.contains_id("path")
        && !matches.contains_id("file")
        && !matches.contains_id("directory")
        && !matches.contains_id("disk")
        && !matches.get_flag("version")
        && !matches.get_flag("help")
        && !matches.get_flag("tree")
        && !matches.get_flag("properties")
        && !matches.get_flag("duplicates")
        && !matches.get_flag("recursive")
        && !matches.get_flag("whole")
        && !matches.contains_id("search")
        && !matches.contains_id("excluding")
        && !matches.contains_id("sort_by")
        && !matches.contains_id("export")
        && !matches.contains_id("lines")
        && !matches.contains_id("preview")
        && !matches.get_flag("exclude_dirs");

    if no_args {
        if color {
            eprintln!("{}", "Warning: Depending on the directory size it can take quite a bit of time to analyze. Use arguments for a more specific and fast result.".yellow());
        } else {
            eprintln!("Warning: Depending on the directory size it can take quite a bit of time to analyze. Use arguments for a more specific and fast result.");
        }
    }

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
    let (preview_mode, preview_lines) = matches
        .get_one::<String>("preview")
        .map(|s| {
            if let Ok(n) = s.parse::<usize>() {
                ("both", n)
            } else if s.starts_with("f:") || s.starts_with("first:") {
                let n = s[2..].parse::<usize>().unwrap_or(10);
                ("first", n)
            } else if s.starts_with("l:") || s.starts_with("last:") {
                let n = s[2..].parse::<usize>().unwrap_or(10);
                ("last", n)
            } else if s.starts_with("f") && s[1..].parse::<usize>().is_ok() {
                let n = s[1..].parse::<usize>().unwrap();
                ("first", n)
            } else if s.starts_with("l") && s[1..].parse::<usize>().is_ok() {
                let n = s[1..].parse::<usize>().unwrap();
                ("last", n)
            } else if s == "f" || s == "first" {
                ("first", 10)
            } else if s == "l" || s == "last" {
                ("last", 10)
            } else {
                ("both", 10)
            }
        })
        .unwrap_or(("both", 10));

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
                matches.get_flag("exclude_dirs"),
            );
            return;
        }
    }

    let file_paths: Vec<&String> = matches.get_many::<String>("file").unwrap_or_default().collect();
    let dir_path = matches.get_one::<String>("directory");
    let mut paths: Vec<&String> = matches.get_many::<String>("path").unwrap_or_default().collect();

    let mut preview_lines = preview_lines;
    let preview_mode = preview_mode;
    if (preview_mode == "first" || preview_mode == "last") && !paths.is_empty() {
        if let Ok(n) = paths[0].parse::<usize>() {
            preview_lines = n;
            paths.remove(0);
        }
    }

    if matches.get_flag("whole") {
        if paths.is_empty() {
            eprintln!("Error: --whole requires a path argument");
            process::exit(1);
        }
        for path_str in &paths {
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

                let extension = get_file_extension(path);

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
                let canonical_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
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
                        canonical_path.file_name().unwrap_or_default().to_string_lossy().blue().bold()
                    );
                    println!("Path: {}", canonical_path.display());
                    println!("Size: {}", size_str.green().bold());
                    println!("Permissions: {}", permissions.yellow());
                    println!("Created: {}", created_str.yellow());
                    println!("Modified: {}", modified_str.yellow());
                } else {
                    println!("Name: {}", canonical_path.file_name().unwrap_or_default().to_string_lossy());
                    println!("Path: {}", canonical_path.display());
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
        }
        return;
    }

    if matches.get_flag("lines") {
        let recursive = matches.get_flag("recursive");

        for file in &file_paths {
            let path = Path::new(file);
            if path.exists() && path.is_file() {
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

                let extension = get_file_extension(path);

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
            }
        }

        let path_args: Vec<&Path> = if paths.is_empty() {
            vec![Path::new(".")]
        } else {
            paths.iter().map(|p| Path::new(p)).collect()
        };

        for lines_path in &path_args {
            if !lines_path.exists() {
                eprintln!("Error: Path '{}' does not exist", lines_path.display());
                process::exit(1);
            }

            if lines_path.is_file() {
                let line_count = count_lines(lines_path);
                let file_name = lines_path.file_name().unwrap_or_default().to_string_lossy();
                println!("");
                println!("Line Count:");
                println!("{}", "─".repeat(50));
                if color {
                    println!("File: {}", file_name.blue().bold());
                    println!("Lines: {}", line_count.to_string().green().bold());
                } else {
                    println!("File: {}", file_name);
                    println!("Lines: {}", line_count);
                }
                continue;
            }

            let files = if recursive {
                collect_files_recursive(lines_path, None, excluding_pattern, None, matches.get_flag("exclude_dirs"))
            } else {
                collect_files(lines_path, None, excluding_pattern, None, matches.get_flag("exclude_dirs"))
            };
            let files = filter_files(files, matches.get_flag("exclude_dirs"));

            if files.is_empty() {
                println!("No files found.");
                continue;
            }

            let mut total_lines: u64 = 0;
            println!("");
            println!("Line Count:");
            println!("{}", "─".repeat(50));
            for file_info in &files {
                if file_info.is_directory {
                    continue;
                }
                let line_count = count_lines(&Path::new(&file_info.path));
                total_lines += line_count;
                if color {
                    println!("{}: {}", file_info.name.blue().bold(), line_count.to_string().green());
                } else {
                    println!("{}: {}", file_info.name, line_count);
                }
            }
            println!("{}", "─".repeat(50));
            if color {
                println!("Total: {}", total_lines.to_string().green().bold());
            } else {
                println!("Total: {}", total_lines);
            }
        }
        return;
    }

    if matches.contains_id("preview") {
        let mut previewed = false;
        for file in &file_paths {
            let path = Path::new(file);
            if path.exists() && path.is_file() {
                preview_file(path, preview_lines, preview_mode);
                previewed = true;
            } else {
                eprintln!("Error: '{}' is not a valid file", file);
            }
        }
        for path_str in &paths {
            let path = Path::new(path_str);
            if path.exists() && path.is_file() {
                preview_file(path, preview_lines, preview_mode);
                previewed = true;
            } else {
                eprintln!("Error: '{}' is not a valid file", path_str);
            }
        }
        if !previewed {
            eprintln!("Error: --preview requires at least one file path");
            process::exit(1);
        }
        return;
    }

    for file in &file_paths {
        let path = Path::new(file);
        if path.exists() && path.is_file() {
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

            let extension = get_file_extension(path);

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
            continue;
        }

        if matches.get_flag("recursive") {
            let search_path = paths.first().map(|p| Path::new(p.as_str())).unwrap_or_else(|| Path::new("."));
            let files = filter_files(
                collect_files_recursive(search_path, Some(file), excluding_pattern, sort_by.clone(), matches.get_flag("exclude_dirs")),
                matches.get_flag("exclude_dirs"),
            );
            let matching: Vec<_> = files.into_iter().filter(|f| f.name == **file || f.name.contains(*file)).collect();
            if matching.is_empty() {
                eprintln!("Error: File '{}' not found", file);
                process::exit(1);
            }
            display_files(
                &matching,
                &size_unit,
                color,
                matches.get_flag("properties"),
                auto_size,
                show_size,
                matches.get_one::<String>("export"),
                show_detailed_permissions,
                false,
            );
            continue;
        }

        eprintln!("Error: File '{}' not found", file);
        process::exit(1);
    }
    if !file_paths.is_empty() {
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

        let canonical_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
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
                canonical_path.file_name().unwrap_or_default().to_string_lossy().blue().bold()
            );
            println!("Path: {}", canonical_path.display());
            println!("Size: {}", size_str.green().bold());
            println!("Permissions: {}", permissions.yellow());
            println!("Created: {}", created_str.yellow());
            println!("Modified: {}", modified_str.yellow());
        } else {
            println!("Name: {}", canonical_path.file_name().unwrap_or_default().to_string_lossy());
            println!("Path: {}", canonical_path.display());
            println!("Size: {}", size_str);
            println!("Permissions: {}", permissions);
            println!("Created: {}", created_str);
            println!("Modified: {}", modified_str);
        }
        return;
    }

    let paths: Vec<&Path> = if paths.is_empty() {
        vec![Path::new(".")]
    } else {
        paths.iter().map(|p| Path::new(p)).collect()
    };

    for path in &paths {
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

            let extension = get_file_extension(path);

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
            continue;
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

                let extension = get_file_extension(path);

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
                    filter_files(collect_files_recursive(path, search_pattern, excluding_pattern, sort_by.clone(), matches.get_flag("exclude_dirs")), matches.get_flag("exclude_dirs"));
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
                    collect_files_recursive(path, search_pattern, excluding_pattern, sort_by.clone(), matches.get_flag("exclude_dirs"))
                } else {
                    collect_files(path, search_pattern, excluding_pattern, sort_by.clone(), matches.get_flag("exclude_dirs"))
                };
                let files = filter_files(files, matches.get_flag("exclude_dirs"));
                if files.is_empty() {
                    if let Some(pattern) = search_pattern {
                        println!("No files found matching pattern: {}", pattern);
                    } else {
                        println!("No files found.");
                    }
                } else {
                    if search_pattern.is_some() {
                        display_files(
                            &files,
                            &size_unit,
                            color,
                            matches.get_flag("properties"),
                            auto_size,
                            show_size,
                            matches.get_one::<String>("export"),
                            show_detailed_permissions,
                            true,
                        );
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
                            false,
                        );
                        if !matches.get_flag("properties") && matches.get_flag("recursive") {
                            show_file_type_stats(&files, color);
                        }
                    }
                }
            }
        }
    }
}

fn run_interactive_mode(color: bool, size_unit: &SizeUnit, auto_size: bool, exclude_dirs: bool) {
    loop {
        clear_screen();
        println!();
        if color {
            println!("{}", "╔══════════════════════════════════════════════════════════╗".cyan());
            println!("{}", "║           FileByte Interactive Menu                      ║".cyan());
            println!("{}", "╚══════════════════════════════════════════════════════════╝".cyan());
        } else {
            println!("╔══════════════════════════════════════════════════════════╗");
            println!("║           FileByte Interactive Menu                      ║");
            println!("╚══════════════════════════════════════════════════════════╝");
        }
        println!();
        println!("1. List files in current directory");
        println!("2. Analyze a specific file");
        println!("3. Analyze a directory");
        println!("4. Find duplicate files");
        println!("5. Show directory tree");
        println!("6. List all disks");
        println!("7. Search for files (regex)");
        println!("8. Show file type statistics");
        println!("9. Bit converter (bits, kbits, mbits, gbits, tbits)");
        println!("0. Exit");
        println!();
        print!("Select an option: ");
        io::stdout().flush().unwrap();

        let mut choice = String::new();
        io::stdin().read_line(&mut choice).unwrap();
        let choice = choice.trim();

        match choice {
            "1" => {
                // List files in current directory
                let current_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")).display().to_string();
                print!("Enter directory path (or press Enter for {}): ", current_dir);
                io::stdout().flush().unwrap();
                let mut path_input = String::new();
                io::stdin().read_line(&mut path_input).unwrap();
                let path_str = path_input.trim();
                let target_path = if path_str.is_empty() {
                    "."
                } else {
                    path_str
                };
                let path = Path::new(target_path);
                if path.is_dir() {
                    let files = filter_files(collect_files(path, None, None, None, exclude_dirs), exclude_dirs);
                    if files.is_empty() {
                        println!("No files found.");
                    } else {
                        display_files(&files, size_unit, color, false, auto_size, false, None, true, false);
                    }
                    println!();
                    print!("Press Enter to return to menu... ");
                    io::stdout().flush().unwrap();
                    let mut _input = String::new();
                    io::stdin().read_line(&mut _input).unwrap();
                    clear_screen();
                } else {
                    eprintln!("Error: '{}' is not a valid directory", target_path);
                }
            }
            "2" => {
                // Analyze a specific file
                print!("Enter file path: ");
                io::stdout().flush().unwrap();
                let mut path_input = String::new();
                io::stdin().read_line(&mut path_input).unwrap();
                let path_str = path_input.trim();
                let path = Path::new(path_str);
                if path.is_file() {
                    let size = get_file_size(path);
                    let size_str = if auto_size {
                        SizeUnit::auto_format_size(size)
                    } else {
                        size_unit.format_size(size)
                    };
                    let file_name = path.file_name().unwrap_or_default().to_string_lossy();
                    
                    let metadata = fs::metadata(path).ok();
                    let permissions = metadata
                        .as_ref()
                        .map(|m| {
                            if m.permissions().readonly() {
                                if can_delete(path) { "r-x" } else { "r--" }
                            } else {
                                if can_delete(path) { "rwx" } else { "rw-" }
                            }
                        })
                        .unwrap_or("unknown");
                    
                    let file_type = infer::get_from_path(path)
                        .ok()
                        .flatten()
                        .map(|kind| kind.mime_type().to_string())
                        .unwrap_or_else(|| "unknown".to_string());

                    if color {
                        println!();
                        println!("{}", "─".repeat(50));
                        println!("Name: {}", file_name.blue().bold());
                        println!("Path: {}", path.display());
                        println!("Size: {}", size_str.green().bold());
                        println!("Type: {}", file_type.magenta());
                        println!("Permissions: {}", permissions.yellow());
                    } else {
                        println!();
                        println!("{}", "─".repeat(50));
                        println!("Name: {}", file_name);
                        println!("Path: {}", path.display());
                        println!("Size: {}", size_str);
                        println!("Type: {}", file_type);
                        println!("Permissions: {}", permissions);
                    }
                    println!();
                    print!("Press Enter to return to menu... ");
                    io::stdout().flush().unwrap();
                    let mut _input = String::new();
                    io::stdin().read_line(&mut _input).unwrap();
                    clear_screen();
                } else {
                    eprintln!("Error: '{}' is not a valid file", path_str);
                }
            }
            "3" => {
                // Analyze a directory
                let current_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")).display().to_string();
                print!("Enter directory path (or press Enter for {}): ", current_dir);
                io::stdout().flush().unwrap();
                let mut path_input = String::new();
                io::stdin().read_line(&mut path_input).unwrap();
                let path_str = path_input.trim();
                let path = if path_str.is_empty() {
                    std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
                } else {
                    std::path::PathBuf::from(path_str)
                };
                let path = path.as_path();
                if path.is_dir() {
                    let dir_size = get_file_size(path);
                    let size_str = if auto_size {
                        SizeUnit::auto_format_size(dir_size)
                    } else {
                        size_unit.format_size(dir_size)
                    };
                    let dir_name = path.file_name().unwrap_or_default().to_string_lossy();
                    
                    let metadata = fs::metadata(path).ok();
                    let permissions = metadata
                        .as_ref()
                        .map(|m| {
                            if m.permissions().readonly() {
                                if can_delete(path) { "r-x" } else { "r--" }
                            } else {
                                if can_delete(path) { "rwx" } else { "rw-" }
                            }
                        })
                        .unwrap_or("unknown");

                    if color {
                        println!();
                        println!("{}", "─".repeat(50));
                        println!("Name: {}", dir_name.blue().bold());
                        println!("Path: {}", path.display());
                        println!("Size: {}", size_str.green().bold());
                        println!("Permissions: {}", permissions.yellow());
                    } else {
                        println!();
                        println!("{}", "─".repeat(50));
                        println!("Name: {}", dir_name);
                        println!("Path: {}", path.display());
                        println!("Size: {}", size_str);
                        println!("Permissions: {}", permissions);
                    }
                    println!();
                    print!("Press Enter to return to menu... ");
                    io::stdout().flush().unwrap();
                    let mut _input = String::new();
                    io::stdin().read_line(&mut _input).unwrap();
                } else {
                    eprintln!("Error: '{}' is not a valid directory", path_str);
                }
            }
            "4" => {
                // Find duplicate files
                let current_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")).display().to_string();
                print!("Enter directory path to search (or press Enter for {}): ", current_dir);
                io::stdout().flush().unwrap();
                let mut path_input = String::new();
                io::stdin().read_line(&mut path_input).unwrap();
                let path_str = path_input.trim();
                let target_path = if path_str.is_empty() {
                    "."
                } else {
                    path_str
                };
                let path = Path::new(target_path);
                if path.is_dir() {
                    find_duplicates(path, color);
                    println!();
                    print!("Press Enter to return to menu... ");
                    io::stdout().flush().unwrap();
                    let mut _input = String::new();
                    io::stdin().read_line(&mut _input).unwrap();
                    clear_screen();
                } else {
                    eprintln!("Error: '{}' is not a valid directory", target_path);
                }
            }
            "5" => {
                // Show directory tree
                let current_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")).display().to_string();
                print!("Enter directory path (or press Enter for {}): ", current_dir);
                io::stdout().flush().unwrap();
                let mut path_input = String::new();
                io::stdin().read_line(&mut path_input).unwrap();
                let path_str = path_input.trim();
                let target_path = if path_str.is_empty() {
                    "."
                } else {
                    path_str
                };
                let path = Path::new(target_path);
                if path.is_dir() {
                    print_tree(path, "", color);
                    println!();
                    print!("Press Enter to return to menu... ");
                    io::stdout().flush().unwrap();
                    let mut _input = String::new();
                    io::stdin().read_line(&mut _input).unwrap();
                    clear_screen();
                } else {
                    eprintln!("Error: '{}' is not a valid directory", target_path);
                }
            }
            "6" => {
                // List all disks
                list_disks(color, size_unit, auto_size);
                println!();
                print!("Press Enter to return to menu... ");
                io::stdout().flush().unwrap();
                let mut _input = String::new();
                io::stdin().read_line(&mut _input).unwrap();
                clear_screen();
            }
            "7" => {
                // Search for files
                let current_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")).display().to_string();
                print!("Enter regex pattern: ");
                io::stdout().flush().unwrap();
                let mut pattern_input = String::new();
                io::stdin().read_line(&mut pattern_input).unwrap();
                let pattern = pattern_input.trim();
                
                print!("Enter directory to search (or press Enter for {}): ", current_dir);
                io::stdout().flush().unwrap();
                let mut path_input = String::new();
                io::stdin().read_line(&mut path_input).unwrap();
                let path_str = path_input.trim();
                let target_path = if path_str.is_empty() {
                    "."
                } else {
                    path_str
                };
                let path = Path::new(target_path);
                
                if path.is_dir() {
                    let files = filter_files(collect_files(path, Some(&pattern.to_string()), None, None, exclude_dirs), exclude_dirs);
                    if files.is_empty() {
                        println!("No files found matching pattern: {}", pattern);
                    } else {
                        show_file_type_stats(&files, color);
                    }
                    println!();
                    print!("Press Enter to return to menu... ");
                    io::stdout().flush().unwrap();
                    let mut _input = String::new();
                    io::stdin().read_line(&mut _input).unwrap();
                    clear_screen();
                } else {
                    eprintln!("Error: '{}' is not a valid directory", target_path);
                }
            }
            "8" => {
                // Show file type statistics
                let current_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")).display().to_string();
                print!("Enter directory path (or press Enter for {}): ", current_dir);
                io::stdout().flush().unwrap();
                let mut path_input = String::new();
                io::stdin().read_line(&mut path_input).unwrap();
                let path_str = path_input.trim();
                let target_path = if path_str.is_empty() {
                    "."
                } else {
                    path_str
                };
                let path = Path::new(target_path);
                if path.is_dir() {
                    let files = filter_files(collect_files_recursive(path, None, None, None, exclude_dirs), exclude_dirs);
                    show_file_type_stats(&files, color);
                    println!();
                    print!("Press Enter to return to menu... ");
                    io::stdout().flush().unwrap();
                    let mut _input = String::new();
                    io::stdin().read_line(&mut _input).unwrap();
                    clear_screen();
                } else {
                    eprintln!("Error: '{}' is not a valid directory", target_path);
                }
            }
            "9" => {
                // Bit converter
                println!("Bit Converter");
                println!("{}", "─".repeat(40));
                println!("Enter a value in bits, kilobits, megabits, gigabits, or terabits");
                println!("Examples: 1000 bits, 500 kbits, 1.5 mbits, 2 gbits");
                println!();
                print!("Enter value and unit: ");
                io::stdout().flush().unwrap();
                let mut input = String::new();
                io::stdin().read_line(&mut input).unwrap();
                let input = input.trim();
                
                // Parse input like "1000 bits" or "500 kbits"
                let parts: Vec<&str> = input.split_whitespace().collect();
                if parts.len() >= 2 {
                    let value: f64 = match parts[0].parse() {
                        Ok(v) => v,
                        Err(_) => {
                            eprintln!("Error: Invalid number '{}'", parts[0]);
                            return_to_menu(color);
                            continue;
                        }
                    };
                    let unit = parts[1].to_lowercase();
                    
                    // Convert to bits first
                    let bits: f64 = match unit.as_str() {
                        "bits" => value,
                        "kbits" | "kilobits" => value * 1000.0,
                        "mbits" | "megabits" => value * 1_000_000.0,
                        "gbits" | "gigabits" => value * 1_000_000_000.0,
                        "tbits" | "terabits" => value * 1_000_000_000_000.0,
                        "bytes" => value * 8.0,
                        "kb" | "kilobytes" => value * 8.0 * 1000.0,
                        "mb" | "megabytes" => value * 8.0 * 1_000_000.0,
                        "gb" | "gigabytes" => value * 8.0 * 1_000_000_000.0,
                        "tb" | "terabytes" => value * 8.0 * 1_000_000_000_000.0,
                        _ => {
                            eprintln!("Error: Unknown unit '{}'. Use bits, kbits, mbits, gbits, tbits", unit);
                            return_to_menu(color);
                            continue;
                        }
                    };
                    
                    println!();
                    println!("Conversion Results:");
                    println!("{}", "─".repeat(40));
                    println!("Bits (b):     {:.0}", bits);
                    println!("Kilobits:     {:.2} Kb", bits / 1000.0);
                    println!("Megabits:     {:.2} Mb", bits / 1_000_000.0);
                    println!("Gigabits:     {:.2} Gb", bits / 1_000_000_000.0);
                    println!("Terabits:     {:.2} Tb", bits / 1_000_000_000_000.0);
                    println!();
                    println!("Bytes (B):    {:.0}", bits / 8.0);
                    println!("Kilobytes:    {:.2} KB", bits / 8.0 / 1000.0);
                    println!("Megabytes:    {:.2} MB", bits / 8.0 / 1_000_000.0);
                    println!("Gigabytes:    {:.2} GB", bits / 8.0 / 1_000_000_000.0);
                    println!("Terabytes:    {:.2} TB", bits / 8.0 / 1_000_000_000_000.0);
                } else {
                    eprintln!("Error: Please enter a value and unit (e.g., '1000 bits' or '500 kbits')");
                }
                return_to_menu(color);
            }
            "0" => {
                println!("Goodbye!");
                break;
            }
            _ => {
                eprintln!("Invalid option. Please try again.");
            }
        }
    }
}
