mod blackjack {
    pub mod game;
}
mod card;
mod db {
    pub mod user_data;
}
mod websocket_manager;

use anyhow::Result;
use axum::{
    body::Body,
    debug_handler,
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        ConnectInfo, FromRef, FromRequestParts, Json, Path, State,
    },
    http::{header, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use axum_extra::TypedHeader;
use bb8::Pool;
use bb8_postgres::PostgresConnectionManager;
use blackjack::game::Blackjack;
use db::user_data;
use futures_util::StreamExt;
use http::{header::CONTENT_TYPE, Method};
use std::{env, sync::Arc};
use std::{net::SocketAddr, ops::ControlFlow};
use tokio::fs;
use tokio_postgres::NoTls;
use tokio_util::io::ReaderStream;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::{DefaultMakeSpan, TraceLayer},
};
use uuid::Uuid;
use websocket_manager::WebSocketManager;

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::db::user_data::{PostUserJson, User};

struct AppState {
    db: DB,
    wm_send: tokio::sync::mpsc::Sender<websocket_manager::Command>,
}

struct DB {
    db_pool: Pool<PostgresConnectionManager<NoTls>>,
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

#[debug_handler]
async fn add_user(
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<PostUserJson>,
) -> Result<Json<User>, (StatusCode, String)> {
    let user_name = payload.name;

    let mut conn = app_state
        .as_ref()
        .db
        .db_pool
        .get()
        .await
        .map_err(internal_error)?;

    let uuid = Uuid::new_v4();

    let rows = conn
        .execute("SELECT id FROM users WHERE username = $1", &[&user_name])
        .await;

    if rows.map_err(internal_error)? != 0 {
        return Err((
            StatusCode::CONFLICT,
            "username was found in the database".to_string(),
        ));
    }

    conn = app_state
        .as_ref()
        .db
        .db_pool
        .get()
        .await
        .map_err(internal_error)?;

    let _ = conn
        .execute(
            "INSERT INTO users (id, username) VALUES ($1, $2)",
            &[&uuid, &user_name.clone()],
        )
        .await
        .map_err(internal_error)?;

    conn = app_state
        .as_ref()
        .db
        .db_pool
        .get()
        .await
        .map_err(internal_error)?;

    let row = conn
        .query_one(
            "SELECT id, username, created_at, wins FROM users WHERE id = $1",
            &[&uuid],
        )
        .await;

    match row {
        Ok(row) => {
            let doc = user_data::User {
                _id: row.get(0),
                name: row.get(1),
                created_at: row.get(2),
                wins: row.get(3),
            };

            Ok(Json(doc))
        }
        Err(e) => Err(internal_error(e)),
    }
}

async fn get_user(
    State(app_state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<User>, (StatusCode, String)> {
    let conn = app_state
        .as_ref()
        .db
        .db_pool
        .get()
        .await
        .map_err(internal_error)?;
    let row = conn
        .query_one(
            "SELECT id, username, created_at, wins FROM users WHERE username = $1",
            &[&name],
        )
        .await;
    match row {
        Ok(row) => {
            let doc = user_data::User {
                _id: row.get(0),
                name: row.get(1),
                created_at: row.get(2),
                wins: row.get(3),
            };

            Ok(Json(doc))
        }
        Err(e) => Err(internal_error(e)),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let db_url: String = env::var("DATABASE_URL").unwrap();

    let pg_manager = PostgresConnectionManager::new_from_stringlike(db_url, NoTls).unwrap();

    let pool = Pool::builder().build(pg_manager).await.unwrap();

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

    let _ = wm_send.clone();

    let _ = tokio::spawn(async move {
        let mut websocket_manager = WebSocketManager::new();

        // Start receiving messages
        while let Some(cmd) = wm_read.recv().await {
            use websocket_manager::Command::*;

            match cmd {
                AddWS { ws_send, resp } => {
                    let id = Uuid::new_v4();
                    websocket_manager.add_ws(id.clone(), ws_send);
                    let _ = resp.send(id);
                    websocket_manager.update_all_list().await;
                }
                DeleteWS { id } => {
                    websocket_manager.remove_ws(id.clone());
                    websocket_manager.update_all_list().await;
                }
                SendWS { id, msg } => {
                    websocket_manager.send_msg(id, msg).await;
                }
                UpdateUserList {} => {
                    websocket_manager.update_all_list().await;
                }
            }
        }
    });

    let shared_db_state = Arc::new(AppState {
        db: DB { db_pool: pool },
        wm_send,
    });

    let cors = CorsLayer::new()
        // allow `GET` and `POST` when accessing the resource
        .allow_methods(vec![Method::GET, Method::POST, Method::CONNECT])
        // allow requests from any origin
        .allow_origin(Any)
        // allow any headers
        .allow_headers([CONTENT_TYPE]);

    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .route("/card/:value/:suit", get(get_card))
        .route("/user/create", post(add_user))
        .route("/user/:name", get(get_user))
        .route("/ws", get(ws_handler))
        .with_state(shared_db_state)
        .layer(cors)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        );

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
    Ok(())
}

async fn ws_handler(
    State(app_state): State<Arc<AppState>>,
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

async fn handle_socket(State(app_state): State<Arc<AppState>>, socket: WebSocket, who: SocketAddr) {
    let (sink, mut stream) = socket.split();

    let wm_send_copy = app_state.as_ref().wm_send.clone();

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

    let id = res.unwrap().unwrap();

    let id_clone = id.clone();

    let wm_send_copy2 = app_state.as_ref().wm_send.clone();

    let _ = tokio::spawn(async move {
        let uuid = id;

        let send_ws = websocket_manager::SendWS {
            msg_type: websocket_manager::MsgType::SelfUuid,
            msg_data_str: Some(uuid.clone().to_string()),
            msg_data_arr: None,
        };

        let cmd = websocket_manager::Command::SendWS {
            id: uuid.clone(),
            msg: send_ws,
        };

        let _ = wm_send_copy2.send(cmd).await;

        let _ = wm_send_copy2
            .send(websocket_manager::Command::UpdateUserList {})
            .await;
    })
    .await;

    let wm_send_copy3 = app_state.as_ref().wm_send.clone();

    let _ = tokio::spawn(async move {
        let wm_send = wm_send_copy3;
        let uuid = id_clone;
        while let Some(Ok(msg)) = stream.next().await {
            if process_message(msg, who, uuid.clone(), &wm_send)
                .await
                .is_break()
            {
                break;
            }
        }
    });

    println!("Websocket context {who} completed handle_socket");
}

async fn process_message(
    msg: Message,
    who: SocketAddr,
    id: Uuid,
    recv: &tokio::sync::mpsc::Sender<websocket_manager::Command>,
) -> ControlFlow<(), ()> {
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
            let cmd = websocket_manager::Command::DeleteWS { id };
            let _ = recv.send(cmd).await;
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

/// Utility function for mapping any error into a `500 Internal Server Error`
/// response.
fn internal_error<E>(err: E) -> (StatusCode, String)
where
    E: std::error::Error,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}
