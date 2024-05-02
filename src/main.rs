// use tide::prelude::*;
// use tide::Request;

// use async_std::task;
// use futures::future::try_join_all;
use serde::Deserialize;
use serde_json::from_value;
use std::env;

mod spotify_client;
use crate::spotify_client::{SpotifyClient, SpotifySearchResponse};

// use spotify_client::SpotifyClient;

#[derive(Debug, Deserialize)]
struct Query {
    titles: String,
}

// #[tokio::main]
// async fn main() -> tide::Result<()> {
//     let mut app = tide::new();
//     app.at("/select").post(select_music);
//     app.listen("0.0.0.0:8080").await?;
//     Ok(())
// }

// async fn select_music(mut req: Request<()>) -> tide::Result {
//     let client_id = env::var("SPOTIFY_CLIENT_ID").expect("Expected a client id");
//     let secret = env::var("SPOTIFY_CLIENT_SECRET").expect("Expected a secret");
//     let client = SpotifyClient::new(client_id, secret);

//     let Query { titles } = req.body_json().await?;
//     let titles = titles.split('\n');

//     for title in titles {
//         match client.api_req(&format!("/search?q={}", title.trim())).await {
//             Ok(res) => {
//                 println!("{:?}", res);
//             }
//             Err(e) => println!("Error: {:?}", e),
//         }
//     }

//     Ok("".into())
// }

use warp::Filter;

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
                title.trim()
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

    println!("{:?}", results);
    Ok(warp::reply::json(&""))
}
