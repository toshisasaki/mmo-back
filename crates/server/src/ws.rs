use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
    routing::get,
    Router,
};
use futures::{sink::SinkExt, stream::StreamExt};
use std::{
    net::SocketAddr,
};
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;
use shared::ClientCommand;
use crossbeam_channel::Sender;

// State shared across all routes/connections
#[derive(Clone)]
struct AppState {
    // Channel to send commands to the game loop (ECS)
    tx: Sender<GamePacket>,
    // Channel to broadcast updates to all connected clients
    broadcast_tx: broadcast::Sender<String>, // We'll serialize ServerEvent to JSON string for simplicity
}

pub enum GamePacket {
    ClientCommand { id: u32, cmd: ClientCommand },
    PlayerJoin { id: u32 },
    PlayerLeave { id: u32 },
}

pub async fn start_ws_server(
    tx: Sender<GamePacket>,
    broadcast_tx: broadcast::Sender<String>,
    shutdown_tx: Sender<()>,
) {
    let state = AppState { tx, broadcast_tx };

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("WebSocket listening on {}", addr);

    let listener = TcpListener::bind(addr).await.unwrap();
    
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();

    println!("WebSocket server stopped");
    let _ = shutdown_tx.send(());
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    println!("Shutdown signal received");
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    // Generate a temporary ID (in a real app, use authentication)
    let id = rand::random::<u32>();
    println!("Client {} connected", id);

    let (mut sender, mut receiver) = socket.split();

    // 1. Send Join event to ECS
    let _ = state.tx.send(GamePacket::PlayerJoin { id });

    // 2. Subscribe to broadcasts
    let mut rx = state.broadcast_tx.subscribe();

    // Task to forward broadcast messages to client
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if sender.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    });

    // Task to read messages from client
    let tx = state.tx.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Text(text) = msg {
                if let Ok(cmd) = serde_json::from_str::<ClientCommand>(&text) {
                    let _ = tx.send(GamePacket::ClientCommand { id, cmd });
                }
            }
        }
    });

    // Wait for either to finish (likely disconnection)
    tokio::select! {
        _ = (&mut send_task) => {},
        _ = (&mut recv_task) => {},
    };

    println!("Client {} disconnected", id);
    let _ = state.tx.send(GamePacket::PlayerLeave { id });
}

