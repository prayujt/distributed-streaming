use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

use crate::spotify_client::Track;

pub fn download_track(track: &Track, url: String) {
    let music_home = env::var("MUSIC_HOME").expect("MUSIC_HOME not set");

    let path = Path::new(&music_home)
        .join(&track.album.artists[0].name)
        .join(&track.album.name);

    fs::create_dir_all(&path).expect("Failed to create directories");
    let output_path = path.join(format!("{}.mp3", track.name));

    let output = Command::new("yt-dlp")
        .arg("-x")
        .arg("--audio-format")
        .arg("mp3")
        .arg("-o")
        .arg(output_path.to_str().unwrap())
        .arg(url)
        .output()
        .expect("Failed to execute command");

    if output.status.success() {
        println!("Downloaded {}", track.name);
    } else {
        println!("Failed to download {}", track.name);
    }
}
