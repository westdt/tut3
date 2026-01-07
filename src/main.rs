use std::{fmt::Display, io};

const RESET: &str = "\x1b[0m";
const RED: &str = "\x1b[31m";
const BLUE: &str = "\x1b[34m";
const COLOR_O: &str = RED;
const COLOR_X: &str = BLUE;

const FILES: [char; 9] = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i'];

const WINS: [[(usize, usize); 3]; 8] = [
    [(0, 0), (0, 1), (0, 2)],
    [(1, 0), (1, 1), (1, 2)],
    [(2, 0), (2, 1), (2, 2)],
    [(0, 0), (1, 0), (2, 0)],
    [(0, 1), (1, 1), (2, 1)],
    [(0, 2), (1, 2), (2, 2)],
    [(0, 0), (1, 1), (2, 2)],
    [(0, 2), (1, 1), (2, 0)],
];

#[derive(PartialEq, Clone, Copy)]
enum Piece {
    None,
    X,
    O,
}

impl Display for Piece {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{}{RESET}",
            match self {
                Piece::None => "",
                Piece::X => COLOR_X,
                Piece::O => COLOR_O,
            },
            match self {
                Piece::None => " ",
                Piece::X => "x",
                Piece::O => "o",
            }
        )
    }
}

type Subgame = [[Piece; 3]; 3];
type Game = [[Subgame; 3]; 3];

struct GameState<A: Player, B: Player> {
    active: Option<(usize, usize)>,
    message: Option<String>,
    game: Game,
    turn: Piece,
    player_1: A,
    player_2: B,
}

impl<A: Player, B: Player> GameState<A, B> {
    fn new() -> Self {
        Self {
            active: None,
            message: None,
            game: new_game(),
            turn: Piece::X,
            player_1: A::new(),
            player_2: B::new(),
        }
    }

    fn print(&self) {
        print_game(&self.game, &self.active);
    }

    fn turn(&mut self) {
        loop {
            if self.message.is_some() {
                println!("{}", self.message.as_ref().unwrap());
                self.message = None;
            }

            if self.turn == Piece::None {
                self.turn = Piece::X
            }

            let pos = if self.turn == Piece::X {
                self.player_1.play(&self.game, &self.turn, self.active)
            } else {
                self.player_2.play(&self.game, &self.turn, self.active)
            };

            if pos.is_none() {
                self.message = Some(format!(
                    "Invalid move! That position is not within the game boundaries!"
                ));
                continue;
            }

            let (x, y) = pos.unwrap();
            let x0 = x % 3;
            let y0 = y % 3;
            let x1 = x / 3;
            let y1 = y / 3;

            let won = subgame_won(&self.game[x1][y1]);
            if won != Piece::None {
                self.message = Some(format!(
                    "Invalid move! That position is within a game that has already been won!"
                ));
                continue;
            }

            if subgame_is_draw(&self.game[x1][y1]) {
                self.message = Some(format!(
                    "Invalid move! That position is within a game that has already been drawn!"
                ));
                continue;
            }

            if self.active.is_some() {
                let (ax, ay) = self.active.unwrap();
                if x1 != ax || y1 != ay {
                    self.message = Some(format!(
                        "Invalid move! That position is not within the current active game!"
                    ));
                    continue;
                }
            }

            let piece = self.game[x1][y1][x0][y0];
            if piece != Piece::None {
                self.message = Some(format!(
                    "Invalid move! There is already a piece at that position!"
                ));
                continue;
            }

            self.game[x1][y1][x0][y0] = self.turn.clone();

            let won = subgame_won(&self.game[x0][y0]);
            if won == Piece::None && !subgame_is_draw(&self.game[x0][y0]) {
                self.active = Some((x0, y0));
            } else {
                self.active = None;
            }

            if self.is_complete() {
                self.active = None;
                break;
            }

            if self.turn == Piece::X {
                self.turn = Piece::O;
            } else {
                self.turn = Piece::X;
            }
            break;
        }
    }

    fn is_complete(&self) -> bool {
        self.is_draw() && self.won() != Piece::None
    }

    fn is_draw(&self) -> bool {
        game_is_draw(&self.game)
    }

    fn won(&self) -> Piece {
        game_won(&self.game)
    }
}

trait Player {
    fn new() -> Self;
    fn play(&self, game: &Game, turn: &Piece, active: Option<(usize, usize)>) -> Option<(usize, usize)>;
}

struct Human;
impl Player for Human {
    fn new() -> Self {
        Self
    }

    fn play(&self, _game: &Game, turn: &Piece, active: Option<(usize, usize)>) -> Option<(usize, usize)> {
        println!(
            "It's {}'s turn! You can move in any open square between {} and {}",
            turn,
            pos_as_string(&move_min(&active)),
            pos_as_string(&move_max(&active))
        );

        // Create a mutable string to store the input
        let mut input_text = String::new();

        // Read the line from stdin, store it in input_text, and handle potential errors
        io::stdin()
            .read_line(&mut input_text)
            .expect("Failed to read line");
        input_text = input_text.trim().to_string();

        let pos = string_as_pos(&input_text);
        pos
    }
}

fn new_empty_subgame() -> Subgame {
    [[Piece::None; 3]; 3]
}

fn new_game() -> Game {
    [[new_empty_subgame(); 3]; 3]
}

fn subgame_won(subgame: &Subgame) -> Piece {
    for win in WINS {
        let a = &subgame[win[0].0][win[0].1];
        let b = &subgame[win[1].0][win[1].1];
        let c = &subgame[win[2].0][win[2].1];
        if a != &Piece::None && a == b && a == c {
            return a.clone();
        }
    }
    Piece::None
}

fn subgame_is_draw(subgame: &Subgame) -> bool {
    for win in WINS {
        let a = &subgame[win[0].0][win[0].1];
        let b = &subgame[win[1].0][win[1].1];
        let c = &subgame[win[2].0][win[2].1];
        let all = [a, b, c];

        if !(all.contains(&&Piece::X) && all.contains(&&Piece::O)) {
            return false;
        }
    }
    true
}

fn game_won(game: &Game) -> Piece {
    for win in WINS {
        let a = subgame_won(&game[win[0].0][win[0].1]);
        let b = subgame_won(&game[win[1].0][win[1].1]);
        let c = subgame_won(&game[win[2].0][win[2].1]);
        if a != Piece::None && a == b && a == c {
            return a;
        }
    }
    Piece::None
}

fn game_is_draw(game: &Game) -> bool {
    for win in WINS {
        let a = subgame_is_draw(&game[win[0].0][win[0].1]);
        let b = subgame_is_draw(&game[win[1].0][win[1].1]);
        let c = subgame_is_draw(&game[win[2].0][win[2].1]);

        if a == false && a == b && a == c {
            return false;
        }
    }
    true
}

fn print_game(game: &Game, active: &Option<(usize, usize)>) {
    let show_active = active.is_some();
    let (active_x, active_y) = match active {
        Some((x, y)) => (*x, *y),
        None => (0, 0),
    };

    println!("{}", "\n".repeat(100));
    if show_active && active_x == 0 {
        println!("     a   b   c");
    } else if show_active && active_x == 1 {
        println!("                   d   e   f");
    } else if show_active && active_x == 2 {
        println!("                                 g   h   i");
    } else {
        println!("     a   b   c     d   e   f     g   h   i");
    }
    println!("   +---+---+---+ +---+---+---+ +---+---+---+");
    for y in 0..9 {
        //print!("   |");
        let y0 = y % 3;
        let y1 = y / 3;
        if show_active == false || y1 == active_y {
            print!(" {} |", y + 1);
        } else {
            print!("   |");
        }

        for x in 0..9 {
            let x0 = x % 3;
            let x1 = x / 3;

            let winner = subgame_won(&game[x1][y1]);

            if subgame_is_draw(&game[x1][y1]) {
                print!(" ⋅ ");
            } else if winner == Piece::None {
                let piece = game[x1][y1][x0][y0];
                print!(" {piece} ");
            } else if winner == Piece::X {
                match (x0, y0) {
                    (0, 0) => print!(" {COLOR_X}╲{RESET} "),
                    (2, 0) => print!(" {COLOR_X}╱{RESET} "),
                    (1, 1) => print!(" {COLOR_X}✕{RESET} "),
                    (0, 2) => print!(" {COLOR_X}╱{RESET} "),
                    (2, 2) => print!(" {COLOR_X}╲{RESET} "),
                    _ => print!("   "),
                }
            } else if winner == Piece::O {
                match (x0, y0) {
                    (0, 0) => print!(" {COLOR_O}╭{RESET} "),
                    (1, 0) => print!(" {COLOR_O}-{RESET} "),
                    (2, 0) => print!(" {COLOR_O}╮{RESET} "),
                    (0, 1) => print!(" {COLOR_O}|{RESET} "),
                    (2, 1) => print!(" {COLOR_O}|{RESET} "),
                    (0, 2) => print!(" {COLOR_O}╰{RESET} "),
                    (2, 2) => print!(" {COLOR_O}╯{RESET} "),
                    (1, 2) => print!(" {COLOR_O}-{RESET} "),
                    _ => print!("   "),
                }
            }

            if x0 < 2 {
                if subgame_won(&game[x1][y1]) == Piece::None
                    && (!show_active || (active_x == x1 && active_y == y1))
                {
                    print!("|");
                } else {
                    print!(" ");
                }
            } else if x1 < 2 {
                print!("| |");
            }
        }

        print!("|\n   ");

        if y < 8 {
            if y % 3 != 2 {
                print!("|");
            } else {
                print!("+");
            }
            for x in 0..9 {
                let x0 = x % 3;
                let x1 = x / 3;

                if !show_active || (active_x == x1 && active_y == y1) || y0 == 2 {
                    if y0 == 2 || subgame_won(&game[x1][y1]) == Piece::None {
                        print!("---");
                    } else {
                        print!("   ");
                    }
                } else {
                    print!("   ");
                }

                if x0 == 2 && x1 < 2 {
                    print!("+ +");
                } else {
                    print!("+");
                }
            }
            println!("");

            if y0 == 2 {
                if y % 3 != 2 {
                    print!("   |");
                } else {
                    print!("   +");
                }
                for x in 0..9 {
                    let x0 = x % 3;
                    let x1 = x / 3;

                    if !show_active || ((active_x == x1 && active_y == y1) || y0 == 2) {
                        print!("---");
                    } else {
                        print!("   ");
                    }

                    if x0 == 2 && x1 < 2 {
                        print!("+ +");
                    } else {
                        print!("+");
                    }
                }
                println!("");
            }
        }
    }
    println!("+---+---+---+ +---+---+---+ +---+---+---+");
}

fn move_max(active: &Option<(usize, usize)>) -> (usize, usize) {
    match active {
        Some((x, y)) => (*x * 3 + 2, *y * 3 + 2),
        None => (8, 8),
    }
}

fn move_min(active: &Option<(usize, usize)>) -> (usize, usize) {
    match active {
        Some((x, y)) => (*x * 3, *y * 3),
        None => (0, 0),
    }
}

fn pos_as_string(pos: &(usize, usize)) -> String {
    format!("{}{}", FILES[pos.0], pos.1 + 1)
}

fn string_as_pos(pos: &str) -> Option<(usize, usize)> {
    let mut chars = pos.chars().collect::<Vec<_>>();
    if chars.len() != 2 {
        return None;
    } else {
        let rank = chars.pop().unwrap();
        let file = chars.pop().unwrap();

        let file_i = FILES
            .iter()
            .enumerate()
            .find_map(|(i, f)| if f == &file { Some(i) } else { None });
        let rank_i = rank.to_digit(10);
        if rank_i.is_none() || file_i.is_none() || rank_i.unwrap() < 1 {
            return None;
        }

        Some((file_i.unwrap(), rank_i.unwrap() as usize - 1))
    }
}

fn main() {
    let mut game = GameState::<Human, Human>::new();
    while !game.is_complete() {
        game.print();
        game.turn();
    }

    if game.is_draw() {
        println!("It's a draw!");
    } else {
        println!("{} wins!", game.turn);
    }
}
