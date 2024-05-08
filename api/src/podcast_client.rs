use reqwest::{header, Client, Error};
use serde::Deserialize;
use serde_json::{Value};
use sha1::{Digest, Sha1};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Deserialize)]
pub struct PodcastSearchResult {
    pub feeds: Vec<PodcastFeed>,
}

#[derive(Debug, Deserialize)]
pub struct PodcastInfo {
    pub feed: PodcastFeed,
}

#[derive(Debug, Deserialize)]
pub struct PodcastFeed {
    pub id: i64,
    pub title: String,
    pub author: String,
    pub description: String,
    pub url: String,
    pub artwork: String,
}

#[derive(Debug, Deserialize)]
pub struct PodcastEpisodes {
   pub items: Vec<Episode>,
}

#[derive(Debug, Deserialize)]
pub struct Episode {
    pub id: i64,
    pub title: String,
    pub link: String,
    pub description: String,
    pub enclosureUrl: String,
}

pub struct PodcastClient {
    api_key: String,
    secret: String,
    client: Client,
}

impl PodcastClient {
    pub fn new(api_key: String, secret: String) -> PodcastClient {
        PodcastClient {
            api_key,
            secret,
            client: Client::new(),
        }
    }

    fn create_headers(&self) -> header::HeaderMap {
        let unix_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();

        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::USER_AGENT,
            header::HeaderValue::from_static("Mozilla"),
        );
        headers.insert("X-Auth-Key", header::HeaderValue::from_str(&self.api_key).unwrap());
        headers.insert(
            "X-Auth-Date",
            header::HeaderValue::from_str(&unix_time.to_string()).unwrap(),
        );

        let auth_value = format!("{}{}{}", &self.api_key, &self.secret, unix_time);
        let mut hasher = Sha1::new();
        hasher.update(auth_value.as_bytes());
        let hash_result = hasher.finalize();
        let auth_header = format!("{:x}", hash_result);
        headers.insert(
            "Authorization",
            header::HeaderValue::from_str(&auth_header).unwrap(),
        );

        headers
    }

    pub async fn api_req(&self, uri: &str) -> Result<Value, Error> {
        let headers = self.create_headers();
        let url = format!("https://api.podcastindex.org/api/1.0{}", uri);

        let res = self
            .client
            .get(&url)
            .headers(headers)
            .send()
            .await?
            .json::<Value>()
            .await?;

        Ok(res)
    }
}
