use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::Response,
    routing::get,
    Router,
};
use futures::{sink::SinkExt, stream::StreamExt};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;

pub async fn start_ws_server() {
    let app = Router::new()
        .route("/ws", get(ws_handler))
        .layer(CorsLayer::permissive());

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("WebSocket listening on {}", addr);

    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn ws_handler(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    println!("Web Client connected");
    
    // Send a welcome message
    if socket.send(Message::Text("Hello from server!".into())).await.is_err() {
        return;
    }

    while let Some(msg) = socket.recv().await {
        if let Ok(msg) = msg {
            match msg {
                Message::Text(t) => {
                    // Try to parse as JSON
                    if let Ok(cmd) = serde_json::from_str::<serde_json::Value>(&t) {
                        println!("Received command: {:?}", cmd);
                        // In real impl, we'd convert to Shared::ClientCommand and send to ECS channel
                    } else {
                        println!("Received text: {}", t);
                    }
                }
                Message::Binary(b) => {
                    println!("Client sent {} bytes", b.len());
                }
                _ => {}
            }
        } else {
            break;
        }
    }
    
    println!("Web Client disconnected");
}
