use std::fs::File;
use std::io::Read;
use std::path::Path;

use base64;
use failure::Error;
use reqwest::Client;

const ENDPOINT: &str = "https://vision.googleapis.com/v1/images:annotate";

#[derive(Deserialize, Debug)]
pub struct Image {
    pub url: String,
}

#[derive(Deserialize, Debug)]
pub struct Matching {
    #[serde(default)]
    #[serde(rename = "fullMatchingImages")]
    pub full_matching_images: Vec<Image>,
}

#[derive(Deserialize, Debug)]
pub struct Detections {
    #[serde(rename = "webDetection")]
    pub web_detection: Matching,
}

#[derive(Deserialize, Debug, Fail)]
#[fail(display = "Request error: {}", message)]
pub struct RequestError {
    message: String,
}

#[derive(Deserialize, Debug)]
pub struct Responses {
    pub responses: Option<Vec<Detections>>,
    pub error: Option<RequestError>,
}

/// Return all images that fully match by doing a reverse image search using the Vision API. Sorted by resolution in descending order.
pub fn matching_images(path: &Path, api_key: &str) -> Result<Vec<Image>, Error> {
    // Read the image into a Vec.
    let mut buf = Vec::new();

    // NOT FATAL: The other images may succesfully open and be readable.
    File::open(path)?.read_to_end(&mut buf)?;

    // Assemble URL with API key.
    let endpoint = format!("{}?key={}", ENDPOINT, api_key);

    // Assemble request body.
    // TODO: Use `serde_json` to serialise a response.
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
    let values = res.json::<Responses>()
        .expect("The API sent an unexpected response.");

    // TODO: Find a better solution.
    if let Some(error) = values.error {
        Err(error)?
    } else if let Some(mut responses) = values.responses {
        Ok(responses.swap_remove(0).web_detection.full_matching_images)
    } else {
        panic!("The API sent an unexpected response.")
    }
}
