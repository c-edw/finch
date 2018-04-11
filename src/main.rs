extern crate base64;
extern crate clap;
extern crate glob;
extern crate image;
extern crate reqwest;

#[macro_use]
extern crate serde_json;

use reqwest::Client;
use base64::encode;
use glob::glob;
use clap::{App, Arg};
use serde_json::Value;
use image::GenericImage;

use std::fs::{self, File};
use std::io::{Error, Read};
use std::path::Path;
use std::env;

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
                .help("Defines the Google Vision API key."),
        )
        .get_matches();

    let key = matches.value_of("key").unwrap();

    // TODO: Surely we can do this better.
    let entries = glob(env::current_dir().unwrap().join("**/*.*").to_str().unwrap()).unwrap();

    for entry in entries {
        process_file(&entry.unwrap(), key).unwrap();
    }
}

fn process_file(path: &Path, key: &str) -> Result<(), Error> {
    let mut file = File::open(path).expect("Unable to find input file.");

    // Read the image into the vector.
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;

    // Grab the highest resolution version of this image.
    if let Some(output) = get_highest_res(&buf, key) {
        // Delete original file.
        fs::remove_file(path).unwrap();

        let image = image::load_from_memory(&output).unwrap();

        // Write out new image as a PNG.
        image::save_buffer(
            path.with_extension("png"),
            &image.raw_pixels(),
            image.width(),
            image.height(),
            image.color(),
        ).unwrap();
    }

    Ok(())
}

/// Use the Google Vision API do a reverse image search, and get the highest resolution version of an image.
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
