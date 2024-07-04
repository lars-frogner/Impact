//! Utilities for input/output.

use std::{
    fs::{self, File},
    io::{self, BufReader, Read, Write},
    path::Path,
};

/// Creates any directories missing in order for the given path to be valid.
pub fn create_directory_if_missing<P: AsRef<Path>>(path: P) -> io::Result<()> {
    let path = path.as_ref();
    if path.extension().is_some() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
        } else {
            Ok(())
        }
    } else {
        fs::create_dir_all(path)
    }
}

/// Creates the file at the given path, as well as any missing parent
/// directories.
pub fn create_file_and_required_directories<P: AsRef<Path>>(file_path: P) -> io::Result<fs::File> {
    create_directory_if_missing(&file_path)?;
    File::create(file_path)
}

/// Reads and returns the content of the specified text file.
pub fn read_text_file<P: AsRef<Path>>(file_path: P) -> io::Result<String> {
    let file = File::open(file_path)?;
    let mut text = String::new();
    let _ = BufReader::new(file).read_to_string(&mut text)?;
    Ok(text)
}

/// Writes the given string as a text file with the specified path, regardless
/// of whether the file already exists.
pub fn write_text_file<P: AsRef<Path>>(text: &str, output_file_path: P) -> io::Result<()> {
    let mut file = create_file_and_required_directories(output_file_path)?;
    write!(&mut file, "{}", text)
}

/// Saves the given byte buffer directly as a binary file at the given path.
pub fn save_data_as_binary<P>(output_file_path: P, byte_buffer: &[u8]) -> io::Result<()>
where
    P: AsRef<Path>,
{
    let mut file = create_file_and_required_directories(output_file_path)?;
    file.write_all(byte_buffer)
}
