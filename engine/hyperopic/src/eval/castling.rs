use crate::Side;
use crate::SideMap;
use crate::constants::{corner_side, side};
use crate::moves::Move;
use crate::node::{EvalFacet, Evaluation};
use crate::position::Position;

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct CastlingFacet {
    castling_status: SideMap<bool>,
    penalty: i32,
}

impl Default for CastlingFacet {
    fn default() -> Self {
        CastlingFacet { castling_status: Default::default(), penalty: 70 }
    }
}

impl CastlingFacet {
    fn penalty(&self, side: Side, rights_left: usize) -> i32 {
        if self.castling_status[side] { 0 } else { (2i32 - rights_left as i32) * self.penalty }
    }
}

impl EvalFacet for CastlingFacet {
    fn static_eval(&self, board: &Position) -> Evaluation {
        let rights = board.castling_rights;

        Evaluation::Single(
            self.penalty(side::B, rights[2] as usize + rights[3] as usize)
                - self.penalty(side::W, rights[0] as usize + rights[1] as usize),
        )
    }

    fn make(&mut self, mv: &Move, _: &Position) {
        if let Move::Castle { corner, .. } = mv {
            self.castling_status[corner_side(*corner)] = true
        }
    }

    fn unmake(&mut self, mv: &Move) {
        if let Move::Castle { corner, .. } = mv {
            self.castling_status[corner_side(*corner)] = false
        }
    }
}

#[cfg(test)]
mod test {
    use crate::constants::side;
    use crate::eval::castling::CastlingFacet;
    use crate::test::facets::test_facet_evolution;

    #[test]
    fn evaluation_not_castled() {
        let under_test = CastlingFacet { penalty: 100, castling_status: [false, false] };

        assert_eq!(200, under_test.penalty(side::W, 0));
        assert_eq!(200, under_test.penalty(side::B, 0));

        assert_eq!(100, under_test.penalty(side::W, 1));
        assert_eq!(100, under_test.penalty(side::B, 1));

        assert_eq!(0, under_test.penalty(side::W, 2));
        assert_eq!(0, under_test.penalty(side::B, 2));
    }

    #[test]
    fn evolution_queenside() {
        test_facet_evolution(
            "1. d4 d5 2. Be3 Bf5 3. Nc3 Qd6 4. Qd2 Nc6 5. O-O-O O-O-O 6. g3 h6",
            vec![
                [false, false],
                [false, false],
                [false, false],
                [false, false],
                [false, false],
                [false, false],
                [false, false],
                [false, false],
                [true, false],
                [true, true],
                [true, true],
                [true, true],
            ]
            .into_iter()
            .map(|status| {
                let mut facet = CastlingFacet::default();
                facet.castling_status = status;
                facet
            })
            .collect(),
        )
    }

    #[test]
    fn evolution_kingside() {
        test_facet_evolution(
            "1. e4 e5 2. Be2 Be7 3. Nf3 Nf6 4. O-O O-O 5. c4 a5",
            vec![
                [false, false],
                [false, false],
                [false, false],
                [false, false],
                [false, false],
                [false, false],
                [true, false],
                [true, true],
                [true, true],
                [true, true],
            ]
            .into_iter()
            .map(|status| {
                let mut facet = CastlingFacet::default();
                facet.castling_status = status;
                facet
            })
            .collect(),
        )
    }
}
