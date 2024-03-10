use axum::{
    body::Body,
    extract::{Json, Path, State},
    http::{header, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use dotenv::dotenv;
use http::Method;
use mongodb::{
    bson::{doc, uuid::Uuid},
    options::ClientOptions,
    options::Credential,
    Client,
};
use std::sync::Arc;
use tokio::fs;
use tokio_util::io::ReaderStream;
use tower_http::cors::{Any, CorsLayer};

use crate::db::user_data::{PostUserJson, UserData};

mod card;
mod db {
    pub mod user_data;
}

struct DB {
    client: Client,
}

async fn get_card(Path((value, suit)): Path<(String, String)>) -> impl IntoResponse {
    let card_result = card::get_card_file(value, suit);

    let file_name: String = match card_result {
        Ok(name) => name,
        Err(_) => return Err(StatusCode::BAD_REQUEST).into(),
    };

    let file = match fs::File::open(&String::from(format!("assets/cards/{}.svg", file_name))).await
    {
        Ok(file) => file,
        Err(_) => return Err(StatusCode::NOT_FOUND).into(),
    };

    let stream = ReaderStream::new(file);

    let body = Body::from_stream(stream);

    let headers = [(header::CONTENT_TYPE, "image/svg+xml")];

    Ok((headers, body))
}

async fn add_user(
    State(db): State<Arc<DB>>,
    Json(payload): Json<PostUserJson>,
) -> impl IntoResponse {
    let collection = db
        .client
        .database("blackjack")
        .collection::<UserData>("user-data");
    let user_name = payload.name;

    match collection.find_one(doc! { "name": &user_name }, None).await {
        Ok(Some(_)) => return (StatusCode::FOUND, "user already created").into_response(),
        Ok(None) => (),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };

    let doc = UserData {
        _id: Uuid::new(),
        name: user_name,
        wins: 0,
    };

    let result = match collection.insert_one(doc, None).await {
        Ok(res) => res,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };

    println!("{:?}", result);

    StatusCode::OK.into_response()
}

async fn get_user(State(db): State<Arc<DB>>, Path(name): Path<String>) -> impl IntoResponse {
    let collection = db
        .client
        .database("blackjack")
        .collection::<UserData>("user-data");
    let user = match collection.find_one(doc! { "name": name }, None).await {
        Ok(user) => user,
        Err(e) => return (StatusCode::NOT_FOUND, e.to_string()).into_response(),
    };

    match user {
        Some(user) => (StatusCode::OK, Json(user)).into_response(),
        None => (StatusCode::NOT_FOUND, "user not found".to_string()).into_response(),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    dotenv().ok();

    let db_credentials = Credential::builder()
        .username(dotenv::var("DB_USERNAME").unwrap())
        .password(dotenv::var("DB_PASSWORD").unwrap())
        .build();

    let mut db_client_options = ClientOptions::parse(dotenv::var("DB_URI").unwrap()).await?;

    db_client_options.credential = Some(db_credentials);

    let client = Client::with_options(db_client_options)?;

    let shared_db_state = Arc::new(DB { client });

    let cors = CorsLayer::new()
        // allow `GET` and `POST` when accessing the resource
        .allow_methods(vec![Method::GET, Method::POST])
        // allow requests from any origin
        .allow_origin(Any);

    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .route("/card/:value/:suit", get(get_card))
        .route("/user/create", post(add_user))
        .route("/user/:name", get(get_user))
        .with_state(shared_db_state)
        .layer(cors);

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
    Ok(())
}
