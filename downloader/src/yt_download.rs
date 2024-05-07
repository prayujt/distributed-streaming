use std::process::Command;

use crate::spotify_client::Track;

pub fn download_track(track: &Track, url: String) {
    let output = Command::new("yt-dlp")
        .arg("-x")
        .arg("--audio-format")
        .arg("mp3")
        .arg("-o")
        .arg(format!("{}.mp3", track.name))
        .arg(url)
        .output()
        .expect("Failed to execute command");

    if output.status.success() {
        println!("Downloaded {}", track.name);
    } else {
        println!("Failed to download {}", track.name);
    }
}
