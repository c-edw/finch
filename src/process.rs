use api;
use hash::{Algorithm, Hash};
use Opt;

use failure::Error;
use image::{self, GenericImage};
use reqwest::Client;
use walkdir::DirEntry;

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
    // NOT FATAL: The image might not be readable due to a permissions error.
    let prev = image::open(path)?;

    // NOT FATAL: The image might not be known to exist anywhere else by Google.
    let images = api::matching_images(&client, path, &opts.key)?;

    // Iterate over each version of an image, starting with the highest resolution/most similar.
    for image in images {
        debug!("Checking version {} for {}.", path.display(), image.url);

        // Get the image from the URL.
        // If the webserver sent a bad status code, skip this image.
        // NOT FATAL: This can fail if the image is missing and a 404 is sent.
        let mut req = match client.get(&image.url).send()?.error_for_status() {
            Ok(req) => req,
            Err(_) => continue,
        };

        let mut buf = Vec::new();

        // Copy the request data into a buffer.
        // FATAL: The system may be out of memory or be in an unstable state.
        req.copy_to(&mut buf).expect("Failed to copy image data.");

        // Load the request body as an image. This can fail if the fetched image is not a supported format.
        // NOT FATAL: The image might be an unsupported format.
        let img = match image::load_from_memory(&buf) {
            Ok(img) => img,
            Err(_) => continue,
        };

        if img.dimensions() > prev.dimensions() {
            debug!("Comparing version {} for {}.", path.display(), image.url);

            // Only calculate the hashes if we know it's a higher resolution.
            let prev_hash = prev.hash(Algorithm::Marr);
            let new_hash = img.hash(Algorithm::Marr);

            // The similarity will be lower if the webserver has served a dummy image, or it is watermarked.
            if prev_hash.similarity(&new_hash) > opts.tolerance {
                debug!("Saving version {} for {}.", path.display(), image.url);

                // Write out the better image.
                // NOT FATAL: This image could not be saved, but other images that are processing might be successfully saved.
                image::save_buffer(
                    path,
                    &img.raw_pixels(),
                    img.width(),
                    img.height(),
                    img.color(),
                )?;

                // A higher resolution image has been found, so we can break out.
                break;
            }
        } else {
            // There are no more higher resolution versions left to iterate over.
            break;
        }
    }

    Ok(())
}
