use hash::{Algorithm, Hash};
use Opt;

use api;

use failure::Error;
use image::{self, GenericImage};
use reqwest;
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
    // Get the path extension. This can fail if it does not have an extension.
    let ext = match dir.path().extension() {
        Some(ext) => ext,
        None => return false,
    };

    // TODO: Remove this unwrap.
    SUPPORTED.contains(&ext.to_str().unwrap().to_lowercase().as_str())
}

/// Returns whether a file is within the filesize limit.
pub fn is_within_filesize_limit(dir: &DirEntry) -> bool {
    dir.metadata().map(|n| n.len()).unwrap_or(0) <= MAX_FILESIZE
}

pub fn process_file(path: &Path, opts: &Opt) -> Result<(), Error> {
    // NOT FATAL: The image might not be readable due to a permissions error.
    let prev = image::open(path)?;

    // NOT FATAL: The image might not be known to exist anywhere else by Google.
    let images = api::matching_images(path, &opts.key)?;

    // Iterate over each version of an image, starting with the highest resolution/most similar.
    for image in images {
        // Get the image from the URL.
        // NOT FATAL: This can fail if the webserver is down.
        let mut req = match reqwest::get(&image) {
            Ok(req) => req,
            Err(_) => continue,
        };

        let mut buf = Vec::new();
        // Copy the request data into a buffer.
        // FATAL: The system may be out of memory or be in an unstable state.
        req.copy_to(&mut buf).expect("Failed to copy image data.");

        // Load the request body as an image. This can fail if the fetched image is not a supported format.
        // NOT FATAL: The image might be an unsupported format.
        let new = match image::load_from_memory(&buf) {
            Ok(buf) => buf,
            Err(_) => continue,
        };

        if new.dimensions() > prev.dimensions() {
            // Only calculate the hashes if we know it's a higher resolution.
            let prev_hash = prev.hash(Algorithm::Marr);
            let new_hash = new.hash(Algorithm::Marr);

            // The similarity will be lower if the webserver has served a dummy image, or it is watermarked.
            if prev_hash.similarity(&new_hash) > opts.tolerance {
                // Write out the better image.
                // NOT FATAL: This image could not be saved, but other images that are processing might be successfully saved.
                image::save_buffer(
                    path,
                    &new.raw_pixels(),
                    new.width(),
                    new.height(),
                    new.color(),
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
