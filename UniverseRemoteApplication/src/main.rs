use std::error::Error;
use std::time::Duration;
use std::thread;
use std::net::IpAddr;

use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::broadcast;
use tokio_tungstenite::accept_async;
use futures_util::{StreamExt, SinkExt};
use local_ip_address::local_ip; // <--- The missing import

use scrap::{Capturer, Display};
use image::codecs::jpeg::JpegEncoder;
use image::{ColorType, ImageEncoder};

// Embed the HTML file inside the .exe
const INDEX_HTML: &str = include_str!("index.html");
const PASSWORD: &str = "secret123";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let addr = "0.0.0.0:8080";
    let listener = TcpListener::bind(addr).await?;

    // --- FIND AND PRINT THE REAL IP ---
    let my_local_ip = local_ip().unwrap_or("127.0.0.1".parse().unwrap());

    println!("========================================");
    println!("   🌌 UNIVERSE REMOTE V2 READY          ");
    println!("========================================");
    println!("To connect, go to another computer and");
    println!("type this URL in the browser:");
    println!("");
    println!("   👉 http://{}:8080", my_local_ip);
    println!("");
    println!("(Password: {})", PASSWORD);
    println!("========================================");

    let (tx, _) = broadcast::channel::<Vec<u8>>(16);
    let tx_for_capture = tx.clone();

    // --- PART 1: BACKGROUND SCREEN CAPTURE TASK ---
    thread::spawn(move || {
        start_screen_capture(tx_for_capture);
    });

    // --- PART 2: NETWORK SERVER LOOP ---
    loop {
        let (socket, _) = listener.accept().await?;
        let tx = tx.clone();

        tokio::spawn(async move {
            let mut buf = [0; 4096];
            let socket = socket; 

            // Peek at the request data
            let n = match socket.peek(&mut buf).await {
                Ok(n) => n,
                Err(_) => return,
            };
            let request = String::from_utf8_lossy(&buf[..n]);

            if request.contains("Upgrade: websocket") {
                handle_websocket(socket, tx).await;
            } 
            else {
                handle_http(socket).await;
            }
        });
    }
}

// --- MISSING FUNCTION RESTORED ---
async fn handle_http(mut socket: TcpStream) {
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: text/html\r\n\r\n{}",
        INDEX_HTML.len(),
        INDEX_HTML
    );
    let mut buffer = [0; 1024];
    let _ = socket.read(&mut buffer).await;
    let _ = socket.write_all(response.as_bytes()).await;
}

async fn handle_websocket(stream: TcpStream, tx: broadcast::Sender<Vec<u8>>) {
    let ws_stream = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            eprintln!("❌ WebSocket Handshake Error: {}", e);
            return;
        }
    };

    let (mut write, mut read) = ws_stream.split();

    // --- STEP 1: AUTHENTICATION ---
    println!("🔐 New Connection: Waiting for password...");
    let mut authenticated = false;

    while let Some(Ok(msg)) = read.next().await {
        if msg.is_text() {
            let text = msg.to_text().unwrap_or("").trim();
            println!("   -> Received Auth Attempt: '{}'", text);

            if text == format!("AUTH:{}", PASSWORD) {
                println!("✅ Client Password Correct!");
                authenticated = true;
                break; 
            } else {
                println!("❌ Wrong Password. Disconnecting.");
                return;
            }
        } 
    }

    if !authenticated {
        return;
    }

    // --- STEP 2: STREAMING ---
    println!("🎥 Starting Video Stream...");
    let mut rx = tx.subscribe();

    loop {
        match rx.recv().await {
            Ok(frame) => {
                let msg = tokio_tungstenite::tungstenite::Message::Binary(frame);
                if let Err(_) = write.send(msg).await {
                    break; 
                }
            }
            Err(_) => { /* Ignore lag */ }
        }
    }
}

fn start_screen_capture(tx: broadcast::Sender<Vec<u8>>) {
    let display = match Display::primary() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("❌ Failed to find display: {}", e);
            return;
        }
    };

    let mut capturer = match Capturer::new(display) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("❌ Failed to start capturer: {}", e);
            return;
        }
    };

    let width = capturer.width();
    let height = capturer.height();

    loop {
        let buffer = match capturer.frame() {
            Ok(buf) => buf,
            Err(error) => {
                if error.kind() == std::io::ErrorKind::WouldBlock {
                    thread::sleep(Duration::from_millis(10));
                    continue;
                }
                thread::sleep(Duration::from_millis(100));
                continue;
            }
        };

        let stride = buffer.len() / height;
        let row_len = width * 4; 
        let mut clean_buffer = vec![0u8; width * height * 4];

        for y in 0..height {
            let src_start = y * stride;
            let src_end = src_start + row_len;
            let dest_start = y * row_len;
            
            if src_end <= buffer.len() && (dest_start + row_len) <= clean_buffer.len() {
                clean_buffer[dest_start..dest_start + row_len]
                    .copy_from_slice(&buffer[src_start..src_end]);
            }
        }

        let mut jpeg_data = Vec::new();
        let encoder = JpegEncoder::new_with_quality(&mut jpeg_data, 60);

        if let Err(_) = encoder.write_image(
            &clean_buffer, 
            width as u32, 
            height as u32, 
            ColorType::Rgba8 
        ) {
             continue;
        }

        let _ = tx.send(jpeg_data);
        thread::sleep(Duration::from_millis(33)); 
    }
}