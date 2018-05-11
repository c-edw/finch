use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use hyper::client::FutureResponse;
use hyper::Client;

const ENDPOINT: &str = "https://vision.googleapis.com/v1/images:annotate";

#[derive(Deserialize)]
pub struct Image {
    pub url: String,
}

#[derive(Deserialize)]
pub struct Matching {
    #[serde(rename = "fullMatchingImages")]
    pub full_matching_images: Vec<Image>,
}

#[derive(Deserialize)]
pub struct Detections {
    #[serde(rename = "webDetection")]
    pub web_detection: Matching,
}

#[derive(Deserialize)]
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
pub fn get_matching_urls(path: &Path, api_key: &str, core: tokio_core::reactor::Core) -> Result<FutureResponse, APIError> {
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

    
    let client = Client::new(&core.handle());

    let mut req = hyper::Request::new(
        hyper::Method::Get,
        "http://www.theuselessweb.com/".parse().unwrap(),
    );
    req.set_body(json.to_string());

    let done = client.request(req);

    Ok(done)
}
