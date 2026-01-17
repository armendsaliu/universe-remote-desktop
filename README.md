# 🌌 Universe Remote Desktop

**A High-Performance, Secure Remote Desktop Solution built in Rust.**

Universe Remote is a custom-built remote administration tool designed for speed, security, and simplicity. Unlike heavy commercial alternatives, this tool provides a lightweight, low-latency video stream (~17ms latency) directly to any web browser.

![Project Status](https://img.shields.io/badge/Status-Active-brightgreen)
![Language](https://img.shields.io/badge/Built%20With-Rust%20%7C%20Tokio%20%7C%20WebSockets-orange)

## ✨ Key Features

* **⚡ Blazing Fast:** Built on Rust's `tokio` asynchronous runtime.
* **🎥 High Performance:** Custom JPEG compression and Zero-Copy memory handling (~60 FPS capable).
* **🌍 Universal Access:** Control your PC from any device (Phone, Tablet, Laptop) via a standard Web Browser.
* **🔒 Secure "Bouncer":** Password-protected WebSocket handshake (`AUTH` protocol) to prevent unauthorized access.
* **⌨️ Full Control:** Real-time Mouse and Keyboard injection using `Enigo`.
* **☁️ Internet Ready:** Works over LAN or via Secure Tunnels (ngrok/Cloudflare).

---

## 🏗️ Architecture

The system consists of three distinct components:

1.  **The Server (`my_server`):** The central traffic controller. It listens on port `8080` and routes data between the desktop and the user.
2.  **The Client (`my_client`):** The silent agent running on the host machine. It captures the screen, compresses it, and sends it to the server.
3.  **The Dashboard (`index.html`):** A client-side web interface that renders the video stream on an HTML5 Canvas and captures user inputs.

---

## 🚀 Getting Started

### Prerequisites
* **Rust:** [Install Rust](https://rustup.rs/)
* **C++ Build Tools:** Required for compiling Windows binaries.

### 1. Run the Server (The Hub)
The server acts as the bridge. It must be running first.

```bash
cd my_server
cargo run --release

### 2. Run the Client (The Host)
This runs on the computer you want to control. It will automatically connect to 127.0.0.1:8080 and authenticate.

Bash
cd my_client
cargo run --release
### 3. Connect (The Viewer)
Simply open the index.html file in any modern web browser.

Login: Enter the session password (default: secret123).

Status: You should see the remote desktop immediately.

🛠️ Configuration
To access this over the internet, we recommend using a secure tunnel like ngrok:

Start the tunnel: ngrok http 8080

Update index.html: Change SERVER_URL to your new wss://...ngrok-free.app address.

🗺️ Roadmap
[x] Basic Screen Streaming

[x] Mouse & Keyboard Control

[x] Password Authentication

[ ] Windows Installer (.msi)

[ ] File Transfer System

[ ] Audio Streaming

Built with ❤️ in Rust.