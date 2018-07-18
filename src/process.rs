use api;
use hash::{Algorithm, PerceptualHash};
use Opt;

use failure::Error;
use image::{self, GenericImage};
use reqwest::Client;
use walkdir::DirEntry;

use std::fs::File;
use std::io::Read;
use std::path::Path;

// List of file types supported by both the Vision API, and the `image` crate.
const SUPPORTED: [&str; 7] = ["jpg", "jpeg", "png", "webp", "gif", "ico", "bmp"];

// Maximum file size to upload (10MB) defined by the Vision API.
const MAX_FILESIZE: u64 = 10 * 1024 * 1024;

/// Returns whether a path is a file.
pub fn is_file(dir: &DirEntry) -> bool {
    dir.file_type().is_file()
}

/// Returns whether the file type is supported by the Vision API.
pub fn is_supported(dir: &DirEntry) -> bool {
    SUPPORTED.contains(&dir.path()
        .extension()
        .and_then(|n| n.to_str())
        .map(|n| n.to_lowercase())
        .unwrap_or_default()
        .as_str())
}

/// Returns whether a file is within the filesize limit.
pub fn is_within_filesize_limit(dir: &DirEntry) -> bool {
    dir.metadata().map(|n| n.len()).unwrap_or(0) <= MAX_FILESIZE
}

pub fn process_file(client: &Client, path: &Path, opts: &Opt) -> Result<(), Error> {
    let mut prev_file = File::open(path)?;

    let mut prev_buf = Vec::new();
    prev_file.read_to_end(&mut prev_buf)?;

    let prev_img = image::load_from_memory(&prev_buf)?;

    debug!("Querying {} with Vision API.", path.display());

    let images = api::matching_images(&client, &prev_buf, &opts.key)?;

    // Iterate over each version of an image, starting with the highest resolution/most similar.
    for image in images {
        debug!("Checking version {} for {}.", path.display(), image.url);

        // Get the image from the URL.
        let mut req = match client.get(&image.url).send() {
            Ok(req) => req,
            Err(_) => continue,
        };

        // If the webserver sent a bad status code, skip this image.
        let mut req = match req.error_for_status() {
            Ok(req) => req,
            Err(_) => continue,
        };

        let mut new_buf = Vec::new();
        // Copy the request data into a buffer.
        req.copy_to(&mut new_buf)
            .expect("Failed to copy image data.");

        // Load the request body as an image. This can fail if the fetched image is not a supported format.
        let new_img = match image::load_from_memory(&new_buf) {
            Ok(new_img) => new_img,
            Err(_) => continue,
        };

        if new_img.dimensions() > prev_img.dimensions() {
            debug!("Comparing version {} for {}.", path.display(), image.url);

            // Only calculate the hashes if we know it's a higher resolution.
            let prev_hash = prev_img.hash(Algorithm::Marr);
            let new_hash = new_img.hash(Algorithm::Marr);

            let similarity = prev_hash.similarity(&new_hash);

            // The similarity will be lower if the webserver has served a dummy image, or it is watermarked.
            if similarity > opts.tolerance {
                info!("Saving version {} for {}.", path.display(), image.url);

                // Write out the better image.
                image::save_buffer(
                    path,
                    &new_img.raw_pixels(),
                    new_img.width(),
                    new_img.height(),
                    new_img.color(),
                )?;

                // A higher resolution image has been found, so we can break out.
                break;
            } else {
                info!(
                    "Version was not similar enough to the target image ({}).",
                    similarity
                );
            }
        } else {
            // There are no more higher resolution versions left to iterate over.
            break;
        }
    }

    Ok(())
}
