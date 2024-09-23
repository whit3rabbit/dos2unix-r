use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Read, Write, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use structopt::StructOpt;
use encoding_rs::{UTF_8, UTF_16LE, UTF_16BE, WINDOWS_1252, Encoding};
use encoding_rs_io::DecodeReaderBytesBuilder;
use walkdir::WalkDir;

/// Command-line options for dos2unix
#[derive(Debug, StructOpt)]
#[structopt(name = "dos2unix", about = "Convert line endings between DOS and Unix formats")]
struct Opt {
    /// Input files or directories
    #[structopt(parse(from_os_str))]
    input: Vec<PathBuf>,

    /// Convert to DOS line endings (CRLF)
    #[structopt(short = "d", long)]
    to_dos: bool,

    /// Keep original file modification time
    #[structopt(short = "p", long)]
    keep_date: bool,

    /// Create backup of original file
    #[structopt(short, long)]
    backup: bool,

    /// Verbose output
    #[structopt(short, long)]
    verbose: bool,

    /// Quiet mode
    #[structopt(short, long)]
    quiet: bool,

    /// Force conversion of binary files
    #[structopt(short, long)]
    force: bool,

    /// Add or keep Byte Order Mark (BOM)
    #[structopt(long)]
    keep_bom: bool,

    /// Remove Byte Order Mark (BOM)
    #[structopt(long)]
    remove_bom: bool,

    /// Specify input encoding (utf8, utf16le, utf16be, iso-8859-1)
    #[structopt(long, default_value = "auto")]
    from_encoding: String,

    /// Recursively process directories
    #[structopt(short, long)]
    recursive: bool,

    /// Follow symbolic links
    #[structopt(long)]
    follow_symlinks: bool,

    /// Print file information
    #[structopt(short, long)]
    info: bool,

    /// Add Byte Order Mark (BOM)
    #[structopt(short = "a", long = "add-bom")]
    add_bom: bool,

    /// Add additional newline
    #[structopt(short = "l", long = "newline")]
    newline: bool,

    /// Safe mode (skip binary files)
    #[structopt(short = "s", long = "safe")]
    safe: bool,

    /// Convert file in-place, keeping original file name
    #[structopt(long = "oldfile")]
    oldfile: bool,

    /// Convert file to new file
    #[structopt(long = "newfile")]
    newfile: bool,
}

fn main() -> io::Result<()> {
    let opt = Opt::from_args();

    if opt.input.is_empty() {
        eprintln!("No input files or directories specified.");
        std::process::exit(1);
    }

    for input in &opt.input {
        if let Err(e) = process_input(input, &opt) {
            eprintln!("Error processing {}: {}", input.display(), e);
        }
    }

    Ok(())
}

/// Process input file or directory
fn process_input(input: &Path, opt: &Opt) -> io::Result<()> {
    if !input.exists() {
        return Err(io::Error::new(io::ErrorKind::NotFound, format!("Path does not exist: {}", input.display())));
    }

    if input.is_dir() {
        process_directory(input, opt)
    } else {
        process_file(input, opt)
    }
}

/// Process a directory
fn process_directory(dir: &Path, opt: &Opt) -> io::Result<()> {
    if opt.recursive {
        let walker = WalkDir::new(dir).follow_links(opt.follow_symlinks);
        for entry in walker.into_iter() {
            match entry {
                Ok(entry) if entry.file_type().is_file() => {
                    if let Err(e) = process_file(entry.path(), opt) {
                        eprintln!("Error processing {}: {}", entry.path().display(), e);
                    }
                }
                Err(e) => eprintln!("Error accessing entry: {}", e),
                _ => {}
            }
        }
    } else {
        if !opt.quiet {
            eprintln!("Skipping directory: {}. Use --recursive to process directories.", dir.display());
        }
    }
    Ok(())
}

/// Process a single file
fn process_file(path: &Path, opt: &Opt) -> io::Result<()> {
    if opt.info {
        print_file_info(path, &opt.from_encoding)
    } else if opt.to_dos {
        convert_file(path, opt, unix2dos)
    } else {
        // Default behavior: convert to Unix
        convert_file(path, opt, dos2unix)
    }
}

/// Print file information
fn print_file_info(input: &Path, from_encoding: &str) -> io::Result<()> {
    let mut file = File::open(input)?;
    let (encoding, bom_size) = detect_encoding_and_bom(&mut file, from_encoding)?;
    file.seek(SeekFrom::Start(0))?;

    let mut reader = DecodeReaderBytesBuilder::new()
        .encoding(Some(encoding))
        .build(file);

    let mut content = String::new();
    reader.read_to_string(&mut content)?;

    let dos_count = content.matches("\r\n").count();
    let unix_count = content.matches('\n').count() - dos_count;
    let mac_count = content.matches('\r').count() - dos_count;

    println!("File: {}", input.display());
    println!("Encoding: {:?}", encoding);
    println!("BOM: {}", if bom_size > 0 { "Present" } else { "Absent" });
    println!("DOS line endings: {}", dos_count);
    println!("Unix line endings: {}", unix_count);
    println!("Mac line endings: {}", mac_count);

    Ok(())
}

/// Detect file encoding and BOM
fn detect_encoding_and_bom<R: Read + Seek>(reader: &mut R, specified_encoding: &str) -> io::Result<(&'static Encoding, usize)> {
    if specified_encoding != "auto" {
        return Ok((get_encoding(specified_encoding), 0));
    }

    let mut buffer = [0; 4];
    let read_bytes = reader.read(&mut buffer)?;
    reader.seek(SeekFrom::Start(0))?;

    if read_bytes >= 3 && buffer.starts_with(&[0xEF, 0xBB, 0xBF]) {
        Ok((UTF_8, 3))
    } else if read_bytes >= 2 && buffer.starts_with(&[0xFF, 0xFE]) {
        Ok((UTF_16LE, 2))
    } else if read_bytes >= 2 && buffer.starts_with(&[0xFE, 0xFF]) {
        Ok((UTF_16BE, 2))
    } else if read_bytes == 0 {
        Err(io::Error::new(io::ErrorKind::UnexpectedEof, "File is empty"))
    } else {
        Ok((UTF_8, 0))
    }
}

/// Convert file line endings
fn convert_file<F>(input: &Path, opt: &Opt, convert_fn: F) -> io::Result<()>
where
    F: Fn(&str) -> String,
{
    let mut input_file = File::open(input)?;
    let metadata = input_file.metadata()?;

    let (encoding, bom_size) = detect_encoding_and_bom(&mut input_file, &opt.from_encoding)?;
    input_file.seek(SeekFrom::Start(0))?;

    let reader = DecodeReaderBytesBuilder::new()
        .encoding(Some(encoding))
        .build(input_file);

    let output_path = if opt.newfile {
        input.with_extension("new")
    } else if opt.oldfile {
        input.to_path_buf()
    } else {
        input.with_extension("tmp")
    };

    let mut writer = BufWriter::new(File::create(&output_path)?);

    // Handle BOM
    if (opt.add_bom || (opt.keep_bom && bom_size > 0)) && !opt.remove_bom {
        let bom = if encoding == UTF_8 {
            vec![0xEF, 0xBB, 0xBF]
        } else if encoding == UTF_16LE {
            vec![0xFF, 0xFE]
        } else if encoding == UTF_16BE {
            vec![0xFE, 0xFF]
        } else {
            vec![]
        };
        writer.write_all(&bom)?;
    }

    let mut content = String::new();
    if metadata.len() > 10_000_000 { // 10 MB threshold
        convert_large_file(reader, &mut writer, &mut content, &convert_fn, opt, encoding)?;
    } else {
        BufReader::new(reader).read_to_string(&mut content)?;
        if !opt.force && is_binary(&content) {
            if opt.safe {
                if !opt.quiet {
                    eprintln!("Skipping binary file: {}", input.display());
                }
                return Ok(());
            } else if !opt.quiet {
                eprintln!("Converting binary file: {}", input.display());
            }
        }
        let converted = convert_fn(&content);
        let (cow, _, _) = encoding.encode(&converted);
        writer.write_all(&cow)?;
    }

    if opt.keep_date {
        let atime = filetime::FileTime::from_last_access_time(&metadata);
        let mtime = filetime::FileTime::from_last_modification_time(&metadata);
        filetime::set_file_times(&output_path, atime, mtime)?;
    }

    if opt.backup && !opt.newfile {
        let backup_path = input.with_extension("bak");
        if let Err(e) = std::fs::rename(input, &backup_path) {
            return Err(io::Error::new(io::ErrorKind::Other, format!("Failed to create backup: {}", e)));
        }
    }

    if !opt.newfile && !opt.oldfile {
        if let Err(e) = std::fs::remove_file(input) {
            return Err(io::Error::new(io::ErrorKind::Other, format!("Failed to remove original file: {}", e)));
        }
        if let Err(e) = std::fs::rename(&output_path, input) {
            return Err(io::Error::new(io::ErrorKind::Other, format!("Failed to rename temporary file: {}", e)));
        }
    }

    if opt.verbose {
        println!("Converted: {}", input.display());
    }

    Ok(())
}

/// Convert large files line by line
fn convert_large_file<R: Read, W: Write>(
    reader: R,
    writer: &mut W,
    content: &mut String,
    convert_fn: &dyn Fn(&str) -> String,
    opt: &Opt,
    encoding: &'static Encoding,
) -> io::Result<()> {
    let mut buf_reader = BufReader::new(reader);

    loop {
        content.clear();
        if buf_reader.read_line(content)? == 0 {
            break;
        }
        let converted_line = convert_fn(content);
        if opt.newline && !converted_line.ends_with('\n') {
            let modified_line = format!("{}\n", converted_line);
            let (cow, _, _) = encoding.encode(&modified_line);
            writer.write_all(&cow)?;
        } else {
            let (cow, _, _) = encoding.encode(&converted_line);
            writer.write_all(&cow)?;
        }
    }

    Ok(())
}

/// Convert DOS line endings to Unix
fn dos2unix(input: &str) -> String {
    input.replace("\r\n", "\n")
}

/// Convert Unix line endings to DOS
fn unix2dos(input: &str) -> String {
    input.replace("\n", "\r\n")
}

/// Get encoding from string
fn get_encoding(encoding: &str) -> &'static Encoding {
    match encoding.to_lowercase().as_str() {
        "utf8" => UTF_8,
        "utf16le" => UTF_16LE,
        "utf16be" => UTF_16BE,
        "iso-8859-1" => WINDOWS_1252,
        _ => UTF_8,
    }
}

/// Check if content is binary
fn is_binary(content: &str) -> bool {
    const BINARY_CHECK_CHARS: usize = 8000;
    let check_size = content.chars().take(BINARY_CHECK_CHARS).count();
    content.chars().take(check_size).any(|c| {
        let code = c as u32;
        code < 32 &&
        c != '\n' &&
        c != '\r' &&
        c != '\t' &&
        c != '\x0C'
    })
}