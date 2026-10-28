let currentPath = '.';
let historyStack = ['.'];
let currentView = 'list';
let recursive = false;
let dirInfoMode = false;
let allFiles = [];

function getFileIcon(file) {
    if (file.is_directory) return '<i class="fas fa-folder" style="color: #ffd700;"></i>';
    const ext = file.name.split('.').pop().toLowerCase();
    const type = file.file_type.toLowerCase();
    
    if (type.includes('image')) return '<i class="fas fa-file-image" style="color: #e91e63;"></i>';
    if (type.includes('video')) return '<i class="fas fa-file-video" style="color: #9c27b0;"></i>';
    if (type.includes('audio')) return '<i class="fas fa-file-audio" style="color: #ff9800;"></i>';
    if (type.includes('pdf')) return '<i class="fas fa-file-pdf" style="color: #f44336;"></i>';
    if (type.includes('zip') || type.includes('tar') || type.includes('compressed')) return '<i class="fas fa-file-archive" style="color: #795548;"></i>';
    if (ext === 'js' || ext === 'ts' || ext === 'py' || ext === 'rs' || ext === 'java' || ext === 'cpp' || ext === 'c') 
        return '<i class="fas fa-file-code" style="color: #4caf50;"></i>';
    if (ext === 'txt' || ext === 'md') return '<i class="fas fa-file-alt" style="color: #607d8b;"></i>';
    return '<i class="fas fa-file" style="color: #2196f3;"></i>';
}

function updateBreadcrumb() {
    const breadcrumb = document.getElementById('breadcrumb');
    breadcrumb.innerHTML = '';
    
    const parts = currentPath === '.' ? ['.'] : currentPath.replace(/\\/g, '/').split('/').filter(p => p);
    if (currentPath === '.' || parts.length === 0) {
        const span = document.createElement('span');
        span.innerHTML = '<i class="fas fa-home"></i> Home';
        span.onclick = () => navigateTo('.');
        breadcrumb.appendChild(span);
        return;
    }
    
    // Build breadcrumb
    let accumulatedPath = '';
    // Home
    const homeSpan = document.createElement('span');
    homeSpan.innerHTML = '<i class="fas fa-home"></i> Home';
    homeSpan.onclick = () => navigateTo('.');
    breadcrumb.appendChild(homeSpan);
    
    parts.forEach((part, idx) => {
        const sep = document.createElement('span');
        sep.className = 'separator';
        sep.textContent = ' / ';
        breadcrumb.appendChild(sep);
        
        if (idx === 0 && currentPath.startsWith('/')) {
            accumulatedPath = '/' + part;
        } else if (accumulatedPath === '' || accumulatedPath === '.') {
            accumulatedPath = part;
        } else {
            accumulatedPath = accumulatedPath + '/' + part;
        }
        
        const span = document.createElement('span');
        span.textContent = part;
        if (idx === parts.length - 1) {
            span.style.fontWeight = 'bold';
        } else {
            span.onclick = () => navigateTo(accumulatedPath);
        }
        breadcrumb.appendChild(span);
    });
}

function navigateTo(path) {
    currentPath = path;
    if (historyStack[historyStack.length - 1] !== path) {
        historyStack.push(path);
    }
    updateBreadcrumb();
    loadFiles();
    updateURL();
}

function goBack() {
    if (historyStack.length > 1) {
        historyStack.pop();
        currentPath = historyStack[historyStack.length - 1];
        updateBreadcrumb();
        loadFiles();
        updateURL();
    }
}

function updateURL() {
    const url = new URL(window.location);
    if (currentPath === '.') {
        url.searchParams.delete('path');
    } else {
        url.searchParams.set('path', currentPath);
    }
    history.pushState({}, '', url);
}

async function loadFiles() {
    const searchTerm = document.getElementById('searchInput').value.toLowerCase();
    const fileList = document.getElementById('fileList');
    fileList.innerHTML = '<div style="text-align: center; padding: 40px;"><i class="fas fa-spinner fa-spin"></i> Loading...</div>';
    
    try {
        const url = `/api/list?path=${encodeURIComponent(currentPath)}&recursive=${recursive}`;
        const response = await fetch(url);
        const data = await response.json();
        
        if (data.success) {
            allFiles = data.data;
            displayFiles(allFiles, searchTerm);
            loadStats();
        } else {
            fileList.innerHTML = '<div style="text-align: center; padding: 40px; color: red;">Error loading files</div>';
        }
    } catch (e) {
        fileList.innerHTML = '<div style="text-align: center; padding: 40px; color: red;">Error: ' + e.message + '</div>';
    }
}

function displayFiles(files, searchTerm = '') {
    const fileList = document.getElementById('fileList');
    fileList.innerHTML = '';
    
    let filtered = files.filter(f => {
        if (!searchTerm) return true;
        return f.name.toLowerCase().includes(searchTerm);
    });
    
    // Sort: directories first, then by name
    filtered.sort((a, b) => {
        if (a.is_directory && !b.is_directory) return -1;
        if (!a.is_directory && b.is_directory) return 1;
        return a.name.localeCompare(b.name);
    });
    
    if (filtered.length === 0) {
        fileList.innerHTML = '<div style="text-align: center; padding: 40px; color: #666;">No files found</div>';
        return;
    }
    
    filtered.forEach(file => {
        const item = document.createElement('div');
        item.className = 'file-item';
        item.innerHTML = `
            <div class="file-icon">${getFileIcon(file)}</div>
            <div class="file-name" title="${file.name}">${file.name}</div>
            <div class="file-size">${file.size_human}</div>
            <div class="file-type">${file.file_type}</div>
            <div class="file-date">${file.modified || file.created || '-'}</div>
        `;
        
        item.onclick = () => {
            if (file.is_directory) {
                if (dirInfoMode) {
                    showDirectoryInfo(file);
                } else {
                    navigateTo(file.path === '.' ? file.name : (currentPath === '.' ? file.name : currentPath + '/' + file.name));
                }
            } else {
                showFileInfo(file);
            }
        };
        
        fileList.appendChild(item);
    });
}

async function loadStats() {
    try {
        const url = `/api/stats?path=${encodeURIComponent(currentPath)}`;
        const response = await fetch(url);
        const data = await response.json();
        if (data.success) {
            const stats = document.getElementById('stats');
            stats.innerHTML = `<i class="fas fa-chart-bar"></i> ${data.data.total_files} files, ${data.data.total_directories} directories, ${data.data.total_size_human} total`;
        }
    } catch (e) {
        // ignore
    }
}

async function showDirectoryInfo(file) {
    try {
        const url = `/api/stats?path=${encodeURIComponent(file.path)}`;
        const response = await fetch(url);
        const data = await response.json();
        
        document.getElementById('modalTitle').innerHTML = getFileIcon(file) + ' ' + file.name + ' (Directory)';
        if (data.success) {
            document.getElementById('modalBody').innerHTML = `
                <div class="modal-row">
                    <div class="modal-label">Name:</div>
                    <div class="modal-value">${file.name}</div>
                </div>
                <div class="modal-row">
                    <div class="modal-label">Path:</div>
                    <div class="modal-value">${file.path}</div>
                </div>
                <div class="modal-row">
                    <div class="modal-label">Files:</div>
                    <div class="modal-value">${data.data.total_files}</div>
                </div>
                <div class="modal-row">
                    <div class="modal-label">Directories:</div>
                    <div class="modal-value">${data.data.total_directories}</div>
                </div>
                <div class="modal-row">
                    <div class="modal-label">Total Size:</div>
                    <div class="modal-value">${data.data.total_size_human} (${data.data.total_size} bytes)</div>
                </div>
                <div class="modal-row">
                    <div class="modal-label">Modified:</div>
                    <div class="modal-value">${file.modified || file.created || 'N/A'}</div>
                </div>
            `;
        } else {
            document.getElementById('modalBody').innerHTML = `
                <div class="modal-row">
                    <div class="modal-label">Name:</div>
                    <div class="modal-value">${file.name}</div>
                </div>
                <div class="modal-row">
                    <div class="modal-label">Path:</div>
                    <div class="modal-value">${file.path}</div>
                </div>
                <div class="modal-row">
                    <div class="modal-label">Type:</div>
                    <div class="modal-value">Directory</div>
                </div>
                <div class="modal-row">
                    <div class="modal-label">Size:</div>
                    <div class="modal-value">${file.size_human}</div>
                </div>
            `;
        }
        document.getElementById('fileModal').classList.add('active');
    } catch (e) {
        alert('Error loading directory info');
    }
}

async function showFileInfo(file) {
    try {
        const url = `/api/file?path=${encodeURIComponent(file.path)}`;
        const response = await fetch(url);
        const data = await response.json();
        const f = data.success ? data.data : file;
        
        document.getElementById('modalTitle').innerHTML = getFileIcon(f) + ' ' + f.name;
        document.getElementById('modalBody').innerHTML = `
            <div class="modal-row">
                <div class="modal-label">Name:</div>
                <div class="modal-value">${f.name}</div>
            </div>
            <div class="modal-row">
                <div class="modal-label">Path:</div>
                <div class="modal-value">${f.path}</div>
            </div>
            <div class="modal-row">
                <div class="modal-label">Size:</div>
                <div class="modal-value">${f.size_human} (${f.size} bytes)</div>
            </div>
            <div class="modal-row">
                <div class="modal-label">Type:</div>
                <div class="modal-value">${f.file_type}</div>
            </div>
            <div class="modal-row">
                <div class="modal-label">Created:</div>
                <div class="modal-value">${f.created || 'N/A'}</div>
            </div>
            <div class="modal-row">
                <div class="modal-label">Modified:</div>
                <div class="modal-value">${f.modified || 'N/A'}</div>
            </div>
            <div class="modal-row">
                <div class="modal-label">Directory:</div>
                <div class="modal-value">${f.is_directory ? 'Yes' : 'No'}</div>
            </div>
        `;
        document.getElementById('fileModal').classList.add('active');
    } catch (e) {
        alert('Error loading file info');
    }
}

async function showTree() {
    try {
        const url = `/api/tree?path=${encodeURIComponent(currentPath)}`;
        const response = await fetch(url);
        const data = await response.json();
        if (data.success) {
            document.getElementById('treeOutput').textContent = buildTreeString(data.data);
            document.getElementById('treeModal').classList.add('active');
        }
    } catch (e) {
        alert('Error loading tree');
    }
}

function buildTreeString(node, prefix = '', isLast = true) {
    let result = prefix + (isLast ? '└── ' : '├── ') + node.name + '\n';
    if (node.is_directory && node.children) {
        const newPrefix = prefix + (isLast ? '    ' : '│   ');
        node.children.forEach((child, idx) => {
            result += buildTreeString(child, newPrefix, idx === node.children.length - 1);
        });
    }
    return result;
}

async function showDuplicates() {
    try {
        const url = `/api/duplicates?path=${encodeURIComponent(currentPath)}`;
        const response = await fetch(url);
        const data = await response.json();
        const body = document.getElementById('duplicatesBody');
        body.innerHTML = '';
        
        if (data.success && data.data.length > 0) {
            data.data.forEach((group, idx) => {
                const groupDiv = document.createElement('div');
                groupDiv.className = 'duplicate-group';
                groupDiv.innerHTML = `<h3><i class="fas fa-exclamation-triangle"></i> Duplicate Group ${idx + 1} (${group.length} files)</h3>`;
                group.forEach(file => {
                    const item = document.createElement('div');
                    item.className = 'duplicate-item';
                    item.innerHTML = `<strong>${file.name}</strong><br><small>${file.path} (${file.size_human})</small>`;
                    groupDiv.appendChild(item);
                });
                body.appendChild(groupDiv);
            });
        } else {
            body.innerHTML = '<p style="text-align: center; color: #666; padding: 20px;"><i class="fas fa-check-circle" style="color: #4caf50;"></i> No duplicates found</p>';
        }
        document.getElementById('duplicatesModal').classList.add('active');
    } catch (e) {
        alert('Error finding duplicates');
    }
}

function init() {
    // Check URL params
    const urlParams = new URLSearchParams(window.location.search);
    if (urlParams.get('path')) {
        currentPath = urlParams.get('path');
        historyStack = [currentPath];
    }
    
    // View toggle
    document.getElementById('listViewBtn').onclick = () => {
        currentView = 'list';
        document.getElementById('fileList').className = 'file-list list-view';
        document.getElementById('listViewBtn').classList.add('active');
        document.getElementById('gridViewBtn').classList.remove('active');
    };
    document.getElementById('gridViewBtn').onclick = () => {
        currentView = 'grid';
        document.getElementById('fileList').className = 'file-list grid-view';
        document.getElementById('gridViewBtn').classList.add('active');
        document.getElementById('listViewBtn').classList.remove('active');
    };
    
    // Back button
    document.getElementById('backBtn').onclick = goBack;
    
    // Tree button
    document.getElementById('treeBtn').onclick = showTree;
    
    // Copy tree button
    document.getElementById('copyTreeBtn').onclick = () => {
        const treeText = document.getElementById('treeOutput').textContent;
        navigator.clipboard.writeText(treeText).then(() => {
            const btn = document.getElementById('copyTreeBtn');
            const original = btn.innerHTML;
            btn.innerHTML = '<i class="fas fa-check"></i> Copied!';
            setTimeout(() => { btn.innerHTML = original; }, 2000);
        }).catch(() => {
            // Fallback
            const textArea = document.createElement('textarea');
            textArea.value = treeText;
            document.body.appendChild(textArea);
            textArea.select();
            document.execCommand('copy');
            document.body.removeChild(textArea);
            const btn = document.getElementById('copyTreeBtn');
            const original = btn.innerHTML;
            btn.innerHTML = '<i class="fas fa-check"></i> Copied!';
            setTimeout(() => { btn.innerHTML = original; }, 2000);
        });
    };
    
    // Duplicates button
    document.getElementById('duplicatesBtn').onclick = showDuplicates;
    
    // Recursive checkbox
    document.getElementById('recursiveCheck').onchange = (e) => {
        recursive = e.target.checked;
        loadFiles();
    };
    
    // Directory info checkbox
    document.getElementById('dirInfoCheck').onchange = (e) => {
        dirInfoMode = e.target.checked;
    };
    
    // Search
    document.getElementById('searchInput').oninput = () => {
        displayFiles(allFiles, document.getElementById('searchInput').value.toLowerCase());
    };
    
    // Modals
    document.getElementById('closeModal').onclick = () => {
        document.getElementById('fileModal').classList.remove('active');
    };
    document.getElementById('closeTreeModal').onclick = () => {
        document.getElementById('treeModal').classList.remove('active');
    };
    document.getElementById('closeDuplicatesModal').onclick = () => {
        document.getElementById('duplicatesModal').classList.remove('active');
    };
    
    window.onclick = (e) => {
        if (e.target.classList.contains('modal')) {
            e.target.classList.remove('active');
        }
    };
    
    // Browser back/forward
    window.onpopstate = () => {
        const urlParams = new URLSearchParams(window.location.search);
        if (urlParams.get('path')) {
            currentPath = urlParams.get('path');
        } else {
            currentPath = '.';
        }
        if (historyStack[historyStack.length - 1] !== currentPath) {
            historyStack = [currentPath];
        }
        updateBreadcrumb();
        loadFiles();
    };
    
    updateBreadcrumb();
    loadFiles();
}

document.addEventListener('DOMContentLoaded', init);
