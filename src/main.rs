use axum::{http::StatusCode, response::IntoResponse, routing::post, Json, Router};
use serde::{Deserialize, Deserializer};
use std::{fs, process::Command};
use tokio::time::{sleep, Duration};

#[derive(Deserialize)]
struct ScrapeParams {
    #[serde(default, deserialize_with = "empty_string_as_none")]
    url: Option<String>,
}

async fn scrape(Json(params): Json<ScrapeParams>) -> impl IntoResponse {
    if let Some(url) = params.url {
        let _ = Command::new("rm").arg("output.md").status();

        let _ = Command::new("node").arg("bundle.js").arg(&url).output();

        sleep(Duration::from_secs(3)).await;

        let mut elapsed_time = 0;
        while elapsed_time < 25 {
            if fs::metadata("output.md").is_ok() {
                match fs::read_to_string("output.md") {
                    Ok(scraped_text) => return (StatusCode::OK, scraped_text),
                    Err(e) => {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            format!("Failed to read output file: {}", e),
                        )
                    }
                }
            }

            sleep(Duration::from_secs(3)).await;
            elapsed_time += 3;
        }

        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Timeout waiting for output file".to_string(),
        )
    } else {
        (
            StatusCode::BAD_REQUEST,
            "Missing or empty 'url' parameter".to_string(),
        )
    }
}

fn empty_string_as_none<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<String>::deserialize(deserializer)?;
    Ok(opt.filter(|s| !s.is_empty()))
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/scrape", post(scrape));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
