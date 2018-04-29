extern crate base64;
extern crate image;
extern crate reqwest;
extern crate serde;
extern crate walkdir;

#[macro_use]
extern crate structopt;

#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate serde_json;

use structopt::StructOpt;
use reqwest::Client;
use walkdir::WalkDir;
use image::GenericImage;

use std::fs::{self, File};
use std::error::Error;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::env;
use std::ffi::OsStr;
use std::thread;

// List of file types supported by the Vision API.
const SUPPORTED: [&str; 6] = ["jpg", "jpeg", "png", "raw", "ico", "bmp"];

#[derive(StructOpt, Debug)]
#[structopt(name = "args")]
struct Opt {
    /// Your Google Vision API key.
    #[structopt(short = "k", long = "key")]
    key: String,

    /// Target directory containing images to enhance.
    #[structopt(name = "DIRECTORY", default_value = "./", parse(from_os_str))]
    dir: PathBuf,
}

fn main() {
    let opt = Opt::from_args();

    let mut cur = env::current_dir().unwrap();
    cur.push(&opt.dir);

    let mut threads = Vec::new();

    for entry in WalkDir::new(cur) {
        threads.push(thread::spawn(move || {
            let opt = Opt::from_args();

            let entry = entry.unwrap();
            let path = entry.path();

            // Some files do not have extensions.
            if let Some(extension) = path.extension() {
                // Only process if the Path is a file and the type supported by the API.
                if entry.file_type().is_file() && is_supported(extension) {
                    println!(
                        "Processing {}...",
                        &path.file_name().unwrap().to_str().unwrap()
                    );
                    process_file(path, &opt.key).ok();
                }
            }
        }));
    }

    for thread in threads {
        thread.join().unwrap();
    }

    println!("Done!");
}

/// Returns whether the file type is supported by the Vision API.
fn is_supported(ext: &OsStr) -> bool {
    SUPPORTED.contains(&ext.to_str().unwrap().to_lowercase().as_str())
}

fn process_file(path: &Path, key: &str) -> Result<(), Box<Error>> {
    let mut file = File::open(path)?;

    // Read the image into the vector.
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;

    // Grab the highest resolution version of this image.
    let output = get_highest_res(&buf, key)?;

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

    Ok(())
}

#[derive(Deserialize)]
struct Images {
    url: String,
}

#[derive(Deserialize)]
struct Matching {
    #[serde(rename = "fullMatchingImages")]
    full_matching_images: Vec<Images>,
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

/// Return the highest resolution version of an image buffer by doing a reverse image search using the Vision API.
fn get_highest_res(buf: &[u8], key: &str) -> Result<Vec<u8>, Box<Error>> {
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

    // Deserialise the JSON into a Value.
    let values = res.json::<Responses>()?;

    // Get the URL of the first image in the list.
    // Returned images are sorted in descending order of resolution, so we can just take the first index.
    let mut new = reqwest::get(&values.responses[0].web_detection.full_matching_images[0].url)?;

    let mut buf = Vec::new();
    new.copy_to(&mut buf).unwrap();

    Ok(buf)
}
