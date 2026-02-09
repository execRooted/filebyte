use colored::Colorize;
use std::fs;
use std::path::Path;

/// Print a directory tree structure
pub fn print_tree(path: &Path, prefix: &str, color: bool) {
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
