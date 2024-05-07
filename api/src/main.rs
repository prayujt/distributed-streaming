use std::collections::HashMap;
use std::{env,cmp,fs};
use std::sync::Mutex;

use serde::Deserialize;
use serde_json::{from_value};
use urlencoding::encode;
use uuid::Uuid;

use kube::{api::{Api, PostParams}, Client};
use k8s_openapi::api::core::v1::{Pod, Container, PodSpec, EnvVar};

use lazy_static::lazy_static;
use warp::Filter;

mod spotify_client;
use crate::spotify_client::{AlbumTrack, ArtistAlbum, Items, SpotifyClient, SpotifySearchResponse};

#[derive(Debug, Deserialize)]
struct SelectQuery {
    titles: String,
}

#[derive(Debug, Deserialize)]
struct DownloadQuery {
    indices: Vec<i8>,
    session_id: String,
}

#[derive(Debug, Clone)]
struct Choice {
    r#type: String,
    id: String,
}

lazy_static! {
    static ref SESSION_CHOICES: Mutex<HashMap<String, Vec<Vec<Choice>>>> =
        Mutex::new(HashMap::new());
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

    // let _ = select_music(SelectQuery {
    //     titles: "taylor swift".to_string(),
    // })
    // .await;

    // let session_id = {
    //     match SESSION_CHOICES.lock() {
    //         Ok(guard) => {
    //             if let Some(key) = guard.keys().next() {
    //                 key.clone()
    //             } else {
    //                 String::new()
    //             }
    //         }
    //         Err(_) => String::new(),
    //     }
    // };
    // let _ = download_music(DownloadQuery {
    //     indices: vec![15],
    //     session_id,
    // })
    // .await;

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

    match SESSION_CHOICES.lock() {
        Ok(mut guard) => guard.insert(session_id.clone(), session),
        Err(_) => return Ok(warp::reply::json(&"Failed to lock mutex".to_string())),
    };

    Ok(warp::reply::json(&session_id))
}

async fn download_music(body: DownloadQuery) -> Result<impl warp::Reply, warp::Rejection> {
    let session_id = body.session_id;
    let indices: Vec<i8> = body.indices;

    let session = {
        let mut mutex_guard = match SESSION_CHOICES.lock() {
            Ok(guard) => guard,
            Err(_) => return Ok(warp::reply::json(&"Failed to lock mutex".to_string())),
        };

        match mutex_guard.remove(&session_id) {
            Some(session) => session,
            None => return Ok(warp::reply::json(&"Session not found".to_string())),
        }
    };

    let it = indices.iter().zip(session.iter());

    let client_id = env::var("SPOTIFY_CLIENT_ID").expect("Expected a client id");
    let secret = env::var("SPOTIFY_CLIENT_SECRET").expect("Expected a secret");
    let client = SpotifyClient::new(client_id, secret);

    for (idx, choices) in it {
        let choice = &choices[*idx as usize];

        match choice.r#type.as_str() {
            "track" => process_track(choice.id.clone(), &client).await,
            "album" => process_album(choice.id.clone(), &client).await,
            "artist" => process_artist(choice.id.clone(), &client).await,
            _ => {
                println!("Unknown type: {}", choice.r#type);
            }
        }
    }
    Ok(warp::reply::json(&session_id))
}

async fn process_track(track_id: String, _client: &SpotifyClient) {
    /* Spawn new Kubernetes pod for track downloading */
    println!("Downloading track: {}", track_id);
    let namespace = get_kubernetes_namespace().unwrap_or_else(|_| "default".to_string());
    let client = Client::try_default().await.expect("Failed to create K8s client");
    let pods: Api<Pod> = Api::namespaced(client, &namespace);

    let uuid = Uuid::new_v4().to_string().to_lowercase();
    let pod_name = format!("downloader-{}", uuid);
    let pod = Pod {
        metadata: kube::api::ObjectMeta {
            name: Some(pod_name),
            ..Default::default()
        },
        spec: Some(PodSpec {
            containers: vec![
                Container {
                    name: "downloader".to_string(),
                    image: Some("docker.prayujt.com/distributed-streaming-downloader".to_string()),
                    env: Some(vec![
                        EnvVar {
                            name: "TRACK_IDS".to_string(),
                            value: Some(track_id),
                            ..Default::default()
                        },
                        EnvVar {
                            name: "SPOTIFY_CLIENT_ID".to_string(),
                            value: Some(env::var("SPOTIFY_CLIENT_ID").unwrap_or_default()),
                            ..Default::default()
                        },
                        EnvVar {
                            name: "SPOTIFY_CLIENT_SECRET".to_string(),
                            value: Some(env::var("SPOTIFY_CLIENT_SECRET").unwrap_or_default()),
                            ..Default::default()
                        },
                    ]),
                    ..Default::default()
                },
            ],
            restart_policy: Some("Never".to_string()),
            ..Default::default()
        }),
        ..Default::default()
    };

    match pods.create(&PostParams::default(), &pod).await {
        Ok(_) => println!("Pod created successfully."),
        Err(e) => println!("Failed to create pod: {:?}", e),
    }
}

async fn process_album(album_id: String, client: &SpotifyClient) {
    println!("Downloading album: {}", album_id);
    match client
        .api_req(&format!("/albums/{}/tracks", album_id))
        .await
    {
        Ok(res) => match from_value::<Items<AlbumTrack>>(res) {
            Ok(tracks) => {
                for track in tracks.items {
                    process_track(track.id, client).await;
                }
            }
            Err(e) => println!("Failed to parse JSON: {:?}", e),
        },
        Err(e) => println!("Error: {:?}", e),
    }
}

async fn process_artist(artist_id: String, client: &SpotifyClient) {
    println!("Downloading artist: {}", artist_id);
    match client
        .api_req(&format!("/artists/{}/albums", artist_id))
        .await
    {
        Ok(res) => match from_value::<Items<ArtistAlbum>>(res) {
            Ok(albums) => {
                for album in albums.items {
                    process_album(album.id, client).await;
                }
            }
            Err(e) => println!("Failed to parse JSON: {:?}", e),
        },
        Err(e) => println!("Error: {:?}", e),
    }
}

fn get_kubernetes_namespace() -> Result<String, std::io::Error> {
    fs::read_to_string("/var/run/secrets/kubernetes.io/serviceaccount/namespace")
}
