use std::fs::File;
use std::io::{self, BufWriter, Read, Write, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use structopt::StructOpt;
use encoding_rs::{UTF_8, UTF_16LE, UTF_16BE, WINDOWS_1252, Encoding};
use encoding_rs_io::DecodeReaderBytesBuilder;
use walkdir::WalkDir;
use log::{error, warn, info, debug};

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
}

fn main() -> io::Result<()> {
    env_logger::init();

    let opt = Opt::from_args();

    if opt.input.is_empty() {
        error!("No input files or directories specified.");
        std::process::exit(1);
    }

    for input in &opt.input {
        if let Err(e) = process_input(input, &opt) {
            error!("Error processing {}: {}", input.display(), e);
        }
    }

    Ok(())
}

fn process_input(input: &Path, opt: &Opt) -> io::Result<()> {
    if !input.exists() {
        return Err(io::Error::new(io::ErrorKind::NotFound, format!("Path does not exist: {}", input.display())));
    }

    if input.is_dir() {
        if opt.recursive {
            let walker = WalkDir::new(input).follow_links(opt.follow_symlinks);
            for entry in walker.into_iter() {
                match entry {
                    Ok(entry) if entry.file_type().is_file() => {
                        if let Err(e) = process_file(entry.path(), opt) {
                            error!("Error processing {}: {}", entry.path().display(), e);
                        }
                    }
                    Err(e) => error!("Error accessing entry: {}", e),
                    _ => {}
                }
            }
        } else {
            warn!("Skipping directory: {}. Use --recursive to process directories.", input.display());
        }
    } else {
        process_file(input, opt)?;
    }

    Ok(())
}

fn process_file(path: &Path, opt: &Opt) -> io::Result<()> {
    if opt.info {
        print_file_info(path, &opt.from_encoding)?;
    } else if opt.to_dos {
        convert_file(path, opt, unix2dos)?;
    } else {
        // Default behavior: convert to Unix
        convert_file(path, opt, dos2unix)?;
    }

    Ok(())
}

fn print_file_info(input: &Path, from_encoding: &str) -> io::Result<()> {
    let mut file = File::open(input)?;
    let (encoding, bom_size) = detect_encoding_and_bom(&mut file, from_encoding)?;
    let (dos_count, unix_count, mac_count) = count_line_endings(&mut file)?;

    info!("File: {}", input.display());
    info!("Encoding: {:?}", encoding);
    info!("BOM: {}", if bom_size > 0 { "Present" } else { "Absent" });
    info!("DOS line endings: {}", dos_count);
    info!("Unix line endings: {}", unix_count);
    info!("Mac line endings: {}", mac_count);

    Ok(())
}

fn detect_encoding_and_bom<R: Read + Seek>(reader: &mut R, specified_encoding: &str) -> io::Result<(&'static Encoding, usize)> {
    if specified_encoding != "auto" {
        return Ok((get_encoding(specified_encoding), 0));
    }

    let mut buffer = [0; 4];
    let read_bytes = reader.read(&mut buffer)?;
    reader.seek(SeekFrom::Start(0))?;

    debug!("Read {} bytes: {:?}", read_bytes, &buffer[..read_bytes]);

    Ok(match read_bytes {
        3..=4 if buffer.starts_with(&[0xEF, 0xBB, 0xBF]) => {
            debug!("Detected UTF-8 BOM");
            (UTF_8, 3)
        },
        2..=4 if buffer.starts_with(&[0xFF, 0xFE]) => {
            debug!("Detected UTF-16LE BOM");
            (UTF_16LE, 2)
        },
        2..=4 if buffer.starts_with(&[0xFE, 0xFF]) => {
            debug!("Detected UTF-16BE BOM");
            (UTF_16BE, 2)
        },
        0 => return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "File is empty")),
        _ => {
            debug!("No BOM detected, defaulting to UTF-8");
            (UTF_8, 0)
        },
    })
}

fn count_line_endings<R: Read>(reader: &mut R) -> io::Result<(usize, usize, usize)> {
    let mut dos_count = 0;
    let mut unix_count = 0;
    let mut mac_count = 0;
    let mut last_char = 0;

    for byte in reader.bytes() {
        let byte = byte?;
        match (last_char, byte) {
            (b'\r', b'\n') => dos_count += 1,
            (_, b'\n') => unix_count += 1,
            (b'\r', _) => mac_count += 1,
            _ => {}
        }
        last_char = byte;
    }

    // Count the last Mac line ending if the file ends with '\r'
    if last_char == b'\r' {
        mac_count += 1;
    }

    Ok((dos_count, unix_count, mac_count))
}

fn convert_file<F>(input: &Path, opt: &Opt, convert_fn: F) -> io::Result<()>
where
    F: Fn(&[u8]) -> Vec<u8>,
{
    let mut input_file = File::open(input)?;
    let metadata = input_file.metadata()?;

    if metadata.len() > 10_000_000 { // 10 MB threshold
        return convert_large_file(input, opt, convert_fn);
    }

    let (encoding, bom_size) = detect_encoding_and_bom(&mut input_file, &opt.from_encoding)?;
    input_file.seek(SeekFrom::Start(0))?;

    let mut reader = DecodeReaderBytesBuilder::new()
        .encoding(Some(encoding))
        .build(input_file);

    let mut content = Vec::new();
    reader.read_to_end(&mut content)?;

    if content.is_empty() {
        return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "File is empty"));
    }

    if !opt.force && is_binary(&content) {
        if !opt.quiet {
            warn!("Skipping binary file: {}", input.display());
        }
        return Ok(());
    }

    // Remove BOM if requested
    if opt.remove_bom && bom_size > 0 {
        content = content[bom_size..].to_vec();
    }

    let converted = convert_fn(&content);

    let output_path = input.with_extension("tmp");
    {
        let mut writer = BufWriter::new(File::create(&output_path)?);
        
        if opt.keep_bom && bom_size > 0 && !opt.remove_bom {
            writer.write_all(&content[..bom_size])?;
        }
        
        // Create a new String that lives for the entire scope
        let converted_string = String::from_utf8_lossy(&converted).into_owned();
        // Encode the owned String
        let (cow, _, _) = encoding.encode(&converted_string);
        writer.write_all(&cow)?;
    }

    if opt.keep_date {
        let atime = filetime::FileTime::from_last_access_time(&metadata);
        let mtime = filetime::FileTime::from_last_modification_time(&metadata);
        filetime::set_file_times(&output_path, atime, mtime)?;
    }

    if opt.backup {
        let backup_path = input.with_extension("bak");
        if let Err(e) = std::fs::rename(input, &backup_path) {
            return Err(io::Error::new(io::ErrorKind::Other, format!("Failed to create backup: {}", e)));
        }
    } else {
        if let Err(e) = std::fs::remove_file(input) {
            return Err(io::Error::new(io::ErrorKind::Other, format!("Failed to remove original file: {}", e)));
        }
    }

    if let Err(e) = std::fs::rename(&output_path, input) {
        return Err(io::Error::new(io::ErrorKind::Other, format!("Failed to rename temporary file: {}", e)));
    }

    if opt.verbose {
        info!("Converted: {}", input.display());
    }

    Ok(())
}

fn convert_large_file<F>(input: &Path, opt: &Opt, convert_fn: F) -> io::Result<()>
where
    F: Fn(&[u8]) -> Vec<u8>,
{
    let mut input_file = File::open(input)?;
    let metadata = input_file.metadata()?;
    let (encoding, bom_size) = detect_encoding_and_bom(&mut input_file, &opt.from_encoding)?;
    input_file.seek(SeekFrom::Start(0))?;

    let output_path = input.with_extension("tmp");
    let mut writer = BufWriter::new(File::create(&output_path)?);

    if opt.keep_bom && bom_size > 0 && !opt.remove_bom {
        let mut bom = vec![0; bom_size];
        input_file.read_exact(&mut bom)?;
        writer.write_all(&bom)?;
    } else if bom_size > 0 {
        input_file.seek(SeekFrom::Start(bom_size as u64))?;
    }

    let mut reader = DecodeReaderBytesBuilder::new()
        .encoding(Some(encoding))
        .build(input_file);

    let mut buffer = [0; 8192];
    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        let converted = convert_fn(&buffer[..bytes_read]);
        writer.write_all(&converted)?;
    }

    if opt.keep_date {
        let atime = filetime::FileTime::from_last_access_time(&metadata);
        let mtime = filetime::FileTime::from_last_modification_time(&metadata);
        filetime::set_file_times(&output_path, atime, mtime)?;
    }

    if opt.backup {
        let backup_path = input.with_extension("bak");
        if let Err(e) = std::fs::rename(input, &backup_path) {
            return Err(io::Error::new(io::ErrorKind::Other, format!("Failed to create backup: {}", e)));
        }
    } else {
        if let Err(e) = std::fs::remove_file(input) {
            return Err(io::Error::new(io::ErrorKind::Other, format!("Failed to remove original file: {}", e)));
        }
    }

    if let Err(e) = std::fs::rename(&output_path, input) {
        return Err(io::Error::new(io::ErrorKind::Other, format!("Failed to rename temporary file: {}", e)));
    }

    if opt.verbose {
        info!("Converted large file: {}", input.display());
    }

    Ok(())
}

fn dos2unix(input: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(input.len());
    let mut iter = input.iter().peekable();
    while let Some(&byte) = iter.next() {
        if byte != b'\r' || iter.peek() != Some(&&b'\n') {
            output.push(byte);
        }
    }
    output
}

fn unix2dos(input: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(input.len() * 2);
    for &byte in input {
        if byte == b'\n' {
            output.push(b'\r');
        }
        output.push(byte);
    }
    output
}

fn get_encoding(encoding: &str) -> &'static Encoding {
    match encoding.to_lowercase().as_str() {
        "utf8" => UTF_8,
        "utf16le" => UTF_16LE,
        "utf16be" => UTF_16BE,
        "iso-8859-1" => WINDOWS_1252,
        _ => UTF_8,
    }
}

fn is_binary(content: &[u8]) -> bool {
    const BINARY_CHECK_BYTES: usize = 8000;
    let check_size = std::cmp::min(content.len(), BINARY_CHECK_BYTES);
    content[..check_size].iter().any(|&byte| byte == 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_dos2unix() {
        let input = b"line1\r\nline2\r\nline3\r\n";
        let expected = b"line1\nline2\nline3\n";
        assert_eq!(dos2unix(input), expected);
    }

    #[test]
    fn test_unix2dos() {
        let input = b"line1\nline2\nline3\n";
        let expected = b"line1\r\nline2\r\nline3\r\n";
        assert_eq!(unix2dos(input), expected);
    }

    #[test]
    fn test_detect_encoding_and_bom() {
        let utf8_content = vec![0xEF, 0xBB, 0xBF, b'a', b'b', b'c'];
        let mut utf8_file = Cursor::new(utf8_content);
        assert_eq!(detect_encoding_and_bom(&mut utf8_file, "auto").unwrap(), (UTF_8, 3));

        let utf16le_content = vec![0xFF, 0xFE, b'a', 0, b'b', 0];
        let mut utf16le_file = Cursor::new(utf16le_content);
        assert_eq!(detect_encoding_and_bom(&mut utf16le_file, "auto").unwrap(), (UTF_16LE, 2));

        let utf16be_content = vec![0xFE, 0xFF, 0, b'a', 0, b'b'];
        let mut utf16be_file = Cursor::new(utf16be_content);
        assert_eq!(detect_encoding_and_bom(&mut utf16be_file, "auto").unwrap(), (UTF_16BE, 2));

        let no_bom_content = vec![b'a', b'b', b'c'];
        let mut no_bom_file = Cursor::new(no_bom_content);
        assert_eq!(detect_encoding_and_bom(&mut no_bom_file, "auto").unwrap(), (UTF_8, 0));
    }

    #[test]
    fn test_count_line_endings() {
        let mut mixed_endings = Cursor::new(b"line1\r\nline2\nline3\rline4\r\n");
        assert_eq!(count_line_endings(&mut mixed_endings).unwrap(), (2, 1, 1));

        let mut unix_endings = Cursor::new(b"line1\nline2\nline3\n");
        assert_eq!(count_line_endings(&mut unix_endings).unwrap(), (0, 3, 0));

        let mut dos_endings = Cursor::new(b"line1\r\nline2\r\nline3\r\n");
        assert_eq!(count_line_endings(&mut dos_endings).unwrap(), (3, 0, 0));

        let mut mac_endings = Cursor::new(b"line1\rline2\rline3\r");
        assert_eq!(count_line_endings(&mut mac_endings).unwrap(), (0, 0, 3));
    }

    #[test]
    fn test_get_encoding() {
        assert_eq!(get_encoding("utf8"), UTF_8);
        assert_eq!(get_encoding("utf16le"), UTF_16LE);
        assert_eq!(get_encoding("utf16be"), UTF_16BE);
        assert_eq!(get_encoding("iso-8859-1"), WINDOWS_1252);
        assert_eq!(get_encoding("unknown"), UTF_8);
    }
}