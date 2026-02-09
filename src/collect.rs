use crate::types::{FileInfo, SizeUnit, SortBy};
use crate::utils::{can_delete, get_file_size};
use chrono::{DateTime, Utc};
use infer;
use regex::Regex;
use std::fs;
use std::path::Path;

/// Collect files from a directory (non-recursively)
pub fn collect_files(
    dir: &Path,
    search_pattern: Option<&String>,
    excluding_pattern: Option<&String>,
    sort_by: Option<SortBy>,
) -> Vec<FileInfo> {
    let mut files = Vec::new();

    fn collect_recursive(
        path: &Path,
        files: &mut Vec<FileInfo>,
        search_pattern: Option<&String>,
        excluding_regex: Option<&Regex>,
    ) {
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                let file_name = entry_path.file_name().unwrap_or_default().to_string_lossy();

                if let Some(regex) = excluding_regex {
                    if regex.is_match(&file_name) {
                        continue;
                    }
                }

                if let Ok(metadata) = entry.metadata() {
                    let should_collect = if let Some(pattern) = search_pattern {
                        let matches = if pattern.starts_with('^')
                            || pattern.ends_with('$')
                            || pattern.contains(".*")
                            || pattern.contains('[')
                            || pattern.contains(']')
                        {
                            if let Ok(regex) = Regex::new(pattern) {
                                regex.is_match(&file_name)
                            } else {
                                false
                            }
                        } else {
                            file_name.contains(pattern)
                        };
                        matches
                    } else {
                        true
                    };

                    if should_collect {
                        let file_type = if entry_path.is_dir() {
                            "directory".to_string()
                        } else {
                            infer::get_from_path(&entry_path)
                                .ok()
                                .flatten()
                                .map(|kind| kind.mime_type().to_string())
                                .unwrap_or_else(|| "unknown".to_string())
                        };

                        let created = metadata
                            .created()
                            .ok()
                            .map(|t| DateTime::<Utc>::from(t).format("%Y-%m-%d %H:%M:%S UTC").to_string());

                        let modified = metadata
                            .modified()
                            .ok()
                            .map(|t| DateTime::<Utc>::from(t).format("%Y-%m-%d %H:%M:%S UTC").to_string());

                        let permissions = if metadata.permissions().readonly() {
                            if can_delete(&entry_path) { "r-x" } else { "r--" }
                        } else {
                            if can_delete(&entry_path) { "rwx" } else { "rw-" }
                        };

                        files.push(FileInfo {
                            name: file_name.to_string(),
                            path: entry_path.to_string_lossy().to_string(),
                            size: get_file_size(&entry_path),
                            size_human: SizeUnit::auto_format_size(get_file_size(&entry_path)),
                            file_type,
                            created,
                            modified,
                            permissions: permissions.to_string(),
                            is_directory: entry_path.is_dir(),
                        });
                    }
                }
            }
        }
    }

    let excluding_regex = excluding_pattern.and_then(|p| Regex::new(p).ok());
    collect_recursive(dir, &mut files, search_pattern, excluding_regex.as_ref());

    if let Some(sort_criteria) = sort_by {
        match sort_criteria {
            SortBy::Name => files.sort_by(|a, b| match (a.is_directory, b.is_directory) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            }),
            SortBy::Size => files.sort_by(|a, b| match (a.is_directory, b.is_directory) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => b.size.cmp(&a.size),
            }),
            SortBy::Date => files.sort_by(|a, b| match (a.is_directory, b.is_directory) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => {
                    let a_date = a.modified.as_ref().map(|s| s.as_str()).unwrap_or("");
                    let b_date = b.modified.as_ref().map(|s| s.as_str()).unwrap_or("");
                    b_date.cmp(a_date)
                }
            }),
        }
    } else {
        files.sort_by(|a, b| match (a.is_directory, b.is_directory) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        });
    }

    files
}

/// Collect files from a directory recursively
pub fn collect_files_recursive(
    dir: &Path,
    search_pattern: Option<&String>,
    excluding_pattern: Option<&String>,
    sort_by: Option<SortBy>,
) -> Vec<FileInfo> {
    let mut files = Vec::new();

    fn collect_all_recursive(
        path: &Path,
        files: &mut Vec<FileInfo>,
        search_pattern: Option<&String>,
        excluding_regex: Option<&Regex>,
    ) {
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                let file_name = entry_path.file_name().unwrap_or_default().to_string_lossy();

                if let Some(regex) = excluding_regex {
                    if regex.is_match(&file_name) {
                        continue;
                    }
                }

                if let Ok(metadata) = entry.metadata() {
                    let should_collect = if let Some(pattern) = search_pattern {
                        let matches = if pattern.starts_with('^')
                            || pattern.ends_with('$')
                            || pattern.contains(".*")
                            || pattern.contains('[')
                            || pattern.contains(']')
                        {
                            if let Ok(regex) = Regex::new(pattern) {
                                regex.is_match(&file_name)
                            } else {
                                false
                            }
                        } else {
                            file_name.contains(pattern)
                        };
                        matches
                    } else {
                        true
                    };

                    if should_collect {
                        let file_type = if entry_path.is_dir() {
                            "directory".to_string()
                        } else {
                            infer::get_from_path(&entry_path)
                                .ok()
                                .flatten()
                                .map(|kind| kind.mime_type().to_string())
                                .unwrap_or_else(|| "unknown".to_string())
                        };

                        let created = metadata
                            .created()
                            .ok()
                            .map(|t| DateTime::<Utc>::from(t).format("%Y-%m-%d %H:%M:%S UTC").to_string());

                        let modified = metadata
                            .modified()
                            .ok()
                            .map(|t| DateTime::<Utc>::from(t).format("%Y-%m-%d %H:%M:%S UTC").to_string());

                        let permissions = if metadata.permissions().readonly() {
                            if can_delete(&entry_path) { "r-x" } else { "r--" }
                        } else {
                            if can_delete(&entry_path) { "rwx" } else { "rw-" }
                        };

                        files.push(FileInfo {
                            name: file_name.to_string(),
                            path: entry_path.to_string_lossy().to_string(),
                            size: get_file_size(&entry_path),
                            size_human: SizeUnit::auto_format_size(get_file_size(&entry_path)),
                            file_type,
                            created,
                            modified,
                            permissions: permissions.to_string(),
                            is_directory: entry_path.is_dir(),
                        });
                    }

                    if entry_path.is_dir() {
                        collect_all_recursive(&entry_path, files, search_pattern, excluding_regex);
                    }
                }
            }
        }
    }

    let excluding_regex = excluding_pattern.and_then(|p| Regex::new(p).ok());
    collect_all_recursive(dir, &mut files, search_pattern, excluding_regex.as_ref());

    if let Some(sort_criteria) = sort_by {
        match sort_criteria {
            SortBy::Name => files.sort_by(|a, b| match (a.is_directory, b.is_directory) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            }),
            SortBy::Size => files.sort_by(|a, b| match (a.is_directory, b.is_directory) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => b.size.cmp(&a.size),
            }),
            SortBy::Date => files.sort_by(|a, b| match (a.is_directory, b.is_directory) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => {
                    let a_date = a.modified.as_ref().map(|s| s.as_str()).unwrap_or("");
                    let b_date = b.modified.as_ref().map(|s| s.as_str()).unwrap_or("");
                    b_date.cmp(a_date)
                }
            }),
        }
    } else {
        files.sort_by(|a, b| match (a.is_directory, b.is_directory) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        });
    }

    files
}
