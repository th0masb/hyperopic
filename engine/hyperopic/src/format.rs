use crate::constants::{piece_class, side};
use crate::moves::Move;
use crate::parse::StringIndexMap;
use crate::position::{CASTLING_DETAILS, Position};

use lazy_static::lazy_static;
use std::fmt::{Display, Formatter};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub enum FenPart {
    Position = 0,
    Active = 1,
    CastlingRights = 2,
    Enpassant = 3,
    HalfMoveCount = 4,
    MoveCount = 5,
}

const ALL_PARTS: [FenPart; 6] = [
    FenPart::Position,
    FenPart::Active,
    FenPart::CastlingRights,
    FenPart::Enpassant,
    FenPart::HalfMoveCount,
    FenPart::MoveCount,
];

impl Display for Position {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", to_fen_impl(self, ALL_PARTS.iter().cloned()))
    }
}

impl Display for Move {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        lazy_static! {
            static ref SQUARES: StringIndexMap = StringIndexMap::squares();
            static ref PIECES: StringIndexMap = StringIndexMap::uci_pieces();
        }
        write!(
            f,
            "{}",
            match self {
                Move::Null => "null".to_owned(),
                &Move::Normal { from, dest, .. } =>
                    format!("{}{}", SQUARES.format(from), SQUARES.format(dest)),
                &Move::Enpassant { from, dest, .. } =>
                    format!("{}{}", SQUARES.format(from), SQUARES.format(dest)),
                &Move::Castle { corner, .. } => {
                    let details = &CASTLING_DETAILS[corner];
                    let (from, dest) = details.king_line;
                    format!("{}{}", SQUARES.format(from), SQUARES.format(dest))
                }
                &Move::Promote { from, dest, promoted, .. } => {
                    let promote_class = PIECES.format(piece_class(promoted));
                    format!("{}{}{}", SQUARES.format(from), SQUARES.format(dest), promote_class)
                }
            }
        )
    }
}

pub fn to_fen_impl<I: Iterator<Item = FenPart>>(board: &Position, parts: I) -> String {
    let mut dest = String::new();
    for cmp in parts {
        let encoded_cmp = match cmp {
            FenPart::Position => to_fen_board(board),
            FenPart::Active => to_fen_side(board),
            FenPart::CastlingRights => to_fen_castling_rights(board),
            FenPart::Enpassant => to_fen_enpassant(board),
            FenPart::HalfMoveCount => board.clock.to_string(),
            FenPart::MoveCount => to_fen_move_count(board),
        };
        dest.push_str(encoded_cmp.as_str());
        dest.push(' ');
    }
    if !dest.is_empty() {
        dest.remove(dest.len() - 1);
    }
    dest
}

fn to_fen_board(board: &Position) -> String {
    let mut dest = String::new();
    let mut empty_count = 0;
    for i in 0..64 {
        match board.piece_locs[63 - i] {
            None => {
                empty_count += 1;
            }
            Some(piece) => {
                if empty_count > 0 {
                    dest.push_str(&empty_count.to_string());
                    empty_count = 0;
                }
                dest.push_str(PIECES[piece]);
            }
        }
        if i != 0 && ((i % 8) == 7) {
            if empty_count > 0 {
                dest.push_str(&empty_count.to_string());
                empty_count = 0;
            }
            dest.push('/');
        }
    }
    dest.remove(dest.len() - 1);
    dest
}

fn to_fen_side(board: &Position) -> String {
    if board.active == side::W { "w" } else { "b" }.to_string()
}

fn to_fen_castling_rights(board: &Position) -> String {
    let rights =
        (0..4).filter(|c| board.castling_rights[*c]).map(|c| CORNERS[c]).collect::<String>();
    if rights.is_empty() { format!("-") } else { rights }
}

fn to_fen_enpassant(board: &Position) -> String {
    lazy_static! {
        static ref SQUARES: StringIndexMap = StringIndexMap::squares();
    }
    match board.enpassant {
        None => format!("-"),
        Some(s) => SQUARES.format(s).to_string(),
    }
}

fn to_fen_move_count(board: &Position) -> String {
    (board.history.len() / 2 + 1).to_string()
}

const CORNERS: [&'static str; 4] = ["K", "Q", "k", "q"];
const PIECES: [&'static str; 12] = ["P", "N", "B", "R", "Q", "K", "p", "n", "b", "r", "q", "k"];

#[cfg(test)]
mod test {
    use super::to_fen_impl;
    use crate::format::FenPart;
    use crate::position::Position;
    use std::iter::once;

    const START_FEN: &'static str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

    #[test]
    fn start_position_board() {
        assert_eq!(
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR",
            to_fen_impl(&START_FEN.parse::<Position>().unwrap(), once(FenPart::Position))
        )
    }

    #[test]
    fn start_position_active() {
        assert_eq!("w", to_fen_impl(&START_FEN.parse::<Position>().unwrap(), once(FenPart::Active)))
    }

    #[test]
    fn start_position_castling_rights() {
        assert_eq!(
            "KQkq",
            to_fen_impl(&START_FEN.parse::<Position>().unwrap(), once(FenPart::CastlingRights))
        )
    }

    #[test]
    fn start_position_enpassant() {
        assert_eq!(
            "-",
            to_fen_impl(&START_FEN.parse::<Position>().unwrap(), once(FenPart::Enpassant))
        )
    }

    #[test]
    fn start_position_half_move_count() {
        assert_eq!(
            "0",
            to_fen_impl(&START_FEN.parse::<Position>().unwrap(), once(FenPart::HalfMoveCount))
        )
    }

    #[test]
    fn start_position_move_count() {
        assert_eq!(
            "1",
            to_fen_impl(&START_FEN.parse::<Position>().unwrap(), once(FenPart::MoveCount))
        )
    }

    #[test]
    fn start_position_all() {
        assert_eq!(START_FEN, START_FEN.parse::<Position>().unwrap().to_string());
    }

    fn position_1() -> Position {
        "1. e4 Nf6 2. Nf3 Rg8 3. Rg1 h6 4. e5 d5".parse().unwrap()
    }

    #[test]
    fn position_1_board() {
        assert_eq!(
            "rnbqkbr1/ppp1ppp1/5n1p/3pP3/8/5N2/PPPP1PPP/RNBQKBR1",
            to_fen_impl(&position_1(), once(FenPart::Position))
        )
    }

    #[test]
    fn position_1_active() {
        assert_eq!("w", to_fen_impl(&position_1(), once(FenPart::Active)))
    }

    #[test]
    fn position_1_castling_rights() {
        assert_eq!("Qq", to_fen_impl(&position_1(), once(FenPart::CastlingRights)))
    }

    #[test]
    fn position_1_enpassant() {
        assert_eq!("d6", to_fen_impl(&position_1(), once(FenPart::Enpassant)))
    }

    #[test]
    fn position_1_half_move_count() {
        assert_eq!("0", to_fen_impl(&position_1(), once(FenPart::HalfMoveCount)))
    }

    #[test]
    fn position_1_move_count() {
        assert_eq!("5", to_fen_impl(&position_1(), once(FenPart::MoveCount)))
    }

    #[test]
    fn position_1_all() {
        let expected = "rnbqkbr1/ppp1ppp1/5n1p/3pP3/8/5N2/PPPP1PPP/RNBQKBR1 w Qq d6 0 5";
        assert_eq!(expected, position_1().to_string());
    }

    fn position_2() -> Position {
        "1. e4 Nf6 2. Nf3 Rg8 3. Rg1 h6 4. e5 d5 5. Ke2 Kd7 6. Rh1".parse().unwrap()
    }

    #[test]
    fn position_2_board() {
        assert_eq!(
            "rnbq1br1/pppkppp1/5n1p/3pP3/8/5N2/PPPPKPPP/RNBQ1B1R",
            to_fen_impl(&position_2(), once(FenPart::Position))
        )
    }

    #[test]
    fn position_2_active() {
        assert_eq!("b", to_fen_impl(&position_2(), once(FenPart::Active)))
    }

    #[test]
    fn position_2_castling_rights() {
        assert_eq!("-", to_fen_impl(&position_2(), once(FenPart::CastlingRights)))
    }

    #[test]
    fn position_2_enpassant() {
        assert_eq!("-", to_fen_impl(&position_2(), once(FenPart::Enpassant)))
    }

    #[test]
    fn position_2_half_move_count() {
        assert_eq!("3", to_fen_impl(&position_2(), once(FenPart::HalfMoveCount)))
    }

    #[test]
    fn position_2_move_count() {
        assert_eq!("6", to_fen_impl(&position_2(), once(FenPart::MoveCount)))
    }

    #[test]
    fn position_2_all() {
        let expected = "rnbq1br1/pppkppp1/5n1p/3pP3/8/5N2/PPPPKPPP/RNBQ1B1R b - - 3 6";
        assert_eq!(expected, position_2().to_string());
    }
}
