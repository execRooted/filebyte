# filebyte

A powerful, colorful CLI tool to list files and directories with intelligent size formatting, advanced filtering, and comprehensive file analysis. Built with Rust for speed and reliability.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

**Latest Version: 1.4.2**

## Features

- **Smart Size Display**: Automatically chooses appropriate units (B, KB, MB, GB, TB)
- **Advanced Filtering**: Regex-based search and exclusion patterns
- **File Statistics**: Type detection, size analysis, and detailed metadata
- **Disk Analysis**: View disk usage and manage storage across mount points
- **Export Support**: Export results to JSON or CSV formats
- **Duplicate Detection**: Find and analyze duplicate files
- **Comprehensive Properties**: Creation/modification dates, permissions, and more
- **File/Directory Analysis**: Dedicated options for analyzing specific files or directories
- **Directory Tree**: With the -t or --tree flag you can make a tree of a directory
- **Interactive Menu**: Launch an interactive menu with `-i` or `--interactive` for easy file operations and bit conversion

## Installation

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
paru -S filebyte
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
paru -R filebyte
# Or any AUR helper
```

## Usage

### Basic Usage

```bash
# List files in current directory
filebyte

# List files in specific directory
filebyte /home/user/Documents

# Show directory tree
filebyte --tree

# Analyze a specific file
filebyte -w /path/to/file.txt

# Analyze a directory as a whole
filebyte -w /path/to/directory
```

### Size Formatting

```bash
# Show permissions and modification dates (default)
filebyte

# Show file sizes in auto-detected units
filebyte -s

# Show file sizes in specific units
filebyte -s mb          # Megabytes
filebyte -s gb          # Gigabytes
filebyte -s b           # Bytes
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
# Show comprehensive analysis for current directory
filebyte -p

# Show detailed properties for specific file
filebyte -p README.md

# Analyze a specific file in detail
filebyte -f src/main.rs

# Analyze a directory's metadata
filebyte -d /home/user

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

### Interactive Menu

```bash
# Launch interactive menu
filebyte -i
filebyte --interactive
```

The interactive menu provides a user-friendly interface with the following options:

| Option | Description |
|--------|-------------|
| 1 | List files in current directory |
| 2 | Analyze a specific file |
| 3 | Analyze a directory |
| 4 | Find duplicate files |
| 5 | Show directory tree |
| 6 | List all disks |
| 7 | Search for files (regex) |
| 8 | Show file type statistics |
| 9 | Bit converter (bits, kbits, mbits, gbits, tbits) |
| 0 | Exit |

**Bit Converter** - Option 9 allows you to convert between bits and bytes:
- Input formats: `1000 bits`, `500 kbits`, `1.5 mbits`, `2 gbits`
- Shows conversions in both bits and bytes formats

The menu automatically clears the screen between operations for a clean interface.

## Command Line Options

| Option | Short | Description |
|--------|-------|-------------|
| `--version` | `-v` | Show version information |
| `--help` | `-h` | Show help information |
| `--size <UNIT>` | `-s` | Show file sizes with specified unit (auto, b/bytes, kb/kilobytes, mb/megabytes, gb/gigabytes, tb/terabytes) |
| `--tree` | `-t` | Show directory tree |
| `--properties` | `-p` | Show comprehensive file/directory analysis |
| `--no-color` | | Disable colored output |
| `--disk <DISK>` | `-m` | Disk operations ('list' or specific disk name) |
| `--search <PATTERN>` | `-e` | Search files using regex pattern |
| `--excluding <PATTERN>` | `-x` | Exclude files matching regex pattern |
| `--sort-by <CRITERIA>` | | Sort by: name, size, date |
| `--duplicates` | | Find duplicate files |
| `--export <FILE>` | | Export results to JSON/CSV |
| `--file <FILE>` | `-f` | Analyze a specific file |
| `--directory <DIR>` | `-d` | Analyze a directory as a whole |
| `--recursive` | `-r` | Enable recursive searching and analysis |
| `--interactive` | `-i` | Enable interactive menu mode |

## Examples

### Everyday Usage
```bash
# Quick directory overview (shows permissions & dates)
filebyte

# Find large files with sizes
filebyte -s --sort-by size

# Analyze disk usage
filebyte --disk list -s gb

# Find all PDFs
filebyte --search "\.pdf$"

# Check a specific file's details
filebyte -f important.txt

# Get directory metadata
filebyte -d /home/user/projects

# Search recursively through directories
filebyte -r --search "\.rs$"

# Recursively exclude hidden files and sort by size
filebyte -r --excluding "^\." --sort-by size
```

### Advanced Analysis
```bash
# Comprehensive file analysis
filebyte -p --export analysis.json

# Find and sort duplicates by size
filebyte --duplicates -s --sort-by size

# Exclude system files and sort by date
filebyte --excluding "^\." --sort-by date

# Recursively analyze entire project structure
filebyte -r -p /home/user/projects

# Find all config files recursively
filebyte -r --search "config" --sort-by date
```

### Power User Tips
```bash
# Monitor large directories
filebyte /var/log -s mb --sort-by size

# Find recently modified config files
filebyte --search "config" --sort-by date -p

# Disk space analysis
filebyte --disk list -s gb | head -10

# Deep analysis of entire filesystem
filebyte -r / -s gb --sort-by size | head -20

# Find all executables recursively
filebyte -r --search "\.(exe|bin|sh)$" --sort-by size

# Quick file analysis - no flags needed!
filebyte important.txt
```


---

***Made by execRooted***
