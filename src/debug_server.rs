use futures_util::{SinkExt, StreamExt};
use prost::Message;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio_tungstenite::accept_async;
use tungstenite::Message as WsMessage;

use crate::cli::CliOptions;
use crate::codex::{buffer_to_hex, unwrap_debug_message_data};
use crate::logger::Logger;
use crate::proto::wa_remote_debug::*;

pub async fn run_debug_server(
    options: CliOptions,
    logger: Arc<Logger>,
    cdp_tx: broadcast::Sender<String>,
    proxy_rx: broadcast::Receiver<String>,
) {
    let addr = format!("127.0.0.1:{}", options.debug_port);
    let listener = TcpListener::bind(&addr).await.unwrap();
    logger.info(&format!(
        "[server] debug server running on ws://localhost:{}",
        options.debug_port
    ));
    logger.info("[server] debug server waiting for miniapp to connect...");

    let mut msg_counter: u32 = 0;

    loop {
        let (stream, _) = match listener.accept().await {
            Ok(s) => s,
            Err(e) => {
                logger.error(&format!("[server] accept error: {}", e));
                continue;
            }
        };

        let logger = logger.clone();
        let cdp_tx = cdp_tx.clone();
        let mut proxy_rx = proxy_rx.resubscribe();

        tokio::spawn(async move {
            logger.info("[miniapp] miniapp client connected");

            let ws_stream = match accept_async(stream).await {
                Ok(s) => s,
                Err(e) => {
                    logger.error(&format!("[miniapp] ws accept error: {}", e));
                    return;
                }
            };

            let (ws_tx, mut ws_rx) = ws_stream.split();

            // Spawn task to forward CDP proxy messages to miniapp
            let ws_tx_fwd = Arc::new(tokio::sync::Mutex::new(ws_tx));
            let ws_tx_clone = ws_tx_fwd.clone();

            tokio::spawn(async move {
                while let Ok(msg) = proxy_rx.recv().await {
                    let mut tx = ws_tx_clone.lock().await;
                    let op_id = now_nanos() % 100;
                    let raw_payload = serde_json::json!({
                        "jscontext_id": "",
                        "op_id": op_id,
                        "payload": msg,
                    });
                    let wrapped = crate::codex::wrap_debug_message_data(
                        &raw_payload,
                        &crate::constants::DebugMessageCategory::ChromeDevtools,
                        0,
                    );

                    msg_counter += 1;
                    let out = WaRemoteDebugDebugMessage {
                        seq: msg_counter,
                        category: "chromeDevtools".to_string(),
                        data: wrapped.buffer,
                        compress_algo: 0,
                        original_size: wrapped.original_size,
                        ..Default::default()
                    };
                    let mut buf = Vec::new();
                    out.encode(&mut buf).unwrap();
                    let _ = tx.send(WsMessage::Binary(buf)).await;
                }
            });

            // Handle incoming messages from miniapp
            while let Some(msg) = ws_rx.next().await {
                match msg {
                    Ok(WsMessage::Binary(data)) => {
                        logger.main_debug(&format!(
                            "[miniapp] client received raw message (hex): {}",
                            buffer_to_hex(&data)
                        ));

                        match WaRemoteDebugDebugMessage::decode(&*data) {
                            Ok(decoded) => {
                                let unwrapped = unwrap_debug_message_data(&decoded);
                                logger.main_debug("[miniapp] [DEBUG] decoded data:");
                                logger.main_debug_raw(&format!("{}", unwrapped));

                                if unwrapped["category"].as_str() == Some("chromeDevtoolsResult") {
                                    if let Some(payload) = unwrapped["data"]["payload"].as_str() {
                                        let _ = cdp_tx.send(payload.to_string());
                                    }
                                }
                            }
                            Err(e) => {
                                logger.error(&format!("[miniapp] miniapp client err: {}", e));
                            }
                        }
                    }
                    Ok(WsMessage::Close(_)) => {
                        logger.info("[miniapp] miniapp client disconnected");
                        break;
                    }
                    Err(e) => {
                        logger.error(&format!("[miniapp] miniapp client err: {}", e));
                        break;
                    }
                    _ => {}
                }
            }
        });
    }
}

fn now_nanos() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    (t % 1000) as u32
}
