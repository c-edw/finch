#![feature(extern_prelude)]

mod api;
mod hash;

extern crate base64;
extern crate image;
extern crate rayon;
extern crate reqwest;
extern crate serde;
extern crate walkdir;

#[macro_use]
extern crate structopt;

#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate serde_json;

use image::GenericImage;
use structopt::StructOpt;
use walkdir::{DirEntry, WalkDir};

use std::env;
use std::error::Error;
use std::path::{Path, PathBuf};

// List of file types supported by both the Vision API, and the `image` crate.
const SUPPORTED: [&str; 7] = ["jpg", "jpeg", "png", "webp", "gif", "ico", "bmp"];

#[derive(StructOpt, Debug)]
#[structopt(name = "finch")]
struct Opt {
    /// Your Google Vision API key.
    #[structopt(short = "k", long = "api_key")]
    api_key: String,

    /// Target directory containing images to enhance.
    #[structopt(name = "DIRECTORY", default_value = ".", parse(from_os_str))]
    dir: PathBuf,

    /// Similarity tolerance. You can probably leave this alone.
    #[structopt(short = "t", long = "tolerance", default_value = "0.9")]
    tolerance: f32,
}

fn main() {
    let opts = Opt::from_args();

    // Get the current working directory. This can fail if the directory does not exist.
    let mut cur = env::current_dir().expect("The current working directory is invalid.");
    cur.push(&opts.dir);

    WalkDir::new(&cur)
        .into_iter()
        .filter_map(|dir| dir.ok())
        .filter(is_supported)
        .filter(is_file)
        .map(|dir| dir.path().to_owned())
        .for_each(|path| {
            process_file(&path, &opts).unwrap_or_else(|_| {
                println!("Failed to process {}, continuing...", path.display())
            });
        });
}

fn is_file(dir: &DirEntry) -> bool {
    dir.file_type().is_file()
}

/// Returns whether the file type is supported by the Vision API.
fn is_supported(dir: &DirEntry) -> bool {
    // Get the path extension. This can fail if it does not have an extension.
    let ext = match dir.path().extension() {
        Some(ext) => ext,
        None => return false,
    };

    SUPPORTED.contains(&ext.to_str().unwrap().to_lowercase().as_str())
}

fn process_file(path: &Path, opts: &Opt) -> Result<(), Box<Error>> {
    let prev = image::open(path)?;
    let prev_hash = hash::average_hash(&prev);

    let matching = api::get_matching_urls(path, &opts.api_key)?;

    // Iterate over each version of an image, starting with the highest resolution/most similar.
    for image in matching.iter() {
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
        let new_hash = hash::average_hash(&new);

        // Only bother saving the image if it's a greater resolution.
        if new.dimensions() > prev.dimensions() {
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
            }
        } else {
            // There are no more higher resolution versions left to iterate over.
            break;
        }
    }

    Ok(())
}
