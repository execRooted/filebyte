use std::fs;
use std::path::Path;

/// Check if a file or directory can be deleted based on parent directory permissions
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

/// Get the size of a file or the total size of a directory
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

/// Format Unix permissions in either detailed or compact format
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
