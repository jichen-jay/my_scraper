use axum::{http::StatusCode, response::IntoResponse, routing::post, Json, Router};
use serde::{Deserialize, Deserializer};
use std::error::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

const END_OF_MESSAGE: &str = "<END_OF_MESSAGE>";

#[derive(Deserialize)]
struct ScrapeParams {
    #[serde(default, deserialize_with = "empty_string_as_none")]
    url: Option<String>,
}

async fn scrape(Json(params): Json<ScrapeParams>) -> impl IntoResponse {
    if let Some(url) = params.url {
        println!("Received URL from HTTP request: {}", url);

        match send_url_to_js_server(&url).await {
            Ok(scraped_text) => (StatusCode::OK, scraped_text),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to scrape URL: {}", e),
            ),
        }
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

async fn send_url_to_js_server(url: &str) -> Result<String, Box<dyn Error>> {
    let mut stream = TcpStream::connect("127.0.0.1:4000").await?;
    let mut reader = BufReader::new(stream);

    reader
        .get_mut()
        .write_all(format!("{}\n", url).as_bytes())
        .await?;
    println!("URL sent to JS server.");

    let mut buffer = vec![0; 65_535]; // Buffer for reading data
    let mut complete_message = String::new();

    loop {
        let n = reader.read(&mut buffer).await?;

        if n == 0 {
            println!("Connection closed by JS server.");
            break;
        }

        complete_message.push_str(&String::from_utf8_lossy(&buffer[..n]));

        if complete_message.contains(END_OF_MESSAGE) {
            complete_message = complete_message.replace(END_OF_MESSAGE, "");
            println!(
                "Complete message received from JS server: {}",
                complete_message
            );

            return Ok(complete_message);
        }
    }

    Err(Box::from("Unexpected end of communication"))
}
