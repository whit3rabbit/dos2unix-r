use std::env;
use std::ffi::OsString;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

#[cfg(windows)]
use winapi::um::consoleapi::GetConsoleMode;
#[cfg(windows)]
use winapi::um::handleapi::INVALID_HANDLE_VALUE;
#[cfg(windows)]
use winapi::um::processenv::GetStdHandle;
#[cfg(windows)]
use winapi::um::winbase::STD_INPUT_HANDLE;
#[cfg(windows)]
use winapi::um::winnt::HANDLE;

fn print_help(progname: &str) {
    println!("Usage: {} [options] [FILE ...] [-n INFILE OUTFILE]", progname);
    println!("Converts text files with DOS or Mac line endings to Unix line endings.");
    println!("Options:");
    println!("  -b             Make a backup of each file.");
    println!("  -f, --force    Force conversion of binary files.");
    println!("  -k, --keep-bom Keep the Byte Order Mark (BOM).");
    println!("  -m, --mac      Convert Mac line endings (CR) to Unix (LF).");
    println!("  -o, --oldfile  Overwrite original file (default behavior).");
    println!("  -n, --newfile  Specify new output file.");
    println!("      --add-eol  Add missing end-of-line at end of file.");
    println!("  -v, --verbose  Increase verbosity level (can be used multiple times).");
    println!("      --help     Display this help and exit.");
    println!("      --version  Output version information and exit.");
}

fn print_version() {
    println!("dos2unix rust version");
}

fn detect_binary(
    content: &[u8],
    force: bool,
    verbose: usize,
    progname: &str,
) -> io::Result<()> {
    let mut line_number = 1;
    for &byte in content {
        if byte < 32 && byte != b'\n' && byte != b'\r' && byte != b'\t' && byte != 0x0C {
            if !force {
                let error_msg = format!(
                    "{}: Binary symbol 0x{:02X} found at line {}",
                    progname, byte, line_number
                );
                if verbose > 0 {
                    eprintln!("{}", error_msg);
                }
                return Err(io::Error::new(io::ErrorKind::InvalidData, error_msg));
            } else {
                if verbose > 0 {
                    eprintln!(
                        "{}: Binary symbol 0x{:02X} found at line {}; continuing due to --force.",
                        progname, byte, line_number
                    );
                }
                break;
            }
        }
        if byte == b'\n' {
            line_number += 1;
        }
    }
    Ok(())
}

fn convert_line_endings(
    content: &[u8],
    keep_bom: bool,
    force: bool,
    mac_mode: bool,
    add_eol: bool,
    verbose: usize,
    progname: &str,
) -> io::Result<Vec<u8>> {
    let mut result = Vec::with_capacity(content.len());
    let mut idx = 0;
    let mut prev_byte = None;
    let mut line_number = 1;
    let mut converted = 0;

    // Check for BOM
    if content.starts_with(&[0xEF, 0xBB, 0xBF]) {
        if keep_bom {
            result.extend_from_slice(&[0xEF, 0xBB, 0xBF]);
        }
        idx = 3;
    }

    detect_binary(&content[idx..], force, verbose, progname)?;

    while idx < content.len() {
        let byte = content[idx];
        idx += 1;

        if mac_mode {
            // Convert CR (\r) to LF (\n), but leave CRLF (\r\n) intact
            if byte == b'\r' {
                if idx < content.len() && content[idx] == b'\n' {
                    // CRLF sequence, leave as is
                    result.push(b'\r');
                    result.push(b'\n');
                    idx += 1;
                } else {
                    result.push(b'\n');
                    converted += 1;
                    line_number += 1;
                    if verbose > 1 {
                        eprintln!(
                            "{}: Converted CR at line {}.",
                            progname, line_number - 1
                        );
                    }
                }
            } else {
                if byte == b'\n' {
                    line_number += 1;
                }
                result.push(byte);
            }
        } else {
            // DOS to UNIX conversion
            if byte == b'\r' {
                if idx < content.len() && content[idx] == b'\n' {
                    // CRLF sequence, convert to LF
                    result.push(b'\n');
                    idx += 1;
                    converted += 1;
                    line_number += 1;
                    if verbose > 1 {
                        eprintln!(
                            "{}: Converted CRLF to LF at line {}.",
                            progname, line_number - 1
                        );
                    }
                } else {
                    // Single CR, leave as is (could be Mac line ending)
                    result.push(b'\r');
                }
            } else {
                if byte == b'\n' {
                    line_number += 1;
                }
                result.push(byte);
            }
        }
        prev_byte = Some(byte);
    }

    if add_eol {
        if let Some(last_byte) = prev_byte {
            if last_byte != b'\n' && last_byte != b'\r' {
                if verbose > 1 {
                    eprintln!("{}: Added line break to last line.", progname);
                }
                result.push(b'\n');
                line_number += 1;
            }
        }
    }

    if verbose > 1 {
        eprintln!(
            "{}: Converted {} out of {} line breaks.",
            progname,
            converted,
            line_number - 1
        );
    }

    Ok(result)
}

fn process_file(
    input_path: &Path,
    output_path: Option<&Path>,
    keep_bom: bool,
    force: bool,
    backup: bool,
    mac_mode: bool,
    add_eol: bool,
    verbose: usize,
    progname: &str,
) -> io::Result<()> {
    let content = fs::read(input_path)?;

    match convert_line_endings(
        &content,
        keep_bom,
        force,
        mac_mode,
        add_eol,
        verbose,
        progname,
    ) {
        Ok(converted_content) => {
            if backup {
                let backup_filename = format!("{}~", input_path.display());
                if verbose > 0 {
                    eprintln!(
                        "{}: creating backup file '{}'",
                        progname, backup_filename
                    );
                }
                fs::copy(input_path, &backup_filename)?;
            }

            let output_path = output_path.unwrap_or(input_path);

            // Preserve file permissions
            let metadata = fs::metadata(input_path)?;
            let permissions = metadata.permissions();

            // Write the converted content to a temporary file first
            let temp_path = output_path.with_extension("tmp");
            fs::write(&temp_path, converted_content)?;

            // Set the permissions of the temp file to match the original
            fs::set_permissions(&temp_path, permissions)?;

            // Replace the original file with the temp file
            fs::rename(&temp_path, output_path)?;

            if verbose > 0 {
                eprintln!("{}: converted '{}'", progname, input_path.display());
            }

            Ok(())
        }
        Err(e) => Err(e),
    }
}

fn is_stdin_tty() -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::io::AsRawFd;
        extern "C" {
            fn isatty(fd: i32) -> i32;
        }
        unsafe { isatty(io::stdin().as_raw_fd()) != 0 }
    }
    #[cfg(windows)]
    {
        unsafe {
            let handle: HANDLE = GetStdHandle(STD_INPUT_HANDLE);
            if handle == INVALID_HANDLE_VALUE || handle.is_null() {
                return false;
            }
            let mut mode: u32 = 0; // Ensure mode is u32
            GetConsoleMode(handle, &mut mode) != 0
        }
    }
    #[cfg(not(any(unix, windows)))]
    {
        // For other platforms, assume stdin is not a TTY
        false
    }
}
fn main() {
    let args: Vec<OsString> = env::args_os().collect();
    let progname = Path::new(&args[0])
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();

    let mut files = Vec::new();
    let mut keep_bom = false;
    let mut force = false;
    let mut backup = false;
    let mut mac_mode = false;
    let mut add_eol = false;
    let mut verbose = 0;
    let mut i = 1;

    while i < args.len() {
        match args[i].to_string_lossy().as_ref() {
            "--help" => {
                print_help(&progname);
                return;
            }
            "--version" => {
                print_version();
                return;
            }
            "-k" | "--keep-bom" => keep_bom = true,
            "-f" | "--force" => force = true,
            "-b" => backup = true,
            "-m" | "--mac" => mac_mode = true,
            "--add-eol" => add_eol = true,
            "-v" | "--verbose" => verbose += 1,
            "-n" | "--newfile" => {
                if i + 2 >= args.len() {
                    eprintln!(
                        "{}: option '{}' requires two arguments.",
                        progname,
                        args[i].to_string_lossy()
                    );
                    return;
                }
                let infile = Path::new(&args[i + 1]);
                let outfile = Path::new(&args[i + 2]);
                i += 2;

                if let Err(e) = process_file(
                    infile,
                    Some(outfile),
                    keep_bom,
                    force,
                    backup,
                    mac_mode,
                    add_eol,
                    verbose,
                    &progname,
                ) {
                    eprintln!("{}: Error converting '{}': {}", progname, infile.display(), e);
                    if !force {
                        eprintln!("{}: Use --force to convert binary files.", progname);
                    }
                }
            }
            arg if arg.starts_with('-') => {
                eprintln!("{}: invalid option '{}'", progname, arg);
                eprintln!("Try '{} --help' for more information.", progname);
                return;
            }
            filename => {
                files.push(PathBuf::from(filename));
            }
        }
        i += 1;
    }

    if files.is_empty() {
        // Check if stdin is connected to a terminal
        if is_stdin_tty() {
            eprintln!("{}: No files specified and no input provided.", progname);
            eprintln!("Try '{} --help' for more information.", progname);
            std::process::exit(1);
        } else {
            // Read from stdin
            let mut input = Vec::new();
            io::stdin().read_to_end(&mut input).unwrap();

            match convert_line_endings(
                &input,
                keep_bom,
                force,
                mac_mode,
                add_eol,
                verbose,
                &progname,
            ) {
                Ok(converted_content) => {
                    io::stdout().write_all(&converted_content).unwrap();
                }
                Err(e) => {
                    eprintln!("{}: Error converting input: {}", progname, e);
                    std::process::exit(1);
                }
            }
        }
    } else {
        // Process files as before
        for input_path in files {
            if let Err(e) = process_file(
                &input_path,
                None,
                keep_bom,
                force,
                backup,
                mac_mode,
                add_eol,
                verbose,
                &progname,
            ) {
                eprintln!("{}: Error converting '{}': {}", progname, input_path.display(), e);
                if !force {
                    eprintln!("{}: Use --force to convert binary files.", progname);
                }
            }
        }
    }
}