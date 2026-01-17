use std::sync::{Arc, Mutex};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, tungstenite::protocol::Message};
use futures_util::{StreamExt, SinkExt};
use std::collections::HashMap;

// A map to store connected clients (Sender <-> Receiver)
type PeerMap = Arc<Mutex<HashMap<std::net::SocketAddr, tokio::sync::mpsc::UnboundedSender<Message>>>>;

#[tokio::main]
async fn main() {
    // Listen on 0.0.0.0 to allow LAN/Internet connections
    let addr = "0.0.0.0:8080"; 
    let state = PeerMap::new(Mutex::new(HashMap::new()));
    let listener = TcpListener::bind(addr).await.expect("Failed to bind");
    println!("High-Speed Secure Server Active on: {}", addr);

    while let Ok((stream, addr)) = listener.accept().await {
        tokio::spawn(handle_connection(state.clone(), stream, addr));
    }
}

async fn handle_connection(peer_map: PeerMap, raw_stream: TcpStream, addr: std::net::SocketAddr) {
    // Optimization: Disable Nagle's Algorithm for instant relay
    raw_stream.set_nodelay(true).ok();

    let ws_stream = accept_async(raw_stream).await.expect("Handshake failed");
    println!("New Connection Attempt: {}", addr);

    let (mut outgoing, mut incoming) = ws_stream.split();

    // =================================================================
    // SECURITY CHECK (The "Bouncer")
    // =================================================================
    // Wait for the very first message. It MUST be the password.
    if let Some(Ok(msg)) = incoming.next().await {
        if let Ok(text) = msg.to_text() {
            if text == "AUTH:secret123" {
                 println!("✅ {} Authenticated successfully.", addr);
            } else {
                 println!("❌ {} Failed authentication. (Sent: '{}')", addr, text);
                 return; // Kick them out.
            }
        } else {
            return; // Non-text message? Kick them out.
        }
    } else {
        return; // No message? Kick them out.
    }
    // =================================================================

    // If we get here, they are trusted. Add them to the chat room.
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    peer_map.lock().unwrap().insert(addr, tx);

    // Task: Send messages TO this user
    let _reply_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            outgoing.send(msg).await.ok();
        }
    });

    // Task: Receive messages FROM this user and relay to others
    while let Some(Ok(msg)) = incoming.next().await {
        if msg.is_binary() || msg.is_text() {
            let peers = peer_map.lock().unwrap();
            for (&peer_addr, rec) in peers.iter() {
                // Broadcast to everyone except the sender
                if peer_addr != addr {
                    rec.send(msg.clone()).ok();
                }
            }
        }
    }

    println!("{} disconnected", addr);
    peer_map.lock().unwrap().remove(&addr);
}