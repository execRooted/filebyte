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
mod i18n;
mod tree;
mod types;
mod utils;

use analysis::{apply_duplicate_action, find_duplicates, show_detailed_analysis};
use collect::{collect_files_extended, collect_files_recursive_extended};
use display::{display_files, show_file_type_stats};
use disk::{list_disks, show_disk_info};
use tree::print_tree;
use types::{DuplicateAction, HashAlgorithm, SizeUnit, SortBy};
use utils::{can_delete, filter_files, format_unix_permissions, get_file_extension, get_file_size, parse_age_threshold, parse_size_threshold, preview_file};

const VERSION: &str = env!("CARGO_PKG_VERSION");

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
    print!("{} ", i18n::tr("menu_return_prompt"));
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
            Arg::new("extension")
                .short('E')
                .long("extension")
                .help("Only list files with the specified extension (e.g. rs, txt, pdf)")
                .value_name("EXT"),
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
            Arg::new("content_dups")
                .long("content-dups")
                .help("Verify duplicates by content hash (slower, true duplicates only)")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("hash")
                .long("hash")
                .help("Hash algorithm for content-based duplicate detection (sha256 or md5)")
                .value_name("ALGORITHM")
                .default_value("sha256"),
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
            Arg::new("max_depth")
                .long("max-depth")
                .help("Limit recursive search depth (requires --recursive)")
                .value_name("N"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .help("Output results as JSON to stdout")
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
            Arg::new("logo")
                .long("logo")
                .help("Show the filebyte logo animation")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("exclude_dirs")
                .short('X')
                .long("exclude-dirs")
                .help("Exclude directories from results (files only)")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("larger_than")
                .long("larger-than")
                .help("Filter files larger than threshold (e.g. 10MB, 1GB, 8 GB, 2MiB, 1GiB, or path to file)")
                .value_name("SIZE")
                .num_args(1..=2)
                .action(clap::ArgAction::Set),
        )
        .arg(
            Arg::new("smaller_than")
                .long("smaller-than")
                .help("Filter files smaller than threshold (e.g. 1KB, 500MB, 500 MB, 2MiB, 1GiB, or path to file)")
                .value_name("SIZE")
                .num_args(1..=2)
                .action(clap::ArgAction::Set),
        )
        .arg(
            Arg::new("equal_to")
                .long("equal-to")
                .help("Filter files equal to threshold (e.g. 10MB, 1GB, 8 GB, 2MiB, 1GiB, or path to file)")
                .value_name("SIZE")
                .num_args(1..=2)
                .action(clap::ArgAction::Set),
        )
        .arg(
            Arg::new("older_than")
                .long("older-than")
                .help("Filter files older than duration (e.g. 30d, 2w, 1y, yyyy-mm-dd, 30 d)")
                .value_name("DURATION")
                .num_args(1..=2)
                .action(clap::ArgAction::Set),
        )
        .arg(
            Arg::new("newer_than")
                .long("newer-than")
                .help("Filter files newer than duration (e.g. 7d, 1w, 7 d)")
                .value_name("DURATION")
                .num_args(1..=2)
                .action(clap::ArgAction::Set),
        )
        .arg(
            Arg::new("empty")
                .long("empty")
                .help("Show only empty files and directories")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("delete_duplicates")
                .long("delete-duplicates")
                .help("Delete duplicate files, keeping the first occurrence")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("merge_duplicates")
                .long("merge-duplicates")
                .help("Merge duplicate files by hard linking (keeps first, links rest)")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("content")
                .long("content")
                .help("Search for pattern inside file contents")
                .value_name("PATTERN")
                .num_args(1),
        )
        .arg(
            Arg::new("top")
                .long("top")
                .help("Show the N largest files in a directory")
                .value_name("N")
                .num_args(1),
        )
        .arg(
            Arg::new("ignore_hidden")
                .long("ignore-hidden")
                .help("Skip hidden files and directories (dotfiles)")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("stat")
                .long("stat")
                .help("Show a summary of directory statistics (file count, total size, etc.)")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("force")
                .long("force")
                .help("Skip confirmation prompts for destructive actions")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("language")
                .long("language")
                .visible_alias("lang")
                .help("Launch interactive language selection menu")
                .value_name("LANGUAGE")
                .num_args(0..=1),
        )
        .get_matches();

    let color = !matches.get_flag("no-color");

    if let Some(lang_val) = matches.get_one::<String>("language") {
        match i18n::get_language_code(lang_val) {
            Some(code) => {
                i18n::set_language(code);
                i18n::save_language(code);
                if color {
                    println!(
                        "{}",
                        i18n::tr_format("language_saved", &[code]).green().bold()
                    );
                } else {
                    println!("{}", i18n::tr_format("language_saved", &[code]));
                }
                return;
            }
            None => {
                eprintln!(
                    "{}",
                    i18n::tr_format("error_language_not_available", &[lang_val])
                );
                eprintln!("{}", i18n::tr("error_language_no_arg"));
                process::exit(1);
            }
        }
    }

    let lang_code = i18n::init(color);

    if matches.contains_id("language") {
        i18n::handle_language_flag(color);
        return;
    }

    if matches.get_flag("logo") {
        let script_path = std::env::current_dir()
            .unwrap_or_default()
            .join("assets/show_logo/logo.sh");
        let script_dir = script_path.parent().unwrap_or_else(|| std::path::Path::new("."));
        let status = std::process::Command::new("bash")
            .arg(script_path.file_name().unwrap_or_default())
            .current_dir(script_dir)
            .status();
        match status {
            Ok(s) => {
                if !s.success() {
                    eprintln!("Error: logo script exited with error");
                    process::exit(1);
                }
            }
            Err(e) => {
                eprintln!("Error: failed to run logo script: {}", e);
                process::exit(1);
            }
        }
        return;
    }

    if matches.get_flag("version") {
        println!("filebyte {}", VERSION);
        println!("Language: {}", lang_code);
        return;
    }

    if matches.get_flag("help") {
        let app_name = i18n::tr("app_name");
        let author = i18n::tr("app_author");
        let desc = i18n::tr("app_description");
        let also = i18n::tr("also_available_as");
        println!();
        println!("{} {}", app_name, VERSION);
        println!("{}", author);
        println!("{}", desc);
        println!("{}", also);
        println!();
        println!("{}", i18n::tr("help_usage"));
        println!("    filebyte [OPTIONS] [PATH]...");
        println!("    filebyte --disk <DISK> [OPTIONS]");
        println!("    filebyte -f <FILE>... | --file <FILE>...");
        println!("    filebyte -d <DIR> | --directory <DIR>");
        println!();
        println!("{}", i18n::tr("help_args"));
        println!("    <PATH>...    {}", i18n::tr("help_path"));
        println!();
        println!("{}", i18n::tr("help_options"));
        println!("    -v, --version                    {}", i18n::tr("opt_version"));
        println!("    -h, --help                       {}", i18n::tr("opt_help"));
        println!("    -s, --size <UNIT>                {}", i18n::tr("opt_size"));
        println!("    -t, --tree                       {}", i18n::tr("opt_tree"));
        println!("    -p, --properties                 {}", i18n::tr("opt_properties"));
        println!("        --no-color                   {}", i18n::tr("opt_no_color"));
        println!("    -m, --disk <DISK>                {}", i18n::tr("opt_disk"));
        println!("    -e, --search <PATTERN>           {}", i18n::tr("opt_search"));
        println!("    -x, --excluding <PATTERN>        {}", i18n::tr("opt_excluding"));
        println!("    -E, --extension <EXT>            {}", i18n::tr("opt_extension"));
        println!("        --sort-by <CRITERIA>         {}", i18n::tr("opt_sort_by"));
        println!("        --duplicates                 {}", i18n::tr("opt_duplicates"));
        println!("        --export <FILE>              {}", i18n::tr("opt_export"));
        println!("    -f, --file <FILE>...             {}", i18n::tr("opt_file"));
        println!("    -d, --directory <DIR>            {}", i18n::tr("opt_directory"));
        println!("    -r, --recursive                  {}", i18n::tr("opt_recursive"));
        println!("    -w, --whole                      {}", i18n::tr("opt_whole"));
        println!("    -i, --interactive                {}", i18n::tr("opt_interactive"));
        println!("    -l, --lines                      {}", i18n::tr("opt_lines"));
        println!("    -P, --preview [MODE]             {}", i18n::tr("opt_preview"));
        println!("    -X, --exclude-dirs               {}", i18n::tr("opt_exclude_dirs"));
        println!("        --logo                       {}", i18n::tr("opt_logo"));
        println!("        --top <N>                    {}", i18n::tr("opt_top"));
        println!("        --ignore-hidden              {}", i18n::tr("opt_ignore_hidden"));
        println!("        --stat                       {}", i18n::tr("opt_stat"));
        println!("        --content-dups               {}", i18n::tr("opt_content_dups"));
        println!("        --hash <ALGORITHM>           {}", i18n::tr("opt_hash"));
        println!("        --larger-than <SIZE>         {}", i18n::tr("opt_larger_than"));
        println!("        --smaller-than <SIZE>        {}", i18n::tr("opt_smaller_than"));
        println!("        --equal-to <SIZE>            {}", i18n::tr("opt_equal_to"));
        println!("        --older-than <DURATION>      {}", i18n::tr("opt_older_than"));
        println!("        --newer-than <DURATION>      {}", i18n::tr("opt_newer_than"));
        println!("        --empty                      {}", i18n::tr("opt_empty"));
        println!("        --delete-duplicates          {}", i18n::tr("opt_delete_duplicates"));
        println!("        --merge-duplicates           {}", i18n::tr("opt_merge_duplicates"));
        println!("        --content <PATTERN>          {}", i18n::tr("opt_content"));
        println!("        --force                      {}", i18n::tr("opt_force"));
        println!("        --language                   {}", i18n::tr("opt_language"));
        println!("        --language <LANG>            {}", i18n::tr("opt_language_value"));
        println!();
        return;
    }

    let show_size = matches.contains_id("size")
        || matches.contains_id("larger_than")
        || matches.contains_id("smaller_than")
        || matches.contains_id("equal_to");
    let size_unit_str = matches
        .get_one::<String>("size")
        .unwrap_or(&"auto".to_string())
        .clone();
    let auto_size = size_unit_str == "auto";
    let size_unit = match SizeUnit::from_str(&size_unit_str) {
        Ok(unit) => unit,
        Err(e) => {
            eprintln!("Error: {}", e);
            eprintln!("{}", i18n::tr("error_size_options"));
            process::exit(1);
        }
    };

    let color = !matches.get_flag("no-color");
    let show_detailed_permissions = true;

    let content_dups = matches.get_flag("content_dups");
    let hash_algorithm = match HashAlgorithm::from_str(
        matches
            .get_one::<String>("hash")
            .unwrap_or(&"sha256".to_string()),
    ) {
        Ok(algo) => algo,
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    };

    let min_size = if let Some(vals) = matches.get_many::<String>("larger_than") {
        let s = vals.map(String::as_str).collect::<Vec<_>>().join(" ");
        match parse_size_threshold(&s) {
            Ok(v) => Some(v),
            Err(e) => {
                eprintln!("Error: {}", e);
                process::exit(1);
            }
        }
    } else {
        None
    };
    let max_size = if let Some(vals) = matches.get_many::<String>("smaller_than") {
        let s = vals.map(String::as_str).collect::<Vec<_>>().join(" ");
        match parse_size_threshold(&s) {
            Ok(v) => Some(v),
            Err(e) => {
                eprintln!("Error: {}", e);
                process::exit(1);
            }
        }
    } else {
        None
    };
    let equal_size = if let Some(vals) = matches.get_many::<String>("equal_to") {
        let s = vals.map(String::as_str).collect::<Vec<_>>().join(" ");
        match parse_size_threshold(&s) {
            Ok(v) => Some(v),
            Err(e) => {
                eprintln!("Error: {}", e);
                process::exit(1);
            }
        }
    } else {
        None
    };

    let min_age_seconds = if let Some(vals) = matches.get_many::<String>("older_than") {
        let s = vals.map(String::as_str).collect::<Vec<_>>().join(" ");
        match parse_age_threshold(&s) {
            Ok(v) => Some(v),
            Err(e) => {
                eprintln!("Error: {}", e);
                process::exit(1);
            }
        }
    } else {
        None
    };
    let max_age_seconds = if let Some(vals) = matches.get_many::<String>("newer_than") {
        let s = vals.map(String::as_str).collect::<Vec<_>>().join(" ");
        match parse_age_threshold(&s) {
            Ok(v) => Some(v),
            Err(e) => {
                eprintln!("Error: {}", e);
                process::exit(1);
            }
        }
    } else {
        None
    };

    let empty_only = matches.get_flag("empty");
    let content_pattern = matches.get_one::<String>("content");
    let json = matches.get_flag("json");

    let delete_duplicates = matches.get_flag("delete_duplicates");
    let merge_duplicates = matches.get_flag("merge_duplicates");
    let force = matches.get_flag("force");
    let duplicate_action = if delete_duplicates {
        DuplicateAction::Delete
    } else if merge_duplicates {
        DuplicateAction::Merge
    } else {
        DuplicateAction::None
    };

    // Interactive menu mode

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
        && !matches.contains_id("extension")
        && !matches.contains_id("sort_by")
        && !matches.contains_id("export")
        && !matches.contains_id("lines")
        && !matches.contains_id("preview")
        && !matches.get_flag("exclude_dirs")
        && !matches.get_flag("content_dups")
        && !matches.get_flag("larger_than")
        && !matches.get_flag("smaller_than")
        && !matches.get_flag("older_than")
        && !matches.get_flag("newer_than")
        && !matches.get_flag("empty")
        && !matches.get_flag("delete_duplicates")
        && !matches.get_flag("merge_duplicates")
        && !matches.contains_id("content")
        && !matches.contains_id("top")
        && !matches.get_flag("stat")
        && !matches.contains_id("language")
        && !matches.get_flag("logo");

    if no_args {
        if color {
            eprintln!("{}", i18n::tr("warning_large_dir").yellow());
        } else {
            eprintln!("{}", i18n::tr("warning_large_dir"));
        }
    }

    let search_pattern = matches.get_one::<String>("search");
    let excluding_pattern = matches.get_one::<String>("excluding");
    let extension = matches.get_one::<String>("extension");
    let max_depth = matches
        .get_one::<String>("max_depth")
        .and_then(|s| s.parse::<usize>().ok());
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

    if matches.get_flag("interactive") {
        run_interactive_mode(
            color,
            &size_unit,
            auto_size,
            matches.get_flag("exclude_dirs"),
            matches.get_flag("ignore_hidden"),
            extension,
            json,
            content_dups,
            hash_algorithm,
            min_size,
            max_size,
            equal_size,
            min_age_seconds,
            max_age_seconds,
            empty_only,
            content_pattern,
            duplicate_action,
            force,
            max_depth,
        );
        return;
    }

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
                content_dups,
                hash_algorithm,
                show_size,
                show_detailed_permissions,
                matches.get_flag("exclude_dirs"),
                min_size,
                max_size,
                equal_size,
                min_age_seconds,
                max_age_seconds,
                empty_only,
                content_pattern,
                duplicate_action,
                force,
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
            eprintln!("{}", i18n::tr("error_whole_requires_path"));
            process::exit(1);
        }
        for path_str in &paths {
            let path = Path::new(path_str);
            if !path.exists() {
            eprintln!("{}", i18n::tr("error_path_not_exist").replace("{}", path_str));
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
                        eprintln!("{}", i18n::tr("error_reading_metadata").replace("{}", &e.to_string()));
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

                if json {
                    let info = serde_json::json!({
                        "name": file_name.to_string(),
                        "path": path.canonicalize().unwrap_or(path.to_path_buf()).display().to_string(),
                        "size": size,
                        "size_human": size_str,
                        "file_type": file_type,
                        "extension": extension,
                        "permissions": permissions,
                        "created": created_str,
                        "modified": modified_str,
                        "is_directory": false,
                    });
                    println!("{}", serde_json::to_string_pretty(&info).unwrap());
                } else {
                    println!("");
                    println!("{}", i18n::tr("file_analysis"));
                    println!("{}", "─".repeat(50));
                    if color {
                        println!("{} {}", i18n::tr("label_name"), file_name.blue().bold());
println!(
                            "{} {}",
                            i18n::tr("label_path"),
                            path.canonicalize().unwrap_or(path.to_path_buf()).display()
                        );
                        println!("{} {}", i18n::tr("label_size"), size_str.green().bold());
                        println!("{} {}", i18n::tr("label_type"), file_type.magenta());
                        println!("{} {}", i18n::tr("label_extension"), extension.cyan());
                        println!("{} {}", i18n::tr("label_permissions"), permissions.yellow());
                        println!("{} {}", i18n::tr("label_created"), created_str.yellow());
                        println!("{} {}", i18n::tr("label_modified"), modified_str.yellow());
                    } else {
                        println!("{} {}", i18n::tr("label_name"), file_name);
                        println!(
                            "{} {}",
                            i18n::tr("label_path"),
                            path.canonicalize().unwrap_or(path.to_path_buf()).display()
                        );
                        println!("{} {}", i18n::tr("label_size"), size_str);
                        println!("{} {}", i18n::tr("label_type"), file_type);
                        println!("{} {}", i18n::tr("label_extension"), extension);
                        println!("{} {}", i18n::tr("label_permissions"), permissions);
                        println!("{} {}", i18n::tr("label_created"), created_str);
                        println!("{} {}", i18n::tr("label_modified"), modified_str);
                    }
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
                        eprintln!("{}", i18n::tr("error_reading_metadata").replace("{}", &e.to_string()));
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
                println!("{}", i18n::tr("directory_analysis"));
                println!("{}", "─".repeat(50));
                if color {
                    println!(
                        "{} {}",
                        i18n::tr("label_name"),
                        canonical_path.file_name().unwrap_or_default().to_string_lossy().blue().bold()
                    );
                    println!("{} {}", i18n::tr("label_path"), canonical_path.display());
                    println!("{} {}", i18n::tr("label_size"), size_str.green().bold());
                    println!("{} {}", i18n::tr("label_permissions"), permissions.yellow());
                    println!("{} {}", i18n::tr("label_created"), created_str.yellow());
                    println!("{} {}", i18n::tr("label_modified"), modified_str.yellow());
                } else {
                    println!("{} {}", i18n::tr("label_name"), canonical_path.file_name().unwrap_or_default().to_string_lossy());
                    println!("{} {}", i18n::tr("label_path"), canonical_path.display());
                    println!("{} {}", i18n::tr("label_size"), size_str);
                    println!("{} {}", i18n::tr("label_permissions"), permissions);
                    println!("{} {}", i18n::tr("label_created"), created_str);
                    println!("{} {}", i18n::tr("label_modified"), modified_str);
                }
            } else {
                eprintln!(
                    "{}",
                    i18n::tr("error_path_neither").replace("{}", path_str)
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
                        eprintln!("{}", i18n::tr("error_reading_metadata").replace("{}", &e.to_string()));
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

                if json {
                    let info = serde_json::json!({
                        "name": file_name.to_string(),
                        "path": path.canonicalize().unwrap_or(path.to_path_buf()).display().to_string(),
                        "size": size,
                        "size_human": size_str,
                        "file_type": file_type,
                        "extension": extension,
                        "permissions": permissions,
                        "created": created_str,
                        "modified": modified_str,
                        "is_directory": false,
                    });
                    println!("{}", serde_json::to_string_pretty(&info).unwrap());
                } else {
                    println!("");
                    println!("{}", i18n::tr("file_analysis"));
                    println!("{}", "─".repeat(50));
                    if color {
                        println!("{} {}", i18n::tr("label_name"), file_name.blue().bold());
println!(
                            "{} {}",
                            i18n::tr("label_path"),
                            path.canonicalize().unwrap_or(path.to_path_buf()).display()
                        );
                        println!("{} {}", i18n::tr("label_size"), size_str.green().bold());
                        println!("{} {}", i18n::tr("label_type"), file_type.magenta());
                        println!("{} {}", i18n::tr("label_extension"), extension.cyan());
                        println!("{} {}", i18n::tr("label_permissions"), permissions.yellow());
                        println!("{} {}", i18n::tr("label_created"), created_str.yellow());
                        println!("{} {}", i18n::tr("label_modified"), modified_str.yellow());
                    } else {
                        println!("{} {}", i18n::tr("label_name"), file_name);
                        println!(
                            "{} {}",
                            i18n::tr("label_path"),
                            path.canonicalize().unwrap_or(path.to_path_buf()).display()
                        );
                        println!("{} {}", i18n::tr("label_size"), size_str);
                        println!("{} {}", i18n::tr("label_type"), file_type);
                        println!("{} {}", i18n::tr("label_extension"), extension);
                        println!("{} {}", i18n::tr("label_permissions"), permissions);
                        println!("{} {}", i18n::tr("label_created"), created_str);
                        println!("{} {}", i18n::tr("label_modified"), modified_str);
                    }
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
                eprintln!("{}", i18n::tr("error_path_not_exist").replace("{}", &lines_path.display().to_string()));
                process::exit(1);
            }

            if lines_path.is_file() {
                let line_count = count_lines(lines_path);
                let file_name = lines_path.file_name().unwrap_or_default().to_string_lossy();
                println!("");
                println!("{}", i18n::tr("line_count"));
                println!("{}", "─".repeat(50));
                if color {
                    println!("{} {}", i18n::tr("label_file_short"), file_name.blue().bold());
                    println!("{} {}", i18n::tr("label_lines"), line_count.to_string().green().bold());
                } else {
                    println!("{} {}", i18n::tr("label_file_short"), file_name);
                    println!("{} {}", i18n::tr("label_lines"), line_count);
                }
                continue;
            }

            let files = if recursive {
                collect_files_recursive_extended(lines_path, None, excluding_pattern, extension, None, matches.get_flag("exclude_dirs"), matches.get_flag("ignore_hidden"), max_depth, min_size, max_size, equal_size, min_age_seconds, max_age_seconds, empty_only, content_pattern)
            } else {
                collect_files_extended(lines_path, None, excluding_pattern, extension, None, matches.get_flag("exclude_dirs"), matches.get_flag("ignore_hidden"), min_size, max_size, equal_size, min_age_seconds, max_age_seconds, empty_only, content_pattern)
            };
            let files = filter_files(files, matches.get_flag("exclude_dirs"));

            if files.is_empty() {
                println!("{}", i18n::tr("no_files_found"));
                continue;
            }

            let mut total_lines: u64 = 0;
            println!("");
            println!("{}", i18n::tr("line_count"));
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
                println!("{} {}", i18n::tr("label_total_short"), total_lines.to_string().green().bold());
            } else {
                println!("{} {}", i18n::tr("label_total_short"), total_lines);
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
                eprintln!("{}", i18n::tr("error_not_valid_file").replace("{}", file));
            }
        }
        for path_str in &paths {
            let path = Path::new(path_str);
            if path.exists() && path.is_file() {
                preview_file(path, preview_lines, preview_mode);
                previewed = true;
            } else {
                eprintln!("{}", i18n::tr("error_not_valid_file").replace("{}", path_str));
            }
        }
        if !previewed {
            eprintln!("{}", i18n::tr("error_preview_requires_file"));
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
                    eprintln!("{}", i18n::tr("error_reading_metadata").replace("{}", &e.to_string()));
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

            if json {
                let info = serde_json::json!({
                    "name": file_name.to_string(),
                    "path": path.canonicalize().unwrap_or(path.to_path_buf()).display().to_string(),
                    "size": size,
                    "size_human": size_str,
                    "file_type": file_type,
                    "extension": extension,
                    "permissions": permissions,
                    "created": created_str,
                    "modified": modified_str,
                    "is_directory": false,
                });
                println!("{}", serde_json::to_string_pretty(&info).unwrap());
            } else {
                println!("");
                println!("{}", i18n::tr("file_analysis"));
                println!("{}", "─".repeat(50));
                if color {
                    println!("{} {}", i18n::tr("label_name"), file_name.blue().bold());
                    println!(
                        "{} {}",
                        i18n::tr("label_path"),
                        path.canonicalize().unwrap_or(path.to_path_buf()).display()
                    );
                    println!("{} {}", i18n::tr("label_size"), size_str.green().bold());
                    println!("{} {}", i18n::tr("label_type"), file_type.magenta());
                    println!("{} {}", i18n::tr("label_extension"), extension.cyan());
                    println!("{} {}", i18n::tr("label_permissions"), permissions.yellow());
                    println!("{} {}", i18n::tr("label_created"), created_str.yellow());
                    println!("{} {}", i18n::tr("label_modified"), modified_str.yellow());
                } else {
                    println!("{} {}", i18n::tr("label_name"), file_name);
                    println!(
                        "{} {}",
                        i18n::tr("label_path"),
                        path.canonicalize().unwrap_or(path.to_path_buf()).display()
                    );
                    println!("{} {}", i18n::tr("label_size"), size_str);
                    println!("{} {}", i18n::tr("label_type"), file_type);
                    println!("{} {}", i18n::tr("label_extension"), extension);
                    println!("{} {}", i18n::tr("label_permissions"), permissions);
                    println!("{} {}", i18n::tr("label_created"), created_str);
                    println!("{} {}", i18n::tr("label_modified"), modified_str);
                }
            }
            continue;
        }

        if matches.get_flag("recursive") {
            let search_path = paths.first().map(|p| Path::new(p.as_str())).unwrap_or_else(|| Path::new("."));
            let files = filter_files(
                collect_files_recursive_extended(search_path, Some(file), excluding_pattern, extension, sort_by.clone(), matches.get_flag("exclude_dirs"), matches.get_flag("ignore_hidden"), max_depth, min_size, max_size, equal_size, min_age_seconds, max_age_seconds, empty_only, content_pattern),
                matches.get_flag("exclude_dirs"),
            );
            let matching: Vec<_> = files.into_iter().filter(|f| f.name == **file || f.name.contains(*file)).collect();
            if matching.is_empty() {
                eprintln!("{}", i18n::tr("error_file_not_found").replace("{}", file));
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
                json,
            );
            continue;
        }

        eprintln!("{}", i18n::tr("error_file_not_found").replace("{}", file));
        process::exit(1);
    }
    if !file_paths.is_empty() {
        return;
    }

    if let Some(dir) = dir_path {
        let path = Path::new(dir);
        if !path.exists() {
            eprintln!("{}", i18n::tr("error_directory_not_found").replace("{}", dir));
            process::exit(1);
        }
        if !path.is_dir() {
            eprintln!("{}", i18n::tr("error_not_a_directory").replace("{}", dir));
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
                eprintln!("{}", i18n::tr("error_reading_metadata").replace("{}", &e.to_string()));
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
        println!("{}", i18n::tr("directory_analysis"));
        println!("{}", "─".repeat(50));
        if color {
            println!(
                "{} {}",
                i18n::tr("label_name"),
                canonical_path.file_name().unwrap_or_default().to_string_lossy().blue().bold()
            );
            println!("{} {}", i18n::tr("label_path"), canonical_path.display());
            println!("{} {}", i18n::tr("label_size"), size_str.green().bold());
            println!("{} {}", i18n::tr("label_permissions"), permissions.yellow());
            println!("{} {}", i18n::tr("label_created"), created_str.yellow());
            println!("{} {}", i18n::tr("label_modified"), modified_str.yellow());
        } else {
            println!("{} {}", i18n::tr("label_name"), canonical_path.file_name().unwrap_or_default().to_string_lossy());
            println!("{} {}", i18n::tr("label_path"), canonical_path.display());
            println!("{} {}", i18n::tr("label_size"), size_str);
            println!("{} {}", i18n::tr("label_permissions"), permissions);
            println!("{} {}", i18n::tr("label_created"), created_str);
            println!("{} {}", i18n::tr("label_modified"), modified_str);
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
            eprintln!("{}", i18n::tr("error_path_not_exist").replace("{}", &path.display().to_string()));
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
                    eprintln!("{}", i18n::tr("error_reading_metadata").replace("{}", &e.to_string()));
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

            if json {
                let info = serde_json::json!({
                    "name": file_name.to_string(),
                    "path": path.canonicalize().unwrap_or(path.to_path_buf()).display().to_string(),
                    "size": size,
                    "size_human": size_str,
                    "file_type": file_type,
                    "extension": extension,
                    "permissions": permissions,
                    "created": created_str,
                    "modified": modified_str,
                    "is_directory": false,
                });
                println!("{}", serde_json::to_string_pretty(&info).unwrap());
            } else {
                println!("");
                println!("{}", i18n::tr("file_analysis"));
                println!("{}", "─".repeat(50));
                if color {
                    println!("{} {}", i18n::tr("label_name"), file_name.blue().bold());
                    println!(
                        "{} {}",
                        i18n::tr("label_path"),
                        path.canonicalize().unwrap_or(path.to_path_buf()).display()
                    );
                    println!("{} {}", i18n::tr("label_size"), size_str.green().bold());
                    println!("{} {}", i18n::tr("label_type"), file_type.magenta());
                    println!("{} {}", i18n::tr("label_extension"), extension.cyan());
                    println!("{} {}", i18n::tr("label_permissions"), permissions.yellow());
                    println!("{} {}", i18n::tr("label_created"), created_str.yellow());
                    println!("{} {}", i18n::tr("label_modified"), modified_str.yellow());
                } else {
                    println!("{} {}", i18n::tr("label_name"), file_name);
                    println!(
                        "{} {}",
                        i18n::tr("label_path"),
                        path.canonicalize().unwrap_or(path.to_path_buf()).display()
                    );
                    println!("{} {}", i18n::tr("label_size"), size_str);
                    println!("{} {}", i18n::tr("label_type"), file_type);
                    println!("{} {}", i18n::tr("label_extension"), extension);
                    println!("{} {}", i18n::tr("label_permissions"), permissions);
                    println!("{} {}", i18n::tr("label_created"), created_str);
                    println!("{} {}", i18n::tr("label_modified"), modified_str);
                }
            }
            continue;
        }

        if matches.get_flag("tree") {
            if path.is_dir() {
                println!("{}", path.display());
                print_tree(path, "", color);
            } else {
                eprintln!("{}", i18n::tr("error_tree_not_dir"));
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
                        eprintln!("{}", i18n::tr("error_reading_metadata").replace("{}", &e.to_string()));
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

                if json {
                    let info = serde_json::json!({
                        "name": file_name.to_string(),
                        "path": path.display().to_string(),
                        "size": size,
                        "size_human": size_str,
                        "file_type": file_type,
                        "extension": extension,
                        "permissions": permissions,
                        "created": created_str,
                        "modified": modified_str,
                        "is_directory": false,
                    });
                    println!("{}", serde_json::to_string_pretty(&info).unwrap());
                } else {
                    println!("");
                    println!("{}", i18n::tr("file_analysis"));
                    println!("{}", "─".repeat(50));
                    if color {
                        println!("{} {}", i18n::tr("label_name"), file_name.blue().bold());
                        println!("{} {}", i18n::tr("label_path"), path.display());
                        println!("{} {}", i18n::tr("label_size"), size_str.green().bold());
                        println!("{} {}", i18n::tr("label_type"), file_type.magenta());
                        println!("{} {}", i18n::tr("label_extension"), extension.cyan());
                        println!("{} {}", i18n::tr("label_permissions"), permissions.yellow());
                        println!("{} {}", i18n::tr("label_created"), created_str.yellow());
                        println!("{} {}", i18n::tr("label_modified"), modified_str.yellow());
                    } else {
                        println!("{} {}", i18n::tr("label_name"), file_name);
                        println!("{} {}", i18n::tr("label_path"), path.display());
                        println!("{} {}", i18n::tr("label_size"), size_str);
                        println!("{} {}", i18n::tr("label_type"), file_type);
                        println!("{} {}", i18n::tr("label_extension"), extension);
                        println!("{} {}", i18n::tr("label_permissions"), permissions);
                        println!("{} {}", i18n::tr("label_created"), created_str);
                        println!("{} {}", i18n::tr("label_modified"), modified_str);
                    }
                }
            } else if path.is_dir() {
                let files =
                    filter_files(collect_files_recursive_extended(path, search_pattern, excluding_pattern, extension, sort_by.clone(), matches.get_flag("exclude_dirs"), matches.get_flag("ignore_hidden"), max_depth, min_size, max_size, equal_size, min_age_seconds, max_age_seconds, empty_only, content_pattern), matches.get_flag("exclude_dirs"));
                if files.is_empty() {
                    println!("{}", i18n::tr("no_files_found_in_dir"));
                } else {
                    let total_files = files.len();
                    let total_dirs = files.iter().filter(|f| f.is_directory).count();
                    let total_regular_files = total_files - total_dirs;
                    let _total_size: u64 = files.iter().map(|f| f.size).sum();
                    let dir_size = get_file_size(path);
                    println!("");
                    if color {
                        let items_label = crate::i18n::tr_format("label_files_dirs_format", &[&total_regular_files.to_string(), &total_dirs.to_string()]);
                        println!("{} {}", i18n::tr("label_directory"), path.display());
                        println!("{}", i18n::tr_format("label_total_items_format", &[&total_files.to_string().cyan().to_string(), &items_label.yellow().to_string()]));
                        println!("{} {}", i18n::tr("total_size"), SizeUnit::auto_format_size(dir_size).green().bold());
                    } else {
                        let items_label = crate::i18n::tr_format("label_files_dirs_format", &[&total_regular_files.to_string(), &total_dirs.to_string()]);
                        println!("{} {}", i18n::tr("label_directory"), path.display());
                        println!("{}", i18n::tr_format("label_total_items_format", &[&total_files.to_string(), &items_label]));
                        println!("{} {}", i18n::tr("total_size"), SizeUnit::auto_format_size(dir_size));
                    }
                    println!("");
                    show_file_type_stats(&files, color);
                    show_detailed_analysis(&files, color);
                }
            } else {
            eprintln!("{}", i18n::tr("error_path_not_exist").replace("{}", &path.display().to_string()));
                process::exit(1);
            }
        } else {
            if matches.get_flag("duplicates") {
                let groups = find_duplicates(path, color, content_dups, hash_algorithm);
                if duplicate_action != DuplicateAction::None {
                    apply_duplicate_action(&groups, duplicate_action, matches.get_flag("delete_duplicates") || matches.get_flag("merge_duplicates"));
                }
            } else if matches.get_flag("tree") {
                if path.is_dir() {
                    println!("{}", path.display());
                    print_tree(path, "", color);
                } else {
                    eprintln!("{}", i18n::tr("error_tree_not_dir"));
                    process::exit(1);
                }
            } else {
                if path.is_dir() {
                }

                let files = if matches.get_flag("recursive") {
                    collect_files_recursive_extended(path, search_pattern, excluding_pattern, extension, sort_by.clone(), matches.get_flag("exclude_dirs"), matches.get_flag("ignore_hidden"), max_depth, min_size, max_size, equal_size, min_age_seconds, max_age_seconds, empty_only, content_pattern)
                } else {
                    collect_files_extended(path, search_pattern, excluding_pattern, extension, sort_by.clone(), matches.get_flag("exclude_dirs"), matches.get_flag("ignore_hidden"), min_size, max_size, equal_size, min_age_seconds, max_age_seconds, empty_only, content_pattern)
                };
                let files = filter_files(files, matches.get_flag("exclude_dirs"));

                if matches.get_flag("stat") {
                    let total_files = files.len();
                    let total_dirs = files.iter().filter(|f| f.is_directory).count();
                    let total_regular = total_files - total_dirs;
                    let total_size: u64 = files.iter().map(|f| f.size).sum();
                    let dir_size = get_file_size(path);
                    println!("");
                    println!("{}", i18n::tr("directory_statistics"));
                    println!("{}", "─".repeat(50));
                    if color {
                        println!("{} {}", i18n::tr("label_path"), path.display());
                        let items_label = crate::i18n::tr_format("label_files_dirs_format", &[&total_regular.to_string(), &total_dirs.to_string()]);
                        println!("{}", i18n::tr_format("label_total_items_format", &[&total_files.to_string().cyan().to_string(), &items_label.yellow().to_string()]));
                        println!("{} {}", i18n::tr("total_size"), SizeUnit::auto_format_size(dir_size).green().bold());
                        println!("{} {}", i18n::tr("sum_of_file_sizes"), SizeUnit::auto_format_size(total_size).green());
                    } else {
                        println!("{} {}", i18n::tr("label_path"), path.display());
                        let items_label = crate::i18n::tr_format("label_files_dirs_format", &[&total_regular.to_string(), &total_dirs.to_string()]);
                        println!("{}", i18n::tr_format("label_total_items_format", &[&total_files.to_string(), &items_label]));
                        println!("{} {}", i18n::tr("total_size"), SizeUnit::auto_format_size(dir_size));
                        println!("{} {}", i18n::tr("sum_of_file_sizes"), SizeUnit::auto_format_size(total_size));
                    }
                    continue;
                }

                if let Some(top_str) = matches.get_one::<String>("top") {
                    let top_n: usize = top_str.parse().unwrap_or(10);
                    let mut top_files: Vec<_> = files.iter().filter(|f| !f.is_directory).collect();
                    top_files.sort_by(|a, b| b.size.cmp(&a.size));
                    top_files.truncate(top_n);
                    if top_files.is_empty() {
                        println!("{}", i18n::tr("no_files_found"));
                    } else {
                        println!("");
                        println!("{}", i18n::tr_format("label_top_files", &[&top_files.len().to_string()]));
                        println!("{}", "─".repeat(50));
                        for (i, f) in top_files.iter().enumerate() {
                            if color {
                                println!("{}. {} - {}", (i + 1).to_string().yellow().bold(), f.name.cyan(), f.size_human.green());
                            } else {
                                println!("{}. {} - {}", i + 1, f.name, f.size_human);
                            }
                        }
                    }
                    continue;
                }

                if files.is_empty() {
                    if let Some(pattern) = search_pattern {
                        println!("{}", i18n::tr("no_files_found_pattern").replace("{}", pattern));
                    } else {
                        println!("{}", i18n::tr("no_files_found"));
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
                            json,
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
                            json,
                        );
                    }
                }
            }
        }
    }
}

fn run_interactive_mode(
    color: bool,
    size_unit: &SizeUnit,
    auto_size: bool,
    exclude_dirs: bool,
    ignore_hidden: bool,
    extension: Option<&String>,
    json: bool,
    content_dups: bool,
    hash_algorithm: HashAlgorithm,
    min_size: Option<u64>,
    max_size: Option<u64>,
    equal_size: Option<u64>,
    min_age_seconds: Option<i64>,
    max_age_seconds: Option<i64>,
    empty_only: bool,
    content_pattern: Option<&String>,
    duplicate_action: DuplicateAction,
    force: bool,
    max_depth: Option<usize>,
) {
    loop {
        clear_screen();
        println!();
        if color {
            println!("{}", "╔══════════════════════════════════════════════════════════╗".cyan());
            println!("{}", format!("║{:^58}║", i18n::tr("interactive_menu_title")).cyan());
            println!("{}", "╚══════════════════════════════════════════════════════════╝".cyan());
        } else {
            println!("╔══════════════════════════════════════════════════════════╗");
            println!("{}", format!("║{:^58}║", i18n::tr("interactive_menu_title")));
            println!("╚══════════════════════════════════════════════════════════╝");
        }
        println!();
        println!("{}", i18n::tr("menu_list_files"));
        println!("{}", i18n::tr("menu_analyze_file"));
        println!("{}", i18n::tr("menu_analyze_directory"));
        println!("{}", i18n::tr("menu_find_duplicates"));
        println!("{}", i18n::tr("menu_show_tree"));
        println!("{}", i18n::tr("menu_list_disks"));
        println!("{}", i18n::tr("menu_search_files"));
        println!("{}", i18n::tr("menu_file_type_stats"));
        println!("{}", i18n::tr("menu_bit_converter"));
        println!("{}", i18n::tr("menu_change_language"));
        println!("{}", i18n::tr("menu_exit"));
        println!();
        print!("{} ", i18n::tr("menu_select_option"));
        io::stdout().flush().unwrap();

        let mut choice = String::new();
        io::stdin().read_line(&mut choice).unwrap();
        let choice = choice.trim();

        match choice {
            "1" => {
                // List files in current directory
                let current_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")).display().to_string();
                print!("{} ", i18n::tr("enter_directory_path").replace("{}", &current_dir));
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
                    let files = filter_files(collect_files_extended(path, None, None, extension, None, exclude_dirs, ignore_hidden, min_size, max_size, equal_size, min_age_seconds, max_age_seconds, empty_only, content_pattern), exclude_dirs);
                    if files.is_empty() {
                        println!("{}", i18n::tr("no_files_found"));
                    } else {
                        display_files(&files, size_unit, color, false, auto_size, false, None, true, false, json);
                    }
                    println!();
                    print!("{} ", i18n::tr("menu_return_prompt"));
                    io::stdout().flush().unwrap();
                    let mut _input = String::new();
                    io::stdin().read_line(&mut _input).unwrap();
                    clear_screen();
                } else {
                    eprintln!("{}", i18n::tr("error_not_valid_directory").replace("{}", target_path));
                }
            }
            "2" => {
                // Analyze a specific file
                print!("{}", i18n::tr("enter_file_path"));
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
                        println!("{} {}", i18n::tr("label_name"), file_name.blue().bold());
                        println!("{} {}", i18n::tr("label_path"), path.display());
                        println!("{} {}", i18n::tr("label_size"), size_str.green().bold());
                        println!("{} {}", i18n::tr("label_type"), file_type.magenta());
                        println!("{} {}", i18n::tr("label_permissions"), permissions.yellow());
                    } else {
                        println!();
                        println!("{}", "─".repeat(50));
                        println!("{} {}", i18n::tr("label_name"), file_name);
                        println!("{} {}", i18n::tr("label_path"), path.display());
                        println!("{} {}", i18n::tr("label_size"), size_str);
                        println!("{} {}", i18n::tr("label_type"), file_type);
                        println!("{} {}", i18n::tr("label_permissions"), permissions);
                    }
                    println!();
                    print!("{} ", i18n::tr("menu_return_prompt"));
                    io::stdout().flush().unwrap();
                    let mut _input = String::new();
                    io::stdin().read_line(&mut _input).unwrap();
                    clear_screen();
                } else {
                    eprintln!("{}", i18n::tr("error_not_valid_file").replace("{}", path_str));
                }
            }
            "3" => {
                // Analyze a directory
                let current_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")).display().to_string();
                print!("{} ", i18n::tr("enter_directory_path").replace("{}", &current_dir));
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
                        println!("{} {}", i18n::tr("label_name"), dir_name.blue().bold());
                        println!("{} {}", i18n::tr("label_path"), path.display());
                        println!("{} {}", i18n::tr("label_size"), size_str.green().bold());
                        println!("{} {}", i18n::tr("label_permissions"), permissions.yellow());
                    } else {
                        println!();
                        println!("{}", "─".repeat(50));
                        println!("{} {}", i18n::tr("label_name"), dir_name);
                        println!("{} {}", i18n::tr("label_path"), path.display());
                        println!("{} {}", i18n::tr("label_size"), size_str);
                        println!("{} {}", i18n::tr("label_permissions"), permissions);
                    }
                    println!();
                    print!("{} ", i18n::tr("menu_return_prompt"));
                    io::stdout().flush().unwrap();
                    let mut _input = String::new();
                    io::stdin().read_line(&mut _input).unwrap();
                } else {
                    eprintln!("{}", i18n::tr("error_not_valid_directory").replace("{}", path_str));
                }
            }
            "4" => {
                // Find duplicate files
                let current_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")).display().to_string();
                print!("{} ", i18n::tr("enter_directory_search").replace("{}", &current_dir));
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
                    print!("{} ", i18n::tr("verify_duplicates_by_hash").replace("{}", if content_dups { "yes" } else { "no" }));
                    io::stdout().flush().unwrap();
                    let mut verify_input = String::new();
                    io::stdin().read_line(&mut verify_input).unwrap();
                    let verify_input = verify_input.trim().to_lowercase();
                    let use_content_dups = if verify_input == "y" || verify_input == "yes" {
                        true
                    } else if verify_input == "n" || verify_input == "no" {
                        false
                    } else {
                        content_dups
                    };

                    if use_content_dups {
                        print!("{} ", i18n::tr("hash_algorithm_prompt").replace("{}", hash_algorithm.as_str()));
                        io::stdout().flush().unwrap();
                        let mut algo_input = String::new();
                        io::stdin().read_line(&mut algo_input).unwrap();
                        let algo_input = algo_input.trim();
                        let chosen_algo = if algo_input.is_empty() {
                            hash_algorithm
                        } else {
                            match HashAlgorithm::from_str(algo_input) {
                                Ok(a) => a,
                                Err(e) => {
                                    eprintln!("Error: {}", e);
                                    return_to_menu(color);
                                    continue;
                                }
                            }
                        };
                        let groups = find_duplicates(path, color, true, chosen_algo);
                        if duplicate_action != DuplicateAction::None {
                            apply_duplicate_action(&groups, duplicate_action, force);
                        }
                    } else {
                        let groups = find_duplicates(path, color, false, hash_algorithm);
                        if duplicate_action != DuplicateAction::None {
                            apply_duplicate_action(&groups, duplicate_action, force);
                        }
                    }
                    println!();
                    print!("{} ", i18n::tr("menu_return_prompt"));
                    io::stdout().flush().unwrap();
                    let mut _input = String::new();
                    io::stdin().read_line(&mut _input).unwrap();
                    clear_screen();
                } else {
                    eprintln!("{}", i18n::tr("error_not_valid_directory").replace("{}", target_path));
                }
            }
            "5" => {
                // Show directory tree
                let current_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")).display().to_string();
                print!("{} ", i18n::tr("enter_directory_path").replace("{}", &current_dir));
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
                    print!("{} ", i18n::tr("menu_return_prompt"));
                    io::stdout().flush().unwrap();
                    let mut _input = String::new();
                    io::stdin().read_line(&mut _input).unwrap();
                    clear_screen();
                } else {
                    eprintln!("{}", i18n::tr("error_not_valid_directory").replace("{}", target_path));
                }
            }
            "6" => {
                // List all disks
                list_disks(color, size_unit, auto_size);
                println!();
                print!("{} ", i18n::tr("menu_return_prompt"));
                io::stdout().flush().unwrap();
                let mut _input = String::new();
                io::stdin().read_line(&mut _input).unwrap();
                clear_screen();
            }
            "7" => {
                // Search for files
                let current_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")).display().to_string();
                print!("{} ", i18n::tr("enter_regex_pattern"));
                io::stdout().flush().unwrap();
                let mut pattern_input = String::new();
                io::stdin().read_line(&mut pattern_input).unwrap();
                let pattern = pattern_input.trim();
                
                print!("{} ", i18n::tr("enter_directory_search").replace("{}", &current_dir));
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
                    let files = filter_files(collect_files_extended(path, Some(&pattern.to_string()), None, extension, None, exclude_dirs, ignore_hidden, min_size, max_size, equal_size, min_age_seconds, max_age_seconds, empty_only, content_pattern), exclude_dirs);
                    if files.is_empty() {
                        println!("{}", i18n::tr("no_files_found_pattern").replace("{}", pattern));
                    } else {
                        show_file_type_stats(&files, color);
                    }
                    println!();
                    print!("{} ", i18n::tr("menu_return_prompt"));
                    io::stdout().flush().unwrap();
                    let mut _input = String::new();
                    io::stdin().read_line(&mut _input).unwrap();
                    clear_screen();
                } else {
                    eprintln!("{}", i18n::tr("error_not_valid_directory").replace("{}", target_path));
                }
            }
            "8" => {
                // Show file type statistics
                let current_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")).display().to_string();
                print!("{} ", i18n::tr("enter_directory_path").replace("{}", &current_dir));
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
                    let files = filter_files(collect_files_recursive_extended(path, None, None, extension, None, exclude_dirs, ignore_hidden, max_depth, min_size, max_size, equal_size, min_age_seconds, max_age_seconds, empty_only, content_pattern), exclude_dirs);
                    show_file_type_stats(&files, color);
                    println!();
                    print!("{} ", i18n::tr("menu_return_prompt"));
                    io::stdout().flush().unwrap();
                    let mut _input = String::new();
                    io::stdin().read_line(&mut _input).unwrap();
                    clear_screen();
                } else {
                    eprintln!("{}", i18n::tr("error_not_valid_directory").replace("{}", target_path));
                }
            }
            "9" => {
                // Bit converter
                println!("{}", i18n::tr("bit_converter_title"));
                println!("{}", "─".repeat(40));
                println!("{}", i18n::tr("bit_converter_instruction"));
                println!("{}", i18n::tr("bit_converter_examples"));
                println!();
                print!("{} ", i18n::tr("enter_value_and_unit"));
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
                        eprintln!("{}", i18n::tr("error_invalid_number").replace("{}", parts[0]));
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
                            eprintln!("{}", i18n::tr("error_unknown_unit").replace("{}", &unit));
                            return_to_menu(color);
                            continue;
                        }
                    };
                    
                    println!();
                    println!("{}", i18n::tr("conversion_results"));
                    println!("{}", "─".repeat(40));
                    println!("{} {:.0}", i18n::tr("bits_b"), bits);
                    println!("{} {:.2} Kb", i18n::tr("kilobits"), bits / 1000.0);
                    println!("{} {:.2} Mb", i18n::tr("megabits"), bits / 1_000_000.0);
                    println!("{} {:.2} Gb", i18n::tr("gigabits"), bits / 1_000_000_000.0);
                    println!("{} {:.2} Tb", i18n::tr("terabits"), bits / 1_000_000_000_000.0);
                    println!();
                    println!("{} {:.0}", i18n::tr("bytes_b"), bits / 8.0);
                    println!("{} {:.2} KB", i18n::tr("kilobytes"), bits / 8.0 / 1000.0);
                    println!("{} {:.2} MB", i18n::tr("megabytes"), bits / 8.0 / 1_000_000.0);
                    println!("{} {:.2} GB", i18n::tr("gigabytes"), bits / 8.0 / 1_000_000_000.0);
                    println!("{} {:.2} TB", i18n::tr("terabytes"), bits / 8.0 / 1_000_000_000_000.0);
                } else {
                    eprintln!("{}", i18n::tr("bit_converter_error_input"));
                }
                return_to_menu(color);
            }
            "L" | "l" => {
                i18n::handle_language_flag(color);
                println!();
                print!("{} ", i18n::tr("menu_return_prompt"));
                io::stdout().flush().unwrap();
                let mut _input = String::new();
                io::stdin().read_line(&mut _input).unwrap();
                clear_screen();
            }
            "0" => {
                println!("{}", i18n::tr("goodbye"));
                break;
            }
            _ => {
                eprintln!("{}", i18n::tr("menu_invalid_option"));
            }
        }
    }
}
