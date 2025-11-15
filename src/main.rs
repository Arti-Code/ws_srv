
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
type PeerMap = Arc<Mutex<HashMap<SocketAddr, Tx>>>;

async fn handle_connection(peer_map: PeerMap, raw_stream: TcpStream, addr: SocketAddr) {
    println!("new connection: {}", addr);
    let ws_stream = tokio_tungstenite::accept_async(raw_stream)
        .await
        .expect("error during the websocket handshake occurred");
    println!("websocket connection established: {}", addr);
    let (tx, rx) = unbounded();
    peer_map.lock().unwrap().insert(addr, tx);
    let (outgoing, incoming) = ws_stream.split();
    let broadcast_incoming = incoming.try_for_each(|msg| {
        let data: Vec<u8> = msg.to_string().into_bytes().iter().map(|char| {
            *char
        }).collect();
        let mut msg2 = String::new();
        data.iter().for_each(|char| {
            msg2.push(*char as char);
        });
        println!("[{}] {} => {}", addr, msg.to_text().unwrap(), &msg2);
        let peers = peer_map.lock().unwrap();
        let broadcast_recipients =
            peers.iter().filter(|(peer_addr, _)| peer_addr != &&addr).map(|(_, ws_sink)| ws_sink);
        for recp in broadcast_recipients {
            //recp.unbounded_send(msg.clone()).unwrap();
            recp.unbounded_send(Message::Text(msg2.clone().into())).unwrap();
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
    let ver = env!("CARGO_PKG_VERSION");
    let author = env!("CARGO_PKG_AUTHORS");
    let year = "2025 year";
    println!("-=WEBSOCKET COMMUNICATION SERVER=-");
    println!("ver: {} | {} | {}", ver, author, year);
    println!("");
    let addr = env::args().nth(1).unwrap_or_else(|| "0.0.0.0:8080".to_string());
    let state = PeerMap::new(Mutex::new(HashMap::new()));
    let try_socket = TcpListener::bind(&addr).await;
    let listener = try_socket.expect("failed to bind");
    println!("listening on: {}", addr);
    while let Ok((stream, addr)) = listener.accept().await {
        tokio::spawn(handle_connection(state.clone(), stream, addr));
    }
    Ok(())
}