use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use base64;
use reqwest::{self, Client};

const ENDPOINT: &str = "https://vision.googleapis.com/v1/images:annotate";

#[derive(Deserialize, Debug)]
pub struct Image {
    pub url: String,
}

#[derive(Deserialize, Debug)]
pub struct Matching {
    #[serde(default)]
    #[serde(rename = "fullMatchingImages")]
    pub full_matching_images: Option<Vec<Image>>,
}

#[derive(Deserialize, Debug)]
pub struct Detections {
    #[serde(rename = "webDetection")]
    pub web_detection: Matching,
}

#[derive(Deserialize, Debug)]
pub struct Responses {
    pub responses: Vec<Detections>,
}

#[derive(Debug)]
pub enum APIError {
    IOError(io::Error),
    RequestError(reqwest::Error),
}

impl From<io::Error> for APIError {
    fn from(error: io::Error) -> Self {
        APIError::IOError(error)
    }
}

impl From<reqwest::Error> for APIError {
    fn from(error: reqwest::Error) -> Self {
        APIError::RequestError(error)
    }
}

/// Return all images that fully match by doing a reverse image search using the Vision API. Sorted by resolution in descending order.
pub fn matching_images(path: &Path, api_key: &str) -> Result<Option<Vec<Image>>, APIError> {
    // Read the image into a Vec.
    let mut buf = Vec::new();
    File::open(path)?.read_to_end(&mut buf)?;

    // Assemble URL with API key.
    let endpoint = format!("{}?key={}", ENDPOINT, api_key);

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
    let mut values = res.json::<Responses>()?;

    Ok(values
        .responses
        .swap_remove(0)
        .web_detection
        .full_matching_images)
}
