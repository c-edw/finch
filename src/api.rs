use reqwest::Client;
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::Path;

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

/// Return all images that fully match by doing a reverse image search using the Vision API. Sorted by resolution in descending order.
pub fn get_matching_urls(path: &Path, api_key: &str) -> Result<Vec<Image>, Box<Error>> {
    // Read the image into a Vec.
    let mut buf = Vec::new();
    File::open(path)?.read_to_end(&mut buf)?;

    // Assemble URL with API key.
    let endpoint = format!(
        "https://vision.googleapis.com/v1/images:annotate?key={}",
        api_key
    );

    // Assemble request body.
    let json = json![{
       "requests": [{
            "image": { 
                "content": base64::encode(&buf) 
            },
            "features": [
                { "type": "WEB_DETECTION" }
            ]
        }]
    }];

    // Assemble request and send it.
    let mut req = Client::new()
        .post(endpoint.as_str())
        .body(json.to_string())
        .send()?;

    // Deserialise the JSON into Responses.
    let mut values = req.json::<Responses>()?;

    Ok(values
        .responses
        .swap_remove(0)
        .web_detection
        .full_matching_images)
}
