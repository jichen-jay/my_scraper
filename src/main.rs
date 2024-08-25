use axum::{extract::Query, response::IntoResponse, routing::get, Router};
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server;
use serde::{de, Deserialize, Deserializer};
use std::{fmt, fs, process::Command, str::FromStr};
use tokio::{
    net::TcpListener,
    time::{sleep, Duration},
};
use tower::Service;

#[derive(Deserialize)]
struct ScrapeParams {
    #[serde(default, deserialize_with = "empty_string_as_none")]
    url: Option<String>,
}

async fn scrape(Query(params): Query<ScrapeParams>) -> impl IntoResponse {
    if let Some(url) = params.url {
        let _ = Command::new("rm").arg("output.md").status();

        let output = Command::new("node").arg("bundle.js").arg(&url).output();

        sleep(Duration::from_secs(3)).await;

        let mut elapsed_time = 0;
        while elapsed_time < 25 {
            if fs::metadata("output.md").is_ok() {
                match fs::read_to_string("output.md") {
                    Ok(scraped_text) => return (axum::http::StatusCode::OK, scraped_text),
                    Err(e) => {
                        return (
                            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                            format!("Failed to read output file: {}", e),
                        )
                    }
                }
            }

            sleep(Duration::from_secs(3)).await;
            elapsed_time += 3;
        }

        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "Timeout waiting for output file".to_string(),
        )
    } else {
        (
            axum::http::StatusCode::BAD_REQUEST,
            "Missing or empty 'url' parameter".to_string(),
        )
    }
}

// change code below to use post to receive user's request to scrape a url

fn empty_string_as_none<'de, D, T>(de: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr,
    T::Err: fmt::Display,
{
    let opt = Option::<String>::deserialize(de)?;
    match opt.as_deref() {
        None | Some("") => Ok(None),
        Some(s) => FromStr::from_str(s).map_err(de::Error::custom).map(Some),
    }
}

#[tokio::main]
async fn main() {
    tokio::join!(serve_scrape());
}

async fn serve_scrape() {
    // Create a router with a single route for scraping
    let app = Router::new().route("/scrape", get(scrape));

    // Create a `TcpListener` using tokio.
    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Listening on http://{}", listener.local_addr().unwrap());

    // Continuously accept new connections.
    loop {
        let (socket, _remote_addr) = listener.accept().await.unwrap();

        // We don't need to call `poll_ready` because `Router` is always ready.
        let tower_service = app.clone();

        tokio::spawn(async move {
            let socket = TokioIo::new(socket);

            let hyper_service =
                hyper::service::service_fn(move |request| tower_service.clone().call(request));

            if let Err(err) = server::conn::auto::Builder::new(TokioExecutor::new())
                .serve_connection_with_upgrades(socket, hyper_service)
                .await
            {
                eprintln!("failed to serve connection: {err:#}");
            }
        });
    }
}
