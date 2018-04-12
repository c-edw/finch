extern crate base64;
extern crate clap;
extern crate image;
extern crate reqwest;
extern crate walkdir;

#[macro_use]
extern crate serde_json;

use reqwest::Client;
use base64::encode;
use walkdir::WalkDir;
use clap::{App, Arg};
use serde_json::Value;
use image::GenericImage;

use std::fs::{self, File};
use std::error::Error;
use std::io::Read;
use std::path::Path;
use std::env;
use std::ffi::OsStr;

// List of file types supported by the Vision API.
const SUPPORTED: [&str; 5] = ["jpg", "png", "raw", "ico", "bmp"];

fn main() {
    let matches = App::new("Finch")
        .version("0.2.1")
        .about("A tool for enhancing and organising an image collection using Google's Vision API.")
        .arg(
            Arg::with_name("key")
                .required(true)
                .short("k")
                .long("key")
                .value_name("key")
                .help("Defines the Google Vision API key."),
        )
        .get_matches();

    let key = matches.value_of("key").unwrap();
    let cur = env::current_dir().unwrap();

    for entry in WalkDir::new(cur) {
        let entry = entry.unwrap();
        let path = entry.path();

        // Only process if the Path is a file and the type supported by the API.
        if entry.file_type().is_file() && is_supported(&path.extension().unwrap()) {
            process_file(&path, key).unwrap();
        }
    }
}

/// Returns whether the file type is supported by the Vision API.
fn is_supported(ext: &OsStr) -> bool {
    SUPPORTED.contains(&ext.to_str().unwrap().to_lowercase().as_str())
}

fn process_file(path: &Path, key: &str) -> Result<(), Box<Error>> {
    let mut file = File::open(path).expect("Unable to find input file.");

    // Read the image into the vector.
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;

    // Grab the highest resolution version of this image.
    if let Some(output) = get_highest_res(&buf, key) {
        // Delete original file.
        fs::remove_file(path).ok();

        let image = image::load_from_memory(&output)?;

        // Write out new image as a PNG.
        image::save_buffer(
            path.with_extension("png"),
            &image.raw_pixels(),
            image.width(),
            image.height(),
            image.color(),
        )?;
    }

    Ok(())
}

/// Return the highest resolution version of an image buffer by doing a reverse image search using the Vision API.
fn get_highest_res(buf: &[u8], key: &str) -> Option<Vec<u8>> {
    // Assemble URL with API key.
    let url = format!(
        "https://vision.googleapis.com/v1/images:annotate?key={}",
        key
    );

    // Encode image buffer as base64.
    let buf = encode(&buf);

    // Assemble request body.
    let json = json!({
       "requests": [{
            "image": { 
                "content": &buf 
            },
            "features": [
                { "type": "WEB_DETECTION" }
            ]
        }]
    });

    // Assemble request and send it.
    let mut res = Client::new()
        .post(url.as_str())
        .body(json.to_string())
        .send()
        .unwrap();

    // Deserialise the JSON into Responses.
    let values = res.json::<Value>().unwrap();
    let matching = &values["responses"][0]["webDetection"]["fullMatchingImages"];

    // Get the URL of the first image returned.
    if let Some(v) = matching[0]["url"].as_str() {
        let mut new = reqwest::get(v).unwrap();

        let mut buf = Vec::new();
        new.copy_to(&mut buf).unwrap();

        Some(buf)
    } else {
        None
    }
}
