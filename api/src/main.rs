use std::collections::{HashMap,VecDeque};
use std::{env,cmp,fs};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use serde_json::from_value;
use urlencoding::encode;
use uuid::Uuid;
use tokio::time::{sleep, Duration};

use k8s_openapi::api::batch::v1::{Job, JobSpec};

use kube::{
    api::{Api, ListParams, PostParams, ObjectMeta},
    Client,
};
use k8s_openapi::api::core::v1::{PodTemplateSpec, Container, EnvVar, Volume, PersistentVolumeClaimVolumeSource};


use lazy_static::lazy_static;
use warp::Filter;

mod spotify_client;
use crate::spotify_client::{AlbumTrack, ArtistAlbum, Items, SpotifyClient, SpotifySearchResponse};

mod podcast_client;
use crate::podcast_client::{PodcastClient, PodcastSearchResult, PodcastEpisodes};

#[derive(Debug, Deserialize)]
struct SelectQuery {
    titles: String,
    r#type: String,
}

#[derive(Debug, Deserialize)]
struct DownloadQuery {
    indices: Vec<i8>,
    session_id: String,
}

#[derive(Debug, Clone, Serialize)]
struct Choice {
    r#type: String,
    id: String,
}

#[derive(Serialize)]
struct SelectResponse {
    session_id: String,
    choices: Vec<Vec<Choice>>,
}

lazy_static! {
    static ref SESSION_CHOICES: Mutex<HashMap<String, Vec<Vec<Choice>>>> =
        Mutex::new(HashMap::new());
}

fn get_kubernetes_namespace() -> Result<String, std::io::Error> {
    fs::read_to_string("/var/run/secrets/kubernetes.io/serviceaccount/namespace")
}

fn create_job_spec(track_ids: String) -> Job {
    let uuid = Uuid::new_v4().to_string().to_lowercase();
    let job_name = format!("downloader-{}", uuid);

    Job {
        metadata: ObjectMeta {
            name: Some(job_name),
            ..Default::default()
        },
        spec: Some(JobSpec {
            template: PodTemplateSpec {
                metadata: Some(ObjectMeta {
                    ..Default::default()
                }),
                spec: Some(k8s_openapi::api::core::v1::PodSpec {
                    restart_policy: Some("Never".to_string()),
                    containers: vec![
                        Container {
                            name: "downloader".to_string(),
                            image: Some("docker.prayujt.com/distributed-streaming-downloader".to_string()),
                            env: Some(vec![
                                EnvVar {
                                    name: "TRACK_IDS".to_string(),
                                    value: Some(track_ids),
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
                                EnvVar {
                                    name: "MUSIC_HOME".to_string(),
                                    value: Some("/music".to_string()),
                                    ..Default::default()
                                },
                            ]),
                            volume_mounts: Some(vec![
                                k8s_openapi::api::core::v1::VolumeMount {
                                    name: "music-storage".to_string(),
                                    mount_path: "/music".to_string(),
                                    ..Default::default()
                                },
                            ]),
                            ..Default::default()
                        },
                    ],
                    volumes: Some(vec![
                        Volume {
                            name: "music-storage".to_string(),
                            persistent_volume_claim: Some(PersistentVolumeClaimVolumeSource {
                                claim_name: env::var("MUSIC_STORAGE_PVC").unwrap_or("music-storage".to_string()),
                                ..Default::default()
                            }),
                            ..Default::default()
                        },
                    ]),
                    ..Default::default()
                }),
            },
            backoff_limit: Some(0),
            ttl_seconds_after_finished: Some(1),
            ..Default::default()
        }),
        ..Default::default()
    }
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
        .and_then(download);
    let routes = select_route.or(download_route);

    warp::serve(routes).run(([0, 0, 0, 0], 8080)).await;
}

async fn select_music(body: SelectQuery) -> Result<impl warp::Reply, warp::Rejection> {
    let client_id = env::var("SPOTIFY_CLIENT_ID").expect("Expected a client id");
    let secret = env::var("SPOTIFY_CLIENT_SECRET").expect("Expected a secret");
    let client = SpotifyClient::new(client_id, secret);

    let titles = body.titles.split('\n');
    let mut session: Vec<Vec<Choice>> = vec![];

    if body.r#type == "music" {
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
    }
    else if body.r#type == "podcast" {
        let api_key = env::var("PODCAST_API_KEY").expect("Expected a podcast client id");
        let secret = env::var("PODCAST_SECRET").expect("Expected a podcast secret");
        let client = PodcastClient::new(api_key, secret);

        for title in titles {
            match client
                .api_req(&format!(
                    "/search/byterm?q={}",
                    encode(title.trim())
                ))
                .await {
                    Ok(res) => match from_value::<PodcastSearchResult>(res) {
                        Ok(result) => {
                            let mut choices: Vec<Choice> = vec![];
                            for podcast in result.feeds {
                                println!(
                                    "Podcast: {} - {}",
                                    podcast.title, podcast.author
                                );
                                choices.push(Choice {
                                    r#type: "podcast".to_string(),
                                    id: podcast.id.to_string(),
                                });
                            }
                            session.push(choices);
                        }
                        Err(e) => println!("Failed to parse JSON: {:?}", e),
                    },
                    Err(e) => println!("Error: {:?}", e),
                }
        }
    }
    let session_id = Uuid::new_v4().to_string();

    match SESSION_CHOICES.lock() {
        Ok(mut guard) => guard.insert(session_id.clone(), session.clone()),
        Err(_) => return Ok(warp::reply::json(&"Failed to lock mutex".to_string())),
    };

    let response = SelectResponse {
        session_id,
        choices: session,
    };

    Ok(warp::reply::json(&response))
}

async fn download(body: DownloadQuery) -> Result<impl warp::Reply, warp::Rejection> {
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

    tokio::spawn(async move {
        for (idx, choices) in indices.iter().zip(session.iter()) {
            let choice = &choices[*idx as usize];

            match choice.r#type.as_str() {
                "track" => process_tracks(choice.id.clone()).await,
                "album" => process_album(choice.id.clone()).await,
                "artist" => process_artist(choice.id.clone()).await,
                "podcast" => process_podcast(choice.id.clone()).await,
                _ => {
                    println!("Unknown type: {}", choice.r#type);
                }
            }
        }
    });
    Ok(warp::reply::json(&session_id))
}

async fn process_podcast(podcast_id: String) {
    /* Spawn new Kubernetes jobs for podcast downloading */
    println!("Downloading podcast: {}", podcast_id);
    let api_key = env::var("PODCAST_API_KEY").expect("Expected a podcast client id");
    let secret = env::var("PODCAST_SECRET").expect("Expected a podcast secret");
    let client = PodcastClient::new(api_key, secret);

    let episodes: PodcastEpisodes = match client
        .api_req(&format!(
            "/episodes/byfeedid?id={}&max=1000",
            encode(podcast_id.as_str())
        ))
        .await {
            Ok(res) => match from_value::<PodcastEpisodes>(res) {
                Ok(result) => result,
                Err(e) => {
                    println!("Failed to parse JSON: {:?}", e);
                    return;
                }
            },
            Err(e) => {
                println!("Error: {:?}", e);
                return;
            }
        };

    if env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string()).as_str().eq("development")
    {
        return;
    }
    let namespace = get_kubernetes_namespace().unwrap_or_else(|_| "default".to_string());
    let client = Client::try_default().await.expect("Failed to create K8s client");
    let jobs: Api<Job> = Api::namespaced(client, &namespace);

    let max_jobs: usize = env::var("NUM_WORKERS")
        .unwrap_or_else(|_| "8".to_string())
        .parse()
        .unwrap_or(8);

    // loop {
    //     let current_jobs = jobs.list(&ListParams::default()).await
    //         .expect("Failed to list jobs")
    //         .items
    //         .into_iter()
    //         .filter(|job| job.status.as_ref().map_or(false, |status| status.active.unwrap_or(0) > 0))
    //         .count();

    //     if current_jobs < max_jobs {
    //         let job = create_job_spec(track_ids.clone());
    //         match jobs.create(&PostParams::default(), &job).await {
    //             Ok(_) => {
    //                 println!("Job created successfully.");
    //                 break;
    //             },
    //             Err(e) => {
    //                 println!("Failed to create job: {:?}", e);
    //                 sleep(Duration::from_secs(10)).await;
    //             }
    //         }
    //     } else {
    //         println!("Job limit reached. Waiting to retry...");
    //         sleep(Duration::from_secs(5)).await;
    //     }
    // }
}

async fn process_tracks(track_ids: String) {
    /* Spawn new Kubernetes jobs for track downloading */
    println!("Downloading tracks: {}", track_ids);
    if env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string()).as_str().eq("development")
    {
        return;
    }
    let namespace = get_kubernetes_namespace().unwrap_or_else(|_| "default".to_string());
    let client = Client::try_default().await.expect("Failed to create K8s client");
    let jobs: Api<Job> = Api::namespaced(client, &namespace);

    let max_jobs: usize = env::var("NUM_WORKERS")
        .unwrap_or_else(|_| "8".to_string())
        .parse()
        .unwrap_or(8);

    loop {
        let current_jobs = jobs.list(&ListParams::default()).await
            .expect("Failed to list jobs")
            .items
            .into_iter()
            .filter(|job| job.status.as_ref().map_or(false, |status| status.active.unwrap_or(0) > 0))
            .count();

        if current_jobs < max_jobs {
            let job = create_job_spec(track_ids.clone());
            match jobs.create(&PostParams::default(), &job).await {
                Ok(_) => {
                    println!("Job created successfully.");
                    break;
                },
                Err(e) => {
                    println!("Failed to create job: {:?}", e);
                    sleep(Duration::from_secs(10)).await;
                }
            }
        } else {
            println!("Job limit reached. Waiting to retry...");
            sleep(Duration::from_secs(5)).await;
        }
    }
}

async fn process_album(album_id: String) {
    let client_id = env::var("SPOTIFY_CLIENT_ID").expect("Expected a client id");
    let secret = env::var("SPOTIFY_CLIENT_SECRET").expect("Expected a secret");
    let client = SpotifyClient::new(client_id, secret);

    println!("Downloading album: {}", album_id);

    let tracks: Vec<AlbumTrack> = collect_album_tracks(album_id, &client).await;

    let worker_size: usize = env::var("WORKER_SIZE")
        .unwrap_or_else(|_| "5".to_string())
        .parse()
        .unwrap_or(5);

    let mut queue = VecDeque::from(tracks);
    while !queue.is_empty() {
        let group = queue.drain(..worker_size.min(queue.len())).collect::<Vec<_>>();
        let track_ids = group.iter().map(|track| track.id.as_str()).collect::<Vec<_>>().join(",");
        process_tracks(track_ids).await;
    }
}

async fn process_artist(artist_id: String) {
    let client_id = env::var("SPOTIFY_CLIENT_ID").expect("Expected a client id");
    let secret = env::var("SPOTIFY_CLIENT_SECRET").expect("Expected a secret");
    let client = SpotifyClient::new(client_id, secret);

    println!("Downloading artist: {}", artist_id);

    let albums: Vec<ArtistAlbum> = {
        let mut albums: Vec<ArtistAlbum> = vec![];
        let mut offset = 0;
        let limit = 50;

        loop {
            match client
                .api_req(&format!("/artists/{}/albums?offset={}&limit={}", artist_id, offset, limit))
                .await
            {
                Ok(res) => match from_value::<Items<ArtistAlbum>>(res) {
                    Ok(mut result) => {
                        albums.append(&mut result.items);
                        if result.items.len() < limit {
                            break;
                        }
                        offset += limit;
                    }
                    Err(e) => {
                        println!("Failed to parse JSON: {:?}", e);
                        break;
                    }
                },
                Err(e) => {
                    println!("Error: {:?}", e);
                    break;
                }
            }
        }
        albums
    };

    let mut all_tracks: Vec<AlbumTrack> = vec![];
    for album in albums {
        all_tracks.append(&mut collect_album_tracks(album.id, &client).await);
    }

    let worker_size: usize = env::var("WORKER_SIZE")
        .unwrap_or_else(|_| "4".to_string())
        .parse()
        .unwrap_or(4);

    let mut queue = VecDeque::from(all_tracks);
    while !queue.is_empty() {
        let group = queue.drain(..worker_size.min(queue.len())).collect::<Vec<_>>();
        let track_ids = group.iter().map(|track| track.id.as_str()).collect::<Vec<_>>().join(",");
        process_tracks(track_ids).await;
    }
}

async fn collect_album_tracks(album_id: String, client: &SpotifyClient) -> Vec<AlbumTrack> {
    let mut tracks: Vec<AlbumTrack> = vec![];
    let mut offset = 0;
    let limit = 50;

    loop {
        match client
            .api_req(&format!("/albums/{}/tracks?offset={}&limit={}", album_id, offset, limit))
            .await
        {
            Ok(res) => match from_value::<Items<AlbumTrack>>(res) {
                Ok(mut result) => {
                    tracks.append(&mut result.items);
                    if result.items.len() < limit {
                        break;
                    }
                    offset += limit;
                }
                Err(e) => {
                    println!("Failed to parse JSON: {:?}", e);
                    break;
                }
            },
            Err(e) => {
                println!("Error: {:?}", e);
                break;
            }
        }
    }
    tracks
}
