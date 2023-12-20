use std::net::TcpStream;
use std::sync::Mutex;
use std::io::Write;

pub fn broadcast_message(message: &str, sender: Option<&TcpStream>, clients: &Mutex<Vec<TcpStream>>) {
    let mut clients_guard = clients.lock().unwrap();

    for client in clients_guard.iter_mut() {
        if let Some(sender_stream) = sender {
            if client.peer_addr().unwrap() == sender_stream.peer_addr().unwrap() {
                continue;
            }
        }

        client.write_all(message.as_bytes()).unwrap();
        client.flush().unwrap();
    }
}

