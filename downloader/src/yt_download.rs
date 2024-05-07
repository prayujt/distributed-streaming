use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

use id3::frame::{Content, PictureType};
use id3::{frame, Frame, Tag, TagLike, Version};

use crate::spotify_client::Track;

pub fn download_track(track: &Track, url: String) {
    let music_home = env::var("MUSIC_HOME").expect("MUSIC_HOME environment variable not set");

    let path = Path::new(&music_home)
        .join(&track.album.artists[0].name)
        .join(&track.album.name);

    fs::create_dir_all(&path).expect("Failed to create directories");
    let output_path = path.join(format!("{}.mp3", track.name));

    let output = Command::new("yt-dlp")
        .arg("-q")
        .arg("-x")
        .arg("--audio-quality")
        .arg("0")
        .arg("--audio-format")
        .arg("mp3")
        .arg("-o")
        .arg(output_path.to_str().unwrap())
        .arg(url)
        .output()
        .expect("Failed to execute command");

    if output.status.success() {
        println!("Downloaded {}", track.name);

        let mut tag = Tag::new();
        tag.set_album(&track.album.name);
        tag.set_artist(&track.album.artists[0].name);
        tag.set_title(&track.name);
        tag.set_year(
            track.album.release_date.split('-').collect::<Vec<&str>>()[0]
                .parse()
                .unwrap(),
        );
        tag.set_track(track.track_number);

        let img_data = reqwest::blocking::get(&track.album.images[0].url)
            .expect("Failed to download image")
            .bytes()
            .expect("Failed to read bytes");

        let picture = frame::Picture {
            mime_type: "image/jpeg".to_string(),
            picture_type: PictureType::CoverFront,
            description: "Cover".to_string(),
            data: img_data.to_vec(),
        };
        let picture_frame = Frame::with_content("APIC", Content::Picture(picture));
        tag.add_frame(picture_frame);

        match tag.write_to_path(&output_path, Version::Id3v24) {
            Ok(_) => println!("Tagged {}", output_path.to_str().unwrap()),
            Err(e) => println!("Failed to tag {}: {}", output_path.to_str().unwrap(), e),
        }
    } else {
        println!("Failed to download {}", track.name);
    }
}
