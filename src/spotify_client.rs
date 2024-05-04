use reqwest::{Client, Error};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Deserialize, Serialize, Debug)]
pub struct SpotifySearchResponse {
    pub tracks: Option<Items<Track>>,
    pub albums: Option<Items<Album>>,
    pub artists: Option<Items<Artist>>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Items<T> {
    pub items: Vec<T>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Track {
    pub id: String,
    pub name: String,
    pub album: Album,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Album {
    pub id: String,
    pub release_date: String,
    pub name: String,
    pub artists: Vec<Artist>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Artist {
    pub id: String,
    pub name: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct AlbumTrack {
    pub id: String,
    pub name: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ArtistAlbum {
    pub id: String,
    pub name: String,
}

pub struct SpotifyClient {
    client_id: String,
    secret: String,
    client: Client,
}

#[derive(Serialize)]
struct AuthRequest<'a> {
    grant_type: &'a str,
    client_id: &'a str,
    client_secret: &'a str,
}

#[derive(Deserialize)]
struct AuthResponse {
    access_token: String,
}

impl SpotifyClient {
    pub fn new(client_id: String, secret: String) -> SpotifyClient {
        SpotifyClient {
            client_id,
            secret,
            client: Client::new(),
        }
    }

    async fn get_access_token(&self) -> Result<String, Error> {
        let req = AuthRequest {
            grant_type: "client_credentials",
            client_id: &self.client_id,
            client_secret: &self.secret,
        };
        let res = self
            .client
            .post("https://accounts.spotify.com/api/token")
            .form(&req)
            .send()
            .await?
            .json::<AuthResponse>()
            .await?;
        Ok(res.access_token)
    }

    pub async fn api_req(&self, uri: &str) -> Result<Value, Error> {
        let token = self.get_access_token().await?;
        let url = format!("https://api.spotify.com/v1{}", uri);
        let res = self
            .client
            .get(&url)
            .header("Content-Type", "application/json")
            .bearer_auth(token)
            .send()
            .await?
            .json::<Value>()
            .await?;
        Ok(res)
    }
}
