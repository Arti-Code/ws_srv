
use std::{
    collections::HashMap,
    env,
    io::Error as IoError,
    net::SocketAddr,
    sync::{Arc, Mutex},
};


use futures_channel::mpsc::{unbounded, UnboundedSender};
use futures_util::{future, pin_mut, stream::TryStreamExt, StreamExt};

use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::protocol::Message;

type Tx = UnboundedSender<Message>;
type PeerMap = Arc<Mutex<HashMap<SocketAddr, (String, Tx)>>>;

async fn handle_connection(peer_map: PeerMap, raw_stream: TcpStream, addr: SocketAddr) {
    let ws_stream = tokio_tungstenite::accept_async(raw_stream)
        .await.expect("error during the websocket handshake occurred");
    println!("new websocket connection established: {}", addr);
    let (tx, rx) = unbounded();
    peer_map.lock().unwrap().insert(addr, (String::new(), tx));
    let (outgoing, incoming) = ws_stream.split();
    let broadcast_incoming = incoming.try_for_each(|msg| {
        //println!("[{}] {}", addr, msg.to_text().unwrap());
        let mut is_command = false;
        let sender_name = {
            let peers = peer_map.lock().unwrap();
            if let Some((name, _)) = peers.get(&addr) {
                name.clone()
            } else {
                addr.to_string().clone()
            }
        };
        let s = format!("[{}]", &sender_name);
        let m = msg.to_text().unwrap().replace(s.as_str(), "");
        println!("[{}]: {}", sender_name, m.as_str());
        match msg.to_text() {
            Ok(text) => {
                if text.starts_with("/register ") {
                    is_command = true;
                    let name = text.trim_start_matches("/register ").to_string();
                    let mut peers = peer_map.lock().unwrap();
                    if let Some((user_name, _)) = peers.get_mut(&addr) {
                        *user_name = name.clone();
                        println!("[{}] registered as {}", addr, name);
                    }
                    return future::ok(());
                }
                if text == "/list" {
                    is_command = true;
                    let peers = peer_map.lock().unwrap();
                    let user_list: Vec<String> = peers.iter()
                        .map(|(_, (name, _))| name.clone())
                        .filter(|name| !name.is_empty())
                        .collect();
                    let list_message = format!("Connected users: {}", user_list.join(", "));
                    if let Some((_, ws_sink)) = peers.get(&addr) {
                        ws_sink.unbounded_send(Message::text(list_message)).unwrap();
                    }
                    //return future::ok(());
                }
            },
            Err(e) => {
                println!("error processing message from {}: {}", addr, e);
                return future::ok(());
            }
        }
        let peers = peer_map.lock().unwrap();
        let broadcast_recipients =
            peers.iter().filter(|(peer_addr, _)| peer_addr != &&addr && !is_command)
            .map(|(_, (_, ws_sink))| ws_sink);
        for recp in broadcast_recipients {
            recp.unbounded_send(msg.clone()).unwrap();
        }
        future::ok(())
    });
    let receive_from_others = rx.map(Ok).forward(outgoing);
    pin_mut!(broadcast_incoming, receive_from_others);
    future::select(broadcast_incoming, receive_from_others).await;
    println!("{} disconnected", &addr);
    peer_map.lock().unwrap().remove(&addr);
}

#[tokio::main]
async fn main() -> Result<(), IoError> {
    init_display().await;
    let peer_map = PeerMap::new(Mutex::new(HashMap::new()));
    let addr = env::args().nth(1).unwrap_or_else(|| "0.0.0.0:8080".to_string());
    let try_socket = TcpListener::bind(&addr).await;
    let listener = try_socket.expect("failed to bind");
    println!("listening on: {}...", addr);
    while let Ok((stream, addr)) = listener.accept().await {
        tokio::spawn(handle_connection(peer_map.clone(), stream, addr));
    }
    Ok(())
}

async fn init_display() {
    let ver = env!("CARGO_PKG_VERSION");
    let author = env!("CARGO_PKG_AUTHORS");
    let year = "2025 year";
    println!("-=WEBSOCKET COMMUNICATION SERVER=-");
    println!("ver: {} | {} | {}", ver, author, year);
    println!("");
}