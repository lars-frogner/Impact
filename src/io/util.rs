//! Utilities for input/output.

use anyhow::{Context, Result};
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

/// Reads the RON (Rusty Object Notation) file at the given path and
/// deserializes the contents into an object of type `T`.
pub fn parse_ron_file<T>(file_path: impl AsRef<Path>) -> Result<T>
where
    T: for<'de> serde::de::Deserialize<'de>,
{
    let file_path = file_path.as_ref();

    let text = read_text_file(file_path)
        .map_err(anyhow::Error::from)
        .with_context(|| format!("Could not open {}", file_path.display()))?;

    ron::from_str::<T>(&text)
        .map_err(anyhow::Error::from)
        .with_context(|| format!("Invalid syntax in {}", file_path.display()))
}

/// Serializes the given value of type `T` to RON (Rusty Object Notation)
/// and writes it to the given path.
pub fn write_ron_file<T>(value: &T, output_file_path: impl AsRef<Path>) -> Result<()>
where
    T: serde::ser::Serialize,
{
    let text = ron::ser::to_string_pretty(value, ron::ser::PrettyConfig::default())?;
    write_text_file(&text, output_file_path).map_err(Into::into)
}
