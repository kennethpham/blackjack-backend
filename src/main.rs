use axum::{
    body::Body,
    extract::Path,
    http::{header, StatusCode},
    response::IntoResponse,
    routing::get,
    Router,
};
use tokio::fs;
use tokio_util::io::ReaderStream;
mod card;

async fn get_card(Path((value, suit)): Path<(String, String)>) -> impl IntoResponse {
    let card_result = card::get_card_file(value, suit);

    let file_name: String = match card_result {
        Ok(name) => name,
        Err(_) => return Err(StatusCode::BAD_REQUEST).into(),
    };

    let file = match fs::File::open(&String::from(
        format!("assets/cards/{}.svg", file_name)
    ))
    .await
    {
        Ok(file) => file,
        Err(_) => return Err(StatusCode::NOT_FOUND).into(),
    };

    let stream = ReaderStream::new(file);

    let body = Body::from_stream(stream);

    let headers = [(header::CONTENT_TYPE, "image/svg+xml")];

    Ok((headers, body))
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .route("/card/:value/:suit", get(get_card));

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
