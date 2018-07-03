use std::fs::File;
use std::io::Read;
use std::path::Path;

use base64;
use failure::Error;
use reqwest::Client;

const ENDPOINT: &str = "https://vision.googleapis.com/v1/images:annotate";

#[derive(Serialize, Debug)]
struct Annotate<'a> {
    requests: Vec<Request<'a>>,
}

#[derive(Serialize, Debug)]
struct Request<'a> {
    image: Content,
    features: Vec<Type<'a>>,
}

#[derive(Serialize, Debug)]
struct Content {
    content: String,
}

#[derive(Serialize, Debug)]
struct Type<'a> {
    #[serde(rename = "type")]
    feature_type: &'a str,
}

#[derive(Deserialize, Debug)]
struct Responses {
    responses: Option<Vec<Detections>>,
    error: Option<RequestError>,
}

#[derive(Deserialize, Debug)]
struct Detections {
    #[serde(rename = "webDetection")]
    web_detection: Matching,
}

#[derive(Deserialize, Debug, Fail)]
#[fail(display = "{}", message)]
struct RequestError {
    message: String,
}

#[derive(Deserialize, Debug)]
struct Matching {
    #[serde(default)]
    #[serde(rename = "fullMatchingImages")]
    full_matching_images: Vec<Image>,
}

#[derive(Deserialize, Debug)]
struct Image {
    url: String,
}

/// Return all images that fully match by doing a reverse image search using the Vision API. Sorted by resolution in descending order.
pub fn matching_images(path: &Path, key: &str) -> Result<Vec<String>, Error> {
    // Read the image into a Vec.
    let mut buf = Vec::new();

    // NOT FATAL: The other images may succesfully open and be readable.
    File::open(path)?.read_to_end(&mut buf)?;

    // Assemble URL with API key.
    let endpoint = format!("{}?key={}", ENDPOINT, key);

    // Serialize request body.
    let json = json!(Annotate {
        requests: vec![Request {
            image: Content {
                content: base64::encode(&buf),
            },
            features: vec![Type {
                feature_type: "WEB_DETECTION",
            }],
        }],
    });

    debug!("Querying {} with Vision API.", path.display());

    // Assemble request and send it.
    let mut res = Client::new()
        .post(endpoint.as_str())
        .body(json.to_string())
        .send()?;

    // Deserialise the JSON into Responses.
    let values = res.json::<Responses>()
        .expect("The API response could not be deserialised.");

    // TODO: Find a better solution.
    if let Some(error) = values.error {
        Err(error)?
    } else if let Some(responses) = values.responses {
        Ok(responses
            .first()
            .expect("The API returned results for zero queries.")
            .web_detection
            .full_matching_images
            .iter()
            .map(|n| n.url.to_owned())
            .collect())
    } else {
        panic!("The API sent an unexpected response.")
    }
}
