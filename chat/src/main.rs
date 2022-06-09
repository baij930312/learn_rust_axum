use std::{
    collections::HashSet,
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use axum::{
    extract::{
        ws::{Message, WebSocket},
        WebSocketUpgrade,
    },
    response::{Html, IntoResponse},
    routing::get,
    Extension, Router,
};
use futures::{SinkExt, StreamExt};
use tokio::sync::broadcast;
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt};

struct AppState {
    user_set: Mutex<HashSet<String>>,
    tx: broadcast::Sender<String>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "example_chat=trace".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let user_set = Mutex::new(HashSet::new());
    let (tx, _rx) = broadcast::channel(100);

    let app_state = Arc::new(AppState { user_set, tx });

    let app = Router::new()
        .route("/", get(index))
        .route("/websocket", get(socket_handler))
        .layer(Extension(app_state));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3001));
    println!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn socket_handler(
    ws: WebSocketUpgrade,
    Extension(state): Extension<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| websocket(socket, state))
}

async fn websocket(ws: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = ws.split();
    let mut user_name = String::new();
    while let Some(Ok(message)) = receiver.next().await {
        if let Message::Text(name) = message {
            check_username(&state, &mut user_name, &name).await;
            if !user_name.is_empty() {
                //获取用户名
                break;
            } else {
                let _ = sender
                    .send(Message::Text(String::from("Username already taken.")))
                    .await;
                return;
            }
        }
    }

    let mut rx = state.tx.subscribe();
    let msg = format!("{} join", user_name);
    println!("{}", msg);
    let _ = state.tx.send(msg);

    //监听广播来的消息发 发送给client
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if sender.send(Message::Text(String::from(msg))).await.is_err() {
                break;
            }
        }
    });

    let tx = state.tx.clone();
    let user_name = user_name.clone();
    let username = user_name.clone();
    //监听client来的消息 发送给广播
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(Message::Text(text))) = receiver.next().await {
            let _ = tx.send(format!("{}:{}", user_name, text));
        }
    });

    tokio::select! {
        _=(&mut send_task)=>{recv_task.abort();println!("12313");},
        _=(&mut recv_task)=>{send_task.abort();println!("2222");},
    }
    let msg = format!("{} left", username);
    println!("{}", msg);
    let _ = state.tx.send(msg);
    state.user_set.lock().unwrap().remove(&username);
}

async fn check_username(state: &AppState, string: &mut String, name: &String) {
    let mut user_set = state.user_set.lock().unwrap();
    if !user_set.contains(name) {
        user_set.insert(name.to_owned());
        string.push_str(name);
    }
}

async fn index() -> Html<&'static str> {
    Html(std::include_str!("../chat.html"))
}
