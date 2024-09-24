# dos2unix-r

`dos2unix-r` is a Rust implementation of the classic `dos2unix` utility. It is meant to mimic the original `dos2unix` utility in behavior and options, but is implemented in Rust for better performance and safety. This is a toy project to demonstrate the feasibility of a Rust-based `dos2unix` utility and is not intended to replace the original `dos2unix` utility.

## Features

- **Convert Line Endings:**
  - **DOS to Unix (CRLF to LF):** `dos2unix.exe`
  - **Unix to DOS (LF to CRLF):** `unix2dos.exe`
  - **Mac to Unix or DOS (CR to LF/CRLF):** Support for Mac line endings.

- **File Encoding Detection and Handling:**
  - Supports various encodings including UTF-8, UTF-16LE, UTF-16BE, and ISO-8859-1.
  
- **File Processing:**
  - Convert single files or recursively process directories.
  - Preserve file modification times.
  - Create backups of original files.
  
- **Byte Order Marks (BOMs):**
  - Handle and preserve BOMs as needed.
  
- **Additional Features:**
  - Add missing end-of-line at the end of files.
  - Force conversion of binary files.
  - Verbose output with multiple verbosity levels.
  - Efficient processing for large files.

## Installation

To install `dos2unix-r`, you need to have [Rust](https://www.rust-lang.org/) and Cargo installed on your system.

1. **Clone the Repository:**
   ```bash
   git clone https://github.com/whit3rabbit/dos2unix-r
   cd dos2unix-r
   ```

2. **Build the Project:**
   ```bash
   cargo build --release
   ```

3. **Locate the Binaries:**
   The compiled binaries will be available in `target/release/` as `dos2unix.exe` and `unix2dos.exe`. You can also download precompiled binaries from the [releases page](https://github.com/whit3rabbit/dos2unix-r/releases).

## Usage

### dos2unix.exe

Convert text files with DOS or Mac line endings to Unix line endings.

```bash
dos2unix.exe [options] [FILE ...] [-n INFILE OUTFILE]
```

### unix2dos.exe

Convert text files with Unix or Mac line endings to DOS line endings.

```bash
unix2dos.exe [options] [FILE ...] [-n INFILE OUTFILE]
```

### Common Usage Examples

1. **Convert a File to Unix Line Endings (Default Behavior):**
   ```bash
   dos2unix.exe file.txt
   ```

2. **Convert a File to DOS Line Endings:**
   ```bash
   unix2dos.exe file.txt
   ```

3. **Recursively Process a Directory, Converting All Files to Unix Line Endings:**
   ```bash
   dos2unix.exe --recursive directory/
   ```

4. **Print File Information Without Converting:**
   ```bash
   dos2unix.exe --info file.txt
   ```

5. **Convert a File to Unix Line Endings, Creating a Backup and Preserving the Modification Time:**
   ```bash
   dos2unix.exe --backup --keep-date file.txt
   ```

6. **Specify a New Output File:**
   ```bash
   dos2unix.exe --newfile input.txt output.txt
   ```

7. **Force Conversion of Binary Files:**
   ```bash
   dos2unix.exe --force binaryfile.bin
   ```

## Command-line Options

### dos2unix.exe

```
Usage: dos2unix.exe [options] [FILE ...] [-n INFILE OUTFILE]

Converts text files with DOS or Mac line endings to Unix line endings.

Options:
  -b, --backup             Make a backup of each file.
  -f, --force              Force conversion of binary files.
  -k, --keep-bom           Keep the Byte Order Mark (BOM).
  -m, --mac                Convert Mac line endings (CR) to Unix (LF).
  -o, --oldfile            Overwrite original file (default behavior).
  -n, --newfile <OUTFILE>  Specify new output file.
      --add-eol            Add missing end-of-line at end of file.
  -v, --verbose            Increase verbosity level (can be used multiple times).
      --help               Display this help and exit.
      --version            Output version information and exit.
```

### unix2dos.exe

```
Usage: unix2dos.exe [options] [FILE ...] [-n INFILE OUTFILE]

Converts text files with Unix or Mac line endings to DOS line endings.

Options:
  -b, --backup             Make a backup of each file.
  -f, --force              Force conversion of binary files.
  -k, --keep-bom           Keep the Byte Order Mark (BOM).
  -m, --mac                Convert Mac line endings (CR) to DOS (CRLF).
  -o, --oldfile            Overwrite original file (default behavior).
  -n, --newfile <OUTFILE>  Specify new output file.
      --add-eol            Add missing end-of-line at end of file.
  -v, --verbose            Increase verbosity level (can be used multiple times).
      --help               Display this help and exit.
      --version            Output version information and exit.
```

## Detailed Command-line Options

### General Options for Both Executables

- **`-b, --backup`**  
  Create a backup of each original file before conversion.

- **`-f, --force`**  
  Force the conversion of binary files. Use with caution as it may corrupt binary data.

- **`-k, --keep-bom`**  
  Preserve the Byte Order Mark (BOM) if present in the file.

- **`-m, --mac`**  
  Handle Mac-style line endings (CR) specifically during conversion.

- **`-o, --oldfile`**  
  Overwrite the original file with the converted content. This is the default behavior.

- **`-n, --newfile <OUTFILE>`**  
  Specify a new output file instead of overwriting the original.

- **`--add-eol`**  
  Add a missing end-of-line character at the end of the file if it's absent.

- **`-v, --verbose`**  
  Increase the verbosity of the output. Can be used multiple times for more detailed logs.

- **`--help`**  
  Display the help message and exit.

- **`--version`**  
  Output the version information and exit.

### Specific Options

Currently, the executables share the same set of options with behavior tailored to their specific conversion direction (DOS to Unix or Unix to DOS).

## Examples

### Converting Multiple Files

```bash
dos2unix.exe file1.txt file2.txt
unix2dos.exe file3.txt file4.txt
```

### Using New Output Files

```bash
dos2unix.exe -n input.txt output.txt
unix2dos.exe -n input.txt output_dos.txt
```

### Recursive Directory Conversion

```bash
dos2unix.exe -b -v --recursive ./my_directory/
unix2dos.exe -b -v --recursive ./my_directory/
```

## Acknowledgments

- This project is inspired by the original `dos2unix` utility.