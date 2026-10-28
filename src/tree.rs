use colored::Colorize;
use std::fs;
use std::path::Path;
use serde::Serialize;

#[derive(Serialize)]
pub struct TreeNode {
    name: String,
    path: String,
    is_directory: bool,
    children: Vec<TreeNode>,
}

pub fn build_tree(path: &Path, depth: usize, max_depth: usize) -> TreeNode {
    let name = path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let path_str = path.to_string_lossy().to_string();
    
    if !path.is_dir() || depth >= max_depth {
        return TreeNode {
            name,
            path: path_str,
            is_directory: path.is_dir(),
            children: vec![],
        };
    }
    
    let mut children = Vec::new();
    if let Ok(entries) = fs::read_dir(path) {
        let mut entries_vec: Vec<_> = entries.flatten().collect();
        entries_vec.sort_by_key(|e| e.file_name());
        for entry in entries_vec {
            let entry_path = entry.path();
            let file_name = entry_path.file_name().unwrap_or_default().to_string_lossy();
            if file_name == ".kilo" && entry_path.is_dir() {
                continue;
            }
            children.push(build_tree(&entry_path, depth + 1, max_depth));
        }
    }
    
    TreeNode {
        name,
        path: path_str,
        is_directory: true,
        children,
    }
}

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
        if file_name == ".kilo" && path.is_dir() {
            continue;
        }
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
