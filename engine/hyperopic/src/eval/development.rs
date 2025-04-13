use std::cmp::min;

use crate::constants::side;
use crate::constants::square::*;
use crate::moves::Move;
use crate::node::{EvalFacet, Evaluation};
use crate::position::Position;
use crate::{Side, SquareMap};
use crate::{SideMap, square_map};
use lazy_static::lazy_static;
use rustc_hash::FxHashSet;

type DevPiece = usize;
type DevPieceMap<T> = [T; 6];

mod dev_piece {
    use super::DevPiece;

    pub const EP: DevPiece = 0;
    pub const DP: DevPiece = 1;
    pub const BN: DevPiece = 2;
    pub const GN: DevPiece = 3;
    pub const CB: DevPiece = 4;
    pub const FB: DevPiece = 5;
}

lazy_static! {
    static ref START_LOCS: SquareMap<Option<(Side, DevPiece)>> = square_map! {
        E2 => Some((side::W, dev_piece::EP)),
        E7 => Some((side::B, dev_piece::EP)),
        D2 => Some((side::W, dev_piece::DP)),
        D7 => Some((side::B, dev_piece::DP)),
        B1 => Some((side::W, dev_piece::BN)),
        B8 => Some((side::B, dev_piece::BN)),
        G1 => Some((side::W, dev_piece::GN)),
        G8 => Some((side::B, dev_piece::GN)),
        C1 => Some((side::W, dev_piece::CB)),
        C8 => Some((side::B, dev_piece::CB)),
        F1 => Some((side::W, dev_piece::FB)),
        F8 => Some((side::B, dev_piece::FB))
    };
}

const MAX_PENALTY: i32 = 300;
type PiecesMoved = SideMap<DevPieceMap<Option<usize>>>;

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct DevelopmentFacet {
    move_index: usize,
    pieces_moved: PiecesMoved,
    undeveloped_cost: usize,
    move_index_divisor: usize,
    max_penalty: i32,
    dev_indices: FxHashSet<usize>,
}

impl Default for DevelopmentFacet {
    fn default() -> Self {
        DevelopmentFacet {
            move_index: 0,
            pieces_moved: Default::default(),
            undeveloped_cost: 10,
            move_index_divisor: 10,
            max_penalty: MAX_PENALTY,
            dev_indices: FxHashSet::default(),
        }
    }
}

impl DevelopmentFacet {
    fn matching_piece(&self, move_count: usize) -> Option<(Side, DevPiece)> {
        if !self.dev_indices.contains(&move_count) {
            return None;
        }
        self.pieces_moved
            .iter()
            .enumerate()
            .flat_map(|(side, ds)| {
                ds.iter()
                    .enumerate()
                    .filter(|&(_, &mv)| mv == Some(move_count))
                    .map(move |(d, _)| (side, d))
            })
            .next()
    }

    fn penalty(&self, side: Side) -> i32 {
        let undeveloped_count =
            self.pieces_moved[side].iter().filter(|&moved_index| moved_index.is_none()).count()
                as f64;

        let move_index_mult = (self.move_index as f64 / self.move_index_divisor as f64).exp2();
        min(
            (move_index_mult * undeveloped_count * (self.undeveloped_cost as f64)).round() as i32,
            self.max_penalty,
        )
    }
}

impl EvalFacet for DevelopmentFacet {
    fn static_eval(&self, _: &Position) -> Evaluation {
        Evaluation::Single(self.penalty(side::B) - self.penalty(side::W))
    }

    fn make(&mut self, mv: &Move, _: &Position) {
        if let &Move::Normal { from, .. } = mv {
            if let Some((side, piece)) = START_LOCS[from] {
                // Don't overwrite an existing entry as the piece was already moved
                if self.pieces_moved[side][piece].is_none() {
                    self.pieces_moved[side][piece] = Some(self.move_index);
                    self.dev_indices.insert(self.move_index);
                }
            }
        }
        self.move_index += 1;
    }

    fn unmake(&mut self, _: &Move) {
        self.move_index -= 1;
        if let Some((side, piece)) = self.matching_piece(self.move_index) {
            self.pieces_moved[side][piece] = None;
            self.dev_indices.remove(&self.move_index);
        }
    }
}

#[cfg(test)]
mod test {
    use rustc_hash::FxHashSet;

    use crate::constants::side;
    use crate::eval::development::DevelopmentFacet;
    use crate::node::{EvalFacet, Evaluation};
    use crate::position::Position;
    use crate::test::facets::test_facet_evolution;

    #[test]
    fn penalty_test() {
        let under_test = DevelopmentFacet {
            move_index: 10,
            undeveloped_cost: 3,
            move_index_divisor: 2,
            max_penalty: 10000,
            dev_indices: [0, 2, 4, 1, 3].into_iter().collect::<FxHashSet<_>>(),
            pieces_moved: [
                [Some(0), None, None, Some(2), None, Some(4)],
                [Some(1), None, Some(3), None, None, None],
            ],
        };

        assert_eq!(3 * 3 * 32, under_test.penalty(side::W));
        assert_eq!(4 * 3 * 32, under_test.penalty(side::B));
        assert_eq!(Evaluation::Single(1 * 3 * 32), under_test.static_eval(&Position::default()));
    }

    #[test]
    fn evolution() {
        test_facet_evolution(
            "1. e4 e5 2. Nf3 Nc6 3. Bb5 Nf6 4. Bxc6 bxc6 5. d4 exd4 6. Nxd4 Bc5 7. Be3 Bb7 8. Nc3 d6",
            vec![
                [[Some(0), None, None, None, None, None], [None; 6]],
                [[Some(0), None, None, None, None, None], [Some(1), None, None, None, None, None]],
                [
                    [Some(0), None, None, Some(2), None, None],
                    [Some(1), None, None, None, None, None],
                ],
                [
                    [Some(0), None, None, Some(2), None, None],
                    [Some(1), None, Some(3), None, None, None],
                ],
                [
                    [Some(0), None, None, Some(2), None, Some(4)],
                    [Some(1), None, Some(3), None, None, None],
                ],
                [
                    [Some(0), None, None, Some(2), None, Some(4)],
                    [Some(1), None, Some(3), Some(5), None, None],
                ],
                [
                    [Some(0), None, None, Some(2), None, Some(4)],
                    [Some(1), None, Some(3), Some(5), None, None],
                ],
                [
                    [Some(0), None, None, Some(2), None, Some(4)],
                    [Some(1), None, Some(3), Some(5), None, None],
                ],
                [
                    [Some(0), Some(8), None, Some(2), None, Some(4)],
                    [Some(1), None, Some(3), Some(5), None, None],
                ],
                [
                    [Some(0), Some(8), None, Some(2), None, Some(4)],
                    [Some(1), None, Some(3), Some(5), None, None],
                ],
                [
                    [Some(0), Some(8), None, Some(2), None, Some(4)],
                    [Some(1), None, Some(3), Some(5), None, None],
                ],
                [
                    [Some(0), Some(8), None, Some(2), None, Some(4)],
                    [Some(1), None, Some(3), Some(5), None, Some(11)],
                ],
                [
                    [Some(0), Some(8), None, Some(2), Some(12), Some(4)],
                    [Some(1), None, Some(3), Some(5), None, Some(11)],
                ],
                [
                    [Some(0), Some(8), None, Some(2), Some(12), Some(4)],
                    [Some(1), None, Some(3), Some(5), Some(13), Some(11)],
                ],
                [
                    [Some(0), Some(8), Some(14), Some(2), Some(12), Some(4)],
                    [Some(1), None, Some(3), Some(5), Some(13), Some(11)],
                ],
                [
                    [Some(0), Some(8), Some(14), Some(2), Some(12), Some(4)],
                    [Some(1), Some(15), Some(3), Some(5), Some(13), Some(11)],
                ],
            ]
            .into_iter()
            .enumerate()
            .map(|(i, pieces)| {
                let mut facet = DevelopmentFacet::default();
                facet.move_index = i + 1;
                facet.pieces_moved = pieces.clone();
                facet.dev_indices =
                    pieces.iter().flat_map(|a| a.iter()).filter_map(|i| *i).collect();
                facet
            })
            .collect(),
        )
    }
}
