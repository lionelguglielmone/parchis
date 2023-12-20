use std::collections::HashMap;
use rand::seq::SliceRandom;
use std::net::SocketAddr;


#[derive(PartialEq)]
pub enum GameStatus {
    WaitingForPlayers,
    InProgress,
    GameOver,
}

#[derive(PartialEq, Debug, Clone)]
pub enum Color {
    Red,
    Green,
    Blue,
    Yellow,
    Unassigned,
}

enum PawnPosition {
    House,
    Board(u8),
    Goal,
}

struct Pawn {
    position: PawnPosition,
}

impl Pawn {
    pub fn new() -> Self {
        Pawn { position: PawnPosition::House }
    }
}

pub struct Player {
    name: String,
    pawns: Vec<Pawn>,
    color: Color,
    pub socket_addr: SocketAddr, 
    is_fully_registered: bool,
}

impl Player {

    pub fn mark_as_fully_registered(&mut self) {
        self.is_fully_registered = true;
    }

    pub fn set_color(&mut self, new_color: Color) {
        self.color = new_color;
    }

    pub fn initialize_pawns(&mut self) {
        for pawn in &mut self.pawns {
            pawn.position = PawnPosition::House;
        }
    }

    pub fn get_pawn_positions(&self) -> String {
        self.pawns.iter().enumerate().map(|(index, pawn)| {
            let position = match &pawn.position {
                PawnPosition::House => "House".to_string(),
                PawnPosition::Board(pos) => pos.to_string(),
                PawnPosition::Goal => "Goal".to_string(),
            };
            format!("Pawn {}: {}", index + 1, position)
        }).collect::<Vec<_>>().join(", ")
    }

    pub fn get_pawn_counts(&self) -> (usize, usize) {
        let pawns_in_house = self.pawns.iter().filter(|p| matches!(p.position, PawnPosition::House)).count();
        let pawns_on_board = self.pawns.iter().filter(|p| matches!(p.position, PawnPosition::Board(_))).count();
        (pawns_in_house, pawns_on_board)
    }

    pub fn move_pawn_out(&mut self) {
        if let Some(pawn) = self.pawns.iter_mut().find(|p| matches!(p.position, PawnPosition::House)) {
            pawn.position = PawnPosition::Board(1);
        }
    }

    pub fn is_pawn_in_goal(&self, pawn_number: usize) -> bool {
        self.pawns.get(pawn_number - 1)
            .map_or(false, |pawn| matches!(pawn.position, PawnPosition::Goal))
    }

    pub fn all_pawns_in_goal(&self) -> bool {
        self.pawns.iter().all(|pawn| matches!(pawn.position, PawnPosition::Goal))
    }


    pub fn first_pawn_on_board_number(&self) -> Option<usize> {
        self.pawns.iter().enumerate()
            .find_map(|(index, pawn)| match pawn.position {
                PawnPosition::Board(pos) if pos < 58 => Some(index + 1),
                _ => None,
            })
    }

    pub fn move_pawn(&mut self, pawn_number: usize, dice_value: u8) {
        if pawn_number == 0 || pawn_number > self.pawns.len() {
            return; 
        }
        let pawn_index = pawn_number - 1;

        if let Some(pawn) = self.pawns.get_mut(pawn_index) {
            match pawn.position {
                PawnPosition::Board(pos) => {
                    let new_position = (pos as u32 + dice_value as u32) as u8;
                    if new_position >= 58 {
                        pawn.position = PawnPosition::Goal;
                    } else {
                        pawn.position = PawnPosition::Board(new_position);
                    }
                }
                _ => {}
            }
        }
    }

    pub fn is_valid_pawn_number(&self, pawn_number: usize) -> bool {
        if pawn_number == 0 || pawn_number > self.pawns.len() {
            return false;
        }
        match self.pawns[pawn_number - 1].position {
            PawnPosition::Board(_) => true,
            _ => false,
        }
    }

}

pub struct Game {
    players: HashMap<String, Player>,
    current_turn: Option<String>,
    last_dice_roll: HashMap<String, u8>,
    status: GameStatus,
}

impl Game {
    pub fn new() -> Self {
        Game {
            players: HashMap::new(),
            current_turn: None,
            last_dice_roll: HashMap::new(),
            status: GameStatus::WaitingForPlayers,
        }
    }

    pub fn get_board_state(&self) -> String {
        self.players.iter().map(|(name, player)| {
            format!("{}: {}", name, player.get_pawn_positions())
        }).collect::<Vec<_>>().join("\n")
    }



    pub fn add_player(&mut self, name: String, addr: SocketAddr) {
        let player = Player {
            name: name.clone(),
            pawns: vec![Pawn::new(), Pawn::new(), Pawn::new(), Pawn::new()],
            color: Color::Unassigned,
            socket_addr: addr,
            is_fully_registered: false,
        };
        self.players.insert(name.to_lowercase(), player);
    }


    pub fn get_player(&self, name: &str) -> Option<&Player> {
        self.players.get(name)
    }


    pub fn start_game(&mut self) {
        if self.status == GameStatus::WaitingForPlayers && self.players.len() >= 2 {
            self.status = GameStatus::InProgress;

            let mut player_names: Vec<_> = self.players.keys().cloned().collect();
            let mut rng = rand::thread_rng();
            player_names.shuffle(&mut rng);

            self.current_turn = Some(player_names[0].clone());
        }
    }

    pub fn get_last_dice_roll(&self, player_name: &str) -> Option<u8> {
        self.last_dice_roll.get(player_name).cloned()
    }

    pub fn get_player_mut(&mut self, name: &str) -> Option<&mut Player> {
        self.players.get_mut(name)
    }

    pub fn iter_players(&self) -> impl Iterator<Item = (&String, &Player)> {
        self.players.iter()
    }

    pub fn available_colors(&self) -> Vec<Color> {
        let all_colors = vec![Color::Red, Color::Green, Color::Blue, Color::Yellow];
        all_colors.into_iter().filter(|color| {
            !self.players.values().any(|player| player.color == *color)
        }).collect()
    }

    pub fn can_add_player(&self) -> bool {
        self.status == GameStatus::WaitingForPlayers && self.players.len() < 4
    }

    pub fn get_turn_order_message(&self) -> String {
        if let Some(current_turn) = &self.current_turn {
            let mut message = String::from("Turn Order:\n");
            let player_names: Vec<String> = self.players.keys().cloned().collect();
            for name in player_names {
                if name == *current_turn {
                    message.push_str(&format!("-> {} (starts)\n", name));
                } else {
                    message.push_str(&format!("-> {}\n", name));
                }
            }
            message
        } else {
            String::from("No turn order set.")
        }
    }

    pub fn get_pawn_positions_message(&self) -> String {
        self.players.iter().map(|(name, player)| {
            let color = format!("{:?}", player.color);
            format!("{} ({}): {}", name, color, player.get_pawn_positions())
        }).collect::<Vec<_>>().join("\n")
    }

    pub fn get_players_mut(&mut self) -> &mut HashMap<String, Player> {
        &mut self.players
    }

    pub fn is_in_progress(&self) -> bool {
        self.status == GameStatus::InProgress
    }

    pub fn set_status(&mut self, new_status: GameStatus) {
        self.status = new_status;
    }

    pub fn get_current_turn(&self) -> Option<&String> {
        self.current_turn.as_ref()
    }

    pub fn num_players(&self) -> usize {
        self.players.len()
    }

    pub fn set_last_dice_roll(&mut self, player_name: &str, value: u8) {
        self.last_dice_roll.insert(player_name.to_string(), value);
    }

    pub fn next_turn(&mut self) {
        let player_names: Vec<String> = self.players.keys().cloned().collect();
        if let Some(current_turn) = &self.current_turn {
            let current_index = player_names.iter().position(|name| name == current_turn).unwrap_or(0);
            self.current_turn = player_names.get((current_index + 1) % player_names.len()).cloned();
        }
    }
}