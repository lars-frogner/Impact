use crate::asset::Asset;
use anyhow::{Context, Result, bail};
use std::{
    fs,
    io::{self, Write},
    path::Path,
};
use tempfile::NamedTempFile;
use ureq::http::{HeaderMap, HeaderValue};

/// Fetches an asset from its provider and extracts it to the target directory.
///
/// Creates a subdirectory named after the asset within `target_dir` and
/// downloads the asset content there. Supports both single and multiple
/// downloads per asset.
pub fn fetch_asset(asset: &Asset, target_dir: &Path) -> Result<()> {
    let asset_dir = target_dir.join(&asset.name);
    if asset_dir.exists() {
        bail!("Asset directory already exists: {}", asset_dir.display());
    }

    println!("Fetching asset '{}'", asset.name);

    // Get all downloads for this asset
    let downloads = asset
        .info
        .get_downloads()
        .context("Failed to get download information for asset")?;

    if downloads.is_empty() {
        bail!("No downloads found for asset '{}'", asset.name);
    }

    // Process each download
    for (index, download) in downloads.iter().enumerate() {
        println!(
            "  Downloading file {}/{}: {}",
            index + 1,
            downloads.len(),
            download.url
        );

        let mut response = ureq::get(&download.url)
            .call()
            .with_context(|| format!("Failed GET request for {}", download.url))?;

        let content_type = get_content_type(response.headers()).map(ToString::to_string);

        let response_body_reader = io::BufReader::new(response.body_mut().as_reader());
        let response_body_file = stream_to_temp_file(response_body_reader, target_dir)?;

        // Verify file size if provided
        if let Some(expected_size) = download.size {
            let actual_size = response_body_file.as_file().metadata()?.len();
            if actual_size != expected_size {
                bail!(
                    "File size mismatch for {}: expected {} bytes, got {} bytes",
                    download.url,
                    expected_size,
                    actual_size
                );
            }
        }

        // TODO: Verify MD5 hash if provided
        // This would require reading the file and computing the hash

        let magic_bytes =
            read_magic_bytes(response_body_file.as_file()).context("Failed to read magic bytes")?;

        if looks_like_zip(content_type.as_deref(), &magic_bytes, &download.url) {
            // Extract ZIP archive directly to asset directory
            let archive_file = response_body_file.reopen()?;
            let archive_file_reader = io::BufReader::new(archive_file);
            extract_zip_archive(archive_file_reader, &asset_dir)
                .context("Failed to extract ZIP archive")?;
        } else {
            // Save individual file to the specified path within asset directory
            let file_path = asset_dir.join(&download.file_path);

            // Create parent directories if needed
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent)?;
            }

            persist_or_copy(response_body_file, &file_path)
                .context("Failed to persist downloaded asset file")?;
        }
    }

    println!(
        "Successfully fetched asset '{}' ({} files)",
        asset.name,
        downloads.len()
    );
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
