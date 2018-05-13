use hash::Hash;
use Opt;

use image::GenericImage;
use walkdir::DirEntry;

use std::path::Path;

// List of file types supported by both the Vision API, and the `image` crate.
const SUPPORTED: [&str; 7] = ["jpg", "jpeg", "png", "webp", "gif", "ico", "bmp"];

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

    SUPPORTED.contains(&ext.to_str().unwrap().to_lowercase().as_str())
}

#[derive(Debug)]
pub enum ProcessError {
    ImageError(image::ImageError),
    APIError(::api::APIError),
}

impl From<image::ImageError> for ProcessError {
    fn from(error: image::ImageError) -> Self {
        ProcessError::ImageError(error)
    }
}

impl From<::api::APIError> for ProcessError {
    fn from(error: ::api::APIError) -> Self {
        ProcessError::APIError(error)
    }
}

pub fn process_file(path: &Path, opts: &Opt) -> Result<(), ProcessError> {
    let prev = image::open(path)?;
    let images = ::api::get_matching_urls(path, &opts.api_key)?;

    // Iterate over each version of an image, starting with the highest resolution/most similar.
    for image in images {
        // Get the image from the URL. This can fail if the webserver is down.
        let mut req = match reqwest::get(&image.url) {
            Ok(req) => req,
            Err(_) => continue,
        };

        let mut buf = Vec::new();
        // Copy the request data into a buffer. This should not fail under normal circumstances.
        req.copy_to(&mut buf).expect("Failed to copy image data.");

        // Load the request body as an image. This can fail if the fetched image is not a supported format.
        let new = match image::load_from_memory(&buf) {
            Ok(buf) => buf,
            Err(_) => continue,
        };

        if new.dimensions() > prev.dimensions() {
            // Only calculate the hashes if we know it's a higher resolution.
            let prev_hash = prev.average_hash();
            let new_hash = new.average_hash();

            // The similarity will be lower if the webserver has served a dummy image, or it is watermarked.
            if prev_hash.similarity(&new_hash) > opts.tolerance {
                // Write out new image.
                image::save_buffer(
                    path,
                    &new.raw_pixels(),
                    new.width(),
                    new.height(),
                    new.color(),
                ).expect("Unable to save image to target directory, exiting...");

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
