mod blackjack {
    pub mod game;
}
mod card;
mod db {
    pub mod user_data;
}
mod websocket_manager;

use axum::{
    body::Body,
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::{ConnectInfo, Json, Path, State},
    http::{header, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use axum_extra::TypedHeader;
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
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::fs;
use tokio_util::io::ReaderStream;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::{DefaultMakeSpan, TraceLayer},
};
use websocket_manager::WebSocketManager;

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::db::user_data::{PostUserJson, UserData};

struct AppState {
    db: DB,
    wm_send: tokio::sync::mpsc::Sender<websocket_manager::Command>,
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
    State(app_state): State<Arc<Mutex<AppState>>>,
    Json(payload): Json<PostUserJson>,
) -> impl IntoResponse {
    let collection = app_state
        .lock()
        .unwrap()
        .db
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

async fn get_user(
    State(app_state): State<Arc<Mutex<AppState>>>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let collection = app_state
        .lock()
        .unwrap()
        .db
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

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "blackjack-backend=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let _game = Blackjack::create_game();

    // Create async task and access WebSocketManager with channels
    let (mut wm_send, mut wm_read) = tokio::sync::mpsc::channel::<websocket_manager::Command>(10);

    let send_clone = wm_send.clone();

    let ws_manager = tokio::spawn(async move {
        let mut websocket_manager = WebSocketManager::new();

        // Start receiving messages
        while let Some(cmd) = wm_read.recv().await {
            use websocket_manager::Command::*;

            match cmd {
                AddWS { ws_send, resp } => {
                    let id = Uuid::new();
                    websocket_manager.add_ws(id.clone(), ws_send);
                    let _ = resp.send(id);
                }
                DeleteWS { id } => {
                    websocket_manager.remove_ws(id);
                }
                SendWS { id, msg } => {
                    websocket_manager.send_msg(id, msg).await;
                }
            }
        }
    });

    let shared_db_state = Arc::new(Mutex::new(AppState {
        db: DB { client },
        wm_send,
    }));

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
    State(app_state): State<Arc<Mutex<AppState>>>,
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
    .on_upgrade(move |socket| handle_socket(State(app_state), socket, addr))
}

async fn handle_socket(
    State(app_state): State<Arc<Mutex<AppState>>>,
    mut socket: WebSocket,
    who: SocketAddr,
) {
    let (mut sink, mut stream) = socket.split();

    let _ = sink
        .send(Message::Text("TEST from server".to_string()))
        .await;

    let wm_send_copy = app_state.lock().unwrap().wm_send.clone();

    let res = tokio::spawn(async move {
        let (resp_send, resp_recv) = tokio::sync::oneshot::channel();
        let cmd = websocket_manager::Command::AddWS {
            ws_send: sink,
            resp: resp_send,
        };

        let _ = wm_send_copy.send(cmd).await;

        resp_recv.await
    })
    .await;

    let wm_send_copy2 = app_state.lock().unwrap().wm_send.clone();

    let _ = tokio::spawn(async move {
        let uuid = res.unwrap().unwrap();

        let send_ws = websocket_manager::SendWS {
            msg_type: "uuid".to_string(),
            msg_data: uuid.clone().to_string(),
        };

        let cmd = websocket_manager::Command::SendWS {
            id: uuid.clone(),
            msg: send_ws,
        };

        let _ = wm_send_copy2.send(cmd).await;
    })
    .await;

    println!("Websocket context {who} completed handle_socket");
}
