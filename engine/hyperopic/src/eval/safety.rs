use crate::board::{control, iter};
use crate::constants::{class, create_piece, lift, reflect_side, side};
use crate::moves::Move;
use crate::node::{EvalFacet, Evaluation};
use crate::position::Position;
use crate::{Side, union_boards};
use std::cmp::min;

#[derive(Debug, Clone, PartialEq)]
struct SafetyCounts {
    total_control: usize,
    attacker_count: usize,
}

pub struct SafetyFacet {
    control_bonus: usize,
    piece_count_multipliers: [f64; 3],
    endgame_multiplier: f64,
}

impl Default for SafetyFacet {
    fn default() -> Self {
        SafetyFacet {
            control_bonus: 10,
            endgame_multiplier: 0.1,
            piece_count_multipliers: [1.0, 1.5, 3.0],
        }
    }
}

impl SafetyFacet {
    fn compute_king_danger(&self, pos: &Position, side: Side) -> i32 {
        self.compute_king_danger_value(&compute_safety_counts(pos, side))
    }

    fn compute_king_danger_value(&self, counts: &SafetyCounts) -> i32 {
        if counts.attacker_count == 0 {
            0
        } else {
            let mul = self.piece_count_multipliers;
            let mul_index = min(mul.len() - 1, counts.attacker_count - 1);
            ((counts.total_control * self.control_bonus) as f64 * mul[mul_index]).round() as i32
        }
    }
}

fn compute_safety_counts(pos: &Position, side: Side) -> SafetyCounts {
    let king = create_piece(side, class::K);
    let king_loc = pos.piece_boards[king].trailing_zeros() as usize;
    // If the king is off the board just skip the computation
    if king_loc == 64 {
        return SafetyCounts { total_control: 0, attacker_count: 0 };
    }
    let occupied = union_boards(&pos.side_boards) & !lift(king_loc);
    let safety_ring = control(king, king_loc, 0) & !occupied;
    let other_side = reflect_side(side);
    let mut total_control = 0usize;
    let mut attacker_count = 0usize;
    for class in [class::N, class::B, class::R, class::Q] {
        let p = create_piece(other_side, class);
        iter(pos.piece_boards[p]).for_each(|sq| {
            let control_count = (control(p, sq, occupied) & safety_ring).count_ones() as usize;
            total_control += control_count;
            attacker_count += min(1, control_count);
        });
    }
    SafetyCounts { total_control, attacker_count }
}

impl EvalFacet for SafetyFacet {
    fn static_eval(&self, board: &Position) -> Evaluation {
        let mid_eval =
            self.compute_king_danger(board, side::B) - self.compute_king_danger(board, side::W);
        Evaluation::Phased {
            mid: mid_eval,
            end: (mid_eval as f64 * self.endgame_multiplier).round() as i32,
        }
    }

    fn make(&mut self, _: &Move, _: &Position) {}

    fn unmake(&mut self, _: &Move) {}
}

#[cfg(test)]
mod test {
    use crate::constants::{reflect_side, side};
    use crate::eval::SafetyFacet;
    use crate::eval::safety::SafetyCounts;
    use crate::position::Position;
    use crate::{Side, Symmetric};

    fn test_facet() -> SafetyFacet {
        SafetyFacet {
            control_bonus: 10,
            piece_count_multipliers: [1.0, 2.1, 5.0],
            endgame_multiplier: 0.1,
        }
    }

    #[test]
    fn value_case_0() {
        let counts = SafetyCounts { total_control: 6, attacker_count: 1 };
        assert_eq!(test_facet().compute_king_danger_value(&counts), 60)
    }

    #[test]
    fn value_case_1() {
        let counts = SafetyCounts { total_control: 6, attacker_count: 2 };
        assert_eq!(test_facet().compute_king_danger_value(&counts), 126)
    }

    #[test]
    fn value_case_2() {
        let counts = SafetyCounts { total_control: 6, attacker_count: 5 };
        assert_eq!(test_facet().compute_king_danger_value(&counts), 300)
    }

    fn execute_test(position: Position, side: Side, expected: SafetyCounts) {
        assert_eq!(super::compute_safety_counts(&position, side), expected);
        assert_eq!(super::compute_safety_counts(&position.reflect(), reflect_side(side)), expected);
    }

    #[test]
    fn case_0() {
        execute_test(
            "4r1k1/2qbbp1p/2p2npB/2p1p3/r1PpP3/3P1N1P/P1N2PP1/R1Q2R1K b - - 1 20".parse().unwrap(),
            side::B,
            SafetyCounts { total_control: 2, attacker_count: 1 },
        )
    }

    #[test]
    fn case_1() {
        execute_test(
            "4r1k1/2qbbp1p/2p2QpB/2p1p3/r1PpP3/3P1N1P/P1N2PP1/R4R1K b - - 1 20".parse().unwrap(),
            side::B,
            SafetyCounts { total_control: 4, attacker_count: 2 },
        )
    }
}
