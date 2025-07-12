use crate::asset::Asset;
use anyhow::{Context, Result, anyhow, bail};
use std::{
    fs,
    io::{self, Write},
    path::Path,
};
use tempfile::NamedTempFile;
use ureq::http::{HeaderMap, HeaderValue};

/// Fetches an asset from its provider and extracts it to the target directory.
///
/// Creates a subdirectory named after the asset within `target_dir` and downloads
/// the asset content there. Supports ZIP archives (automatically extracted) and
/// individual files.
pub fn fetch_asset(asset: &Asset, target_dir: &Path) -> Result<()> {
    let asset_dir = target_dir.join(&asset.name);
    if asset_dir.exists() {
        bail!("Asset directory already exists: {}", asset_dir.display());
    }

    let uri = asset.info.obtain_fetch_uri();

    println!("Fetching asset '{}' from {}", asset.name, uri);

    let mut response = ureq::get(&uri).call().context("Failed GET request")?;

    let content_type = get_content_type(response.headers()).map(ToString::to_string);

    let response_body_reader = io::BufReader::new(response.body_mut().as_reader());
    let response_body_file = stream_to_temp_file(response_body_reader, target_dir)?;

    let magic_bytes =
        read_magic_bytes(response_body_file.as_file()).context("Failed to read magic bytes")?;

    if looks_like_zip(content_type.as_deref(), &magic_bytes, &uri) {
        let archive_file = response_body_file.reopen()?;
        let archive_file_reader = io::BufReader::new(archive_file);
        extract_zip_archive(archive_file_reader, &asset_dir)
            .context("Failed to extract ZIP archive")?;
    } else {
        let asset_file_name = file_name_from_uri(&uri)?;
        let file_path = asset_dir.join(asset_file_name);
        fs::create_dir_all(&asset_dir)?;
        persist_or_copy(response_body_file, &file_path)
            .context("Failed to persist downloaded asset file")?;
    }

    println!("Successfully fetched asset '{}'", asset.name);
    Ok(())
}

/// Streams response data to a temporary file in the target directory.
///
/// This allows handling large downloads without loading everything into memory.
fn stream_to_temp_file(
    mut response_body_reader: impl io::Read,
    target_dir: &Path,
) -> Result<NamedTempFile> {
    let mut temp_file = tempfile::Builder::new()
        .prefix("asset_fetcher_download_")
        .tempfile_in(target_dir)
        .context("Failed creating temporary file for download")?;

    io::copy(&mut response_body_reader, &mut temp_file)
        .context("Failed to copy GET response to temporary file")?;

    Ok(temp_file)
}

/// Extracts the Content-Type header value from HTTP response headers.
fn get_content_type(headers: &HeaderMap<HeaderValue>) -> Option<&str> {
    headers.get("content-type")?.to_str().ok()
}

/// Reads the first 4 bytes from a file to determine file type.
fn read_magic_bytes<R>(mut data: R) -> Result<[u8; 4]>
where
    R: io::Read + io::Seek,
{
    let mut buffer = [0u8; 4];
    data.seek(io::SeekFrom::Start(0))?;
    data.read_exact(&mut buffer)?;
    Ok(buffer)
}

/// Determines if the downloaded content is a ZIP archive based on multiple
/// indicators.
///
/// Checks Content-Type header, magic bytes, and URI extension.
fn looks_like_zip(content_type: Option<&str>, magic_bytes: &[u8; 4], uri: &str) -> bool {
    let content_type_is_zip = content_type
        .map(|ct| {
            ct.eq_ignore_ascii_case("application/zip")
                || ct.eq_ignore_ascii_case("application/x-zip-compressed")
        })
        .unwrap_or(false);

    content_type_is_zip || magic_bytes == b"PK\x03\x04" || uri.ends_with(".zip")
}

/// Extracts all files from a ZIP archive to the target directory.
///
/// Preserves the directory structure from the archive and creates necessary
/// parent directories.
fn extract_zip_archive<R>(archive_file: R, target_dir: &Path) -> Result<()>
where
    R: io::Read + io::Seek,
{
    let mut archive = zip::ZipArchive::new(archive_file)?;

    for i in 0..archive.len() {
        let mut zip_file = archive.by_index(i)?;

        let Some(zip_file_path) = zip_file.enclosed_name() else {
            bail!("Invalid path for zip file: {}", zip_file.name());
        };

        let output_file_path = target_dir.join(zip_file_path);

        if zip_file.is_dir() {
            fs::create_dir_all(&output_file_path)?;
        } else {
            if let Some(parent) = output_file_path.parent() {
                fs::create_dir_all(parent)?;
            }

            let output_file = fs::File::create(&output_file_path)?;
            let mut output_file_writer = io::BufWriter::new(output_file);
            io::copy(&mut zip_file, &mut output_file_writer)?;
            output_file_writer.flush()?;
        }
    }

    Ok(())
}

/// Extracts the filename from a URI by taking the last path segment.
fn file_name_from_uri(uri: &str) -> Result<&str> {
    uri.rsplit('/')
        .next()
        .ok_or_else(|| anyhow!("Got empty URI"))
}

/// Attempts to move a temporary file to the target path, falling back to copy
/// if needed.
fn persist_or_copy(temp_file: NamedTempFile, target_path: &Path) -> Result<()> {
    match temp_file.persist_noclobber(target_path) {
        Ok(_) => Ok(()),
        Err(e) => {
            io::copy(
                &mut io::BufReader::new(fs::File::open(&e.file)?),
                &mut io::BufWriter::new(fs::File::create(target_path)?),
            )?;
            Ok(())
        }
    }
}
