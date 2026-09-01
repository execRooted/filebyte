# filebyte

A CLI tool to list files and directories with intelligent size formatting, advanced filtering and file analysis. Made in Rust

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)


--- 

If you found this project interesting and want to help me out, buy me a coffee :)

[![Buy Me a Coffee](https://img.shields.io/badge/Buy%20Me%20a%20Coffee-%23FFDD00?style=for-the-badge&logo=buy-me-a-coffee&logoColor=black)](https://buymeacoffee.com/execrooted)


---

## Aliases

You can invoke the tool as any of the following:

- `filebyte` — full name
- `fbt` — short alias

The commands below work identically with any of these names. For example:

```bash
fbt file
filebyte file 
```


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
**!!! Note: I don't use Arch Linux anymore. I will keep publishing to the AUR but I won't test the packages. If you find a problem, please don't hesitate to contact me**

### Windows

1. Install Rust from https://www.rust-lang.org/tools/install
2. Clone the repository and run `install.bat`:

```powershell
git clone https://github.com/execRooted/filebyte.git
cd filebyte
.\install.bat
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

### Windows

```powershell
cd filebyte
.\uninstall.bat
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

# Analyze a specific file in detail
filebyte -f src/main.rs

# Analyze a directory's metadata
filebyte -d /home/user

# Find duplicate files
filebyte --duplicates

# Find true duplicates by content hash (SHA-256, slower but accurate)
filebyte --duplicates --content-dups

# Find true duplicates using MD5 for faster hashing
filebyte --duplicates --content-dups --hash md5
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

Directory-based prompts in the interactive menu display the current working directory and default to it when pressing Enter.

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
| `--content-dups` | | Verify duplicates by content hash instead of size only (true duplicates) |
| `--hash <ALGORITHM>` | | Hash algorithm for content-based deduplication (`sha256` or `md5`) |
| `--export <FILE>` | | Export results to JSON/CSV |
| `--file <FILE>` | `-f` | Analyze a specific file |
| `--directory <DIR>` | `-d` | Analyze a directory as a whole |
| `--recursive` | `-r` | Enable recursive searching and analysis |
| `--whole` | `-w` | Analyze the path as a whole (auto-detects if file or directory) |
| `--interactive` | `-i` | Enable interactive menu mode |
| `--lines` | `-l` | Count lines in files |
| `--preview [MODE]` | `-P` | Preview file contents (`N`, `f/N`, or `l/N` for first/last N lines) |
| `--exclude-dirs` | `-X` | Exclude all directories from results |

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

# Count lines in all files in current directory
filebyte -l

# Count lines recursively, excluding target directories
filebyte -lr -x target

# Exclude all directories from output
filebyte -X

# Recursively search for a file by name
filebyte -r -f filename

# Gets the file info for foo and counts the lines of the bar file
filebyte -f foo -l bar

# Gets the info for foo and bar
filebyte foo bar

# Gets the info for foo and bar directory
filebyte foo bar/

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

# Analyze path as whole (auto-detects file or directory)
filebyte -w /path/to/something

# Search with full paths shown
filebyte -r -e kilo

# Preview file contents (first/last N lines)
filebyte --preview 20 important.txt
filebyte -P 5 src/main.rs
filebyte -Pf notes.txt
filebyte -Pl notes.txt
filebyte -Pf 20 notes.txt
filebyte -Pl 20 notes.txt
filebyte -Pf20 notes.txt
filebyte -Pl20 notes.txt
filebyte -P f:5 notes.txt
filebyte -P l:5 notes.txt
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
