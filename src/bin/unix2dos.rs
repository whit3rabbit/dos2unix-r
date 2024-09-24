// src/bin/unix2dos.rs
use std::env;
use std::ffi::OsString;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use dos2unix_lib::{convert_line_endings, process_file, is_stdin_tty, ConversionMode};

fn print_help(progname: &str) {
    println!("Usage: {} [options] [FILE ...] [-n INFILE OUTFILE]", progname);
    println!("Converts text files with Unix or Mac line endings to DOS line endings.");
    println!("Options:");
    println!("  -b             Make a backup of each file.");
    println!("  -f, --force    Force conversion of binary files.");
    println!("  -k, --keep-bom Keep the Byte Order Mark (BOM).");
    println!("  -m, --mac      Convert Mac line endings (CR) to DOS (CRLF).");
    println!("  -o, --oldfile  Overwrite original file (default behavior).");
    println!("  -n, --newfile  Specify new output file.");
    println!("      --add-eol  Add missing end-of-line at end of file.");
    println!("  -v, --verbose  Increase verbosity level (can be used multiple times).");
    println!("      --help     Display this help and exit.");
    println!("      --version  Output version information and exit.");
}

fn print_version() {
    println!("unix2dos-rust version 0.1.0");
}

fn main() {
    let args: Vec<OsString> = env::args_os().collect();
    let progname = Path::new(&args[0])
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();

    let mut files: Vec<PathBuf> = Vec::new();
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
                let infile = PathBuf::from(&args[i + 1]);
                let outfile = PathBuf::from(&args[i + 2]);
                i += 2;

                let conversion_mode = if mac_mode {
                    ConversionMode::ToDos  // Convert Mac line endings to DOS
                } else {
                    ConversionMode::ToDos
                };

                if let Err(e) = process_file(
                    &infile,
                    Some(&outfile),
                    keep_bom,
                    force,
                    backup,
                    conversion_mode,
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

            let conversion_mode = if mac_mode {
                ConversionMode::ToDos  // Convert Mac line endings to DOS
            } else {
                ConversionMode::ToDos  // Default conversion mode
            };

            match convert_line_endings(
                &input,
                keep_bom,
                force,
                conversion_mode,
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
        // Determine the conversion mode once for all files
        let conversion_mode = if mac_mode {
            ConversionMode::ToDos  // Convert Mac line endings to DOS
        } else {
            ConversionMode::ToDos  // Default conversion mode
        };

        for input_path in &files {
            if let Err(e) = process_file(
                input_path,
                None,
                keep_bom,
                force,
                backup,
                conversion_mode,
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
