use std::io;
use std::fs;
use std::path::Path;

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

#[derive(Copy, Clone)]
pub enum ConversionMode {
    ToUnix,
    ToDos,
    ToMac,
}

pub fn detect_binary(
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

pub fn convert_line_endings(
    content: &[u8],
    keep_bom: bool,
    force: bool,
    conversion_mode: ConversionMode,
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

        match conversion_mode {
            ConversionMode::ToUnix => {
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
            ConversionMode::ToDos => {
                // UNIX to DOS conversion
                if byte == b'\n' {
                    if prev_byte != Some(b'\r') {
                        // LF not preceded by CR, insert CR
                        result.push(b'\r');
                        converted += 1;
                        if verbose > 1 {
                            eprintln!(
                                "{}: Converted LF to CRLF at line {}.",
                                progname, line_number
                            );
                        }
                    }
                    result.push(b'\n');
                    line_number += 1;
                } else {
                    result.push(byte);
                }
            }
            ConversionMode::ToMac => {
                // UNIX/Mac conversion
                if byte == b'\n' {
                    if prev_byte != Some(b'\r') {
                        // LF not part of CRLF, convert LF to CR
                        result.push(b'\r');
                        converted += 1;
                        if verbose > 1 {
                            eprintln!(
                                "{}: Converted LF to CR at line {}.",
                                progname, line_number
                            );
                        }
                    } else {
                        // Part of CRLF, keep as is
                        result.push(b'\n');
                    }
                    line_number += 1;
                } else {
                    result.push(byte);
                }
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
                match conversion_mode {
                    ConversionMode::ToUnix => result.push(b'\n'),
                    ConversionMode::ToDos => {
                        result.push(b'\r');
                        result.push(b'\n');
                    }
                    ConversionMode::ToMac => result.push(b'\r'),
                }
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

pub fn process_file(
    input_path: &Path,
    output_path: Option<&Path>,
    keep_bom: bool,
    force: bool,
    backup: bool,
    conversion_mode: ConversionMode,
    add_eol: bool,
    verbose: usize,
    progname: &str,
) -> io::Result<()> {
    let content = fs::read(input_path)?;

    match convert_line_endings(
        &content,
        keep_bom,
        force,
        conversion_mode,
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

pub fn is_stdin_tty() -> bool {
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