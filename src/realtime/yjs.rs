//! Yjs collaborative editing WebSocket handler.
//!
//! Protocol (y-protocol v1, binary):
//!   Message type 0 = Sync
//!     Sub-type 0 = Step 1: client sends its state vector
//!     Sub-type 1 = Step 2: server responds with missing updates
//!     Sub-type 2 = Update: client sends incremental update
//!   Message type 1 = Awareness (forwarded verbatim to other clients)

use std::collections::HashMap;
use std::sync::{Arc, Mutex as StdMutex};

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    response::IntoResponse,
};
use tokio::sync::{broadcast, RwLock};
use yrs::{
    updates::decoder::Decode,
    updates::encoder::Encode,
    Doc, ReadTxn, StateVector, Transact, Update,
};

use crate::error::AppError;
use crate::state::AppState;

// ── Room ──────────────────────────────────────────────────────────────────────

/// A collaborative editing room for a single document.
/// Uses `std::sync::Mutex` so guards are never held across `.await` points.
pub struct Room {
    doc: Arc<StdMutex<Doc>>,
    /// Broadcast channel for update messages to all connected clients.
    tx: broadcast::Sender<Vec<u8>>,
}

impl Room {
    fn new() -> Self {
        let (tx, _) = broadcast::channel(256);
        Room {
            doc: Arc::new(StdMutex::new(Doc::new())),
            tx,
        }
    }
}

pub type Rooms = Arc<RwLock<HashMap<String, Arc<Room>>>>;

pub fn new_rooms() -> Rooms {
    Arc::new(RwLock::new(HashMap::new()))
}

// ── WebSocket upgrade ─────────────────────────────────────────────────────────

/// GET /ws/yjs/:doc_path — upgrades to WebSocket.
pub async fn yjs_ws_handler(
    ws: WebSocketUpgrade,
    Path(doc_path): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, doc_path, state))
}

async fn handle_socket(mut socket: WebSocket, doc_path: String, state: AppState) {
    // Get or create room.
    let room = {
        let mut rooms = state.rooms.write().await;
        rooms
            .entry(doc_path.clone())
            .or_insert_with(|| Arc::new(Room::new()))
            .clone()
    };

    let mut rx = room.tx.subscribe();

    // Send sync step 1 to the new client (server's state vector).
    // All yrs work done in a synchronous block — no await inside.
    let step1_msg: Vec<u8> = {
        let doc = room.doc.lock().expect("doc mutex poisoned");
        let sv_bytes = doc.transact().state_vector().encode_v1();
        // doc guard dropped at end of block
        encode_sync_step1(&sv_bytes)
    };
    if socket.send(Message::Binary(step1_msg.into())).await.is_err() {
        return;
    }

    loop {
        tokio::select! {
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Binary(data))) => {
                        if let Err(e) = handle_message(&data, &room, &mut socket).await {
                            tracing::warn!(path = %doc_path, error = %e, "ws message error");
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
            Ok(broadcast_msg) = rx.recv() => {
                if socket.send(Message::Binary(broadcast_msg.into())).await.is_err() {
                    break;
                }
            }
        }
    }
}

async fn handle_message(
    data: &[u8],
    room: &Room,
    socket: &mut WebSocket,
) -> Result<(), AppError> {
    if data.is_empty() {
        return Ok(());
    }

    let msg_type = data[0];

    match msg_type {
        0 => handle_sync_message(&data[1..], room, socket).await,
        1 => {
            // Awareness: forward to all clients verbatim.
            let _ = room.tx.send(data.to_vec());
            Ok(())
        }
        _ => Ok(()),
    }
}

async fn handle_sync_message(
    data: &[u8],
    room: &Room,
    socket: &mut WebSocket,
) -> Result<(), AppError> {
    if data.is_empty() {
        return Ok(());
    }

    let sub_type = data[0];
    let payload = &data[1..];

    match sub_type {
        0 => {
            // Sync step 1: client sends state vector → reply with step 2.
            let (sv_bytes, _) = read_var_buf(payload)?;
            let reply: Vec<u8> = {
                let client_sv = StateVector::decode_v1(&sv_bytes)
                    .map_err(|_| AppError::BadRequest("invalid state vector".into()))?;
                let doc = room.doc.lock().map_err(|_| AppError::Internal("doc mutex poisoned".into()))?;
                let update = doc.transact().encode_state_as_update_v1(&client_sv);
                // doc guard dropped here
                encode_sync_step2(&update)
            };
            socket
                .send(Message::Binary(reply.into()))
                .await
                .map_err(|e| AppError::Internal(format!("ws send: {e}")))?;
        }
        1 | 2 => {
            // Sync step 2 or update: apply and broadcast.
            let (update_bytes, _) = read_var_buf(payload)?;
            let broadcast_msg: Option<Vec<u8>> = {
                let update = Update::decode_v1(&update_bytes)
                    .map_err(|_| AppError::BadRequest("invalid yjs update".into()))?;
                let doc = room.doc.lock().map_err(|_| AppError::Internal("doc mutex poisoned".into()))?;
                let mut txn = doc.transact_mut();
                txn.apply_update(update)
                    .map_err(|e| AppError::Internal(format!("yjs apply: {e}")))?;
                // txn and doc dropped here
                Some(encode_sync_update(&update_bytes))
            };
            if let Some(msg) = broadcast_msg {
                let _ = room.tx.send(msg);
            }
        }
        _ => {}
    }

    Ok(())
}

// ── y-protocol encoding ───────────────────────────────────────────────────────

fn encode_sync_step1(sv: &[u8]) -> Vec<u8> {
    let mut buf = vec![0u8, 0u8];
    write_var_buf(&mut buf, sv);
    buf
}

fn encode_sync_step2(update: &[u8]) -> Vec<u8> {
    let mut buf = vec![0u8, 1u8];
    write_var_buf(&mut buf, update);
    buf
}

fn encode_sync_update(update: &[u8]) -> Vec<u8> {
    let mut buf = vec![0u8, 2u8];
    write_var_buf(&mut buf, update);
    buf
}

fn write_var_buf(buf: &mut Vec<u8>, data: &[u8]) {
    write_varint(buf, data.len() as u64);
    buf.extend_from_slice(data);
}

fn write_varint(buf: &mut Vec<u8>, mut n: u64) {
    loop {
        let byte = (n & 0x7F) as u8;
        n >>= 7;
        if n == 0 {
            buf.push(byte);
            break;
        } else {
            buf.push(byte | 0x80);
        }
    }
}

fn read_var_buf(data: &[u8]) -> Result<(Vec<u8>, &[u8]), AppError> {
    let (len, rest) = read_varint(data)?;
    let len = len as usize;
    if rest.len() < len {
        return Err(AppError::BadRequest("truncated yjs buffer".into()));
    }
    Ok((rest[..len].to_vec(), &rest[len..]))
}

fn read_varint(data: &[u8]) -> Result<(u64, &[u8]), AppError> {
    let mut result: u64 = 0;
    let mut shift = 0u32;
    for (i, &byte) in data.iter().enumerate() {
        result |= ((byte & 0x7F) as u64) << shift;
        if byte & 0x80 == 0 {
            return Ok((result, &data[i + 1..]));
        }
        shift += 7;
        if shift > 63 {
            return Err(AppError::BadRequest("varint overflow".into()));
        }
    }
    Err(AppError::BadRequest("truncated varint".into()))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_varint_roundtrip_small() {
        let mut buf = Vec::new();
        write_varint(&mut buf, 42);
        let (n, rest) = read_varint(&buf).unwrap();
        assert_eq!(n, 42);
        assert!(rest.is_empty());
    }

    #[test]
    fn test_varint_roundtrip_large() {
        let mut buf = Vec::new();
        write_varint(&mut buf, 1_000_000);
        let (n, _) = read_varint(&buf).unwrap();
        assert_eq!(n, 1_000_000);
    }

    #[test]
    fn test_var_buf_roundtrip() {
        let data = b"hello world";
        let mut buf = Vec::new();
        write_var_buf(&mut buf, data);
        let (out, rest) = read_var_buf(&buf).unwrap();
        assert_eq!(out, data);
        assert!(rest.is_empty());
    }

    #[test]
    fn test_encode_sync_step1() {
        let sv = b"\x00\x00";
        let msg = encode_sync_step1(sv);
        assert_eq!(msg[0], 0);
        assert_eq!(msg[1], 0);
    }

    #[test]
    fn test_encode_sync_step2() {
        let update = b"\x01\x02\x03";
        let msg = encode_sync_step2(update);
        assert_eq!(msg[0], 0);
        assert_eq!(msg[1], 1);
    }

    #[test]
    fn test_encode_sync_update() {
        let update = b"\xAB";
        let msg = encode_sync_update(update);
        assert_eq!(msg[0], 0);
        assert_eq!(msg[1], 2);
    }

    #[tokio::test]
    async fn test_room_creation() {
        let rooms = new_rooms();
        {
            let mut map = rooms.write().await;
            map.insert("test-doc".to_string(), Arc::new(Room::new()));
        }
        let map = rooms.read().await;
        assert!(map.contains_key("test-doc"));
    }

    #[test]
    fn test_yrs_doc_state_vector() {
        let doc = Doc::new();
        let sv_bytes = doc.transact().state_vector().encode_v1();
        assert!(!sv_bytes.is_empty());
    }

    #[test]
    fn test_apply_empty_update() {
        let doc = Doc::new();
        // Empty update is valid
        let sv = doc.transact().state_vector();
        let update_bytes = doc.transact().encode_state_as_update_v1(&sv);
        let update = Update::decode_v1(&update_bytes).unwrap();
        doc.transact_mut().apply_update(update).unwrap();
    }
}
