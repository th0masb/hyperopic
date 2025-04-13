use crate::constants::boards::RIM;
use crate::constants::in_board;
use crate::constants::{side, square::*};
use crate::moves::Move;
use crate::node::{EvalFacet, Evaluation};
use crate::position::Position;
use crate::{Side, SideMap, Square, SquareMap, square_map};
use lazy_static::lazy_static;

type Knight = usize;
type KnightMap<T> = [T; 2];

mod knight {
    use super::Knight;

    pub const B: Knight = 0;
    pub const G: Knight = 1;
}

type FirstMoveStore = SideMap<KnightMap<Option<(usize, Square)>>>;

/// Give penalty for each knight whose first move is onto the board rim
#[derive(Debug, Clone, PartialEq)]
pub struct KnightRimFacet {
    penalty: i32,
    first_move: FirstMoveStore,
    move_index: usize,
}

impl Default for KnightRimFacet {
    fn default() -> Self {
        KnightRimFacet { penalty: 80, first_move: Default::default(), move_index: 0 }
    }
}

impl KnightRimFacet {
    fn pattern_count(&self, side: Side) -> i32 {
        self.first_move[side]
            .iter()
            .filter(|&fm| fm.is_some() && in_board(RIM, fm.unwrap().1))
            .count() as i32
    }
}

lazy_static! {
    static ref START_LOCS: SquareMap<Option<(Side, Knight)>> = square_map! {
        B1 => Some((side::W, knight::B)),
        B8 => Some((side::B, knight::B)),
        G1 => Some((side::W, knight::G)),
        G8 => Some((side::B, knight::G))
    };
}

impl EvalFacet for KnightRimFacet {
    fn static_eval(&self, _: &Position) -> Evaluation {
        Evaluation::Single(
            self.penalty * (self.pattern_count(side::B) - self.pattern_count(side::W)),
        )
    }

    fn make(&mut self, mv: &Move, _: &Position) {
        if let Move::Normal { from, dest, .. } = mv {
            if let Some((side, knight)) = START_LOCS[*from] {
                if self.first_move[side][knight].is_none() {
                    self.first_move[side][knight] = Some((self.move_index, *dest))
                }
            }
        }
        self.move_index += 1
    }

    fn unmake(&mut self, mv: &Move) {
        self.move_index -= 1;
        if let Move::Normal { from, .. } = mv {
            if let Some((side, knight)) = START_LOCS[*from] {
                if let Some((index, _)) = self.first_move[side][knight] {
                    if index == self.move_index {
                        self.first_move[side][knight] = None
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod test {

    use crate::constants::square::*;
    use crate::eval::knightrim::KnightRimFacet;
    use crate::node::{EvalFacet, Evaluation};
    use crate::position::Position;
    use crate::test::facets::test_facet_evolution;

    #[test]
    fn evaluation() {
        let facet = KnightRimFacet {
            penalty: 75,
            move_index: 9,
            first_move: [[Some((3, C3)), Some((3, H3))], [Some((3, C6)), None]],
        };

        assert_eq!(Evaluation::Single(-75), facet.static_eval(&Position::default()));
    }

    #[test]
    fn evolution() {
        test_facet_evolution(
            "1. e4 Nc6 2. d4 d5 3. Nf3 Nh6 4. Na3 f6",
            vec![
                [[None; 2], [None; 2]],
                [[None; 2], [Some((1, C6)), None]],
                [[None; 2], [Some((1, C6)), None]],
                [[None; 2], [Some((1, C6)), None]],
                [[None, Some((4, F3))], [Some((1, C6)), None]],
                [[None, Some((4, F3))], [Some((1, C6)), Some((5, H6))]],
                [[Some((6, A3)), Some((4, F3))], [Some((1, C6)), Some((5, H6))]],
                [[Some((6, A3)), Some((4, F3))], [Some((1, C6)), Some((5, H6))]],
            ]
            .into_iter()
            .enumerate()
            .map(|(i, first_move)| {
                let mut facet = KnightRimFacet::default();
                facet.first_move = first_move;
                facet.move_index = i + 1;
                facet
            })
            .collect(),
        )
    }
}
