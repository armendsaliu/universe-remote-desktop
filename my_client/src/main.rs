// UNCOMMENT THE LINE BELOW TO HIDE THE WINDOW (SILENT MODE)
// #![windows_subsystem = "windows"]

use std::time::Instant;
use std::io::Cursor;
use xcap::Monitor;
use image::codecs::jpeg::JpegEncoder;
use image::{RgbImage, ImageBuffer};
use tokio::net::TcpStream;
use tokio_tungstenite::{client_async, tungstenite::protocol::Message};
use url::Url;
use futures_util::{SinkExt, StreamExt};
use enigo::{Enigo, MouseControllable, KeyboardControllable, MouseButton, Key};
use serde::Deserialize;

#[derive(Deserialize)]
struct RemoteAction {
    action: String,
    x: Option<i32>,
    y: Option<i32>,
    button: Option<String>,
    key: Option<String>,
}

#[tokio::main]
async fn main() {
    let monitors = Monitor::all().expect("No monitors");
    let monitor = monitors[0].clone();
    
    let w = monitor.width();
    let h = monitor.height();
    let scale = monitor.scale_factor();
    
    println!("Monitor: {}x{} | Scale: {}", w, h, scale);

    // Speed Optimization: Integer Scaling (1/2 size)
    let target_w = w / 2;
    let target_h = h / 2;
    let mut rgb_buffer = vec![0u8; (target_w * target_h * 3) as usize];

    println!("Connecting to Localhost...");
    
    // 1. Establish TCP Connection
    // Note: Client always connects to 127.0.0.1 because it runs ON the server computer.
    let stream = TcpStream::connect("127.0.0.1:8080").await.expect("Failed to connect");
    stream.set_nodelay(true).expect("Failed to set nodelay");
    
    let url = Url::parse("ws://127.0.0.1:8080").unwrap();
    let (ws_stream, _) = client_async(url, stream).await.expect("Handshake failed");
    let (mut write, mut read) = ws_stream.split();

    // =============================================================
    // SECURITY FIX: AUTO-LOGIN
    // We must send the password immediately, or the Server kicks us out.
    // =============================================================
    println!("Authenticating...");
    write.send(Message::Text("AUTH:secret123".to_string())).await.expect("Auth failed");
    println!("Authenticated! Starting Stream...");
    // =============================================================

    let mut enigo = Enigo::new();

    // ----------------------------------------------------------------
    // TASK 1: Receive Mouse/Keyboard Inputs
    // ----------------------------------------------------------------
    tokio::spawn(async move {
        let ratio = 2.0; // Because we send 50% size, inputs must be x2

        while let Some(Ok(msg)) = read.next().await {
            if let Ok(text) = msg.to_text() {
                if let Ok(action) = serde_json::from_str::<RemoteAction>(text) {
                    // MOUSE
                    if action.action == "click" {
                        let bx = action.x.unwrap_or(0) as f32;
                        let by = action.y.unwrap_or(0) as f32;
                        
                        let tx = (bx * ratio * scale) as i32;
                        let ty = (by * ratio * scale) as i32;

                        enigo.mouse_move_to(tx, ty);
                        match action.button.as_deref() {
                            Some("right") => enigo.mouse_click(MouseButton::Right),
                            _ => enigo.mouse_click(MouseButton::Left),
                        }
                    } 
                    // KEYBOARD
                    else if action.action == "key" {
                         if let Some(k) = action.key {
                            match k.as_str() {
                                "Enter" => enigo.key_click(Key::Return),
                                "Backspace" => enigo.key_click(Key::Backspace),
                                "Tab" => enigo.key_click(Key::Tab),
                                "Escape" => enigo.key_click(Key::Escape),
                                "ArrowUp" => enigo.key_click(Key::UpArrow),
                                "ArrowDown" => enigo.key_click(Key::DownArrow),
                                "ArrowLeft" => enigo.key_click(Key::LeftArrow),
                                "ArrowRight" => enigo.key_click(Key::RightArrow),
                                "Shift" | "Control" | "Alt" => {},
                                text => enigo.key_sequence(text),
                            }
                        }
                    }
                }
            }
        }
    });

    // ----------------------------------------------------------------
    // TASK 2: Capture & Stream Video
    // ----------------------------------------------------------------
    loop {
        let loop_start = Instant::now();
        
        if let Ok(image) = monitor.capture_image() {
            let cap_time = loop_start.elapsed();
            let raw_rgba = image.as_raw();

            // 1. FAST DOWNSCALE (RGBA -> RGB, 50% size)
            fast_downscale_rgba_to_rgb(
                raw_rgba, 
                w as usize, 
                h as usize, 
                &mut rgb_buffer, 
                target_w as usize
            );

            // 2. Wrap bytes for Encoder
            let img_wrapper: RgbImage = ImageBuffer::from_raw(target_w, target_h, rgb_buffer.clone()).unwrap();
            let mut buffer = Cursor::new(Vec::new());
            
            // 3. Compress (Quality 70)
            let mut encoder = JpegEncoder::new_with_quality(&mut buffer, 70);
            
            if encoder.encode_image(&img_wrapper).is_ok() {
                let enc_time = loop_start.elapsed() - cap_time;
                
                let data = buffer.into_inner();
                // Send video frame
                if write.send(Message::Binary(data)).await.is_err() { 
                    println!("Disconnected from Server.");
                    break; 
                }
                
                let fps = 1.0 / loop_start.elapsed().as_secs_f32();
                // If silent mode is OFF, print stats
                print!("\rFPS: {:.1} | Cap: {:?} | Enc: {:?}", fps, cap_time, enc_time);
            }
        }
        // Run as fast as possible
    }
}

// Helper function for fast scaling
fn fast_downscale_rgba_to_rgb(src: &[u8], src_w: usize, _src_h: usize, dst: &mut [u8], dst_w: usize) {
    let mut dst_idx = 0;
    for y in 0..(_src_h / 2) {
        let row_start = (y * 2) * src_w * 4;
        for x in 0..dst_w {
            let src_idx = row_start + (x * 2) * 4;
            dst[dst_idx]     = src[src_idx];
            dst[dst_idx + 1] = src[src_idx + 1];
            dst[dst_idx + 2] = src[src_idx + 2];
            dst_idx += 3;
        }
    }
}