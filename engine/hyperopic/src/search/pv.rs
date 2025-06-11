use crate::moves::Move;
use std::cmp::min;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PrincipleVariation {
    pub path: Vec<Move>,
}

impl PrincipleVariation {
    pub fn get_next_move(&self, curr_depth: usize) -> Option<Move> {
        // If pv depth is n then we would map
        // n+1 -> 0
        // n   -> 1
        // ..
        // 1   -> n-1
        self.path.get((1 + self.path.len()) - curr_depth).cloned()
    }

    pub fn is_next_on_pv(&self, curr_depth: u8, candidate: &Move) -> bool {
        self.get_next_move(curr_depth as usize).is_some_and(|pvm| &pvm == candidate)
    }
}

#[cfg(test)]
mod test {
    use crate::constants::piece;
    use crate::constants::square::{E2, E4, E5, E7, F1, G3};
    use crate::moves::Move::Normal;
    use crate::search::pv::PrincipleVariation;
    
    fn create_test_pv() -> PrincipleVariation {
        PrincipleVariation {
            path: vec![
                Normal { moving: piece::WP, from: E2, dest: E4, capture: None },
                Normal { moving: piece::BP, from: E5, dest: E7, capture: None },
                Normal { moving: piece::WN, from: F1, dest: G3, capture: None },
            ]
        }
    }

    #[test]
    fn is_next_on_pv() {
        let pv = create_test_pv();
        assert!(pv.is_next_on_pv(4, &Normal { moving: piece::WP, from: E2, dest: E4, capture: None }));
    }

    #[test]
    fn get_next_move() {
        let pv = create_test_pv();

        assert_eq!(Some(Normal { moving: piece::WP, from: E2, dest: E4, capture: None }), pv.get_next_move(4));
        assert_eq!(Some(Normal { moving: piece::BP, from: E5, dest: E7, capture: None }), pv.get_next_move(3));
        assert_eq!(Some(Normal { moving: piece::WN, from: F1, dest: G3, capture: None }), pv.get_next_move(2));
        assert_eq!(None, pv.get_next_move(1));
    }
}
