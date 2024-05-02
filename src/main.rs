use std::cmp;
use std::env;
use std::collections::HashMap;
use std::sync::Mutex;

use dotenv;

use serde::Deserialize;
use serde_json::from_value;
use urlencoding::encode;
use uuid::Uuid;

use warp::Filter;
use lazy_static::lazy_static;

mod spotify_client;
use crate::spotify_client::{SpotifyClient, SpotifySearchResponse};

#[derive(Debug, Deserialize)]
struct SelectQuery {
    titles: String,
}

#[derive(Debug, Deserialize)]
struct DownloadQuery {
    indices: Vec<i8>,
    session_id: String,
}

#[derive(Debug)]
struct Choice {
    r#type: String,
    id: String,
}

lazy_static! {
    static ref SESSION_CHOICES: Mutex<HashMap<String, Vec<Vec<Choice>>>> = Mutex::new(HashMap::new());
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    let select_route = warp::post()
        .and(warp::path("select"))
        .and(warp::body::json())
        .and_then(select_music);
    let download_route = warp::post()
        .and(warp::path("download"))
        .and(warp::body::json())
        .and_then(download_music);
    let routes = select_route.or(download_route);

    warp::serve(routes).run(([0, 0, 0, 0], 8080)).await;
}

async fn select_music(body: SelectQuery) -> Result<impl warp::Reply, warp::Rejection> {
    let client_id = env::var("SPOTIFY_CLIENT_ID").expect("Expected a client id");
    let secret = env::var("SPOTIFY_CLIENT_SECRET").expect("Expected a secret");
    let client = SpotifyClient::new(client_id, secret);

    let titles = body.titles.split('\n');

    let mut results: Vec<SpotifySearchResponse> = vec![];
    for title in titles {
        match client
            .api_req(&format!(
                "/search?q={}&type=track,album,artist",
                encode(title.trim())
            ))
            .await
        {
            Ok(res) => match from_value::<SpotifySearchResponse>(res) {
                Ok(result) => results.push(result),
                Err(e) => println!("Failed to parse JSON: {:?}", e),
            },
            Err(e) => println!("Error: {:?}", e),
        }
    }

    let mut session: Vec<Vec<Choice>> = vec![];
    for result in results {
        let tracks = result.tracks.unwrap().items;
        let albums = result.albums.unwrap().items;
        let artists = result.artists.unwrap().items;

        let mut track_count = 10;
        let mut album_count = 5;
        let mut artist_count = 3;

        if albums.len() < album_count {
            track_count += album_count - albums.len();
            album_count = albums.len();
        }
        if artists.len() < artist_count {
            track_count += artist_count - artists.len();
            artist_count = artists.len();
        }

        let mut choices: Vec<Choice> = vec![];
        for i in 0..cmp::min(track_count, tracks.len()) {
            println!(
                "Track: {} - {} [{}]",
                tracks[i].name, tracks[i].album.artists[0].name, tracks[i].album.name
            );
            choices.push(Choice {
                r#type: "track".to_string(),
                id: tracks[i].id.clone(),
            });
        }
        for i in 0..cmp::min(album_count, albums.len()) {
            println!("Album: {} - {}", albums[i].name, albums[i].artists[0].name);
            choices.push(Choice {
                r#type: "album".to_string(),
                id: albums[i].id.clone(),
            });
        }
        for i in 0..cmp::min(artist_count, artists.len()) {
            println!("Artist: {}", artists[i].name);
            choices.push(Choice {
                r#type: "artist".to_string(),
                id: artists[i].id.clone(),
            });
        }
        session.push(choices);
    }
    let session_id = Uuid::new_v4().to_string();

    SESSION_CHOICES.lock().unwrap().insert(session_id.clone(), session);

    Ok(warp::reply::json(&session_id))
}

async fn download_music(body: DownloadQuery) -> Result<impl warp::Reply, warp::Rejection> {
    let session_id = body.session_id;
    let indices: Vec<i8> = body.indices;

    let mutex_guard = match SESSION_CHOICES.lock() {
        Ok(guard) => guard,
        Err(_) => return Ok(warp::reply::json(&"Failed to lock mutex".to_string())),
    };

    let session = match mutex_guard.get(&session_id) {
        Some(session) => session,
        None => return Ok(warp::reply::json(&"Session not found".to_string())),
    };

    let it = indices.iter().zip(session.iter());

    for (_, (idx, choices)) in it.enumerate() {
        let choice = &choices[*idx as usize];
        println!("{}: {}",choice.r#type, choice.id);
    }
    Ok(warp::reply::json(&session_id))
}
