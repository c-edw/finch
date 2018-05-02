extern crate base64;
extern crate image;
extern crate indicatif;
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
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use reqwest::Client;
use structopt::StructOpt;
use walkdir::{DirEntry, WalkDir};

use std::env;
use std::error::Error;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

// List of file types supported by the Vision API.
const SUPPORTED: [&str; 6] = ["jpg", "jpeg", "png", "raw", "ico", "bmp"];

#[derive(StructOpt, Debug)]
#[structopt(name = "finch")]
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

    // Walk the target directory and create a Vec<DirEntry>.
    let dirs = WalkDir::new(&cur)
        .into_iter()
        .map(|dir| dir.unwrap())
        .filter(|dir| dir.file_type().is_file())
        .collect::<Vec<DirEntry>>();

    let bar = ProgressBar::new(dirs.len() as u64);

    bar.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {msg}")
            .progress_chars("|| "),
    );

    bar.enable_steady_tick(1000);

    dirs.par_iter().for_each(|dir| {
        let path = dir.path();

        if let Some(extension) = path.extension() {
            // Only process if the Path is a file and the type supported by the API.
            if is_supported(extension) {
                process_file(path, &opt.key).ok();
                bar.set_message(path.strip_prefix(&cur).unwrap().to_str().unwrap());
            }
        }

        bar.inc(1);
    });

    bar.finish_with_message(format!("Completed processing {} images.", dirs.len()).as_str());
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

    // Delete original file, discard Error.
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
