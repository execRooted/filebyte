use crate::types::{FileInfo, HashAlgorithm};
use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;
use chrono::Utc;

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

pub fn get_file_age_seconds(path: &Path) -> i64 {
    if let Ok(metadata) = fs::metadata(path) {
        if let Ok(modified) = metadata.modified() {
            if let Ok(duration) = std::time::SystemTime::now().duration_since(modified) {
                return duration.as_secs() as i64;
            }
        }
    }
    0
}

#[allow(dead_code)]
pub fn is_empty_dir(path: &Path) -> bool {
    if let Ok(entries) = fs::read_dir(path) {
        entries.count() == 0
    } else {
        false
    }
}

pub fn parse_size_threshold(s: &str) -> Result<u64, String> {
    let s = s.trim();

    fn try_parse_size(s: &str) -> Result<u64, String> {
        let s_lower = s.to_lowercase();
        let parts: Vec<&str> = s_lower.split_whitespace().collect();

        let (num_str, unit) = match parts.as_slice() {
            [num, unit] => (*num, *unit),
            [combined] => {
                if combined.ends_with("tb") {
                    (&combined[..combined.len() - 2], "tb")
                } else if combined.ends_with("gb") {
                    (&combined[..combined.len() - 2], "gb")
                } else if combined.ends_with("mb") {
                    (&combined[..combined.len() - 2], "mb")
                } else if combined.ends_with("kb") {
                    (&combined[..combined.len() - 2], "kb")
                } else if combined.ends_with("b") {
                    (&combined[..combined.len() - 1], "b")
                } else if combined.chars().all(|c| c.is_ascii_digit() || c == '.') {
                    (*combined, "b")
                } else {
                    return Err(format!("Invalid size format: {}", combined));
                }
            }
            _ => return Err(format!("Invalid size format: {}", s)),
        };

        let num: f64 = num_str.parse().map_err(|_| format!("Invalid size number: {}", num_str))?;
        match unit {
            "b" => Ok(num.round() as u64),
            "kb" => Ok((num * 1024.0).round() as u64),
            "mb" => Ok((num * 1024.0 * 1024.0).round() as u64),
            "gb" => Ok((num * 1024.0 * 1024.0 * 1024.0).round() as u64),
            "tb" => Ok((num * 1024.0 * 1024.0 * 1024.0 * 1024.0).round() as u64),
            _ => Err(format!("Unknown size unit: {}. Use b, kb, mb, gb, tb", unit)),
        }
    }

    if let Ok(size) = try_parse_size(s) {
        return Ok(size);
    }

    let path = Path::new(s);
    if path.exists() && path.is_file() {
        if let Ok(metadata) = fs::metadata(path) {
            return Ok(metadata.len());
        }
        return Err(format!("Cannot read file metadata: {}", s));
    }

    Err(format!("Invalid size format or file not found: {}", s))
}

pub fn parse_age_threshold(s: &str) -> Result<i64, String> {
    let s = s.trim().to_lowercase();

    if let Ok(date) = chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d") {
        if let Some(target_dt) = date.and_hms_opt(0, 0, 0) {
            let target = chrono::DateTime::<Utc>::from_naive_utc_and_offset(target_dt, Utc).timestamp();
            let now = chrono::Utc::now().timestamp();
            return Ok(now - target);
        }
    }

    let parts: Vec<&str> = s.split_whitespace().collect();
    let (num_str, unit) = match parts.as_slice() {
        [num, unit] => (*num, *unit),
        [combined] => {
            if combined.len() < 2 {
                return Err(format!("Invalid age format: {}", combined));
            }
            (&combined[..combined.len() - 1], &combined[combined.len() - 1..])
        }
        _ => return Err(format!("Invalid age format: {}", s)),
    };

    let num: i64 = num_str.parse().map_err(|_| format!("Invalid number: {}", num_str))?;

    match unit {
        "d" => Ok(num * 86400),
        "w" => Ok(num * 604800),
        "m" => Ok(num * 2592000),
        "y" => Ok(num * 31536000),
        _ => Err(format!(
            "Unknown time unit: '{}'. Use d, w, m, y or YYYY-MM-DD",
            unit
        )),
    }
}

pub fn file_contains_text(path: &Path, pattern: &str) -> bool {
    if let Ok(content) = fs::read_to_string(path) {
        content.contains(pattern)
    } else {
        false
    }
}

pub fn delete_duplicate_file(path: &Path, force: bool) -> bool {
    if !force {
        print!("Delete {}? (y/N): ", path.display());
        io::stdout().flush().unwrap();
        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            return false;
        }
        let input = input.trim().to_lowercase();
        if input != "y" && input != "yes" {
            return false;
        }
    }
    fs::remove_file(path).is_ok()
}

pub fn merge_duplicate_file(path: &Path, target: &Path) -> bool {
    if path == target {
        return true;
    }
    let _ = fs::remove_file(path);
    if fs::hard_link(target, path).is_ok() {
        return true;
    }
    if std::os::unix::fs::symlink(target, path).is_err() {
        eprintln!(
            "Error linking {} -> {}: hard link and symlink both failed",
            path.display(),
            target.display()
        );
        return false;
    }
    true
}

pub fn format_unix_permissions(metadata: &fs::Metadata, detailed: bool) -> String {
    if detailed {
        #[cfg(unix)]
        let mode = {
            use std::os::unix::fs::PermissionsExt;
            metadata.permissions().mode()
        };
        #[cfg(not(unix))]
        let mode: u32 = {
            if metadata.permissions().readonly() {
                0o555
            } else {
                0o777
            }
        };

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
            if can_delete(&std::path::Path::new("")) {
                "r-x"
            } else {
                "r--"
            }
        } else {
            if can_delete(&std::path::Path::new("")) {
                "rwx"
            } else {
                "rw-"
            }
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

pub fn compute_file_hash(path: &Path, algorithm: HashAlgorithm) -> Option<String> {
    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return None,
    };
    let mut reader = std::io::BufReader::new(file);
    let mut buffer = [0u8; 65536];

    match algorithm {
        HashAlgorithm::Sha256 => {
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            loop {
                let bytes_read = match reader.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(n) => n,
                    Err(_) => return None,
                };
                hasher.update(&buffer[..bytes_read]);
            }
            let result = hasher.finalize();
            Some(format!("{:x}", result))
        }
        HashAlgorithm::Md5 => {
            use md5::{Digest, Md5};
            let mut hasher = Md5::new();
            loop {
                let bytes_read = match reader.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(n) => n,
                    Err(_) => return None,
                };
                hasher.update(&buffer[..bytes_read]);
            }
            let result = hasher.finalize();
            Some(format!("{:x}", result))
        }
    }
}