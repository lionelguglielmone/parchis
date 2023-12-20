mod client_handler;
mod communication;
mod game_state;

use std::net::{TcpListener};
use std::thread;
use std::sync::{Arc, Mutex};
use client_handler::handle_client;
use game_state::Game; 

fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").expect("Could not bind to port");
    println!("Server running on port 7878");

    let clients = Arc::new(Mutex::new(Vec::new()));

    let game = Arc::new(Mutex::new(Game::new()));

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("New connection: {}", stream.peer_addr().unwrap());
                
                let clients = Arc::clone(&clients);
                let game = Arc::clone(&game);

                thread::spawn(move || {
                    handle_client(stream, clients, game);
                });
            }
            Err(e) => {
                eprintln!("Connection failed: {}", e);
            }
        }
    }
}
