use crate::board::iter;
use crate::constants::{reflect_corner, reflect_piece, reflect_side, reflect_square};
use crate::moves::Move;
use crate::position::{ConstrainedPieces, Position};
use crate::{Board, Symmetric, constants};
use std::array;

mod best_move;
mod control;
pub(crate) mod facets;
mod make;
mod move_comparison;
mod moves;
mod pinned;
mod termination;

pub fn assert_boards_equal(expected: Board, actual: Board) {
    assert_eq!(expected, actual, "expected ^ actual {:#064b}", expected ^ actual)
}

pub fn reflect_board(board: Board) -> Board {
    iter(board).map(|sq| reflect_square(sq)).fold(0u64, |a, n| a | constants::lift(n))
}

impl Symmetric for Move {
    fn reflect(&self) -> Self {
        use crate::moves::Move::*;

        match self {
            Null => Null,
            Castle { corner } => Castle { corner: reflect_corner(*corner) },
            Normal { moving, from, dest, capture } => Normal {
                moving: reflect_piece(*moving),
                from: reflect_square(*from),
                dest: reflect_square(*dest),
                capture: capture.map(|p| reflect_piece(p)),
            },
            Enpassant { side, from, dest, capture } => Enpassant {
                side: reflect_side(*side),
                from: reflect_square(*from),
                dest: reflect_square(*dest),
                capture: reflect_square(*capture),
            },
            Promote { from, dest, promoted, capture } => Promote {
                from: reflect_square(*from),
                dest: reflect_square(*dest),
                promoted: reflect_piece(*promoted),
                capture: capture.map(|p| reflect_piece(p)),
            },
        }
    }
}

impl Symmetric for Position {
    fn reflect(&self) -> Self {
        let mut cloned = self.clone();
        let mut moves = vec![];
        while let Ok(m) = cloned.unmake() {
            moves.insert(0, m);
        }
        let mut reflected = Position::new(
            reflect_side(cloned.active),
            cloned.enpassant.map(|sq| reflect_square(sq)),
            cloned.clock,
            array::from_fn(|c| cloned.castling_rights[reflect_corner(c)]),
            array::from_fn(|sq| cloned.piece_locs[reflect_square(sq)].map(|p| reflect_piece(p))),
        );
        moves.into_iter().for_each(|m| reflected.make(m.reflect()).unwrap());
        reflected
    }
}

impl Symmetric for ConstrainedPieces {
    fn reflect(&self) -> Self {
        ConstrainedPieces(
            reflect_board(self.0),
            array::from_fn(|sq| reflect_board(self.1[reflect_square(sq)])),
        )
    }
}

mod symmetry_test {
    use crate::Symmetric;
    use crate::constants::piece;
    use crate::constants::square::*;
    use crate::moves::Move;
    use crate::position::Position;
    use Move::Normal;

    #[test]
    fn position_symmetry_1() {
        assert_eq!(
            "r2qkb1r/1p1b1pp1/p1nppn2/1B5p/3NPP2/2N4P/PPP3P1/R1BQ1RK1 w kq - 0 1"
                .parse::<Position>()
                .unwrap()
                .reflect(),
            "r1bq1rk1/ppp3p1/2n4p/3npp2/1b5P/P1NPPN2/1P1B1PP1/R2QKB1R b KQ - 0 1"
                .parse::<Position>()
                .unwrap()
        );
    }

    #[test]
    fn position_symmetry_2() {
        let mut start = "r2qkb1r/1p1b1pp1/p1nppn2/1B5p/3NPP2/2N4P/PPP3P1/R1BQ1RK1 w kq - 0 1"
            .parse::<Position>()
            .unwrap();
        start.make(Normal { from: G1, dest: H1, moving: piece::WK, capture: None }).unwrap();
        let mut reflected_start =
            "r1bq1rk1/ppp3p1/2n4p/3npp2/1b5P/P1NPPN2/1P1B1PP1/R2QKB1R b KQ - 0 1"
                .parse::<Position>()
                .unwrap();
        reflected_start
            .make(Normal { from: G8, dest: H8, moving: piece::BK, capture: None })
            .unwrap();
        assert_eq!(start.reflect(), reflected_start);
    }
}
