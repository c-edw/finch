#![feature(extern_prelude)]

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
use rayon::prelude::*;
use reqwest::Client;
use structopt::StructOpt;
use walkdir::{DirEntry, WalkDir};

use std::env;
use std::error::Error;
use std::ffi::OsStr;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

// List of file types supported by the Vision API.
const SUPPORTED: [&str; 5] = ["jpg", "jpeg", "png", "ico", "bmp"];

#[derive(StructOpt, Debug)]
#[structopt(name = "finch")]
struct Opt {
    /// Your Google Vision API key.
    #[structopt(short = "k", long = "key")]
    key: String,

    /// Similarity tolerance. You can probably leave this alone.
    #[structopt(short = "t", long = "tolerance", default_value = "0.9")]
    tolerance: f32,

    /// Target directory containing images to enhance.
    #[structopt(name = "DIRECTORY", default_value = ".", parse(from_os_str))]
    dir: PathBuf,
}

fn main() {
    let opt = Opt::from_args();

    let mut cur = env::current_dir().unwrap();
    cur.push(&opt.dir);

    // Walk the target directory and create a Vec<DirEntry>.
    let dirs = WalkDir::new(&cur)
        .into_iter()
        .map(|dir| dir.unwrap())
        .filter(|dir| dir.file_type().is_file())
        .collect::<Vec<DirEntry>>();

    // Iterate over the directories in parallel.
    dirs.par_iter().for_each(|dir| {
        let path = dir.path();

        if let Some(extension) = path.extension() {
            // Only process if the Path is a file and the type supported by the API.
            if is_supported(extension) {
                process_file(path, &opt.key).unwrap();
            }
        }
    });
}

/// Returns whether the file type is supported by the Vision API.
fn is_supported(ext: &OsStr) -> bool {
    SUPPORTED.contains(&ext.to_str().unwrap().to_lowercase().as_str())
}

fn process_file(path: &Path, key: &str) -> Result<(), Box<Error>> {
    let mut file = File::open(path)?;

    // Read the image into a Vec.
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;

    let prev = image::load_from_memory(&buf)?;
    let prev_hash = hash::average_hash(&prev);

    if let Ok(versions) = get_versions(buf, key) {
        // Iterate over each version of an image, starting with the highest resolution/most similar.
        for image in versions.iter() {
            let mut req = reqwest::get(&image.url)?;

            let mut buf = Vec::new();
            req.copy_to(&mut buf)?;

            // The fetched image may not be a supported format.
            if let Ok(new) = image::load_from_memory(&buf) {
                let new_hash = hash::average_hash(&new);

                // Only bother saving the image if it's a greater resolution.
                if new.dimensions() > prev.dimensions() {
                    // The similarity will be lower if the webserver has served a dummy image, or it is watermarked.
                    if prev_hash.similarity(&new_hash) > 0.9 {
                        // Write out new image.
                        image::save_buffer(
                            path,
                            &new.raw_pixels(),
                            new.width(),
                            new.height(),
                            new.color(),
                        ).unwrap();
                    }
                } else {
                    // There are no more higher resolution versions left to iterate over.
                    break;
                }
            }
        }
    }

    Ok(())
}

#[derive(Deserialize, Clone)]
struct Image {
    url: String,
}

#[derive(Deserialize)]
struct Matching {
    #[serde(rename = "fullMatchingImages")]
    full_matching_images: Vec<Image>,
}

#[derive(Deserialize)]
struct Detections {
    #[serde(rename = "webDetection")]
    web_detection: Matching,
}

#[derive(Deserialize)]
struct Responses {
    responses: Vec<Detections>,
}

/// Return all versions of an image found by doing a reverse image search using the Vision API.
fn get_versions(buf: Vec<u8>, key: &str) -> Result<Vec<Image>, Box<Error>> {
    // Assemble URL with API key.
    let endpoint = format!(
        "https://vision.googleapis.com/v1/images:annotate?key={}",
        key
    );

    // Assemble request body.
    let json = json!({
       "requests": [{
            "image": { 
                "content": base64::encode(&buf) 
            },
            "features": [
                { "type": "WEB_DETECTION" }
            ]
        }]
    });

    // Assemble request and send it.
    let mut res = Client::new()
        .post(endpoint.as_str())
        .body(json.to_string())
        .send()?;

    // Deserialise the JSON into Responses.
    let values = res.json::<Responses>()?;

    Ok(values.responses[0]
        .web_detection
        .full_matching_images
        .clone())
}
