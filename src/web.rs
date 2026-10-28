use crate::collect::{collect_files_extended, collect_files_recursive_extended};
use crate::types::{FileInfo, SizeUnit};
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::thread;

#[derive(Serialize)]
struct FileEntry {
    name: String,
    path: String,
    size: u64,
    size_human: String,
    file_type: String,
    is_directory: bool,
    created: Option<String>,
    modified: Option<String>,
}

#[derive(Serialize)]
struct ApiResponse<T> {
    success: bool,
    data: T,
}

fn get_content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase()).as_deref() {
        Some("html") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "application/javascript; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("svg") => "image/svg+xml",
        Some("ico") => "image/x-icon",
        Some("txt") => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

fn find_web_root() -> PathBuf {
    // Look for web assets in order: current dir, next to binary, or /usr/local/share/filebyte/web/
    if Path::new("./web").exists() {
        return PathBuf::from("./web");
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join("web");
            if candidate.exists() {
                return candidate;
            }
            let candidate = dir.join("../share/filebyte/web");
            if candidate.exists() {
                return candidate;
            }
        }
    }
    if Path::new("/usr/local/share/filebyte/web").exists() {
        return PathBuf::from("/usr/local/share/filebyte/web");
    }
    // Fallback to current dir
    PathBuf::from("./web")
}

fn file_info_to_entry(f: &FileInfo) -> FileEntry {
    FileEntry {
        name: f.name.clone(),
        path: f.path.clone(),
        size: f.size,
        size_human: f.size_human.clone(),
        file_type: f.file_type.clone(),
        is_directory: f.is_directory,
        created: f.created.clone(),
        modified: f.modified.clone(),
    }
}

fn handle_request(mut stream: TcpStream, web_root: PathBuf) {
    let mut buffer = [0; 4096];
    if let Err(_) = stream.read(&mut buffer) {
        return;
    }
    let request = String::from_utf8_lossy(&buffer);
    let mut lines = request.lines();
    let first_line = match lines.next() {
        Some(l) => l,
        None => return,
    };
    let mut parts = first_line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let path = parts.next().unwrap_or("/");
    
    // Only handle GET
    if method != "GET" {
        let response = "HTTP/1.1 405 Method Not Allowed\r\nContent-Type: text/plain\r\n\r\nMethod Not Allowed";
        let _ = stream.write_all(response.as_bytes());
        return;
    }
    
    // Parse query parameters
    let (base_path, query) = if let Some(idx) = path.find('?') {
        (&path[..idx], &path[idx + 1..])
    } else {
        (path, "")
    };
    
    let mut query_params: HashMap<String, String> = HashMap::new();
    for pair in query.split('&') {
        if let Some(eq) = pair.find('=') {
            let key = urlencoding::decode(&pair[..eq]).unwrap_or_default().to_string();
            let value = urlencoding::decode(&pair[eq + 1..]).unwrap_or_default().to_string();
            query_params.insert(key, value);
        }
    }
    
    // API routes
    if base_path.starts_with("/api/") {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let json_response = match base_path {
            "/api/list" => {
                let rel_path = query_params.get("path").map(|s| s.as_str()).unwrap_or(".");
                let recursive = query_params.get("recursive").map(|s| s == "true").unwrap_or(false);
                let target_path = if rel_path.is_empty() || rel_path == "." {
                    cwd.clone()
                } else {
                    cwd.join(rel_path)
                };
                if target_path.is_dir() {
                    let files = if recursive {
                        collect_files_recursive_extended(&target_path, None, None, None, None, false, false, None, None, None, None, None, None, false, None)
                    } else {
                        collect_files_extended(&target_path, None, None, None, None, false, false, None, None, None, None, None, false, None)
                    };
                    let entries: Vec<FileEntry> = files.iter().map(file_info_to_entry).collect();
                    serde_json::to_string(&ApiResponse { success: true, data: entries }).unwrap_or("{}".to_string())
                } else {
                    serde_json::to_string(&ApiResponse { success: false, data: Vec::<FileEntry>::new() }).unwrap_or("{}".to_string())
                }
            }
            "/api/file" => {
                let rel_path = query_params.get("path").cloned().unwrap_or_default();
                let target_path = cwd.join(&rel_path);
                if let Ok(metadata) = fs::metadata(&target_path) {
                    let file_type = if metadata.is_dir() { "directory".to_string() } else {
                        infer::get_from_path(&target_path)
                            .ok()
                            .flatten()
                            .map(|k| k.mime_type().to_string())
                            .unwrap_or_else(|| "unknown".to_string())
                    };
                    let entry = FileEntry {
                        name: target_path.file_name().unwrap_or_default().to_string_lossy().to_string(),
                        path: target_path.to_string_lossy().to_string(),
                        size: metadata.len(),
                        size_human: SizeUnit::auto_format_size(metadata.len()),
                        file_type,
                        is_directory: metadata.is_dir(),
                        created: metadata.created().ok().map(|t| chrono::DateTime::<chrono::Utc>::from(t).format("%Y-%m-%d %H:%M:%S UTC").to_string()),
                        modified: metadata.modified().ok().map(|t| chrono::DateTime::<chrono::Utc>::from(t).format("%Y-%m-%d %H:%M:%S UTC").to_string()),
                    };
                    serde_json::to_string(&ApiResponse { success: true, data: entry }).unwrap_or("{}".to_string())
                } else {
                    serde_json::to_string(&ApiResponse { success: false, data: FileEntry { name: "".to_string(), path: "".to_string(), size: 0, size_human: "0 B".to_string(), file_type: "unknown".to_string(), is_directory: false, created: None, modified: None } }).unwrap_or("{}".to_string())
                }
            }
            "/api/tree" => {
                let rel_path = query_params.get("path").map(|s| s.as_str()).unwrap_or(".");
                let target_path = if rel_path.is_empty() || rel_path == "." { cwd.clone() } else { cwd.join(rel_path) };
                let tree = crate::tree::build_tree(&target_path, 0, 10);
                serde_json::to_string(&ApiResponse { success: true, data: tree }).unwrap_or("{}".to_string())
            }
            "/api/duplicates" => {
                let rel_path = query_params.get("path").map(|s| s.as_str()).unwrap_or(".");
                let target_path = if rel_path.is_empty() || rel_path == "." { cwd.clone() } else { cwd.join(rel_path) };
                let files = collect_files_extended(&target_path, None, None, None, None, false, false, None, None, None, None, None, false, None);
                let mut size_map: HashMap<u64, Vec<FileInfo>> = HashMap::new();
                for f in files {
                    if !f.is_directory {
                        size_map.entry(f.size).or_default().push(f);
                    }
                }
                let mut duplicates: Vec<Vec<FileEntry>> = Vec::new();
                for (_size, group) in size_map {
                    if group.len() > 1 {
                        duplicates.push(group.iter().map(file_info_to_entry).collect());
                    }
                }
                serde_json::to_string(&ApiResponse { success: true, data: duplicates }).unwrap_or("{}".to_string())
            }
            "/api/stats" => {
                let rel_path = query_params.get("path").map(|s| s.as_str()).unwrap_or(".");
                let target_path = if rel_path.is_empty() || rel_path == "." { cwd.clone() } else { cwd.join(rel_path) };
                let files = collect_files_extended(&target_path, None, None, None, None, false, false, None, None, None, None, None, false, None);
                let total_files = files.iter().filter(|f| !f.is_directory).count();
                let total_dirs = files.iter().filter(|f| f.is_directory).count();
                let total_size: u64 = files.iter().filter(|f| !f.is_directory).map(|f| f.size).sum();
                #[derive(Serialize)]
                struct Stats {
                    total_files: usize,
                    total_directories: usize,
                    total_size: u64,
                    total_size_human: String,
                }
                let stats = Stats { total_files, total_directories: total_dirs, total_size, total_size_human: SizeUnit::auto_format_size(total_size) };
                serde_json::to_string(&ApiResponse { success: true, data: stats }).unwrap_or("{}".to_string())
            }
            _ => {
                serde_json::to_string(&ApiResponse { success: false, data: Vec::<FileEntry>::new() }).unwrap_or("{}".to_string())
            }
        };
        let response = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json; charset=utf-8\r\nAccess-Control-Allow-Origin: *\r\n\r\n{}", json_response);
        let _ = stream.write_all(response.as_bytes());
        return;
    }
    
    // Serve static files
    let requested_path = if base_path == "/" { "/index.html" } else { base_path };
    let mut file_path = web_root.clone();
    // Remove leading slash
    let clean_path = requested_path.trim_start_matches('/');
    file_path.push(clean_path);
    
    if file_path.exists() && file_path.is_file() {
        if let Ok(mut file) = fs::File::open(&file_path) {
            let mut contents = Vec::new();
            if file.read_to_end(&mut contents).is_ok() {
                let content_type = get_content_type(&file_path);
                let header = format!("HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\n\r\n", content_type, contents.len());
                let _ = stream.write_all(header.as_bytes());
                let _ = stream.write_all(&contents);
                return;
            }
        }
    }
    
    // Fallback to index.html for client-side routing
    let index_path = web_root.join("index.html");
    if let Ok(mut file) = fs::File::open(&index_path) {
        let mut contents = Vec::new();
        if file.read_to_end(&mut contents).is_ok() {
            let header = format!("HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\n\r\n", contents.len());
            let _ = stream.write_all(header.as_bytes());
            let _ = stream.write_all(&contents);
            return;
        }
    }
    
    let response = "HTTP/1.1 404 Not Found\r\nContent-Type: text/plain\r\n\r\n404 Not Found";
    let _ = stream.write_all(response.as_bytes());
}

pub fn start_server(port: u16) -> std::io::Result<()> {
    let web_root = find_web_root();
    let addr = format!("127.0.0.1:{}", port);
    let listener = TcpListener::bind(&addr)?;
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let web_root_clone = web_root.clone();
                thread::spawn(move || {
                    handle_request(stream, web_root_clone);
                });
            }
            Err(_) => continue,
        }
    }
    Ok(())
}
