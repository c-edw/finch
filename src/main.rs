extern crate base64;
extern crate image;
extern crate reqwest;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate glob;

use reqwest::Client;
use base64::encode;
use image::{DynamicImage, GenericImage};
use glob::glob;

use std::fs::File;
use std::io::Read;

const API_KEY: &str = "";

fn main() {
    for entry in glob("assets/*").unwrap() {
        if let Ok(file) = entry {
            let name = file.to_str().unwrap();
            process(name);
            println!("Processed {}.", name);
        }
    }
}

fn process(file: &str) {
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
    let output = get_highest_res(&buf);

    if output.width() > input.width() && output.height() > input.height() {
        image::save_buffer(
            file,
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
fn get_highest_res(buf: &[u8]) -> DynamicImage {
    // Construct request body.
    let json = json!({
        "requests": [{
            "image": {
                "content": encode(&buf)
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
            API_KEY
        ))
        .body(json.to_string())
        .send()
        .unwrap();

    // Deserialise the JSON into Responses.
    let values = res.json::<Responses>().unwrap();

    // Get the URL of the first matching image returned.
    let url = values.responses[0]
        .webDetection
        .fullMatchingImages[0]
        .url
        .to_owned();

    let mut buf = Vec::new();

    let mut result = reqwest::get(&url).unwrap();
    result.copy_to(&mut buf).unwrap();

    image::load_from_memory(buf.as_slice()).unwrap()
}
