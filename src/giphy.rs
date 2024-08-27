use anyhow::Result;
use reqwest::Client;
use serde::Deserialize;
use url::Url;
use url_macro::url;

pub struct Giphy<'a> {
    client: Client,
    token: &'a str,
    random_url: Url,
}

#[derive(Debug)]
pub struct Gif {
    pub url: String,
    pub alt_text: String,
}

#[derive(Debug, Deserialize)]
struct GifResponse {
    alt_text: String,
    images: Images,
}

#[derive(Debug, Deserialize)]
struct Response {
    data: GifResponse,
}

#[derive(Debug, Deserialize)]
struct Images {
    original: OriginalImage,
}

#[derive(Debug, Deserialize)]
struct OriginalImage {
    webp: String,
}

impl<'a> Giphy<'a> {
    pub fn new(giphy_token: &'a str) -> Giphy<'a> {
        Giphy {
            client: Client::builder().https_only(true).build().unwrap(),
            token: giphy_token,
            random_url: url!("https://api.giphy.com/v1/gifs/random"),
        }
    }

    pub async fn random(&self, search: &str) -> Result<Gif> {
        Ok(self
            .client
            .get(self.random_url.clone())
            .query(&[("api_key", self.token), ("tag", search), ("rating", "pg")])
            .send()
            .await?
            .json::<Response>()
            .await?
            .data
            .into())
    }
}

impl From<GifResponse> for Gif {
    fn from(value: GifResponse) -> Self {
        Gif {
            url: value.images.original.webp,
            alt_text: value.alt_text,
        }
    }
}
