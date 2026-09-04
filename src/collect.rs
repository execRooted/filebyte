use crate::types::{FileInfo, SizeUnit, SortBy};
use crate::utils::{
    can_delete, file_contains_text, get_file_age_seconds, get_file_size,
};
use chrono::{DateTime, Utc};
use infer;
use regex::Regex;
use std::fs;
use std::path::Path;

fn sizes_equal_with_tolerance(file_size: u64, target: u64) -> bool {
    file_size == target
}

/// Collect files from a directory (non-recursively)
#[allow(dead_code)]
pub fn collect_files(
    dir: &Path,
    search_pattern: Option<&String>,
    excluding_pattern: Option<&String>,
    sort_by: Option<SortBy>,
    exclude_dirs: bool,
) -> Vec<FileInfo> {
    collect_files_extended(
        dir,
        search_pattern,
        excluding_pattern,
        sort_by,
        exclude_dirs,
        None,
        None,
        None,
        None,
        None,
        false,
        None,
    )
}

/// Collect files from a directory with extended filters (non-recursively)
pub fn collect_files_extended(
    dir: &Path,
    search_pattern: Option<&String>,
    excluding_pattern: Option<&String>,
    sort_by: Option<SortBy>,
    exclude_dirs: bool,
    min_size: Option<u64>,
    max_size: Option<u64>,
    equal_size: Option<u64>,
    min_age_seconds: Option<i64>,
    max_age_seconds: Option<i64>,
    empty_only: bool,
    content_pattern: Option<&String>,
) -> Vec<FileInfo> {
    let mut files = Vec::new();

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let entry_path = entry.path();
            let file_name = entry_path.file_name().unwrap_or_default().to_string_lossy();

            if let Some(regex) = excluding_pattern.and_then(|p| Regex::new(p).ok()) {
                let normalized_name = if file_name.ends_with('/') {
                    file_name.trim_end_matches('/')
                } else {
                    &file_name
                };
                if regex.is_match(normalized_name) {
                    continue;
                }
            }

            if let Ok(metadata) = entry.metadata() {
                if entry_path.is_dir() {
                    if !exclude_dirs {
                        let inner = collect_files_recursive_extended(
                            &entry_path,
                            search_pattern,
                            None,
                            None,
                            false,
                            min_size,
                            max_size,
                            equal_size,
                            min_age_seconds,
                            max_age_seconds,
                            empty_only,
                            content_pattern,
                        );
                        if !inner.is_empty() {
                            let file_type = "directory".to_string();
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
                            let file_size = get_file_size(&entry_path);

                            files.push(FileInfo {
                                name: file_name.to_string(),
                                path: entry_path.to_string_lossy().to_string(),
                                size: file_size,
                                size_human: SizeUnit::auto_format_size(file_size),
                                file_type,
                                created,
                                modified,
                                permissions: permissions.to_string(),
                                is_directory: true,
                            });
                        }
                    }
                } else {
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
                        let file_size = get_file_size(&entry_path);
                        let file_age = get_file_age_seconds(&entry_path);

                        if let Some(min) = min_size {
                            if file_size <= min {
                                continue;
                            }
                        }
                        if let Some(max) = max_size {
                            if file_size >= max {
                                continue;
                            }
                        }
                        if let Some(equal) = equal_size {
                            if !sizes_equal_with_tolerance(file_size, equal) {
                                continue;
                            }
                        }
                        if let Some(min_age) = min_age_seconds {
                            if file_age <= min_age {
                                continue;
                            }
                        }
                        if let Some(max_age) = max_age_seconds {
                            if file_age >= max_age {
                                continue;
                            }
                        }
                        if empty_only && file_size > 0 {
                            continue;
                        }
                        if let Some(pattern) = content_pattern {
                            if !file_contains_text(&entry_path, pattern) {
                                continue;
                            }
                        }

                        let file_type = infer::get_from_path(&entry_path)
                            .ok()
                            .flatten()
                            .map(|kind| kind.mime_type().to_string())
                            .unwrap_or_else(|| "unknown".to_string());

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
                            size: file_size,
                            size_human: SizeUnit::auto_format_size(file_size),
                            file_type,
                            created,
                            modified,
                            permissions: permissions.to_string(),
                            is_directory: false,
                        });
                    }
                }
            }
        }
    }

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
#[allow(dead_code)]
pub fn collect_files_recursive(
    dir: &Path,
    search_pattern: Option<&String>,
    excluding_pattern: Option<&String>,
    sort_by: Option<SortBy>,
    exclude_dirs: bool,
) -> Vec<FileInfo> {
    collect_files_recursive_extended(
        dir,
        search_pattern,
        excluding_pattern,
        sort_by,
        exclude_dirs,
        None,
        None,
        None,
        None,
        None,
        false,
        None,
    )
}

/// Collect files from a directory recursively with extended filters
pub fn collect_files_recursive_extended(
    dir: &Path,
    search_pattern: Option<&String>,
    excluding_pattern: Option<&String>,
    sort_by: Option<SortBy>,
    _exclude_dirs: bool,
    min_size: Option<u64>,
    max_size: Option<u64>,
    equal_size: Option<u64>,
    min_age_seconds: Option<i64>,
    max_age_seconds: Option<i64>,
    empty_only: bool,
    content_pattern: Option<&String>,
) -> Vec<FileInfo> {
    let mut files = Vec::new();

    fn collect_all_recursive(
        path: &Path,
        files: &mut Vec<FileInfo>,
        search_pattern: Option<&String>,
        excluding_regex: Option<&Regex>,
        min_size: Option<u64>,
        max_size: Option<u64>,
        equal_size: Option<u64>,
        min_age_seconds: Option<i64>,
        max_age_seconds: Option<i64>,
        empty_only: bool,
        content_pattern: Option<&String>,
    ) {
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                let file_name = entry_path.file_name().unwrap_or_default().to_string_lossy();

                if let Some(regex) = excluding_regex {
                    let normalized_name = if file_name.ends_with('/') {
                        file_name.trim_end_matches('/')
                    } else {
                        &file_name
                    };
                    if regex.is_match(normalized_name) {
                        continue;
                    }
                }

                if let Ok(metadata) = entry.metadata() {
                    if entry_path.is_dir() {
                        collect_all_recursive(
                            &entry_path,
                            files,
                            search_pattern,
                            excluding_regex,
                            min_size,
                            max_size,
                            equal_size,
                            min_age_seconds,
                            max_age_seconds,
                            empty_only,
                            content_pattern,
                        );
                    } else {
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
                            let file_size = get_file_size(&entry_path);
                            let file_age = get_file_age_seconds(&entry_path);

                        if let Some(min) = min_size {
                            if file_size <= min {
                                continue;
                            }
                        }
                        if let Some(max) = max_size {
                            if file_size >= max {
                                continue;
                            }
                        }
                        if let Some(equal) = equal_size {
                            if !sizes_equal_with_tolerance(file_size, equal) {
                                continue;
                            }
                        }
                        if let Some(min_age) = min_age_seconds {
                            if file_age <= min_age {
                                continue;
                            }
                        }
                        if let Some(max_age) = max_age_seconds {
                            if file_age >= max_age {
                                continue;
                            }
                        }
                        if empty_only && file_size > 0 {
                            continue;
                        }
                        if let Some(pattern) = content_pattern {
                            if !file_contains_text(&entry_path, pattern) {
                                continue;
                            }
                        }

                            let file_type = infer::get_from_path(&entry_path)
                                .ok()
                                .flatten()
                                .map(|kind| kind.mime_type().to_string())
                                .unwrap_or_else(|| "unknown".to_string());

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
                                size: file_size,
                                size_human: SizeUnit::auto_format_size(file_size),
                                file_type,
                                created,
                                modified,
                                permissions: permissions.to_string(),
                                is_directory: false,
                            });
                        }
                    }
                }
            }
        }
    }

    let excluding_regex = excluding_pattern.and_then(|p| Regex::new(p).ok());
    collect_all_recursive(
        dir,
        &mut files,
        search_pattern,
        excluding_regex.as_ref(),
        min_size,
        max_size,
        equal_size,
        min_age_seconds,
        max_age_seconds,
        empty_only,
        content_pattern,
    );

    if let Some(sort_criteria) = sort_by {
        match sort_criteria {
            SortBy::Name => files.sort_by(|a, b| a.name.cmp(&b.name)),
            SortBy::Size => files.sort_by(|a, b| b.size.cmp(&a.size)),
            SortBy::Date => files.sort_by(|a, b| {
                let a_date = a.modified.as_ref().map(|s| s.as_str()).unwrap_or("");
                let b_date = b.modified.as_ref().map(|s| s.as_str()).unwrap_or("");
                b_date.cmp(a_date)
            }),
        }
    } else {
        files.sort_by(|a, b| a.name.cmp(&b.name));
    }

    files
}
