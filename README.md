# dos2unix-r

`dos2unix-r` is a Rust implementation of the classic dos2unix utility, with additional features for file encoding detection and handling. This tool allows you to convert text files between DOS/Windows and Unix line endings, as well as provide information about file encodings and line ending types.

## Features

- Convert between DOS (CRLF) and Unix (LF) line endings (Unix is the default)
- Detect and handle various file encodings (UTF-8, UTF-16LE, UTF-16BE, ISO-8859-1)
- Process single files or recursively handle directories
- Preserve file modification times
- Create backups of original files
- Handle Byte Order Marks (BOMs)
- Provide detailed file information (encoding, BOM presence, line ending counts)
- Large file support with efficient processing

## Installation

To install `dos2unix-r`, you need to have Rust and Cargo installed on your system. If you don't have them installed, you can get them from [https://www.rust-lang.org/](https://www.rust-lang.org/).

Once you have Rust and Cargo, you can build the project:

```
git clone https://github.com/whit3rabbit/dos2unix-r
cd dos2unix-r
cargo build --release
```

The compiled binary will be available in `target/release/dos2unix` or on the releases page to download.

## Usage

Here are some common usage examples:

1. Convert a file to Unix line endings (default behavior):
   ```
   dos2unix file.txt
   ```

2. Convert a file to DOS line endings:
   ```
   dos2unix -d file.txt
   ```

3. Recursively process a directory, converting all files to Unix line endings:
   ```
   dos2unix -r directory/
   ```

4. Print file information without converting:
   ```
   dos2unix -i file.txt
   ```

5. Convert a file to Unix line endings, creating a backup and preserving the modification time:
   ```
   dos2unix -b -p file.txt
   ```

## Command-line Options

```
dos2unix 0.1.0
Convert line endings between DOS and Unix formats

USAGE:
    dos2unix [FLAGS] [OPTIONS] [input]...

FLAGS:
    -b, --backup             Create backup of original file
        --follow-symlinks    Follow symbolic links
    -f, --force              Force conversion of binary files
    -h, --help               Prints help information
    -i, --info               Print file information
        --keep-bom           Add or keep Byte Order Mark (BOM)
    -p, --keep-date          Keep original file modification time
    -q, --quiet              Quiet mode
    -r, --recursive          Recursively process directories
        --remove-bom         Remove Byte Order Mark (BOM)
    -d, --to-dos             Convert to DOS line endings (CRLF)
    -V, --version            Prints version information
    -v, --verbose            Verbose output

OPTIONS:
        --from-encoding <from-encoding>    Specify input encoding (utf8, utf16le, utf16be, iso-8859-1) [default: auto]

ARGS:
    <input>...    Input files or directories
```

Note: The default behavior is now to convert to Unix line endings (LF) without needing to specify `-u` or `--to-unix`.

## Acknowledgments

- This project is inspired by the original dos2unix utility.
