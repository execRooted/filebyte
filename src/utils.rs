use crate::types::FileInfo;
use std::fs;
use std::path::Path;
pub fn can_delete(path: &Path) -> bool {
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

pub fn get_file_extension(path: &Path) -> String {
    let file_name = match path.file_name().and_then(|n| n.to_str()) {
        Some(name) => name,
        None => return "none".to_string(),
    };
    let parts: Vec<&str> = file_name.split('.').collect();
    if parts.len() >= 2 {
        format!(".{}", parts[1..].join("."))
    } else {
        "none".to_string()
    }
}

pub fn get_file_size(path: &Path) -> u64 {
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

pub fn format_unix_permissions(metadata: &fs::Metadata, detailed: bool) -> String {
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

        format!(
            "{}{}{}{}{}{}{}{}{}{}",
            file_type, user_read, user_write, user_exec,
            group_read, group_write, group_exec,
            other_read, other_write, other_exec
        )
    } else {
        if metadata.permissions().readonly() {
            if can_delete(&std::path::Path::new("")) { "r-x" } else { "r--" }
        } else {
            if can_delete(&std::path::Path::new("")) { "rwx" } else { "rw-" }
        }
        .to_string()
    }
}

pub fn filter_files(files: Vec<FileInfo>, exclude_dirs: bool) -> Vec<FileInfo> {
    if exclude_dirs {
        files.into_iter().filter(|f| !f.is_directory).collect()
    } else {
        files
    }
}

pub fn preview_file(path: &Path, lines: usize, mode: &str) {
    match fs::read_to_string(path) {
        Ok(content) => {
            let file_lines: Vec<&str> = content.lines().collect();
            let total = file_lines.len();
            if total == 0 {
                println!("(empty file)");
                return;
            }
            println!("");
            if mode == "first" {
                println!("Preview (first {} lines):", lines);
            } else if mode == "last" {
                println!("Preview (last {} lines):", lines);
            } else {
                println!("Preview (first {} / last {} lines):", lines, lines);
            }
            println!("{}", "─".repeat(50));
            if total <= lines * 2 && mode == "both" {
                for line in &file_lines {
                    println!("{}", line);
                }
            } else if mode == "first" {
                let head_end = lines.min(total);
                for line in file_lines[..head_end].iter() {
                    println!("{}", line);
                }
            } else if mode == "last" {
                let tail_start = total.saturating_sub(lines);
                for line in file_lines[tail_start..].iter() {
                    println!("{}", line);
                }
            } else {
                let head_end = lines.min(total);
                for line in file_lines[..head_end].iter() {
                    println!("{}", line);
                }
                println!("{}", "... (lines omitted) ...");
                let tail_start = total.saturating_sub(lines);
                for line in file_lines[tail_start..].iter() {
                    println!("{}", line);
                }
            }
        }
        Err(_) => {
            eprintln!("Error: Could not read file (not a text file or permission denied)");
        }
    }
}
