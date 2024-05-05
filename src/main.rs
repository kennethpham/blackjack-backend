mod blackjack {
    pub mod game;
}
mod card;
mod db {
    pub mod user_data;
}

use axum::{
    body::Body,
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::{ConnectInfo, Json, Path, State},
    http::{header, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Router, ServiceExt,
};
use axum_extra::{headers::UserAgent, TypedHeader};
use blackjack::game::Blackjack;
use db::user_data;
use dotenv::dotenv;
use futures_util::{SinkExt, StreamExt};
use http::Method;
use mongodb::{
    bson::{doc, uuid::Uuid},
    options::ClientOptions,
    options::Credential,
    Client,
};
use std::{sync::Arc, ops::ControlFlow};
use std::{net::SocketAddr, path::PathBuf};
use tokio::fs;
use tokio_util::io::ReaderStream;
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
    trace::{DefaultMakeSpan, TraceLayer},
};

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::db::user_data::{PostUserJson, UserData};

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

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "blackjack-backend=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let _game = Blackjack::create_game();

    let cors = CorsLayer::new()
        // allow `GET` and `POST` when accessing the resource
        .allow_methods(vec![Method::GET, Method::POST, Method::CONNECT])
        // allow requests from any origin
        .allow_origin(Any);

    let app = Router::new()
        // .route("/", get(|| async { "Hello, World!" }))
        .route("/card/:value/:suit", get(get_card))
        // .route("/user/create", post(add_user))
        .route("/user/:name", get(get_user))
        .route("/ws", get(ws_handler))
        .with_state(shared_db_state)
        .layer(cors)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        );

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
    Ok(())
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    user_agent: Option<TypedHeader<headers::UserAgent>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    let user_agent = match user_agent {
        Some(user_data) => user_data.to_string(),
        None => String::from("Unknown browser"),
    };
    println!("{} at {}", user_agent, addr);

    ws.on_failed_upgrade(|error| {
        println!("Error: {}", error);
    })
    .on_upgrade(move |socket| handle_socket(socket, addr))
}

async fn handle_socket(mut socket: WebSocket, who: SocketAddr) {
    let (mut sender, mut receiver) = socket.split();

    let mut send_task = tokio::spawn(async move {
        if sender
            .send(Message::Text("Hello from server".to_string()))
            .await
            .is_err()
        {
            return 1;
        }
        0
    });

    let mut recv_task = tokio::spawn(async move {
        let mut cnt = 0;
        while let Some(Ok(msg)) = receiver.next().await {
            cnt += 1;
            // print message and break if instructed to do so
            if process_message(msg, who).is_break() {
                break;
            }
        }
        cnt
    });
    println!("Websocket context {who} completed handle_socket");
}

/// helper to print contents of messages to stdout. Has special treatment for Close.
fn process_message(msg: Message, who: SocketAddr) -> ControlFlow<(), ()> {
    match msg {
        Message::Text(t) => {
            println!(">>> {who} sent str: {t:?}");
        }
        Message::Binary(d) => {
            println!(">>> {} sent {} bytes: {:?}", who, d.len(), d);
        }
        Message::Close(c) => {
            if let Some(cf) = c {
                println!(
                    ">>> {} sent close with code {} and reason `{}`",
                    who, cf.code, cf.reason
                );
            } else {
                println!(">>> {who} somehow sent close message without CloseFrame");
            }
            return ControlFlow::Break(());
        }

        Message::Pong(v) => {
            println!(">>> {who} sent pong with {v:?}");
        }
        // You should never need to manually handle Message::Ping, as axum's websocket library
        // will do so for you automagically by replying with Pong and copying the v according to
        // spec. But if you need the contents of the pings you can see them here.
        Message::Ping(v) => {
            println!(">>> {who} sent ping with {v:?}");
        }
    }
    ControlFlow::Continue(())
}
