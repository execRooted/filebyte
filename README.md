# filebyte


[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A powerful, colorful CLI tool to list files and directories with intelligent size formatting, advanced filtering, and comprehensive file analysis. Built with Rust for speed and reliability.

##  Features

-  **Fast & Efficient**: Written in Rust with optimized file system operations
-  **Smart Size Display**: Automatically chooses appropriate units (B, KB, MB, GB, TB)
-  **Advanced Filtering**: Regex-based search and exclusion patterns
-  **File Statistics**: Type detection, size analysis, and detailed metadata
-  **Disk Analysis**: View disk usage and manage storage across mount points
-  **Export Support**: Export results to JSON or CSV formats
-  **Duplicate Detection**: Find and analyze duplicate files
-  **Comprehensive Properties**: Creation/modification dates, permissions, and more

##  Installation

### Cargo (Recommended)

```bash
cargo install filebyte
```

### Automated Installation

1. Clone the repository:
```bash
git clone https://github.com/execRooted/filebyte.git
cd filebyte
```

2. Run the installer:
```bash
sudo ./install.sh
```

The installer will automatically:
- Install Rust if not present
- Build filebyte with optimizations
- Install it system-wide to `/usr/local/bin/filebyte`

### Arch Linux (AUR)

```bash
yay -S filebyte
# Or any other AUR helper
```

### Manual Build

```bash
git clone https://github.com/execRooted/filebyte.git
cd filebyte
cargo build --release
# Binary will be available at target/release/filebyte
```

## Uninstallation

```bash
cd filebyte
```
```
sudo ./uninstall.sh
```

### Arch Linux

```
yay -R filebyte
# Or any AUR helper
```

##  Usage

### Basic Usage

```bash
# List files in current directory
filebyte

# List files in specific directory
filebyte /home/user/Documents

# Show directory tree
filebyte --tree
```

### Size Formatting

```bash
# Auto-detect appropriate units (default)
filebyte

# Force specific units
filebyte --size mb          # Megabytes
filebyte --size gb          # Gigabytes
filebyte --size b           # Bytes
```

### Advanced Filtering

```bash
# Search for specific files
filebyte --search "\.rs$"           # Find Rust files
filebyte --search "config"          # Find files containing "config"

# Exclude files
filebyte --excluding "^\."          # Hide hidden files
filebyte --excluding "temp"         # Exclude temp files

# Combine search and exclusion
filebyte --search "\.txt$" --excluding "old"
```

### File Analysis

```bash
# Show detailed properties
filebyte --properties

# Analyze all files recursively
filebyte --properties-all

# Find duplicate files
filebyte --duplicates
```

### Disk Operations

```bash
# List all disks
filebyte --disk list

# Analyze specific disk
filebyte --disk /dev/sda1

# Disk info with custom size units
filebyte --disk list --size gb
```

### Sorting & Export

```bash
# Sort by different criteria
filebyte --sort-by size     # Largest files first
filebyte --sort-by date     # Newest files first
filebyte --sort-by name     # Alphabetical

# Export results
filebyte --export results.json
filebyte --export analysis.csv
```

## 📋 Command Line Options

| Option | Short | Description |
|--------|-------|-------------|
| `--version` | `-v` | Show version information |
| `--help` | `-h` | Show help information |
| `--directory <DIR>` | `-d` | Specify directory to list |
| `--file <FILE>` | `-f` | Show properties of specific file |
| `--size <UNIT>` | `-s` | Size unit (auto, b/bytes, kb/kilobytes, mb/megabytes, gb/gigabytes, tb/terabytes) |
| `--tree` | `-t` | Show directory tree |
| `--properties` | `-p` | Show file properties |
| `--properties-all` | | Show properties for all files recursively |
| `--no-color` | | Disable colored output |
| `--disk <DISK>` | `-m` | Disk operations ('list' or specific disk name) |
| `--search <PATTERN>` | `-e` | Search files using regex pattern |
| `--excluding <PATTERN>` | `-x` | Exclude files matching regex pattern |
| `--sort-by <CRITERIA>` | | Sort by: name, size, date |
| `--duplicates` | | Find duplicate files |
| `--export <FILE>` | | Export results to JSON/CSV |

## 🎯 Examples

### Everyday Usage
```bash
# Quick directory overview
filebyte

# Find large files
filebyte --sort-by size --size mb

# Analyze disk usage
filebyte --disk list --size gb

# Find all PDFs
filebyte --search "\.pdf$"
```

### Advanced Analysis
```bash
# Comprehensive file analysis
filebyte --properties-all --export analysis.json

# Find and sort duplicates by size
filebyte --duplicates --sort-by size

# Exclude system files and sort by date
filebyte --excluding "^\." --sort-by date
```

### Power User Tips
```bash
# Monitor large directories
filebyte /var/log --size mb --sort-by size

# Find recently modified config files
filebyte --search "config" --sort-by date --properties

# Disk space analysis
filebyte --disk list --size gb | head -10
```


##  Output Features

- **Directories first**: Always listed before files for better navigation
- **Colored output**: Intuitive color coding (directories=blue, files=green, sizes=cyan)
- **Smart sizing**: Automatically chooses appropriate units
- **File type detection**: MIME type identification
- **Timestamps**: Creation and modification dates
- **Permissions**: Read/write access indicators

## 🛠️ Development

### Prerequisites
- Rust 1.70 or higher
- Cargo package manager

### Building from Source
```bash
git clone https://github.com/execRooted/filebyte.git
cd filebyte
cargo build --release
```

### Running Tests
```bash
cargo test
```

### Contributing
1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

##  License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

**Made by execRooted**