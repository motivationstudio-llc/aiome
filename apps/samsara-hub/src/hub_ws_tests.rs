use super::*;
use futures_util::{SinkExt, StreamExt};
use sqlx::sqlite::SqlitePoolOptions;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

async fn spawn_test_hub() -> (SocketAddr, Arc<HubState>) {
    let pool = SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .expect("Failed to create memory db");

    init_hub_db(&pool).await.expect("Failed to init db");

    let (tx, _) = broadcast::channel(100);
    let state = Arc::new(HubState {
        pool,
        secret: secrecy::SecretString::new("test_secret".to_string()),
        tx,
        active_connections: std::sync::atomic::AtomicUsize::new(0),
    });

    let app = build_app(state.clone());
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    (addr, state)
}

#[tokio::test]
async fn test_ws_authentication_unauthorized() {
    let (addr, _state) = spawn_test_hub().await;
    let ws_url = format!("ws://{}/api/v1/federation/ws", addr);

    let result = connect_async(&ws_url).await;
    // Should fail because no auth header
    assert!(result.is_err(), "Expected connection to fail without auth");
}

#[tokio::test]
async fn test_ws_authentication_authorized_and_ping() {
    let (addr, state) = spawn_test_hub().await;
    let ws_url = format!("ws://{}/api/v1/federation/ws", addr);

    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    let mut request = ws_url.into_client_request().unwrap();
    request.headers_mut().insert(
        axum::http::header::AUTHORIZATION,
        axum::http::HeaderValue::from_static("Bearer test_secret"),
    );

    let (mut ws_stream, _) = connect_async(request).await.expect("Failed to connect");

    assert_eq!(
        state
            .active_connections
            .load(std::sync::atomic::Ordering::SeqCst),
        1
    );

    // Send Ping
    use aiome_core::contracts::HubMessage;
    let ping = HubMessage::Ping {
        client_time: chrono::Utc::now().to_rfc3339(),
    };
    ws_stream
        .send(Message::Text(serde_json::to_string(&ping).unwrap().into()))
        .await
        .unwrap();

    // Receive Pong
    loop {
        if let Some(msg) = ws_stream.next().await {
            let msg = msg.unwrap();
            if msg.is_text() {
                let text = msg.to_text().unwrap();
                if let Ok(HubMessage::Pong { server_time }) =
                    serde_json::from_str::<HubMessage>(text)
                {
                    assert!(!server_time.is_empty());
                    break;
                }
            }
        } else {
            panic!("No Pong message received");
        }
    }

    // Disconnect
    drop(ws_stream);

    // Give it a moment to process disconnect
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    assert_eq!(
        state
            .active_connections
            .load(std::sync::atomic::Ordering::SeqCst),
        0
    );
}
