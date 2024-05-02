// use tide::prelude::*;
// use tide::Request;

// use async_std::task;
// use futures::future::try_join_all;
use serde::Deserialize;
use serde_json::from_value;
use urlencoding::encode;

use warp::Filter;

use std::cmp;
use std::env;

mod spotify_client;
use crate::spotify_client::{SpotifyClient, SpotifySearchResponse};

// use spotify_client::SpotifyClient;

#[derive(Debug, Deserialize)]
struct Query {
    titles: String,
}

#[tokio::main]
async fn main() {
    let route = warp::post()
        .and(warp::path("select"))
        .and(warp::body::json())
        .and_then(select_music);

    warp::serve(route).run(([0, 0, 0, 0], 8080)).await;
}

async fn select_music(body: Query) -> Result<impl warp::Reply, warp::Rejection> {
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

        for i in 0..cmp::min(track_count, tracks.len()) {
            println!(
                "{} - {} [{}]",
                tracks[i].name, tracks[i].album.artists[0].name, tracks[i].album.name
            );
        }
        for i in 0..cmp::min(album_count, albums.len()) {
            println!("Album: {}", albums[i].name);
        }
        for i in 0..cmp::min(artist_count, artists.len()) {
            println!("Artist: {}", artists[i].name);
        }
    }

    // println!("{:?}", results);
    Ok(warp::reply::json(&""))
}
