extern crate base64;
extern crate clap;
extern crate glob;
extern crate image;
extern crate reqwest;

#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;

use reqwest::Client;
use image::{DynamicImage, GenericImage};
use base64::encode;
use glob::glob;
use clap::{App, Arg};

use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::env::current_dir;

fn main() {
    let matches = App::new("Finch")
        .version("0.1.0")
        .about("A tool for enhancing and organising an image collection using Google's Vision API.")
        .arg(
            Arg::with_name("key")
                .short("k")
                .long("key")
                .value_name("key")
                .required(true)
                .help("Sets the Google Vision API key."),
        )
        .get_matches();

    let key = matches.value_of("key").unwrap();

    // TODO: Surely we can do this better.
    let iter = glob(current_dir().unwrap().join("**/*").to_str().unwrap()).unwrap();

    for entry in iter {
        process_file(&entry.unwrap(), key);
    }
}

fn process_file(file: &Path, key: &str) {
    // Create vector to hold image data.
    let mut buf = Vec::new();

    // Read the image into the vector.
    File::open(file)
        .expect("Unable to find input file.")
        .read_to_end(&mut buf)
        .unwrap();

    // Read the image into the vector.
    let input = image::load_from_memory(&buf).unwrap();

    // Grab the highest resolution version of this image.
    let output = get_highest_res(&buf, key);

    // Only bother replacing the file if it's a higher resolution.
    if output.width() > input.width() && output.height() > input.height() {
        image::save_buffer(
            file.with_extension("png"),
            output.raw_pixels().as_slice(),
            output.width(),
            output.height(),
            output.color(),
        ).unwrap();
    }
}

#[derive(Deserialize)]
struct Images {
    url: String,
}

#[allow(non_snake_case)]
#[derive(Deserialize)]
struct Matching {
    fullMatchingImages: Vec<Images>,
}

#[allow(non_snake_case)]
#[derive(Deserialize)]
struct Detections {
    webDetection: Matching,
}

#[derive(Deserialize)]
struct Responses {
    responses: Vec<Detections>,
}

/// Use the Google Vision API do a reverse image search, and get the highest resolution version of an image.
fn get_highest_res(buf: &[u8], key: &str) -> DynamicImage {
    // Encode image buffer as base64.
    let buf = encode(&buf);

    // Construct request body.
    let json = json!({
        "requests": [{
            "image": {
                "content": &buf
            },
            "features": [{
                "type": "WEB_DETECTION"
            }]
        }]
    });

    // Assemble request and send it.
    let mut res = Client::new()
        .post(&format!(
            "https://vision.googleapis.com/v1/images:annotate?key={}",
            key
        ))
        .body(json.to_string())
        .send()
        .unwrap();

    // Deserialise the JSON into Responses.
    let values = res.json::<Responses>().unwrap();

    // Get the URL of the first matching image returned.
    let url = values.responses[0].webDetection.fullMatchingImages[0]
        .url
        .to_owned();

    let mut buf = Vec::new();

    let mut result = reqwest::get(&url).unwrap();
    result.copy_to(&mut buf).unwrap();

    image::load_from_memory(buf.as_slice()).unwrap()
}
