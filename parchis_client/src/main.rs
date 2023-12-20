use std::io::{self, Write, BufRead, BufReader};
use std::net::TcpStream;
use std::thread;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use crossterm::event::{self, Event, KeyCode};

fn main() -> std::io::Result<()> {

    let mut stream = TcpStream::connect("127.0.0.1:7878")?;
    println!("Successfully connected to the server at 127.0.0.1:7878");

    let (tx, rx): (mpsc::Sender<String>, Receiver<String>) = mpsc::channel();
    let mut reader = BufReader::new(stream.try_clone()?);

    thread::spawn(move || {
        let mut message_accumulator = String::new();

        loop {
            let mut response = String::new();
            match reader.read_line(&mut response) {
                Ok(_) => {
                    message_accumulator.push_str(&response);

                    if message_accumulator.ends_with("END_OF_MESSAGE\n") {
                        let clean_message = message_accumulator
                            .replace("END_OF_MESSAGE\n", "");
                        tx.send(clean_message).expect("Failed to send response to main thread");
                        message_accumulator.clear();
                    }
                }
                Err(e) => {
                    eprintln!("Failed to read from server: {}", e);
                    break;
                }
            }
        }
    });

    let mut input = String::new();
    loop {
        match rx.try_recv() {
            Ok(message) => print!("{}", message),
            Err(TryRecvError::Empty) => {},
            Err(TryRecvError::Disconnected) => break,
        }

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key_event) = event::read()? {
                match key_event.code {
                    KeyCode::Char(c) => {
                        input.push(c);
                        print!("{}", c);
                        io::stdout().flush()?;
                    }
                    KeyCode::Enter => {
                        println!(); 
                        stream.write_all(input.as_bytes())?;
                        input.clear();
                    }
                    KeyCode::Backspace => {
                        input.pop();
                    }
                    _ => {} 
                }
            }
        }
    }

    Ok(())
}
