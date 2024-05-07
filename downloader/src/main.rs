use std::env;
use std::process::Command;

use dotenv;

// use serde::Deserialize;
use serde_json::from_value;

mod spotify_client;
use crate::spotify_client::{SpotifyClient, Tracks};

fn search_yt_music(
    track_name: &String,
    album_name: &String,
    artist_name: &String,
) -> Result<String, std::io::Error> {
    let output = Command::new("python3")
        .arg("scripts/yt-music.py")
        .arg(track_name)
        .arg(album_name)
        .arg(artist_name)
        .output()
        .expect("Failed to execute command");

    if output.status.success() {
        let output_str = String::from_utf8_lossy(&output.stdout);
        Ok(output_str.to_string())
    } else {
        let error_message = String::from_utf8_lossy(&output.stderr);
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            error_message.to_string(),
        ))
    }
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    let client_id = env::var("SPOTIFY_CLIENT_ID").expect("Expected a client id");
    let secret = env::var("SPOTIFY_CLIENT_SECRET").expect("Expected a secret");
    let client = SpotifyClient::new(client_id, secret);

    let mut track_ids = env::var("TRACK_IDS").expect("Expected track ids");
    track_ids = track_ids.split(",").map(|s| s.to_string()).collect();

    let tracks = match client.api_req(&format!("/tracks?ids={}", track_ids)).await {
        Ok(res) => match from_value::<Tracks>(res) {
            Ok(result) => result,
            Err(e) => {
                println!("Error: {:?}", e);
                Tracks { tracks: vec![] }
            }
        },
        Err(e) => {
            println!("Error: {:?}", e);
            Tracks { tracks: vec![] }
        }
    };

    for track in tracks.tracks {
        let track_name = &track.name;
        let album_name = &track.album.name;
        let artist_name = &track.album.artists[0].name;

        println!("Searching for: {} - {} - {}", artist_name, album_name, track_name);

        match search_yt_music(&track_name, &album_name, &artist_name) {
            Ok(output) => println!("Script output: {}", output),
            Err(e) => println!("Error running script: {}", e),
        }
    }
}
