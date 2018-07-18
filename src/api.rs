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
    features: Vec<FeatureType<'a>>,
}

#[derive(Serialize, Debug)]
struct Content {
    content: String,
}

#[derive(Serialize, Debug)]
struct FeatureType<'a> {
    #[serde(rename = "type")]
    feature_type: &'a str,

    #[serde(rename = "maxResults")]
    max_results: u32,
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
pub struct Image {
    pub url: String,
}

/// Return all images that fully match by doing a reverse image search using the Vision API. Sorted by resolution in descending order.
pub fn matching_images(client: &Client, buf: &[u8], key: &str) -> Result<Vec<Image>, Error> {
    // Assemble URL with API key.
    let endpoint = format!("{}?key={}", ENDPOINT, key);

    // Serialize request body.
    let json = json!(Annotate {
        requests: vec![Request {
            image: Content {
                content: base64::encode(&buf),
            },
            features: vec![FeatureType {
                feature_type: "WEB_DETECTION",
                max_results: 1000,
            }],
        }],
    });

    // Assemble request and send it.
    let mut req = client
        .post(endpoint.as_str())
        .body(json.to_string())
        .send()?;

    // Deserialise the JSON into Responses.
    let res = req.json::<Responses>()
        .expect("The API response could not be deserialised.");

    if req.status().is_success() {
        Ok(res.responses
            .unwrap()
            .remove(0)
            .web_detection
            .full_matching_images)
    } else {
        Err(res.error.unwrap())?
    }
}
