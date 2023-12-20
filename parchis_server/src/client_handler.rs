use std::sync::MutexGuard;
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use crate::game_state::Game;
use crate::communication::broadcast_message;
use std::io::{Read, Write};
use rand::Rng;
use crate::game_state::Color;
use crate::game_state::GameStatus;




pub fn handle_client(mut stream: TcpStream, clients: Arc<Mutex<Vec<TcpStream>>>, game: Arc<Mutex<Game>>) {
    let mut buffer = [0; 1024];

    let welcome_msg = "Welcome to Parchis! Enter 'JOIN <name>' to join the game.\nEND_OF_MESSAGE\n";
    stream.write_all(welcome_msg.as_bytes()).expect("Failed to send welcome message");
    stream.flush().expect("Failed to flush stream"); 

    let clients_clone = Arc::clone(&clients);

    {
        let mut clients = clients.lock().unwrap();
        clients.push(stream.try_clone().expect("Failed to clone stream"));
    }

    loop {
        match stream.read(&mut buffer) {
            Ok(size) => {
                if size == 0 {
                    break;
                }

                let message = String::from_utf8_lossy(&buffer[..size]).trim().to_string();
                println!("Received message: {}", message);

                handle_message(&message, &stream, &game, &clients_clone);
            }
            Err(e) => {
                eprintln!("Error reading from stream: {}", e);
                break;
            }
        }
    }

    remove_client(&stream, &clients_clone);
}



fn handle_message(message: &str, mut stream: &TcpStream, game: &Arc<Mutex<Game>>, clients: &Arc<Mutex<Vec<TcpStream>>>) {
    let command_parts: Vec<&str> = message.split_whitespace().collect();
    match command_parts[0] {

        "JOIN" => {
            let mut game_guard = game.lock().unwrap();
            if !game_guard.can_add_player() {
                let response = "Game is full or already started.\nEND_OF_MESSAGE\n";
                stream.write_all(response.as_bytes()).expect("Failed to write response");
                return;
            }
        
            if let Some(name) = command_parts.get(1) {
                let player_addr = stream.peer_addr().expect("Failed to get peer address");
                
                game_guard.add_player(name.to_string(), player_addr);
        
                let available_colors = game_guard.available_colors();
                let color_options: String = available_colors
                    .iter()
                    .enumerate()
                    .map(|(i, color)| format!("{}. {:?}", i + 1, color))
                    .collect::<Vec<_>>()
                    .join("\n");
        
                let color_options_message = format!("Choose your color by typing 'COLOR <color>' where <color> is one of the following:\n{}\nEND_OF_MESSAGE\n", color_options);
                stream.write_all(color_options_message.as_bytes()).expect("Failed to send color options");
                stream.flush().expect("Failed to flush stream");
            } else {
                let response = "Please provide a name. Usage: JOIN <name>\nEND_OF_MESSAGE\n";
                stream.write_all(response.as_bytes()).expect("Failed to write response");
            }
        },
        
        "COLOR" => {
            if let Some(player_name) = get_player_name_from_connection(&stream, &game) {
                if let Some(color_str) = command_parts.get(1) {
                    let mut game_guard = game.lock().unwrap();
        
                    let num_players = game_guard.num_players();
        
                    match parse_color(color_str) {
                        Some(color) if game_guard.available_colors().contains(&color) => {
                            if let Some(player) = game_guard.get_player_mut(&player_name.to_lowercase()) {
                                player.set_color(color.clone());
                                player.mark_as_fully_registered();
        
                                let success_message = format!(
                                    "{} successfully registered with color {:?}. Waiting for other players...\nEND_OF_MESSAGE\n",
                                    player_name, color
                                );
        
                                broadcast_message(&success_message, Some(&stream), &clients);
        
                                let personal_success_message = format!("You have successfully registered with color {:?}. Waiting for other players...\nEND_OF_MESSAGE\n", color);
                                stream.write_all(personal_success_message.as_bytes()).expect("Failed to send personal confirmation");
        
                                if num_players >= 2 {
                                    let start_game_message = format!(
                                        "{} players have joined. Anyone can start the game by typing 'BEGIN'.\nEND_OF_MESSAGE\n",
                                        num_players
                                    );
                                    broadcast_message(&start_game_message, None, &clients);
                                }
                            } else {
                                let error_message = "Error: Player not found or already registered.\nEND_OF_MESSAGE\n";
                                stream.write_all(error_message.as_bytes()).expect("Failed to write response");
                                stream.flush().expect("Failed to flush stream");
                            }
                        },
                        _ => {
                            let response = "Invalid color choice or color not available.\nEND_OF_MESSAGE\n";
                            stream.write_all(response.as_bytes()).expect("Failed to write response");
                            stream.flush().expect("Failed to flush stream");
                        }
                    }
                }
            } else {
                let error_message = "Unable to identify player.\nEND_OF_MESSAGE\n";
                stream.write_all(error_message.as_bytes()).expect("Failed to write response");
                stream.flush().expect("Failed to flush stream");
            }
        },
        
        "BEGIN" => {
            let mut game_guard = game.lock().unwrap();
            if game_guard.num_players() >= 2 && !game_guard.is_in_progress() {
                game_guard.start_game();
        
                for player in game_guard.get_players_mut().values_mut() {
                    player.initialize_pawns();
                }
        
                let turn_order_message = game_guard.get_turn_order_message();
                let pawn_positions_message = game_guard.get_pawn_positions_message();
        
                let current_turn = game_guard.get_current_turn()
                    .cloned()
                    .unwrap_or_else(|| "Unknown".to_string());
        
                let game_start_message = format!(
                    "Game started!\n{}\n{}\nGood luck to all players!\nIt's {}'s turn to roll the dice.\nEND_OF_MESSAGE\n",
                    turn_order_message, pawn_positions_message, current_turn
                );
                broadcast_message(&game_start_message, None, &clients);
        
                game_guard.set_status(GameStatus::InProgress);
            } else if game_guard.is_in_progress() {
                let response = "Game has already started.\nEND_OF_MESSAGE\n";
                stream.write_all(response.as_bytes()).expect("Failed to write response");
            } else {
                let response = "Not enough players to start the game.\nEND_OF_MESSAGE\n";
                stream.write_all(response.as_bytes()).expect("Failed to write response");
            }
        },
        
        "ROLL" => {
            println!("Received ROLL command");
        
            let player_addr_str = match stream.peer_addr() {
                Ok(addr) => {
                    println!("Player's Address for ROLL: {:?}", addr);
                    addr.to_string()
                },
                Err(e) => {
                    eprintln!("Error getting player's address: {}", e);
                    return;
                }
            };
        
            let mut game_guard = game.lock().unwrap();
        
            if let Some(current_turn) = game_guard.get_current_turn().cloned() {
                let (player_socket_addr, (pawns_in_house, pawns_on_board)) = {
                    let current_player = game_guard.get_player_mut(&current_turn).expect("Current player not found");
                    (current_player.socket_addr.clone(), current_player.get_pawn_counts())
                };
        
                if player_socket_addr.to_string() == player_addr_str {
                    println!("It's {}'s turn", current_turn);
        
                    let dice_value = rand::thread_rng().gen_range(1..=6);
                    game_guard.set_last_dice_roll(&current_turn, dice_value);
        
                    let broadcast_roll_message = format!("{} rolled a {}.\nEND_OF_MESSAGE\n", current_turn, dice_value);
                    broadcast_message(&broadcast_roll_message, Some(&stream), &clients);

                    let personal_roll_message = format!("You rolled a {}.\nEND_OF_MESSAGE\n", dice_value);
                    stream.write_all(personal_roll_message.as_bytes()).expect("Failed to send roll message");

                    match (pawns_in_house, pawns_on_board, dice_value) {
                        (4, 0, 6) => {
                            let move_out_message = "You can move a pawn out of the house. Type 'MOVE_OUT'.\nEND_OF_MESSAGE\n";
                            stream.write_all(move_out_message.as_bytes()).expect("Failed to send move out message");
                        },
                        (4, 0, _) => {
                            let cannot_move_message = "You need a 6 to move a pawn out of the house.\nEND_OF_MESSAGE\n";
                            stream.write_all(cannot_move_message.as_bytes()).expect("Failed to send cannot move message");

                            let board_state = game_guard.get_board_state();
                            broadcast_message(&format!("{}\nEND_OF_MESSAGE\n", board_state), None, &clients);
                    
                            game_guard.next_turn();
                            notify_next_player_turn(&game_guard, &clients);
                        },
                            (1..=3, _, 6) => {
                            let move_message = "Type 'MOVE_OUT' to move a pawn out of the house or 'MOVE <pawn number>' to move a pawn on the board.\nEND_OF_MESSAGE\n";
                            stream.write_all(move_message.as_bytes()).expect("Failed to send move message");
                        },


                        (_, _, _) => {
                            if let Some(current_player) = game_guard.get_player_mut(&current_turn) {
                                let (pawns_in_house, pawns_on_board) = current_player.get_pawn_counts();
                                if pawns_in_house == 3 && pawns_on_board == 1 {
                                    if let Some(pawn_number) = current_player.first_pawn_on_board_number() {
                                        current_player.move_pawn(pawn_number, dice_value);
                
                                        if current_player.is_pawn_in_goal(pawn_number) {
                                            let goal_message = format!("Your pawn {} reached the goal!\nEND_OF_MESSAGE\n", pawn_number);
                                            stream.write_all(goal_message.as_bytes()).expect("Failed to send goal message");
                                        } else {
                                            let auto_move_message = format!("Your pawn {} on the board has been moved.\nEND_OF_MESSAGE\n", pawn_number);
                                            stream.write_all(auto_move_message.as_bytes()).expect("Failed to send auto move message");
                                        }

                                        if current_player.all_pawns_in_goal() {
                                            let winner_announcement = format!("{} has won the game!\nType 'END' to close the game.\nEND_OF_MESSAGE\n", current_turn);
                                            broadcast_message(&winner_announcement, None, &clients);
                                            game_guard.set_status(GameStatus::GameOver);
                                            return;
                                        }
                                    }
                
                                    let board_state = game_guard.get_board_state();
                                    broadcast_message(&format!("{}\nEND_OF_MESSAGE\n", board_state), None, &clients);
                            
                                    game_guard.next_turn();
                                    notify_next_player_turn(&game_guard, &clients);
                                } else {
                                    let move_prompt = "Choose a pawn to move. Type 'MOVE <pawn number>'.\nEND_OF_MESSAGE\n";
                                    stream.write_all(move_prompt.as_bytes()).expect("Failed to send move prompt");
                                }
                            } else {
                                let error_message = "Error: Current player not found.\nEND_OF_MESSAGE\n";
                                stream.write_all(error_message.as_bytes()).expect("Failed to send error message");
                            }
                        }
                    }

                } else {
                    let not_your_turn_message = format!("It's not your turn, it's {}'s turn.\nEND_OF_MESSAGE\n", current_turn);
                    stream.write_all(not_your_turn_message.as_bytes()).expect("Failed to write response");
                }
            } else {
                let error_message = "The game hasn't started yet.\nEND_OF_MESSAGE\n";
                stream.write_all(error_message.as_bytes()).expect("Failed to write response");
            }
        }
        

        "MOVE_OUT" => {
            let player_addr_str = match stream.peer_addr() {
                Ok(addr) => addr.to_string(),
                Err(e) => {
                    eprintln!("Error getting player's address: {}", e);
                    return;
                }
            };
        
            let mut game_guard = game.lock().unwrap();
            let current_turn = game_guard.get_current_turn().cloned();
        
            if let Some(current_turn) = current_turn {
                let last_dice_roll = game_guard.get_last_dice_roll(&current_turn);
        
                if let Some(current_player) = game_guard.get_player_mut(&current_turn) {
                    if current_player.socket_addr.to_string() == player_addr_str {
                        if let Some(6) = last_dice_roll {
                            let (pawns_in_house, _) = current_player.get_pawn_counts();
        
                            if pawns_in_house >= 1 {
                                current_player.move_pawn_out();
        
                                let move_out_success_message = "A pawn has been moved out of the house.\nEND_OF_MESSAGE\n";
                                stream.write_all(move_out_success_message.as_bytes()).expect("Failed to send move out success message");
        
                                let board_state = game_guard.get_board_state();
                                broadcast_message(&format!("{}\nEND_OF_MESSAGE\n", board_state), None, &clients);
        
                                game_guard.next_turn();
                                notify_next_player_turn(&game_guard, &clients);
                            } else {
                                let invalid_move_out_message = "You cannot move a pawn out right now.\nEND_OF_MESSAGE\n";
                                stream.write_all(invalid_move_out_message.as_bytes()).expect("Failed to send invalid move out message");
                            }
                        } else {
                            let not_your_turn_message = "It's not your turn.\nEND_OF_MESSAGE\n";
                            stream.write_all(not_your_turn_message.as_bytes()).expect("Failed to write response");
                        }
                    } else {
                        println!("Current turn player not found.");
                    }
                } else {
                    let error_message = "The game hasn't started yet.\nEND_OF_MESSAGE\n";
                    stream.write_all(error_message.as_bytes()).expect("Failed to write response");
                }
            }
        }

        "MOVE" => {
            let player_addr_str = match stream.peer_addr() {
                Ok(addr) => addr.to_string(),
                Err(e) => {
                    eprintln!("Error getting player's address: {}", e);
                    return;
                }
            };
        
            let current_turn_clone;
            let last_dice_roll;
        
            {
                let game_guard = game.lock().unwrap();
                current_turn_clone = game_guard.get_current_turn().cloned();
                last_dice_roll = game_guard.get_last_dice_roll(&current_turn_clone.clone().unwrap_or_default());
            }
        
            let move_result = {
                let mut game_guard = game.lock().unwrap();
        
                if let Some(current_turn) = &current_turn_clone {
                    if let Some(current_player) = game_guard.get_player_mut(current_turn) {
                        if current_player.socket_addr.to_string() == player_addr_str {
                            if let Some(pawn_number_str) = command_parts.get(1) {
                                if let Ok(pawn_number) = pawn_number_str.parse::<usize>() {
                                    if current_player.is_valid_pawn_number(pawn_number) && !current_player.is_pawn_in_goal(pawn_number) {
                                        if let Some(dice_value) = last_dice_roll {
                                            current_player.move_pawn(pawn_number, dice_value);
                                            Some((current_turn.clone(), current_player.all_pawns_in_goal()))
                                        } else {
                                            let error_message = "No dice roll found.\nEND_OF_MESSAGE\n";
                                            stream.write_all(error_message.as_bytes()).expect("Failed to send error message");
                                            None
                                        }
                                    } else {
                                        let error_message = "Invalid pawn number, pawn not on board, or pawn already in goal.\nEND_OF_MESSAGE\n";
                                        stream.write_all(error_message.as_bytes()).expect("Failed to send error message");
                                        None
                                    }
                                } else {
                                    let error_message = "Invalid pawn number format.\nEND_OF_MESSAGE\n";
                                    stream.write_all(error_message.as_bytes()).expect("Failed to send error message");
                                    None
                                }
                            } else {
                                let error_message = "Please specify which pawn to move (e.g., 'MOVE 1').\nEND_OF_MESSAGE\n";
                                stream.write_all(error_message.as_bytes()).expect("Failed to send error message");
                                None
                            }
                        } else {
                            let not_your_turn_message = "It's not your turn.\nEND_OF_MESSAGE\n";
                            stream.write_all(not_your_turn_message.as_bytes()).expect("Failed to write response");
                            None
                        }
                    } else {
                        let error_message = "Current turn player not found.\nEND_OF_MESSAGE\n";
                        stream.write_all(error_message.as_bytes()).expect("Failed to send error message");
                        None
                    }
                } else {
                    let error_message = "The game hasn't started yet.\nEND_OF_MESSAGE\n";
                    stream.write_all(error_message.as_bytes()).expect("Failed to write response");
                    None
                }
            };
        
            if let Some((current_turn, all_pawns_in_goal)) = move_result {
                let mut game_guard = game.lock().unwrap();
                let board_state = game_guard.get_board_state();
                broadcast_message(&format!("{}\nEND_OF_MESSAGE\n", board_state), None, &clients);
        
                game_guard.next_turn();
                notify_next_player_turn(&game_guard, &clients);
        
                if all_pawns_in_goal {
                    let winner_announcement = format!("{} has won the game!\nType 'END' to close the game.\nEND_OF_MESSAGE\n", current_turn);
                    broadcast_message(&winner_announcement, None, &clients);
                    game_guard.set_status(GameStatus::GameOver);
                }
            }
        },
        
        _ => {
            println!("Unknown command received: {}", message);
            let error_message = "Unknown or invalid command.\nEND_OF_MESSAGE\n";
            stream.write_all(error_message.as_bytes()).expect("Failed to send error message");

        }
    }
}


//METHODS

fn notify_next_player_turn(game_guard: &MutexGuard<Game>, clients: &Arc<Mutex<Vec<TcpStream>>>) {
    if let Some(next_player) = game_guard.get_current_turn() {
        let next_turn_message_personal = "It's now your turn to roll the dice.\nEND_OF_MESSAGE\n";
        let next_turn_message_broadcast = format!("It's now {}'s turn to roll the dice.\nEND_OF_MESSAGE\n", next_player);

        for mut client in clients.lock().unwrap().iter() {
            let client_addr_str = client.peer_addr().unwrap().to_string();
            if let Some(next_player_data) = game_guard.get_player(next_player) {
                if next_player_data.socket_addr.to_string() == client_addr_str {
                    client.write_all(next_turn_message_personal.as_bytes()).expect("Failed to send personal turn message");
                } else {
                    client.write_all(next_turn_message_broadcast.as_bytes()).expect("Failed to send broadcast turn message");
                }
            }
        }
    }
}



fn remove_client(stream: &TcpStream, clients: &Arc<Mutex<Vec<TcpStream>>>) {
    let mut clients = clients.lock().unwrap();
    clients.retain(|client| client.peer_addr().unwrap() != stream.peer_addr().unwrap());
}

fn parse_color(color_str: &str) -> Option<Color> {
    match color_str.to_lowercase().as_str() {
        "red" => Some(Color::Red),
        "green" => Some(Color::Green),
        "blue" => Some(Color::Blue),
        "yellow" => Some(Color::Yellow),
        _ => None,
    }
}

fn get_player_name_from_connection(stream: &TcpStream, game: &Arc<Mutex<Game>>) -> Option<String> {
    let player_addr = match stream.peer_addr() {
        Ok(addr) => {
            println!("Player's Address: {:?}", addr);
            addr
        },
        Err(e) => {
            eprintln!("Error getting player's address: {}", e);
            return None;
        }
    };

    let game_guard = game.lock().unwrap();

    for (name, player) in game_guard.iter_players() {
        if player.socket_addr == player_addr {
            return Some(name.clone());
        }
    }
    None
}