#![allow(dead_code)]
use std::{
    env,
    fmt::Display,
    io::{self, Cursor, Read},
    isize,
    net::{IpAddr, UdpSocket},
    rc::Rc,
    str::FromStr,
};

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

impl Piece {
    fn other(&self) -> Piece {
        match self {
            Piece::None => Piece::None,
            Piece::X => Piece::O,
            Piece::O => Piece::X,
        }
    }

    fn as_u8(&self) -> u8 {
        match self {
            Piece::None => 0,
            Piece::X => 1,
            Piece::O => 2,
        }
    }

    fn from_u8(u8: u8) -> Piece {
        match u8 {
            0 => Piece::None,
            1 => Piece::X,
            2 => Piece::O,
            _ => panic!(),
        }
    }
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
type Player = Rc<dyn PlayerTrait>;

#[derive(Clone)]
struct GameState {
    active: Option<(usize, usize)>,
    message: Option<String>,
    game: Game,
    turn: Piece,
    player_1: Player,
    player_2: Player,
}

impl GameState {
    fn update_to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        match self.active {
            Some((x, y)) => bytes.extend([2, x as u8, y as u8]),
            None => bytes.push(0),
        };
        bytes.push(self.turn.as_u8());

        let mut positions = Vec::new();
        for x1 in 0..3 {
            for y1 in 0..3 {
                for x0 in 0..3 {
                    for y0 in 0..3 {
                        positions.push(self.game[x1][y1][x0][y0].as_u8());
                    }
                }
            }
        }

        bytes
    }

    fn update_from_bytes(&mut self, bytes: &[u8]) {
        let mut cursor = Cursor::new(bytes);
        let mut buf = [0u8; 1];
        cursor.read_exact(&mut buf).unwrap();
        if buf[0] > 0 {
            let mut buf = [0u8; 2];
            cursor.read_exact(&mut buf).unwrap();
            self.active = Some((buf[0] as usize, buf[1] as usize));
        } else {
            self.active = None;
        }

        cursor.read_exact(&mut buf).unwrap();
        self.turn = Piece::from_u8(buf[0]);

        for x1 in 0..3 {
            for y1 in 0..3 {
                for x0 in 0..3 {
                    for y0 in 0..3 {
                        cursor.read_exact(&mut buf).unwrap();
                        self.game[x1][y1][x0][y0] = Piece::from_u8(buf[0]);
                    }
                }
            }
        }
    }

    fn new(player_1: Player, player_2: Player) -> Self {
        Self {
            active: None,
            message: None,
            game: new_game(),
            turn: Piece::X,
            player_1,
            player_2,
        }
    }

    fn print(&self) {
        print_game(&self.game, &self.active);
    }

    fn manual_turn(&mut self, x: usize, y: usize) -> bool {
        if self.turn == Piece::None {
            self.turn = Piece::X
        }

        let x0 = x % 3;
        let y0 = y % 3;
        let x1 = x / 3;
        let y1 = y / 3;

        let won = subgame_won(&self.game[x1][y1]);
        if won != Piece::None {
            return false;
        }

        if subgame_is_draw(&self.game[x1][y1]) {
            return false;
        }

        if self.active.is_some() {
            let (ax, ay) = self.active.unwrap();
            if x1 != ax || y1 != ay {
                return false;
            }
        }

        let piece = self.game[x1][y1][x0][y0];
        if piece != Piece::None {
            return false;
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
            return true;
        }

        if self.turn == Piece::X {
            self.turn = Piece::O;
        } else {
            self.turn = Piece::X;
        }
        return true;
    }

    fn turn(&mut self) -> Option<(usize, usize)> {
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
                return pos;
            }

            if self.turn == Piece::X {
                self.turn = Piece::O;
            } else {
                self.turn = Piece::X;
            }
            return pos;
        }
    }

    fn is_complete(&self) -> bool {
        self.is_draw() || self.won() != Piece::None
    }

    fn is_draw(&self) -> bool {
        game_is_draw(&self.game)
    }

    fn won(&self) -> Piece {
        game_won(&self.game)
    }
}

trait PlayerTrait {
    fn play(
        &self,
        game: &Game,
        turn: &Piece,
        active: Option<(usize, usize)>,
    ) -> Option<(usize, usize)>;
}

#[derive(Clone)]
struct Human;
impl PlayerTrait for Human {
    fn play(
        &self,
        _game: &Game,
        turn: &Piece,
        active: Option<(usize, usize)>,
    ) -> Option<(usize, usize)> {
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

#[derive(Clone)]
struct Random;
impl PlayerTrait for Random {
    fn play(
        &self,
        _game: &Game,
        _turn: &Piece,
        _active: Option<(usize, usize)>,
    ) -> Option<(usize, usize)> {
        let x: u8 = rand::random();
        let y: u8 = rand::random();
        Some((x as usize % 9, y as usize % 9))
    }
}

const WIN_SCORE: isize = 1_000_000;
const MAX_DEPTH: usize = 8;
struct Minimax;

impl Minimax {
    fn eval(game: &Game, me: &Piece) -> isize {
        let mut points: isize = 0;

        for x in 0..3 {
            for y in 0..3 {
                let subgame = game[x][y];

                let subgame_won = subgame_won(&subgame);
                if subgame_won == Piece::None {
                    if subgame_is_draw(&subgame) {
                        points -= 10;
                    } else {
                        points += subgame_score(&subgame, me);
                    }
                } else if subgame_won == *me {
                    points += 100;
                } else {
                    points -= 100;
                }
            }
        }

        let game_won = game_won(&game);
        if game_won == Piece::None {
        } else if game_won == *me {
            points += WIN_SCORE;
        } else {
            points -= WIN_SCORE;
        }

        points
    }

    fn play_inner(
        depth: usize,
        game: &GameState,
        me: Piece,
        mut alpha: isize,
        beta: isize,
    ) -> isize {
        // terminal or cutoff
        if depth >= MAX_DEPTH || game_won(&game.game) != Piece::None {
            return Self::eval(&game.game, &me);
        }

        let mut best = isize::MIN;

        for x1 in 0..3 {
            for y1 in 0..3 {
                if subgame_won(&game.game[x1][y1]) != Piece::None
                    || subgame_is_draw(&game.game[x1][y1])
                {
                    continue;
                }

                if let Some((x, y)) = game.active {
                    if x1 != x || y1 != y {
                        continue;
                    }
                }

                for x0 in 0..3 {
                    for y0 in 0..3 {
                        if game.game[x1][y1][x0][y0] == Piece::None {
                            let mut next = game.clone();

                            if next.manual_turn(x1 * 3 + x0, y1 * 3 + y0) {
                                // match game_won(&next.game) {
                                //     p if p == me => return WIN_SCORE - depth as isize,
                                //     p if p != Piece::None => return -WIN_SCORE + depth as isize,
                                //     _ => {}
                                // }

                                // NEGAMAX RECURSION
                                let score = -Self::play_inner(depth + 1, &next, me, -beta, -alpha);

                                best = best.max(score);
                                alpha = alpha.max(best);

                                // ✂️ BETA CUTOFF
                                if alpha >= beta {
                                    return alpha;
                                }
                            }
                        }
                    }
                }
            }
        }

        best
    }
}

impl PlayerTrait for Minimax {
    fn play(
        &self,
        game: &Game,
        turn: &Piece,
        active: Option<(usize, usize)>,
    ) -> Option<(usize, usize)> {
        println!("Thinking...");
        // let eval = Self::eval(game, turn);
        // println!("Current score: {eval}");

        let mut moves = Vec::new();
        for x1 in 0..3 {
            for y1 in 0..3 {
                if subgame_won(&game[x1][y1]) != Piece::None || subgame_is_draw(&game[x1][y1]) {
                    continue;
                }

                if let Some((x, y)) = active {
                    if !(x1 == x && y1 == y) {
                        continue;
                    }
                }

                for x0 in 0..3 {
                    for y0 in 0..3 {
                        if game[x1][y1][x0][y0] == Piece::None {
                            moves.push((x1, y1, x0, y0));
                        }
                    }
                }
            }
        }

        let mut bests = Vec::new();
        let mut highest_score = isize::MIN;

        let game_ = game;
        let mut game = GameState::new(Rc::new(Minimax), Rc::new(Minimax));
        game.game = *game_;
        game.active = active;
        game.turn = *turn;
        for (x1, y1, x0, y0) in moves.iter() {
            let mut game = game.clone();

            if game.manual_turn(x1 * 3 + x0, y1 * 3 + y0) {
                let score = Self::play_inner(0, &game, *turn, isize::MIN + 1, isize::MAX);
                // println!("{}: {}", pos_as_string(&(x1 * 3 + x0, y1 * 3 + y0)), score);

                if score > highest_score {
                    bests.clear();
                    bests.push((x1 * 3 + x0, y1 * 3 + y0));
                    highest_score = score;
                } else if score == highest_score {
                    bests.push((x1 * 3 + x0, y1 * 3 + y0));
                }
            }
        }

        if bests.len() == 0 {
            let rand: u32 = rand::random();
            let rand = rand as usize % moves.len();
            let (x1, y1, x0, y0) = moves[rand];

            bests.push((x1 * 3 + x0, y1 * 3 + y0));
        }

        let rand: u32 = rand::random();
        let rand = rand as usize % bests.len();

        Some(bests[rand])
    }
}

struct Local(IpAddr);
impl PlayerTrait for Local {
    fn play(
        &self,
        game: &Game,
        turn: &Piece,
        active: Option<(usize, usize)>,
    ) -> Option<(usize, usize)> {
        let game_ = game;
        let mut game = GameState::new(Rc::new(Human), Rc::new(Human));
        game.game = *game_;
        game.active = active;
        game.turn = *turn;

        let pos = game.turn();
        let update = game.update_to_bytes();

        if let Ok(socket) = UdpSocket::bind("0.0.0.0") {
            socket
                .send_to(&update, (self.0, 2003))
                .expect("Failed to send game update");
        } else {
            panic!("Failed to bind socket")
        }

        pos
    }
}

struct Remote(IpAddr);

impl PlayerTrait for Remote {
    fn play(
        &self,
        _game: &Game,
        _turn: &Piece,
        _active: Option<(usize, usize)>,
    ) -> Option<(usize, usize)> {
        // if let Ok(socket) = UdpSocket::bind("0.0.0.0") {
        //     socket.connect(addr)
        // }

        None
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

fn subgame_score(subgame: &Subgame, me: &Piece) -> isize {
    if me == &Piece::None {
        return 0;
    }

    let mut score = 0;
    for win in WINS {
        let a = &subgame[win[0].0][win[0].1];
        let b = &subgame[win[1].0][win[1].1];
        let c = &subgame[win[2].0][win[2].1];
        let all = [a, b, c];
        let me_count = all.iter().filter(|piece| piece == &&me).count();
        let them_count = all.iter().filter(|piece| piece == &&&me.other()).count();

        if me_count == 2 && them_count == 0 {
            score += 1;
        }

        if me_count == 0 && them_count == 2 {
            score -= 1;
        }
    }
    score
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
                    (1, 1) => print!(" {COLOR_X}x{RESET} "),
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
                    (1, 1) => print!(" {COLOR_O}o{RESET} "),
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
            print!("+");
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
                    if subgame_won(&game[x1][y1]) == Piece::None {
                        print!("+");
                    } else if subgame_won(&game[x1][y1]) == Piece::O {
                        print!(" ");
                    } else if subgame_won(&game[x1][y1]) == Piece::X {
                        match (x0, y0) {
                            (0, 0) => print!("{COLOR_X}╲{RESET}"),
                            (1, 0) => print!("{COLOR_X}╱{RESET}"),
                            (0, 1) => print!("{COLOR_X}╱{RESET}"),
                            (1, 1) => print!("{COLOR_X}╲{RESET}"),
                            (_, 2) => print!("+"),
                            _ => print!(" "),
                        }
                    }
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
    let mut chars = pos.to_lowercase().chars().collect::<Vec<_>>();
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

fn player_from_string(string: &str) -> Option<Player> {
    match string.to_lowercase().as_str() {
        "human" => Some(Rc::new(Human)),
        "random" => Some(Rc::new(Random)),
        "smart" => Some(Rc::new(Minimax)),
        other => match IpAddr::from_str(other) {
            Ok(addr) => Some(Rc::new(Remote(addr))),
            Err(_) => None,
        },
    }
}

fn main() {
    let mut player_1: Player = Rc::new(Human);
    let mut player_2: Player = Rc::new(Human);

    let mut args = env::args();
    args.next();

    if let Some(next) = args.next() {
        if let Ok(addr) = IpAddr::from_str(&next) {
            player_1 = match args.next().unwrap().as_str() {
                "local" => Rc::new(Local(addr)),
                "remote" => Rc::new(Remote(addr)),
                _ => panic!("Must be either 'local' or 'remote'"),
            };

            player_2 = match args.next().unwrap().as_str() {
                "local" => Rc::new(Local(addr)),
                "remote" => Rc::new(Remote(addr)),
                _ => panic!("Must be either 'local' or 'remote'"),
            };
        } else {
            player_1 = player_from_string(&next).unwrap();

            if let Some(next) = args.next() {
                player_2 = player_from_string(&next).unwrap();
            }
        }
    }

    let mut game = GameState::new(player_1, player_2);
    while !game.is_complete() {
        game.print();
        game.turn();
    }

    game.print();
    if game.is_draw() {
        println!("It's a draw!");
    } else {
        println!("{} wins!", game.turn);
    }
}
